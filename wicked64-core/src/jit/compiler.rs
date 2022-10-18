use std::cell::RefCell;
use std::rc::Rc;

use iced_x86::code_asm::{self, AsmRegister64, CodeAssembler};

use crate::cpu::instruction::{ImmediateType, Instruction, JumpType};
use crate::jit::{
    bridge,
    register::{GuestRegister, Registers, ARGS_REGS, CALLEE_SAVED_REGISTERS},
};
use crate::n64::State;

use super::state::JitState;

const SCRATCHY_REGISTERS: [AsmRegister64; 2] = [code_asm::r14, code_asm::r15];

#[derive(Debug, PartialEq, Eq)]
enum AssembleStatus {
    Continue,
    InvalidateCache,
    Branch,
}

#[derive(thiserror::Error, Debug)]
enum AssembleError {
    #[error(transparent)]
    Asm(#[from] iced_x86::IcedError),
    #[error(transparent)]
    Memory(#[from] region::Error),
    // TODO: implement an error enum for CPU errors
    #[error("Error interacting with the CPU")]
    Cpu,
}

type AssembleResult<T> = Result<T, AssembleError>;

enum CallArgument {
    Register(AsmRegister64),
    Value(u64),
}

macro_rules! arg_list {
    (reg => $arg:expr) => {
        CallArgument::Register($arg)
    };
    (val => $arg:expr) => {
        CallArgument::Value($arg)
    };
    ($($kind:tt : $arg:expr),*) => {
        &[$(arg_list!($kind => $arg)),*]
    }
}

macro_rules! cast_arg {
    ($_arg:tt) => {
        _
    };
}

macro_rules! wrap_call {
    ($compiler:ident, $function:path[$($kind:ident: $arg:expr),*]) => {{
        $compiler.wrap_call(arg_list!($($kind : $arg),*), |emitter| {
            let function_ptr = $function as extern "C" fn($(cast_arg!($arg),)*) -> _ as usize;
            emitter.mov(code_asm::rax, function_ptr as u64)?;

            // align the stack before calling the function
            emitter.push(code_asm::rbx)?;
            emitter.mov(code_asm::bl, code_asm::spl)?;
            emitter.and(code_asm::rsp, -16)?;

            emitter.call(code_asm::rax)?;

            // restore the stack
            emitter.mov(code_asm::spl, code_asm::bl)?;
            emitter.pop(code_asm::rbx)?;

            Ok(())
        })
    }};
}

#[derive(Clone)]
pub struct ExecBuffer {
    ptr: *const u8,
    buf: Vec<u8>,
    state: Rc<RefCell<State>>,
}

impl ExecBuffer {
    unsafe fn new(buffer: Vec<u8>, state: Rc<RefCell<State>>) -> region::Result<Self> {
        let ptr = buffer.as_ptr();

        region::protect(ptr, buffer.len(), region::Protection::READ_WRITE_EXECUTE)?;

        Ok(Self {
            buf: buffer,
            ptr,
            state,
        })
    }

    pub fn execute(&self) {
        let _state = self.state.borrow_mut();
        unsafe {
            let f: unsafe extern "C" fn() = std::mem::transmute(self.ptr);
            f();
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.buf.as_slice()
    }
}

fn assemble_code(
    mut emitter: CodeAssembler,
    state: Rc<RefCell<State>>,
) -> Result<ExecBuffer, AssembleError> {
    let code = emitter.assemble(0)?;
    let map = unsafe { ExecBuffer::new(code, state)? };
    Ok(map)
}

/// The JIT compiler
pub struct Compiler {
    state: JitState,
    pc: u64,
    phys_pc: u64,
    regs: Registers,
    emitter: CodeAssembler,
    saved_regs: Vec<AsmRegister64>,
}

impl Compiler {
    /// Create a new Jit compiler
    /// # Panics
    /// Panics if the cpu architecture is not 64-bit
    pub fn new(state: Rc<RefCell<State>>) -> Self {
        let (pc, phys_pc) = {
            let cpu = &state.borrow().cpu;
            (cpu.pc, cpu.translate_virtual(cpu.pc as usize) as u64)
        };
        let mut regs = Registers::new();

        for reg in SCRATCHY_REGISTERS {
            regs.exclude_register(reg);
        }

        Self {
            pc,
            phys_pc,
            regs,
            state: JitState::new(state),
            emitter: CodeAssembler::new(64).unwrap(),
            saved_regs: Vec::new(),
        }
    }

    /// Compile the code
    /// # Panics
    /// Panics if the generated assembly code is invalid
    pub fn compile(mut self, cycles: usize) -> (ExecBuffer, usize) {
        tracing::debug!("Generating compiled block");

        let initial_pc = self.state.borrow().cpu.pc;

        let compiled_cycles = self.compile_block(cycles).unwrap();

        let compiled = match assemble_code(self.emitter, self.state.into_inner()) {
            Ok(compiled) => compiled,
            Err(error) => panic!("Could not compile the code properly: {error:?}"),
        };

        tracing::info!(
            "{compiled_cycles} PClocks block compiled from pc: ({:08x}..{:08x})",
            initial_pc,
            self.pc
        );

        // we can ensure that `len >= 0`, as we stop the compilation whenever an instruction changes the pc to
        // an arbitrary value (i.e: a branch instruction)
        let len = (self.pc - initial_pc) as usize;

        (compiled, len)
    }

    fn compile_block(&mut self, cycles: usize) -> AssembleResult<usize> {
        let state_addr = self.state.state_ptr() as *mut State as u64;

        self.save_register(code_asm::rsi);
        self.save_register(code_asm::rdi);
        self.emitter.mov(code_asm::rsi, state_addr)?;

        for reg in SCRATCHY_REGISTERS {
            self.save_register(reg);
        }

        let mut total_cycles = 0;
        while total_cycles < cycles {
            // fetch the next instruction and update the PC and cycles
            let instruction = {
                let state = self.state.borrow();
                let instruction = state
                    .cpu
                    .fetch_instruction(&state.mmu, self.pc)
                    .map_err(|_| AssembleError::Cpu)?;

                self.pc += 4;
                self.phys_pc += 4;
                total_cycles += instruction.cycles();

                instruction
            };

            // early return
            match self.compile_instruction(instruction).unwrap() {
                AssembleStatus::Continue => {}
                AssembleStatus::InvalidateCache => {
                    let guest_pc_offset = self.state.offset_of(|state| &state.cpu.pc) as i32;
                    self.emitter.mov(code_asm::r14, self.pc)?;
                    self.emitter.mov(
                        code_asm::ptr(code_asm::rsi + guest_pc_offset),
                        code_asm::r14,
                    )?;
                    break;
                }
                AssembleStatus::Branch => break,
            }
        }

        self.sync_all_registers().unwrap();

        // restore callee saved registers and return
        self.restore_registers();
        self.emitter.ret().unwrap();

        Ok(total_cycles)
    }

    /// Compiles the given instruction and save the generated code into `buf`
    fn compile_instruction(&mut self, instruction: Instruction) -> AssembleResult<AssembleStatus> {
        tracing::debug!("Compiling {instruction:02x?}");
        match instruction {
            Instruction::LUI(ImmediateType { rt, immediate, .. }) => {
                // rt = immediate << 16
                let host_rt = self.get_cpu_register(rt);
                let imm = (immediate as u64) << 16;
                self.emitter.mov(host_rt, imm)?;

                Ok(AssembleStatus::Continue)
            }
            Instruction::ORI(ImmediateType {
                rs, rt, immediate, ..
            }) => {
                // rt = rs | immediate
                let host_rt = self.get_cpu_register(rt);
                let host_rs = self.get_cpu_register(rs);

                self.emitter.mov(host_rt, immediate as u64)?;
                self.emitter.or(host_rt, host_rs)?;

                Ok(AssembleStatus::Continue)
            }
            Instruction::SW(ImmediateType {
                rs,
                rt,
                immediate: offset,
                ..
            }) => {
                // mmu[rs + offset] = rt
                {
                    let rs = self.get_cpu_register(rs);
                    let rt = self.get_cpu_register(rt);

                    let state_addr = self.state.state_ptr();

                    self.emitter.mov(code_asm::r14, offset as u16 as u64)?;
                    self.emitter.add(code_asm::r14, rs)?;

                    wrap_call!(self, bridge::mmu_store[val: state_addr as u64, reg: code_asm::r14, reg: rt])?;
                }

                // `mmu_store` might invalidate the current memory region
                Ok(AssembleStatus::InvalidateCache)
            }
            Instruction::JAL(JumpType { target, .. }) => {
                // r31 = pc + 8
                // pc = (pc & 0xf000_0000) | (target << 2)
                let guest_pc = self.get_cpu_pc();
                let r31 = self.get_cpu_register(31);
                self.emitter.mov(code_asm::r14, guest_pc)?;
                self.emitter.mov(code_asm::r15d, target)?;

                self.emitter.add(guest_pc, 8)?;
                self.emitter.mov(r31, guest_pc)?;

                self.emitter.shl(code_asm::r15d, 2)?;
                self.emitter.and(code_asm::r14d, 0xf000_0000u32 as i32)?;
                self.emitter.or(code_asm::r14d, code_asm::r15d)?;

                self.emitter.mov(guest_pc, code_asm::r14)?;

                Ok(AssembleStatus::Branch)
            }
            _ => todo!("Implement the rest of the instructions"),
        }
    }

    /// A wrapper that saves and syncs all registers before calling a `call` instruction.
    fn wrap_call<F>(&mut self, args: &[CallArgument], call_fn: F) -> AssembleResult<()>
    where
        F: FnOnce(&mut CodeAssembler) -> AssembleResult<()>,
    {
        assert!(
            args.len() <= ARGS_REGS.len(),
            "Argument list exceeds the limit of {} arguments.",
            ARGS_REGS.len() - 1
        );
        self.emitter.push(code_asm::rsi)?;

        self.sync_all_registers()?;

        let reg_args = args
            .iter()
            .filter_map(|arg| {
                if let CallArgument::Register(reg_arg) = arg {
                    Some(reg_arg)
                } else {
                    None
                }
            })
            .collect::<Vec<&AsmRegister64>>();

        let ptr_size = std::mem::size_of::<usize>();
        let stack_size = reg_args.len() * ptr_size;
        let mut stack_index = stack_size;

        let mut save_registers: [Option<usize>; 16] = [None; 16];

        macro_rules! save_reg {
            ($reg:expr) => {{
                let reg: AsmRegister64 = $reg;
                let reg_num = iced_x86::Register::from(reg).number();
                if save_registers[reg_num].is_none()
                    && reg_args.iter().copied().find(|&&r| r == reg).is_some()
                {
                    stack_index -= ptr_size;
                    save_registers[reg_num] = Some(stack_index);
                    self.emitter
                        .mov(code_asm::ptr(code_asm::rsp + stack_index), reg)?;
                }
            }};
        }

        // set the stack size
        if stack_size > 0 {
            self.emitter.push(code_asm::rbp)?;
            self.emitter.mov(code_asm::rbp, code_asm::rsp)?;
            self.emitter.add_instruction(iced_x86::Instruction::with2(
                iced_x86::Code::Sub_rm64_imm32,
                iced_x86::Register::RSP,
                stack_size as u32,
            )?)?;
        };
        for (&dst, src) in ARGS_REGS.iter().zip(args.iter()) {
            match *src {
                CallArgument::Register(src) => {
                    save_reg!(dst);
                    if let Some(index) = save_registers[iced_x86::Register::from(src).number()] {
                        self.emitter
                            .mov(dst, code_asm::ptr(code_asm::rsp + index))?;
                    } else {
                        self.emitter.mov(dst, src)?;
                    }
                }
                CallArgument::Value(src) => {
                    if !reg_args.is_empty() {
                        save_reg!(dst);
                    }
                    self.emitter.mov(dst, src)?;
                }
            }
        }
        // restore the stack
        if stack_size > 0 {
            self.emitter.add_instruction(iced_x86::Instruction::with2(
                iced_x86::Code::Add_rm64_imm32,
                iced_x86::Register::RSP,
                stack_size as u32,
            )?)?;
            self.emitter.pop(code_asm::rbp)?;
        }

        call_fn(&mut self.emitter)?;

        self.emitter.pop(code_asm::rsi)?;

        Ok(())
    }

    #[must_use]
    fn get_cpu_register(&mut self, register: u8) -> AsmRegister64 {
        self.get_host_register(GuestRegister::cpu(register), |emitter, state, host_reg| {
            // load the register value
            let reg_offset = state.offset_of(|state| &state.cpu.gpr[register as usize]);
            emitter
                .mov(host_reg, code_asm::ptr(code_asm::rsi + reg_offset))
                .unwrap();
        })
    }

    #[must_use]
    fn get_cpu_pc(&mut self) -> AsmRegister64 {
        self.get_host_register(GuestRegister::pc(), |emitter, state, host_reg| {
            // load the register value
            let reg_offset = state.offset_of(|state| &state.cpu.pc);
            emitter
                .mov(host_reg, code_asm::ptr(code_asm::rsi + reg_offset))
                .unwrap();
        })
    }

    /// Gets a host register from the given guest register
    #[must_use]
    fn get_host_register(
        &mut self,
        guest_reg: GuestRegister,
        initialize_with: impl FnOnce(&mut CodeAssembler, &JitState, AsmRegister64),
    ) -> AsmRegister64 {
        if let Some(reg) = self.regs.get(guest_reg) {
            *reg
        } else {
            let (&reg, dropped) = self.regs.insert(guest_reg).unwrap();

            if CALLEE_SAVED_REGISTERS.contains(&reg) && !self.saved_regs.contains(&reg) {
                self.emitter.push(reg).unwrap();
                self.saved_regs.push(reg);
            }

            tracing::debug!(
                "Allocated {:?} for {guest_reg:?}",
                iced_x86::Register::from(reg)
            );

            if let Some(dropped) = dropped {
                self.sync_guest_with(dropped, reg).unwrap();
            };

            initialize_with(&mut self.emitter, &self.state, reg);
            reg
        }
    }

    /// Sync all registers and return the temporary registers
    #[allow(unreachable_patterns)]
    fn sync_all_registers(&mut self) -> AssembleResult<()> {
        for (guest, host) in self
            .regs
            .clone()
            .iter()
            .filter_map(|(guest, host)| match guest {
                GuestRegister::Cpu(_) | GuestRegister::Pc => Some((*guest, *host)),
                _ => None,
            })
        {
            self.sync_guest_with(guest, host)?;
        }
        Ok(())
    }

    fn sync_guest_with(
        &mut self,
        guest_reg: GuestRegister,
        host_reg: AsmRegister64,
    ) -> AssembleResult<()> {
        let guest_offset = {
            i32::try_from(match guest_reg {
                GuestRegister::Cpu(id) => self.state.offset_of(|state| &state.cpu.gpr[id as usize]),
                GuestRegister::Pc => self.state.offset_of(|state| &state.cpu.pc),
            })
        }
        .unwrap();

        self.emitter
            .mov(code_asm::ptr(code_asm::rsi + guest_offset), host_reg)?;

        self.regs.free(guest_reg);

        Ok(())
    }

    fn save_register(&mut self, reg: AsmRegister64) {
        self.saved_regs.push(reg);
        self.emitter.push(reg).unwrap();
    }
    fn restore_registers(&mut self) {
        for reg in self.saved_regs.drain(..).rev() {
            self.emitter.pop(reg).unwrap();
        }
    }
}
