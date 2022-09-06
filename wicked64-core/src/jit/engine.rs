use std::cell::RefCell;
use std::rc::Rc;

use w64_codegen::register::{Register, CALLEE_SAVED_REGISTERS};
use w64_codegen::{emit, Emitter, ExecBuffer};

use crate::cpu::instruction::{ImmediateType, Instruction};
use crate::jit::register::GuestRegisterKind;
use crate::mmu::MemoryUnit;
use crate::n64::State;

use super::register::{GuestRegister, Registers};
use super::state::EmulatorState;

/// Wasm codegen engine
#[derive(Debug)]
pub struct JitEngine {}

impl JitEngine {
    pub fn new() -> JitEngine {
        Self {}
    }

    pub fn compile_block(&self, state: Rc<RefCell<State>>) -> (ExecBuffer, usize) {
        // TODO: Replace this with the actual number of cycles it should compile
        // For testing purposes, we will run a fixed amount of instructions
        let pclock_size = 5;
        let n_instructions = 1 * pclock_size;

        Jit::new(state).compile(n_instructions)
    }
}

/// The JIT compiler
struct Jit {
    state: EmulatorState,
    pc: u64,
    regs: Registers,
    emitter: Emitter,
    saved_regs: Vec<Register>,
}

impl Jit {
    /// Create a new Jit compiler
    pub fn new(state: Rc<RefCell<State>>) -> Self {
        let pc = { state.borrow().cpu.pc };

        Self {
            pc,
            state: EmulatorState::new(state),
            regs: Registers::new(),
            emitter: Emitter::default(),
            saved_regs: Vec::new(),
        }
    }

