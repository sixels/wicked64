use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::cpu::instruction::{ImmediateType, Instruction};
use crate::jit::codegen::CallArg;
use crate::mmu::MemoryUnit;
use crate::n64::State;

use super::code::{CompiledBlock, RawBlock};
use super::codegen::callable::Callable;
use super::codegen::register::CALLEE_SAVED_REGISTERS;
use super::codegen::AddressingMode;
use super::codegen::{register::X64Gpr, Emitter};
use super::register::{GuestRegister, Registers};

/// Wasm codegen engine
#[derive(Debug)]
pub struct JitEngine {}

impl JitEngine {
    pub fn new() -> JitEngine {
        Self {}
    }

    pub fn compile_block(&self, state: Rc<RefCell<State>>) -> (CompiledBlock, usize) {
        // TODO: Replace this with the actual number of cycles it should compile
        // For testing purposes, we will run a fixed amount of instructions
        let pclock_size = 5;
        let n_instructions = 10 * pclock_size;

        Jit::new(state).compile(n_instructions)
    }
}

struct EmulatorState(Rc<RefCell<State>>);

impl EmulatorState {
    pub fn new(state: Rc<RefCell<State>>) -> Self {
        Self(state)
    }

    pub fn offset_of<F, T>(&self, get_offset: F) -> usize
    where
        F: FnOnce(&State) -> &T,
    {
        let state = self.0.borrow();

        let data_addr = get_offset(&state) as *const T as usize;
        let state_addr = self.state_ptr() as usize;

        debug_assert!(state_addr <= data_addr);
        data_addr - state_addr
    }

    pub fn state_ptr(&self) -> *const State {
        &*self.0.borrow()
    }
}

impl Deref for EmulatorState {
    type Target = Rc<RefCell<State>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EmulatorState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The JIT compiler
struct Jit {
    state: EmulatorState,
    pc: u64,
    regs: Registers,
    code: RawBlock,
}

impl Jit {
    /// Create a new Jit compiler
    pub fn new(state: Rc<RefCell<State>>) -> Self {
        let pc = { state.borrow().cpu.pc };

        let code = RawBlock::new().unwrap();

        Self {
            pc,
            state: EmulatorState::new(state),
            regs: Registers::new(),
            code,
        }
    }

    /// Compile the code
    pub fn compile(mut self, cycles: usize) -> (CompiledBlock, usize) {
        tracing::debug!("Generating compiled block");

        // Initialize the code generation
        let initial_pc = self.state.borrow().cpu.pc;

        // Compile the code
        tracing::info!("Compiling the code");
        let compiled_cycles = self.compile_block(cycles);
        // TODO: Sync guest registers with host registers
        // **********************************************

        let compiled = match self.code.compile(self.state.clone()) {
            Ok(compiled) => compiled,
            Err(error) => panic!("Could not compile the code: {error:?}"),
        };

        tracing::info!(
            "{compiled_cycles} PClocks block compiled from pc: ({:08x}..{:08x})",
            initial_pc,
            self.pc
        );

        (compiled, (self.pc - self.state.borrow().cpu.pc) as usize)
    }

    fn compile_block(&mut self, cycles: usize) -> usize {
        // save the state address into `rsi` so we can easily access guest registers later
        let state_addr = self.state.state_ptr() as *mut State as u64;
        self.code.emit_movabs_reg(X64Gpr::Rsi, state_addr).unwrap();

        let mut total_cycles = 0;
        while total_cycles < cycles {
            // fetch the next instruction and update the PC and cycles
            let instruction = {
                let state = self.state.borrow();
                let instruction = state.cpu.fetch_instruction(&state.mmu, self.pc).unwrap();

                self.pc += 4;
                total_cycles += instruction.cycles();

                instruction
            };

            // early return
            if self.compile_instruction(instruction) && total_cycles < cycles {
                break;
            }
        }

        total_cycles
    }

