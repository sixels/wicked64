use iced_x86::code_asm::{self, AsmRegister64, CodeAssembler};

use crate::{
    cpu::instruction::{ImmediateType, JumpType},
    jit::{bridge, register::ARGS_REGS, Interruption},
};

use super::{AssembleResult, AssembleStatus, Compiler};

type Result = AssembleResult<AssembleStatus>;

enum CallArgument {
    Register(AsmRegister64),
    Value(u64),
}

macro_rules! arg_list {
    (reg, $arg:expr) => {
        self::CallArgument::Register($arg)
    };
    (val, $arg:expr) => {
        self::CallArgument::Value($arg)
    };
    ($($kind:tt : $arg:expr),*) => {
        &[$(arg_list!($kind, $arg)),*]
    }
}

macro_rules! cast_arg {
    ($_arg:tt) => {
        _
    };
}

macro_rules! raw_call {
    ($compiler:ident, $function:path[$($reg:ident: $value:expr),*]) => {{
        $(
            $compiler.emitter.mov(code_asm::$reg, $value)?;
        )*
        $compiler.emitter.mov(code_asm::rax, $function as extern "C" fn($(cast_arg!($value),)*) -> _ as *const u8 as u64)?;
        $compiler.emitter.call(code_asm::rax)
    }};
}

