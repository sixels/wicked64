use std::cell::RefCell;
use std::rc::Rc;

use iced_x86::code_asm::{self, AsmRegister64, CodeAssembler};

use crate::cpu::instruction::{ImmediateType, Instruction, JumpType};
use crate::jit::{
    bridge,
    register::{GuestRegister, Registers, ARGS_REGS, CALLEE_SAVED_REGISTERS},
};
use crate::n64::State;

use super::code::ExecBuffer;
use super::interruption::Interruption;
use super::jump_table::JumpTable;
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

fn assemble_code(
    mut emitter: CodeAssembler,
    state: Rc<RefCell<State>>,
) -> Result<ExecBuffer, AssembleError> {
    let code = emitter.assemble(0)?;
    let map = unsafe { ExecBuffer::new(code, state)? };
    Ok(map)
}

/// The JIT compiler
pub struct Compiler<'jt> {
    state: JitState,
    pc: u64,
    regs: Registers,
    emitter: CodeAssembler,
    saved_regs: Vec<AsmRegister64>,
    jump_table: &'jt mut JumpTable,
}

impl<'jt> Compiler<'jt> {
    /// Create a new Jit compiler
    /// # Panics
    /// Panics if the cpu architecture is not 64-bit
    pub fn new(state: Rc<RefCell<State>>, jump_table: &'jt mut JumpTable, addr: usize) -> Self {
        let mut regs = Registers::new();

        for reg in SCRATCHY_REGISTERS {
            regs.exclude_register(reg);
        }
        regs.exclude_register(code_asm::r13);

        Self {
            pc: addr as u64,
            regs,
            state: JitState::new(state),
            emitter: CodeAssembler::new(64).unwrap(),
            saved_regs: Vec::new(),
            jump_table,
        }
    }