    /// Compiles the given instruction and save the generated code into `buf`
    fn compile_instruction(&mut self, instruction: Instruction) -> bool {
        // let state = self.state.borrow();
        // let cpu = &state.cpu;

        tracing::debug!("Compiling {instruction:02x?}");
        match instruction {
            Instruction::LUI(ImmediateType { rt, immediate, .. }) => {
                // - Get host register for `rt`
                // - Move `immediate << 16` into host register
                let host_rt = self.get_host_cpu_register(rt as usize);
                self.code
                    .emit_mov_reg_immediate(host_rt, (immediate as u64) << 16)
                    .unwrap();
                false
            }
            Instruction::ORI(ImmediateType {
                rs, rt, immediate, ..
            }) => {
                // - Get host register for `rs`
                // - Get host register for `rt`
                // - Compute `*rs | immediate`, saving the result in a tmp register
                // - Move the result into `rt`
                let host_rs = self.get_host_cpu_register(rs as usize);
                let host_rt = self.get_host_cpu_register(rt as usize);

                let tmp_reg = self.get_tmp_register(0);
                self.code
                    .emit_mov_reg_immediate(tmp_reg, immediate as u64)
                    .unwrap();

                // TODO: Implement `emit_or_reg_reg`
                // or host_rs, tmp_reg
                self.code.emit_or_reg_reg(tmp_reg, host_rs).unwrap();
                self.code
                    .emit_mov(
                        AddressingMode::Register(host_rt),
                        AddressingMode::Register(tmp_reg),
                    )
                    .unwrap();

                self.regs.drop_guest_register(GuestRegister::Tmp(0));

                false
            }
            Instruction::SW(ImmediateType {
                rs: base,
                rt,
                immediate: offset,
                ..
            }) => {
                // TODO: 64-bit offset support

                fn mmu_store(state: *mut State, vaddr: usize, rt: u64) {
                    let State { cpu, mmu } = unsafe { &mut *state };

                    let addr = cpu.translate_virtual(vaddr);

                    mmu.store::<_, byteorder::BigEndian>(addr, rt);
                }

                let vaddr_reg = self.get_tmp_register(0);
                {
                    let offset_ex = offset as i16 as usize;
                    let base = self.host_reg_from_guest(GuestRegister::Cpu(base as usize));

                    self.code
                        .emit_mov_reg_immediate(vaddr_reg, offset_ex as u64)
                        .unwrap();
                    self.code.emit_add_reg_reg(vaddr_reg, base).unwrap();
                }

                self.sync_guest_register(X64Gpr::Rdx);
                self.code.emit_push_reg(X64Gpr::Rsi).unwrap();
                let rt = self.get_host_cpu_register(rt as usize);
                self.code
                    .emit_mov(
                        AddressingMode::Register(X64Gpr::Rdx),
                        AddressingMode::Register(rt),
                    )
                    .unwrap();
                self.emit_call_wrapper(
                    mmu_store as fn(_, _, _),
                    &[
                        CallArg::Val(self.state.state_ptr() as u64),
                        CallArg::Reg(vaddr_reg),
                    ],
                    None,
                );
                self.code.emit_pop_reg(X64Gpr::Rsi).unwrap();

                true
            }
            _ => todo!("Implement the rest of the instructions"),
        }
    }

    fn emit_call_wrapper<const N: usize, I, O, C: Callable<N, I, O>>(
        &mut self,
        funct: C,
        args: &[CallArg],
        return_to: Option<X64Gpr>,
    ) {
        self.code.emit_push_reg(X64Gpr::Rsi).unwrap();

        let mut tmp_regs = Vec::new();
        for (guest, host) in self.regs.iter() {
            match guest {
                GuestRegister::Tmp(_) => {
                    if !CALLEE_SAVED_REGISTERS.contains(&host) {
                        self.code.emit_push_reg(host).unwrap();
                        tmp_regs.insert(0, host);
                    }
                }
                _ => self.sync_guest_register(host),
            }
        }

        self.code.emit_call(funct, args).unwrap();
        return_to.map(|ret| {
            self.code.emit_mov(
                AddressingMode::Register(ret),
                AddressingMode::Register(X64Gpr::Rax),
            )
        });

        for host in tmp_regs.into_iter() {
            self.code.emit_pop_reg(host).unwrap();
        }

        self.code.emit_pop_reg(X64Gpr::Rsi).unwrap();
    }

    fn get_host_cpu_register(&mut self, register: usize) -> X64Gpr {
        let host_reg = self.host_reg_from_guest(GuestRegister::Cpu(register));

        // load the register value
        let reg_offset = self.state.offset_of(|state| &state.cpu.gpr[register]);
        self.code
            .emit_mov_reg_qword_ptr(host_reg, reg_offset)
            .unwrap();

        host_reg
    }
    fn get_tmp_register(&mut self, register: usize) -> X64Gpr {
        self.host_reg_from_guest(GuestRegister::Tmp(register))
    }

    /// Gets an unused host register from a guest one
    fn host_reg_from_guest(&mut self, guest_reg: GuestRegister) -> X64Gpr {
        self.regs
            .get_mapped_register(guest_reg, |old_guest_reg, host_reg| {
                let old_guest_reg_offset = match old_guest_reg {
                    GuestRegister::Cpu(reg) => self.state.offset_of(|state| &state.cpu.gpr[reg]),
                    GuestRegister::Cp0(reg) => self
                        .state
                        .offset_of(|state| state.cpu.cp0.get_register(reg)),
                    GuestRegister::Tmp(_) => return false,
                };

                self.code
                    .emit_mov_rsi_rel_reg(old_guest_reg_offset as i32, host_reg)
                    .unwrap();

                true
            })
    }

    fn sync_guest_register(&mut self, reg: X64Gpr) {
        if let Some((guest_reg, host_reg)) = self.regs.find_host_register(reg) {
            let guest_offset = match guest_reg {
                GuestRegister::Cpu(reg) => self.state.offset_of(|state| &state.cpu.gpr[reg]),
                GuestRegister::Cp0(reg) => self
                    .state
                    .offset_of(|state| state.cpu.cp0.get_register(reg)),
                GuestRegister::Tmp(_) => return,
            };

            debug_assert!(guest_offset <= i32::MAX as _);
            self.code
                .emit_mov_rsi_rel_reg(guest_offset as i32, host_reg)
                .unwrap();
            self.regs.drop_guest_register(guest_reg);
        }
    }
}