macro_rules! wrap_call {
    ($compiler:ident, $function:path[$($kind:ident: $arg:expr),*]) => {{
        $compiler.wrap_call(arg_list!($($kind : $arg),*), |emitter| {
            let function_ptr = $function as extern "C" fn($(cast_arg!($arg),)*) -> _ as *const u8 as u64;
            emitter.mov(code_asm::rax, function_ptr)?;

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

impl<'jt> Compiler<'jt> {
    /// ```txt
    /// rt = immediate << 16
    /// ```
    pub(super) fn emit_lui(&mut self, inst: ImmediateType) -> Result {
        let ImmediateType { rt, imm, .. } = inst;

        let rt = self.get_cpu_register(rt)?;
        let imm = (imm as u64) << 16;
        self.emitter.mov(rt, imm)?;

        Ok(AssembleStatus::Continue)
    }
    /// ```txt
    /// rt = rs | immediate
    /// ```
    pub(super) fn emit_ori(&mut self, inst: ImmediateType) -> Result {
        let ImmediateType { rt, rs, imm, .. } = inst;

        let rt = self.get_cpu_register(rt)?;
        let rs = self.get_cpu_register(rs)?;

        self.emitter.mov(rt, imm as u64)?;
        self.emitter.or(rt, rs)?;

        Ok(AssembleStatus::Continue)
    }
    /// ```txt
    /// mmu[rs + offset] = rt
    /// ```
    pub(super) fn emit_sw(&mut self, inst: ImmediateType) -> Result {
        let ImmediateType {
            rs,
            rt,
            imm: offset,
            ..
        } = inst;

        let rs = self.get_cpu_register(rs)?;
        let rt = self.get_cpu_register(rt)?;

        let state_addr = self.state.state_ptr();

        self.emitter.mov(code_asm::r14, offset as i16 as u64)?;
        self.emitter.add(code_asm::r14, rs)?;

        wrap_call!(self, bridge::mmu_store[val: state_addr as u64, reg: code_asm::r14, reg: rt])?;

        // `mmu_store` might invalidate the current memory region
        Ok(AssembleStatus::InvalidateCache)
    }
    /// ```txt
    /// r31 = pc + 8
    /// pc = (pc & 0xf000_0000) | (target << 2)
    /// ```
    pub(super) fn emit_jal(&mut self, inst: JumpType) -> Result {
        let target = inst.target;
        let jump_table_addr = self.jump_table as *mut _ as u64;

        let r31 = self.get_cpu_register(31)?;
        self.emitter.mov(r31, self.pc + 8)?;

        self.emitter.mov(code_asm::r15, self.pc & 0xf000_0000)?;
        self.emitter.or(code_asm::r15d, (target as u32) << 2)?;
        wrap_call!(
            self,
            bridge::get_host_jump_addr[
                val:  self.state.state_ptr() as u64,
                val: jump_table_addr,
                reg: code_asm::r15
            ]
        )?;
        self.emit_interruption(Interruption::PrepareJump(0), Some(code_asm::r15))?;
        // `resume` sets the jump address to r15
        self.emitter.jmp(code_asm::r15)?;

        Ok(AssembleStatus::Branch)
    }
    /// ```txt
    /// pc = (pc & 0xf000_0000) | (target << 2)
    /// ```
    pub(super) fn emit_j(&mut self, inst: JumpType) -> Result {
        let target = inst.target;
        let jump_table_addr = self.jump_table as *mut _ as u64;

        self.emitter.mov(code_asm::r15, self.pc & 0xf000_0000)?;
        self.emitter.or(code_asm::r15d, (target as u32) << 2)?;
        wrap_call!(
            self,
            bridge::get_host_jump_addr[
                val:  self.state.state_ptr() as u64,
                val: jump_table_addr,
                reg: code_asm::r15
            ]
        )?;
        self.emit_interruption(Interruption::PrepareJump(0), Some(code_asm::r15))?;
        // `resume` sets the jump address to r15
        self.emitter.jmp(code_asm::r15)?;

        Ok(AssembleStatus::Branch)
    }
    /// ```txt
    /// rt = rs + immediate_u32
    /// ```
    pub(super) fn emit_addi(&mut self, inst: ImmediateType) -> Result {
        let ImmediateType { rs, rt, imm, .. } = inst;

        let rt = self.get_cpu_register(rt)?;
        let rs = self.get_cpu_register(rs)?;

        self.emitter.mov(code_asm::r14, imm as i16 as u32 as u64)?;
        self.emitter.add_instruction(iced_x86::Instruction::with2(
            iced_x86::Code::Add_r32_rm32,
            iced_x86::Register::R14D,
            iced_x86::Register::from(rs).full_register32(),
        )?)?;
        self.emitter.add_instruction(iced_x86::Instruction::with2(
            iced_x86::Code::Movsxd_r64_rm32,
            iced_x86::Register::from(rt),
            iced_x86::Register::R14D,
        )?)?;

        Ok(AssembleStatus::Continue)
    }
    /// ```txt
    /// if rs != rt { pc = pc + (offset_u32 << 2) }
    /// ```
    pub(super) fn emit_bne(&mut self, inst: ImmediateType) -> Result {
        let ImmediateType {
            rs,
            rt,
            imm: offset,
            ..
        } = inst;

        let jump_table_addr = self.jump_table as *mut _ as u64;

        let rs = self.get_cpu_register(rs)?;
        let rt = self.get_cpu_register(rt)?;

        self.sync_all_registers()?;
        self.restore_registers()?;

        self.emitter.cmp(rs, rt)?;

        let mut skip = self.emitter.create_label();
        self.emitter.je(skip)?;

        {
            // Not equal, jump to pc + (offset_u32 << 2)
            self.emitter.mov(
                code_asm::r15,
                self.pc + ((offset as i16 as u32 as u64) << 2),
            )?;
            wrap_call!(
                self,
                bridge::get_host_jump_addr[
                    val:  self.state.state_ptr() as u64,
                    val: jump_table_addr,
                    reg: code_asm::r15
                ]
            )?;
            self.emit_interruption(Interruption::PrepareJump(0), Some(code_asm::r15))?;
            self.emitter.jmp(code_asm::r15)?;
        }

        // Jump to the next instruction
        self.emitter.set_label(&mut skip)?;

        self.emitter.mov(code_asm::r15, self.pc + 4)?;
        wrap_call!(
            self,
            bridge::get_host_jump_addr[
                val:  self.state.state_ptr() as u64,
                val: jump_table_addr,
                reg: code_asm::r15
            ]
        )?;
        self.emit_interruption(Interruption::PrepareJump(0), Some(code_asm::r15))?;
        self.emitter.jmp(code_asm::r15)?;

        Ok(AssembleStatus::Branch)
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

        // read only the first byte of `interruption`
        let n = {
            let ptr: *const Interruption = &interruption;
            let bytes_ptr = ptr.cast::<u8>();
            unsafe { std::ptr::read(bytes_ptr) }
        };

        // `state.interruption = Interruption::KIND(*data_reg)`
        self.emitter.mov(code_asm::r14, state_interruption)?;
        self.emitter
            .mov(code_asm::byte_ptr(code_asm::r14), n as u32)?;
        if let Some(data_reg) = data_reg {
            self.emitter
                .mov(code_asm::qword_ptr(code_asm::r14 + 8), data_reg)?;
        }

        // `state.resume_addr = (instruction after jmp r13)`
        // mov [r14], rax ; 0x00
        // pop 15         ; 0x03
        // pop 14         ; 0x05
        // jmp r13        ; 0x07
        // ...            ; 0x0a

        self.emitter.mov(code_asm::r14, state_resume)?;

        raw_call!(self, bridge::get_rip_value[edi: 0x0au32])?;

        self.emitter
            .mov(code_asm::qword_ptr(code_asm::r14), code_asm::rax)?;
        self.emitter.pop(code_asm::r15)?;
        self.emitter.pop(code_asm::r14)?;

        self.emitter.jmp(code_asm::r13)?;

        Ok(())
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
}