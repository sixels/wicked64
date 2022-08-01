use std::cell::RefCell;
use std::rc::Rc;

use hashbrown::{HashMap, HashSet};

use crate::cpu::instruction::{ImmediateType, Instruction};
use crate::jit::{
    code::RawBlock,
    codegen::register::X64Gpr,
    codegen::Emitter,
    register::{GuestRegister, HostRegister},
};
use crate::n64::State;

use super::code::CompiledBlock;

/// Wasm codegen engine
#[derive(Debug)]
pub struct JitEngine {}

impl JitEngine {
    pub fn new() -> JitEngine {
        Self {}
    }

    pub fn compile_block(&self, state: Rc<RefCell<State>>) -> (CompiledBlock, usize) {
        // TODO: Change size to the actual number of cycles it should compile
        // For testing purposes, we will run a fixed amount of instructions
        let pclock_size = 5;
        let n_instructions = 10 * pclock_size;

        Jit::new(state).compile(n_instructions)
    }
}

/// The JIT compiler
struct Jit {
    state: Rc<RefCell<State>>,
    pc: u64,
    // len: usize,
    /// Maps a host register to a guest register number
    registers: HashMap<GuestRegister, HostRegister>,
    /// Free host registers
    free_regs: HashSet<X64Gpr>,
    code: RawBlock,
}

impl Jit {
    /// Create a new Jit compiler
    pub fn new(state: Rc<RefCell<State>>) -> Self {
        let pc = { state.borrow().cpu.pc };

        let free_regs = (0u8..15)
            .map(|r| X64Gpr::try_from(r).unwrap())
            .filter(|r| !r.is_reserved())
            .collect();

        let code = RawBlock::new().unwrap();

        Self {
            pc,
            state,
            // len: 0,
            registers: HashMap::new(),
            free_regs,
            code,
        }
    }

    /// Compile the code
    pub fn compile(mut self, cycles: usize) -> (CompiledBlock, usize) {
        tracing::debug!("Generating compiled block");

        // Initialize the code generation
        let initial_pc = self.state.borrow().cpu.pc;

        // Generate the code
        let compiled_cycles = self.compile_block(cycles);
        // TODO: Sinc guest registers with host registers
        // **********************************************

        // Compile the code
        tracing::info!("Compiling the code");
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
        let state_addr = self.state_address() as *mut State as u64;
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
                    .emit_mov_reg_immediate(host_rt.host_reg, (immediate as u64) << 16)
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
                    .emit_mov_reg_immediate(tmp_reg.host_reg, immediate as u64)
                    .unwrap();

                // TODO: Implement `emit_or_reg_reg`
                // or host_rs, tmp_reg
                self.code
                    .emit_raw(&[
                        0x48,
                        0x09,
                        (0b11 << 6)
                            | ((host_rs.host_reg as u8) << 3)
                            | ((tmp_reg.host_reg as u8) << 0),
                    ])
                    .unwrap();

                self.code
                    .emit_mov_reg_reg(host_rt.host_reg, tmp_reg.host_reg)
                    .unwrap();

                // TODO: self.drop_register(tmp_reg);

                todo!()
            }
            _ => todo!("Implement the rest of the instructions"),
        }
    }

    fn state_address(&self) -> *const State {
        &*self.state.borrow()
    }
    /// Get the relative address of a value inside the state
    fn offset_of<T, F: FnOnce(&State) -> &T>(&self, rel_offset: F) -> usize {
        let state = self.state.borrow();

        let data_addr = rel_offset(&state) as *const T as usize;
        let state_addr = self.state_address() as usize;

        debug_assert!(state_addr <= data_addr);
        data_addr - state_addr
    }

    fn get_host_cpu_register(&mut self, register: usize) -> HostRegister {
        let host_reg = self.host_reg_from_guest(GuestRegister::Cpu(register));

        // load the register value
        let reg_offset = self.offset_of(|state| &state.cpu.gpr[register]);
        self.code
            .emit_mov_reg_qword_ptr(host_reg.host_reg, reg_offset)
            .unwrap();

        host_reg
    }
    fn get_tmp_register(&mut self, register: usize) -> HostRegister {
        let host_reg = self.host_reg_from_guest(GuestRegister::Tmp(register));

        host_reg
    }

    /// Gets an unused host register from a guest one
    pub fn host_reg_from_guest(&mut self, guest_reg: GuestRegister) -> HostRegister {
        if let Some(host_reg) = self.registers.get_mut(&guest_reg) {
            host_reg.frequency += 1;
            return *host_reg;
        }

        // NOTE: At this point we already know the key does not exist, that's why we call `insert_unique_unchecked`
        let host_reg = if let Some(&next) = self.free_regs.iter().next().clone() {
            // we still have free registers
            let entry = self
                .registers
                .insert_unique_unchecked(guest_reg, HostRegister::new(next));
            *entry.1
        } else {
            // TODO: free up the least recently accessed register instead of the least accessed
            // no free registers available
            match self
                .registers
                .iter()
                .min_by_key(|(_, HostRegister { frequency, .. })| *frequency)
            {
                Some((old_guest_reg, host_reg)) => {
                    // sync guest with host
                    {
                        let old_reg_offset = match *old_guest_reg {
                            GuestRegister::Cpu(reg) => {
                                Some(self.offset_of(|state| &state.cpu.gpr[reg]))
                            }
                            GuestRegister::Cp0(reg) => {
                                Some(self.offset_of(|state| state.cpu.cp0.get_register(reg)))
                            }
                            GuestRegister::Tmp(_) => None,
                        };

                        if let Some(_reg_offset) = old_reg_offset {
                            todo!("*reg_offset = host_reg")
                        }
                    }

                    let entry = self
                        .registers
                        .insert_unique_unchecked(guest_reg, HostRegister::new(host_reg.host_reg));
                    *entry.1
                }
                None => unreachable!(),
            }
        };

        self.free_regs.remove(&host_reg.host_reg);
        host_reg
    }
}
