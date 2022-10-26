mod instructions;
mod register;
mod state;

use std::cell::RefCell;
use std::rc::Rc;

use iced_x86::code_asm::{self, AsmRegister64, CodeAssembler};

use crate::cpu::instruction::Instruction;
use crate::n64::State;

use self::register::{GuestRegister, Registers, CALLEE_SAVED_REGISTERS};
use self::state::JitState;

use super::code::ExecBuffer;
use super::jump_table::JumpTable;

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
    Cpu(#[from] anyhow::Error),
}

type AssembleResult<T> = Result<T, AssembleError>;

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
        let initial_pc = self.pc;
        let _compiled_cycles = self.compile_block(cycles).unwrap();

        let compiled = match assemble_code(self.emitter, self.state.into_inner()) {
            Ok(compiled) => compiled,
            Err(error) => panic!("Could not compile the code properly: {error:?}"),
        };

        println!("{:02x?}", compiled.as_slice());

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
                    .map_err(AssembleError::Cpu)?;

                total_cycles += instruction.cycles();

                instruction
            };

            // check early return
            let status = self.compile_instruction(instruction).unwrap();
            self.pc += 4;
            match status {
                AssembleStatus::Continue => {}
                AssembleStatus::InvalidateCache => {
                    break;
                }
                AssembleStatus::Branch => {
                    return Ok(total_cycles);
                }
            }
        }

        let cpu_pc = self.get_cpu_pc()?;
        self.emitter.mov(cpu_pc, self.pc)?;

        self.sync_all_registers()?;
        self.restore_registers()?;

        self.emitter.jmp(code_asm::r13)?;

        Ok(total_cycles)
    }

    #[allow(clippy::too_many_lines)]
    /// Compiles the given instruction and save the generated code into `buf`
    fn compile_instruction(&mut self, instruction: Instruction) -> AssembleResult<AssembleStatus> {
        tracing::debug!("Compiling {instruction:02x?}");
        match instruction {
            Instruction::NOP => Ok(AssembleStatus::Continue),

            Instruction::SpecialAND(inst) => self.emit_and(inst),
            Instruction::ANDI(inst) => self.emit_andi(inst),
            Instruction::SpecialOR(inst) => self.emit_or(inst),
            Instruction::ORI(inst) => self.emit_ori(inst),
            Instruction::SpecialXOR(inst) => self.emit_xor(inst),
            Instruction::XORI(inst) => self.emit_xori(inst),
            Instruction::SpecialNOR(inst) => self.emit_nor(inst),

            Instruction::SpecialADD(inst) => self.emit_add(inst),
            Instruction::SpecialADDU(inst) => self.emit_addu(inst),
            Instruction::ADDI(inst) => self.emit_addi(inst),
            Instruction::ADDIU(inst) => self.emit_addiu(inst),
            Instruction::SpecialSUB(inst) => self.emit_sub(inst),
            Instruction::SpecialSUBU(inst) => self.emit_subu(inst),
            Instruction::SpecialMULT(inst) => self.emit_mult(inst),
            Instruction::SpecialMULTU(inst) => self.emit_multu(inst),
            Instruction::SpecialDIV(inst) => self.emit_div(inst),
            Instruction::SpecialDIVU(inst) => self.emit_divu(inst),
            Instruction::SpecialSLL(inst) => self.emit_sll(inst),
            Instruction::SpecialSLLV(inst) => self.emit_sllv(inst),
            Instruction::SpecialSRA(inst) => self.emit_sra(inst),
            Instruction::SpecialSRAV(inst) => self.emit_srav(inst),
            Instruction::SpecialSRL(inst) => self.emit_srl(inst),
            Instruction::SpecialSRLV(inst) => self.emit_srlv(inst),

            Instruction::BNE(inst) => self.emit_bne(inst),

            Instruction::J(inst) => self.emit_j(inst),
            Instruction::JAL(inst) => self.emit_jal(inst),
            Instruction::SpecialJR(inst) => self.emit_jr(inst),

            Instruction::SW(inst) => self.emit_sw(inst),

            Instruction::LUI(inst) => self.emit_lui(inst),
            Instruction::LB(inst) => self.emit_lb(inst),
            Instruction::LBU(inst) => self.emit_lbu(inst),
            Instruction::LH(inst) => self.emit_lh(inst),
            Instruction::LHU(inst) => self.emit_lhu(inst),
            Instruction::LW(inst) => self.emit_lw(inst),
            Instruction::LWU(inst) => self.emit_lwu(inst),

            _ => todo!("Instruction not implemented: {instruction:02x?}"),
        }
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

fn assemble_code(
    mut emitter: CodeAssembler,
    state: Rc<RefCell<State>>,
) -> Result<ExecBuffer, AssembleError> {
    let code = emitter.assemble(0)?;
    let map = unsafe { ExecBuffer::new(code, state)? };
    Ok(map)
}