    /// Compile the code
    /// # Panics
    /// Panics if the generated assembly code is invalid
    pub fn compile(mut self, cycles: usize) -> (ExecBuffer, usize) {
        tracing::debug!("Generating compiled block");

        let initial_pc = self.pc;
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
                AssembleStatus::Branch => {
                    return Ok(total_cycles);
                }
            }
        }

        self.sync_all_registers()?;
        self.restore_registers()?;

        self.emitter.jmp(code_asm::r13)?;

        Ok(total_cycles)
    }

    /// Compiles the given instruction and save the generated code into `buf`
    fn compile_instruction(&mut self, instruction: Instruction) -> AssembleResult<AssembleStatus> {
        tracing::debug!("Compiling {instruction:02x?}");
        match instruction {
            Instruction::LUI(ImmediateType { rt, immediate, .. }) => {
                // rt = immediate << 16
                let host_rt = self.get_cpu_register(rt)?;
                let imm = (immediate as u64) << 16;
                self.emitter.mov(host_rt, imm)?;

                Ok(AssembleStatus::Continue)
            }
            Instruction::ORI(ImmediateType {
                rs, rt, immediate, ..
            }) => {
                // rt = rs | immediate
                let host_rt = self.get_cpu_register(rt)?;
                let host_rs = self.get_cpu_register(rs)?;

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
                    let rs = self.get_cpu_register(rs)?;
                    let rt = self.get_cpu_register(rt)?;

                    let state_addr = self.state.state_ptr();

                    self.emitter.mov(code_asm::r14, offset as u16 as u64)?;
                    self.emitter.add(code_asm::r14, rs)?;

                    wrap_call!(self, bridge::mmu_store[val: state_addr as u64, reg: code_asm::r14, reg: rt])?;
                }

                // `mmu_store` might invalidate the current memory region
                Ok(AssembleStatus::InvalidateCache)
            }
            Instruction::JAL(JumpType { target, .. }) => {
                extern "C" fn get_host_jump_addr(
                    jump_table: &mut JumpTable,
                    n64_addr: u32,
                ) -> &usize {
                    tracing::debug!("Getting jump location for n64 addr {n64_addr:08x}");
                    &jump_table.get(n64_addr as usize).jump_to
                }

                // r31 = pc + 8
                // pc = (pc & 0xf000_0000) | (target << 2)
                let guest_pc = self.get_cpu_pc()?;
                let r31 = self.get_cpu_register(31)?;
                self.emitter.mov(code_asm::r14, guest_pc)?;
                self.emitter.mov(code_asm::r15d, target)?;

                self.emitter.add(guest_pc, 8)?;
                self.emitter.mov(r31, guest_pc)?;

                self.emitter.shl(code_asm::r15d, 2)?;
                self.emitter.and(code_asm::r14d, 0xf000_0000u32 as i32)?;
                self.emitter.or(code_asm::r14d, code_asm::r15d)?;

                let jump_table_addr = self.jump_table as *mut _ as u64;
                wrap_call!(
                self,
                get_host_jump_addr[
                    val: jump_table_addr,
                    reg: code_asm::r14
                    ]
                )?;

                self.emitter.mov(code_asm::r15, code_asm::r14)?;
                self.emit_interruption(Interruption::PrepareJump(0), Some(code_asm::r15))?;

                // resume passes the jump address in r15
                self.emitter.jmp(code_asm::qword_ptr(code_asm::r15))?;

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
        }

        call_fn(&mut self.emitter)?;

        self.emitter.pop(code_asm::rsi)?;

        Ok(())
    }

    fn emit_interruption(
        &mut self,
        interruption: Interruption,
        data_reg: Option<AsmRegister64>,
    ) -> AssembleResult<()> {
        tracing::debug!("Generating interruption: {interruption:?}");

        let (state_interruption, state_resume) = {
            let state = self.state.borrow();
            (
                &state.interruption as *const _ as u64,
                &state.resume_addr as *const _ as u64,
            )
        };

        self.sync_all_registers()?;
        self.restore_registers()?;

        self.emitter.push(code_asm::r14)?;
        self.emitter.push(code_asm::r15)?;

        let n = {
            let ptr: *const Interruption = &interruption;
            let bytes_ptr = ptr.cast::<u8>();
            unsafe { std::ptr::read(bytes_ptr) }
        };

        // state.interruption = Interruption::KIND(*data_reg)
        self.emitter.mov(code_asm::r14, state_interruption)?;
        self.emitter
            .mov(code_asm::byte_ptr(code_asm::r14), n as u32)?;
        if let Some(data_reg) = data_reg {
            self.emitter
                .mov(code_asm::qword_ptr(code_asm::r14 + 8), data_reg)?;
        }

        // state.resume_addr = rip+10
        self.emitter.mov(code_asm::r14, state_resume)?;
        self.emitter.add_instruction(iced_x86::Instruction::with2(
            iced_x86::Code::Lea_r64_m,
            iced_x86::Register::R15,
            iced_x86::MemoryOperand::with_base_displ(iced_x86::Register::RIP, 10),
        )?)?;
        self.emitter
            .mov(code_asm::ptr(code_asm::r14), code_asm::r15)?;

        self.emitter.pop(code_asm::r15)?;
        self.emitter.pop(code_asm::r14)?;

        self.emitter.jmp(code_asm::r13)?;

        Ok(())
    }

    fn get_cpu_register(&mut self, register: u8) -> AssembleResult<AsmRegister64> {
        self.get_host_register(GuestRegister::cpu(register), |emitter, state, host_reg| {
            // load the register value
            let reg_offset = state.offset_of(|state| &state.cpu.gpr[register as usize]);
            emitter.mov(host_reg, code_asm::ptr(code_asm::rsi + reg_offset))?;
            Ok(())
        })
    }

    fn get_cpu_pc(&mut self) -> AssembleResult<AsmRegister64> {
        self.get_host_register(GuestRegister::pc(), |emitter, state, host_reg| {
            // load the register value
            let reg_offset = state.offset_of(|state| &state.cpu.pc);
            emitter.mov(host_reg, code_asm::ptr(code_asm::rsi + reg_offset))?;
            Ok(())
        })
    }

    /// Gets a host register from the given guest register
    fn get_host_register<F>(
        &mut self,
        guest_reg: GuestRegister,
        initialize_with: F,
    ) -> AssembleResult<AsmRegister64>
    where
        F: FnOnce(&mut CodeAssembler, &JitState, AsmRegister64) -> AssembleResult<()>,
    {
        if let Some(&reg) = self.regs.get(guest_reg) {
            Ok(reg)
        } else {
            let (&reg, dropped) = self.regs.insert(guest_reg).unwrap();

            if CALLEE_SAVED_REGISTERS.contains(&reg) && !self.saved_regs.contains(&reg) {
                self.save_register(reg)?;
            }

            tracing::debug!(
                "Allocated {:?} for {guest_reg:?}",
                iced_x86::Register::from(reg)
            );

            if let Some(dropped) = dropped {
                self.sync_guest_with(dropped, reg)?;
            };

            initialize_with(&mut self.emitter, &self.state, reg)?;
            Ok(reg)
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

    fn save_register(&mut self, reg: AsmRegister64) -> AssembleResult<()> {
        self.saved_regs.push(reg);
        self.emitter.push(reg)?;
        Ok(())
    }
    fn restore_registers(&mut self) -> AssembleResult<()> {
        for reg in self.saved_regs.drain(..).rev() {
            self.emitter.pop(reg)?;
        }
        Ok(())
    }
}