    /// Compile the code
    pub fn compile(mut self, cycles: usize) -> (ExecBuffer, usize) {
        tracing::debug!("Generating compiled block");

        // Initialize the code generation
        let initial_pc = self.state.borrow().cpu.pc;

        // Compile the code
        tracing::info!("Compiling the code");
        let compiled_cycles = self.compile_block(cycles);
        // TODO: Sync guest registers with host registers
        // **********************************************

        let compiled = match unsafe { self.emitter.make_exec() } {
            Ok(compiled) => compiled,
            Err(error) => panic!("Could not compile the code properly: {error:?}"),
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
        emit!(self.emitter,
            push rax;
            push rsi;
            movabs rsi, $state_addr;
        );

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

        // if we used a callee-saved register, we need to restore then at the end
        // of the code
        for reg in self.saved_regs.iter().rev().copied() {
            emit!(self.emitter,
                pop %reg;
            );
        }

        emit!(self.emitter,
            pop rsi;
            pop rax;
            ret;
        );

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
                let host_rt = self.get_cpu_register(rt as usize);
                let imm = (immediate as u64) << 16;
                emit!(self.emitter,
                    mov %host_rt, $imm;
                );

                false
            }
            Instruction::ORI(ImmediateType {
                rs, rt, immediate, ..
            }) => {
                // - Get host register for `rs`
                // - Get host register for `rt`
                // - Compute `*rs | immediate`, saving the result in a tmp register
                // - Move the result into `rt`
                let host_rs = self.get_cpu_register(rs as usize);
                let host_rt = self.get_cpu_register(rt as usize);

                let tmp_reg = self.get_tmp_register(0);
                emit!(self.emitter,
                    mov %tmp_reg, $immediate;
                    or %tmp_reg, %host_rs;
                    mov %host_rt, %tmp_reg;
                );

                self.regs.drop(GuestRegister::tmp(0));

                false
            }
            Instruction::SW(ImmediateType {
                rs: base,
                rt,
                immediate: offset,
                ..
            }) => {
                fn mmu_store(state: *mut State, vaddr: usize, rt: u64) {
                    let State { cpu, mmu } = unsafe { &mut *state };

                    let addr = cpu.translate_virtual(vaddr);

                    mmu.store::<_, byteorder::BigEndian>(addr, rt);
                }

                let vaddr_reg = self.get_tmp_register(0);
                {
                    let offset_ex = offset as i16 as usize;
                    let base = self.get_cpu_register(base as usize);

                    emit!(self.emitter,
                        mov %vaddr_reg, $offset_ex;
                        add %vaddr_reg, %base;
                    );
                }

                self.sync_register(Register::Rdx);
                let rt = self.get_cpu_register(rt as usize);

                emit!(self.emitter,
                    push rsi;
                    mov rdx, %rt;
                );
                // self.emit_call_wrapper(
                //     mmu_store as fn(_, _, _),
                //     &[
                //         // CallArg::Val(self.state.state_ptr() as u64),
                //         // CallArg::Reg(vaddr_reg),
                //     ],
                //     None,
                // );
                // TODO: ^^^^^^^^^^^^^^^^^^^^^^^
                emit!(self.emitter,
                    pop rsi;
                );

                true
            }
            _ => todo!("Implement the rest of the instructions"),
        }
    }

    // fn emit_call_wrapper<const N: usize, I, O, C: Callable<N, I, O>>(
    //     &mut self,
    //     funct: C,
    //     args: &[CallArg],
    //     return_to: Option<X64Gpr>,
    // ) {
    //     self.emitter.emit_push_reg(X64Gpr::Rsi).unwrap();

    //     let mut tmp_regs = Vec::new();
    //     for (guest, host) in self.regs.iter() {
    //         match guest {
    //             GuestRegister::Tmp(_) => {
    //                 if !CALLEE_SAVED_REGISTERS.contains(&host) {
    //                     self.emitter.emit_push_reg(host).unwrap();
    //                     tmp_regs.insert(0, host);
    //                 }
    //             }
    //             _ => self.sync_guest_register(host),
    //         }
    //     }

    //     self.emitter.emit_call(funct, args).unwrap();
    //     return_to.map(|ret| {
    //         self.emitter.emit_mov(
    //             AddressingMode::Register(ret),
    //             AddressingMode::Register(X64Gpr::Rax),
    //         )
    //     });

    //     for host in tmp_regs.into_iter() {
    //         self.emitter.emit_pop_reg(host).unwrap();
    //     }

    //     self.emitter.emit_pop_reg(X64Gpr::Rsi).unwrap();
    // }

    fn get_cpu_register(&mut self, register: usize) -> Register {
        let host_reg = self.get_host_register(GuestRegister::cpu(register), false);

        // load the register value
        let reg_offset = self.state.offset_of(|state| &state.cpu.gpr[register]);
        emit!(self.emitter,
            mov %host_reg, [rsi + $reg_offset];
        );

        host_reg
    }

    fn get_tmp_register(&mut self, register: usize) -> Register {
        self.get_host_register(GuestRegister::tmp(register), true)
    }

    /// Gets a host register from the given guest register
    fn get_host_register(&mut self, guest_reg: GuestRegister, locked: bool) -> Register {
        if let Some(reg) = self.regs.get(&guest_reg) {
            *reg
        } else {
            let (&reg, dropped) = self.regs.insert(guest_reg, locked).unwrap();

            if CALLEE_SAVED_REGISTERS.contains(&reg) && !self.saved_regs.contains(&reg) {
                emit!(self.emitter,
                    push %reg;
                );
                self.saved_regs.push(reg);
            }

            if let Some(dropped) = dropped {
                let id = dropped.id();
                let guest_offset = match dropped.kind() {
                    GuestRegisterKind::Cpu => self.state.offset_of(|state| &state.cpu.gpr[id]),
                    // GuestRegisterKind::Cop0 => {
                    //     self.state.offset_of(|state| state.cpu.cp0.get_register(id))
                    // }
                    GuestRegisterKind::Temporary => return reg,
                } as i32;

                emit!(self.emitter,
                    mov [rsi + $guest_offset], %reg;
                );
            };
            reg
        }
    }

    /// Sync the given host register with the emulator register
    fn sync_register(&mut self, reg: Register) {
        if let Some((guest_reg, host_reg)) = self.regs.find_by_host(reg) {
            self.sync_guest_with(guest_reg, host_reg)
        }
    }

    fn sync_guest_with(&mut self, guest_reg: GuestRegister, host_reg: Register) {
        let id = guest_reg.id();
        let guest_offset = match guest_reg.kind() {
            GuestRegisterKind::Cpu => self.state.offset_of(|state| &state.cpu.gpr[id]),
            // GuestRegisterKind::Cop0 => {
            //     self.state.offset_of(|state| state.cpu.cp0.get_register(id))
            // }
            GuestRegisterKind::Temporary => return,
        } as i32;

        emit!(self.emitter,
            mov [rsi + $guest_offset], %host_reg;
        );
        self.regs.drop(guest_reg);
    }
}
