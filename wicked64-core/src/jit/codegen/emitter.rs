use std::io;

use byteorder::{LittleEndian, WriteBytesExt};

use super::{callable::Callable, register::X64Gpr};

// TODO: General MOV encoding

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum CallArg {
    Val(u64),
    Reg(X64Gpr),
}

#[derive(Debug, Clone, Copy)]
pub enum AddressingMode {
    Immediate(u64),
    Register(X64Gpr),
    Direct(i32),
    Indirect(X64Gpr),
    IndirectDisplacement(X64Gpr, i32),
}

pub trait Emitter: io::Write + Sized {
    fn emit_raw(&mut self, raw_bytes: &[u8]) -> io::Result<()> {
        self.write(raw_bytes)?;
        Ok(())
    }
    fn emit_mov(&mut self, dst: AddressingMode, src: AddressingMode) -> io::Result<()> {
        #[cfg(debug_assertions)]
        match dst {
            AddressingMode::Register(X64Gpr::Rsp) | AddressingMode::Immediate(_) => {
                panic!("Invalid or unhandled destination for `mov` instruction: {dst:?}")
            }
            _ => {}
        }

        match (dst, src) {
            (AddressingMode::Register(dst), AddressingMode::Register(src)) => {
                debug_assert!(dst != X64Gpr::Rsp);
                let base = (0b1001 << 3)
                    | (u8::from(src >= X64Gpr::R8) << 2)
                    | (u8::from(dst >= X64Gpr::R8) << 0);

                let s = (src as u8) % 8;
                let d = (dst as u8) % 8;
                let mod_rm = (0b11 << 6) | (s << 3) | (d << 0);

                self.write(&[base, 0x89, mod_rm])?;
            }
            (AddressingMode::Register(dst), AddressingMode::Immediate(im)) => {
                debug_assert!(dst != X64Gpr::Rsp);
                let dst_n = (dst as u8) % 8;

                let base = dst_n + 0xb8;
                let im = im as i32;

                if dst >= X64Gpr::R8 {
                    self.write_u8(0x41)?;
                }
                self.write_u8(base)?;
                self.write_i32::<LittleEndian>(im)?;
            }
            (AddressingMode::Register(dst), AddressingMode::Direct(addr)) => {
                debug_assert!(dst != X64Gpr::Rsp);
                let base = (0b1001 << 3) | (u8::from(dst >= X64Gpr::R8) << 2);

                let d = (dst as u8) % 8;
                let mod_rm = (0b00 << 6) | (d << 3) | (0b100 << 0);

                self.write(&[base, 0x8b, mod_rm, 0x25])?;
                self.write_i32::<LittleEndian>(addr)?;
            }
            (AddressingMode::Register(dst), AddressingMode::Indirect(src)) => {
                debug_assert!(dst != X64Gpr::Rsp);
                let base = (0b1001 << 3)
                    | (u8::from(dst >= X64Gpr::R8) << 2)
                    | (u8::from(src >= X64Gpr::R8) << 0);

                let s = (src as u8) % 8;
                let d = (dst as u8) % 8;
                let mod_rm = (0b00 << 6) | (d << 3) | (s << 0);

                self.write(&[base, 0x8b, mod_rm])?;
                if src == X64Gpr::Rsp {
                    self.write_u8(0x24)?;
                }
            }
            (AddressingMode::Register(dst), AddressingMode::IndirectDisplacement(src, disp)) => {
                debug_assert!(dst != X64Gpr::Rsp);
                let base = (0b1001 << 3)
                    | (u8::from(dst >= X64Gpr::R8) << 2)
                    | (u8::from(src >= X64Gpr::R8) << 0);

                let s = (src as u8) % 8;
                let d = (dst as u8) % 8;
                let mod_rm = (0b10 << 6) | (d << 3) | (s << 0);

                self.write(&[base, 0x8b, mod_rm])?;
                if src == X64Gpr::Rsp {
                    self.write_u8(0x24)?;
                }
                self.write_i32::<LittleEndian>(disp)?;
            }
            _ => unimplemented!(),
        };
        Ok(())
    }
    /// push `reg`
    fn emit_push_reg(&mut self, reg: X64Gpr) -> io::Result<()> {
        let reg_number = reg as u8 % 8;
        if reg >= X64Gpr::R8 {
            self.write_u8(0x41)?;
        }
        self.write_u8(0x50 + reg_number)
    }
    /// pop `reg`
    fn emit_pop_reg(&mut self, reg: X64Gpr) -> io::Result<()> {
        let reg_number = reg as u8 % 8;
        if reg >= X64Gpr::R8 {
            self.write_u8(0x41)?;
        }
        self.write_u8(0x58 + reg_number)
    }
    /// movabs `reg`, `immediate`
    fn emit_movabs_reg(&mut self, reg: X64Gpr, immediate: u64) -> io::Result<()> {
        let reg_number = reg as u8 % 8;
        let base = if reg >= X64Gpr::R8 { 0x49 } else { 0x48 };

        self.write(&[base, 0xb8 + reg_number])?;
        self.write_u64::<LittleEndian>(immediate)
    }
    /// mov `reg`, `immediate`
    fn emit_mov_reg_immediate(&mut self, reg: X64Gpr, immediate: u64) -> io::Result<()> {
        if immediate > i32::MAX as _ {
            return self.emit_movabs_reg(reg, immediate);
        }

        let reg_number = reg as u8 % 8;
        if reg >= X64Gpr::R8 {
            self.write_u8(0x41)?;
        }
        if reg == X64Gpr::Rbx {
            self.write_u8(0xbb)?;
        } else {
            self.write_u8(0xb8 + reg_number)?;
        }
        self.write_u32::<LittleEndian>(immediate as u32)
    }
    /// mov `reg`, qword ptr \[`offset`\]
    fn emit_mov_reg_qword_ptr(&mut self, reg: X64Gpr, offset: usize) -> io::Result<()> {
        let reg_number = reg as u8 % 8;

        let base = if reg >= X64Gpr::R8 { 0x4c } else { 0x48 };
        // TODO: Document MOD R/M
        let mod_rm = (0b10 << 6) | (reg_number << 3) | (0b110 << 0);

        self.write(&[base, 0x8b, mod_rm])?;
        self.write_i32::<LittleEndian>(offset as i32)
    }
    /// mov qword ptr \[`dst` + `displacement`\], `src`
    fn emit_mov_qword_ptr_reg_reg(
        &mut self,
        dst: X64Gpr,
        src: X64Gpr,
        displacement: i32,
    ) -> io::Result<()> {
        let base = match (dst < X64Gpr::R8, src < X64Gpr::R8) {
            (true, true) => 0x48,
            (true, false) => 0x4c,
            (false, true) => 0x49,
            (false, false) => 0x4d,
        };
        let op = if displacement == 0 { 0x8b } else { 0x89 };

        let s = src as u8 % 8;
        let d = dst as u8 % 8;
        let mod_rm = (((displacement != 0) as u8) << 7) | (s << 3) | (d << 0);

        self.write(&[base, op, mod_rm])?;
        if displacement != 0 {
            self.write_i32::<LittleEndian>(displacement)?;
        }

        Ok(())
    }
    /// mov qword ptr \[rsi+`offset`\], `reg`
    fn emit_mov_rsi_rel_reg(&mut self, offset: i32, reg: X64Gpr) -> io::Result<()> {
        let base = if reg >= X64Gpr::R8 { 0x4c } else { 0x48 };
        let r = reg as u8 % 8;

        self.write(&[base, 0x89, (0b10 << 6) | (r << 3) | (0b110 << 0)])?;
        self.write_i32::<LittleEndian>(offset)
    }

    /// ret
    fn emit_ret(&mut self) -> io::Result<()> {
        self.write_u8(0xc3)
    }
    fn emit_call_safe<const N: usize, I, O, C: Callable<N, I, O>>(
        &mut self,
        funct: C,
        args: &[CallArg; N],
    ) -> io::Result<()> {
        self.emit_call(funct, args)
    }
    fn emit_call<const N: usize, I, O, C: Callable<N, I, O>>(
        &mut self,
        funct: C,
        args: &[CallArg],
    ) -> io::Result<()> {
        static ARGS_REGS: &[X64Gpr] = &[X64Gpr::Rdi, X64Gpr::Rsi, X64Gpr::Rdx, X64Gpr::Rcx];

        // organize the registers so we don't write to a register before reading its value
        let reg_has_deps = |reg: X64Gpr| args.contains(&CallArg::Reg(reg)).then(|| reg);

        let mut save = Vec::new();
        // we will store a max of 32 bytes in the stack, as we only handle 4 arguments
        let stack_size = 8 * args.len();
        let mut stack_index = stack_size as i32;

        self.emit_sub_reg_dword(X64Gpr::Rsp, stack_size as u32)?;

        for (&dst, &src) in ARGS_REGS.iter().zip(args) {
            match src {
                CallArg::Reg(src) => {
                    if let Some(_) = reg_has_deps(dst) {
                        stack_index -= 8;
                        save.push((dst, stack_index));
                        todo!()
                        // self.emit_mov_qword_ptr_reg_reg(X64Gpr::Rsp, dst, stack_index)?;
                    }
                    self.emit_mov(AddressingMode::Register(dst), AddressingMode::Register(src))?;
                }
                CallArg::Val(v) => self.emit_movabs_reg(dst, v)?,
            };
        }

        for (&reg, &arg) in ARGS_REGS.iter().zip(args).rev() {
            match arg {
                CallArg::Val(val) => self.emit_movabs_reg(reg, val)?,
                CallArg::Reg(src) => self.emit_mov(AddressingMode::Register(reg), AddressingMode::Register(src))?,
            }
        }
        let funct_addr = funct.addr();

        self.emit_movabs_reg(X64Gpr::Rax, funct_addr as u64)?;
        self.write(&[0xff, 0xd0])?; // call rax

        for (reg, index) in save {
            todo!()
            // self.emit_mov_reg_qword_ptr(reg, offset);
        }
        self.emit_add_reg_dword(X64Gpr::Rsp, stack_size as u32)?;

        Ok(())
    }

    //-------------- ALU

    // TODO: qword xor (i.e move qword immediate into a register and do a xor a,b)
    /// xor `reg`, dword `immediate`
    fn emit_xor_reg_dword(&mut self, reg: X64Gpr, immediate: u32) -> io::Result<()> {
        if reg == X64Gpr::Rax {
            self.write(&[0x48, 0x35])?;
            return self.write_u32::<LittleEndian>(immediate);
        }

        let base = if reg >= X64Gpr::R8 { 0x49 } else { 0x48 };
        let mod_rm = (0b11_110 << 3) | (reg as u8) % 8;

        self.write(&[base, 0x81, mod_rm])?;
        self.write_u32::<LittleEndian>(immediate)
    }

    fn emit_sub_reg_dword(&mut self, reg: X64Gpr, immediate: u32) -> io::Result<()> {
        if reg == X64Gpr::Rax {
            self.write(&[0x48, 0x2d])?;
            return self.write_u32::<LittleEndian>(immediate);
        }

        let base = if reg >= X64Gpr::R8 { 0x49 } else { 0x48 };
        let mod_rm = (0b11_101 << 3) | (reg as u8) % 8;

        self.write(&[base, 0x81, mod_rm])?;
        self.write_u32::<LittleEndian>(immediate)
    }

    fn emit_or_reg_reg(&mut self, a: X64Gpr, b: X64Gpr) -> io::Result<()> {
        let base = match (a < X64Gpr::R8, b < X64Gpr::R8) {
            (true, true) => 0x48,
            (true, false) => 0x4c,
            (false, true) => 0x49,
            (false, false) => 0x4d,
        };

        let a = a as u8 % 8;
        let b = b as u8 % 8;

        self.write(&[base, 0x09, (0b11 << 6) | (b << 3) | (a << 0)])?;
        Ok(())
    }
    fn emit_add_reg_reg(&mut self, a: X64Gpr, b: X64Gpr) -> io::Result<()> {
        let base = match (a < X64Gpr::R8, b < X64Gpr::R8) {
            (true, true) => 0x48,
            (true, false) => 0x4c,
            (false, true) => 0x49,
            (false, false) => 0x4d,
        };

        let a = a as u8 % 8;
        let b = b as u8 % 8;

        self.write(&[base, 0x01, (0b11 << 6) | (b << 3) | (a << 0)])?;
        Ok(())
    }
    fn emit_add_reg_dword(&mut self, reg: X64Gpr, immediate: u32) -> io::Result<()> {
        if reg == X64Gpr::Rax {
            self.write(&[0x48, 0x05])?;
            return self.write_u32::<LittleEndian>(immediate);
        }

        let base = if reg >= X64Gpr::R8 { 0x49 } else { 0x48 };
        let mod_rm = (0b11_000 << 3) | (reg as u8) % 8;

        self.write(&[base, 0x81, mod_rm])?;
        self.write_u32::<LittleEndian>(immediate)
    }

    #[cfg(test)]
    fn emit_assert_reg_eq(&mut self, reg: X64Gpr, expected: u64) -> io::Result<()> {
        fn assert_eq_wrapper(reg: X64Gpr, expected: u64, actual: u64) {
            assert_eq!(expected, actual, "{reg:?} is not correct");
        }

        // Save caller-saved regs.
        // As this function is only intended for tests, we don't have much
        // control on which registers are being actually used.
        let save_regs = [
            X64Gpr::Rax,
            X64Gpr::Rdi,
            X64Gpr::Rsi,
            X64Gpr::Rdx,
            X64Gpr::Rcx,
            X64Gpr::R8,
            X64Gpr::R9,
            X64Gpr::R10,
            X64Gpr::R11,
        ];
        for reg in save_regs.iter() {
            self.emit_push_reg(*reg).unwrap();
        }

        self.emit_call(
            assert_eq_wrapper as fn(_, _, _),
            &[
                CallArg::Val(reg as u64),
                CallArg::Val(expected),
                CallArg::Reg(reg),
            ],
        )?;

        for reg in save_regs.iter().rev() {
            self.emit_pop_reg(*reg).unwrap();
        }

        Ok(())
    }
}

impl<T: io::Write> Emitter for T {}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! _emit {
        (mov $dst:tt, $src:tt) => {{
            let mut buf = Vec::new();
            buf.emit_mov(_addressing!($dst), _addressing!($src))
                .unwrap();
            buf
        }};
    }
    macro_rules! _register {
        (rax) => {
            X64Gpr::Rax
        };
        (rbx) => {
            X64Gpr::Rbx
        };
        (rcx) => {
            X64Gpr::Rcx
        };
        (rdx) => {
            X64Gpr::Rdx
        };
        (rsi) => {
            X64Gpr::Rsi
        };
        (rdi) => {
            X64Gpr::Rdi
        };
        (rsp) => {
            X64Gpr::Rsp
        };
        (rbp) => {
            X64Gpr::Rbp
        };
        (r8) => {
            X64Gpr::R8
        };
        (r9) => {
            X64Gpr::R9
        };
        (r10) => {
            X64Gpr::R10
        };
        (r11) => {
            X64Gpr::R11
        };
        (r12) => {
            X64Gpr::R12
        };
        (r13) => {
            X64Gpr::R13
        };
        (r14) => {
            X64Gpr::R14
        };
        (r15) => {
            X64Gpr::R15
        };
    }
    macro_rules! _addressing {
        ([$reg:tt + $displacement:literal]) => {
            AddressingMode::IndirectDisplacement(_register!($reg), $displacement)
        };
        ([$reg:tt - $displacement:literal]) => {
            AddressingMode::IndirectDisplacement(_register!($reg), -$displacement)
        };
        ([$addr:literal]) => {
            AddressingMode::Direct($addr)
        };
        ([$reg:tt]) => {
            AddressingMode::Indirect(_register!($reg))
        };
        ($im:literal) => {
            AddressingMode::Immediate($im)
        };
        ($reg:tt) => {
            AddressingMode::Register(_register!($reg))
        };
    }

    #[test]
    fn it_should_emit_mov_with_reg_reg_addressing_mode() {
        // mov rcx,r8
        let code = _emit!(mov rcx, r8);
        assert_eq!(code, vec![0x4c, 0x89, 0xc1]);
        // mov r9, rax
        let code = _emit!(mov r9, rax);
        assert_eq!(code, vec![0x49, 0x89, 0xc1]);
        // mov rcx, rbx
        let code = _emit!(mov rcx, rbx);
        assert_eq!(code, vec![0x48, 0x89, 0xd9]);
        // mov r9, r11
        let code = _emit!(mov r9, r11);
        assert_eq!(code, vec![0x4d, 0x89, 0xd9]);
    }

    #[test]
    fn it_should_emit_mov_with_reg_immediate_addressing_mode() {
        //mov rcx, 0x3412
        let code = _emit!(mov rcx, 0x3412);
        assert_eq!(code, vec![0xb9, 0x12, 0x34, 0x00, 0x00]);
        //mov rbx, 0x3412
        let code = _emit!(mov rbx, 0x3412);
        assert_eq!(code, vec![0xbb, 0x12, 0x34, 0x00, 0x00]);
        //mov r9, 0x3412
        let code = _emit!(mov r9, 0x3412);
        assert_eq!(code, vec![0x41, 0xb9, 0x12, 0x34, 0x00, 0x00]);
        //mov r11, 0x3412
        let code = _emit!(mov r11, 0x3412);
        assert_eq!(code, vec![0x41, 0xbb, 0x12, 0x34, 0x00, 0x00]);
        //mov rax, 0x3412
        let code = _emit!(mov rax, 0x3412);
        assert_eq!(code, vec![0xb8, 0x12, 0x34, 0x00, 0x00]);
        //mov r8, 0x3412
        let code = _emit!(mov r8, 0x3412);
        assert_eq!(code, vec![0x41, 0xb8, 0x12, 0x34, 0x00, 0x00]);
    }

    #[test]
    fn it_should_emit_mov_with_reg_direct_addressing_mode() {
        // mov rcx, [0x78563412]
        let code = _emit!(mov rcx, [0x78563412]);
        assert_eq!(code, vec![0x48, 0x8b, 0x0c, 0x25, 0x12, 0x34, 0x56, 0x78]);
        // mov rbx, [0x78563412]
        let code = _emit!(mov rbx, [0x78563412]);
        assert_eq!(code, vec![0x48, 0x8b, 0x1c, 0x25, 0x12, 0x34, 0x56, 0x78]);
        // mov r9, [0x78563412]
        let code = _emit!(mov r9, [0x78563412]);
        assert_eq!(code, vec![0x4c, 0x8b, 0x0c, 0x25, 0x12, 0x34, 0x56, 0x78]);
        // mov r11, [0x78563412]
        let code = _emit!(mov r11, [0x78563412]);
        assert_eq!(code, vec![0x4c, 0x8b, 0x1c, 0x25, 0x12, 0x34, 0x56, 0x78]);
        // mov rax, [0x78563412]
        let code = _emit!(mov rax, [0x78563412]);
        assert_eq!(code, vec![0x48, 0x8b, 0x04, 0x25, 0x12, 0x34, 0x56, 0x78]);
        // mov r8, [0x78563412]
        let code = _emit!(mov r8, [0x78563412]);
        assert_eq!(code, vec![0x4c, 0x8b, 0x04, 0x25, 0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn it_should_emit_mov_with_reg_indirect_addressing_mode() {
        // mov rcx, [r8]
        let code = _emit!(mov rcx, [r8]);
        assert_eq!(code, vec![0x49, 0x8b, 0x08]);
        // mov r9, [rax]
        let code = _emit!(mov r9, [rax]);
        assert_eq!(code, vec![0x4c, 0x8b, 0x08]);
        // mov rcx, [rbx]
        let code = _emit!(mov rcx, [rbx]);
        assert_eq!(code, vec![0x48, 0x8b, 0x0b]);
        // mov r9, [r11]
        let code = _emit!(mov r9, [r11]);
        assert_eq!(code, vec![0x4d, 0x8b, 0x0b]);
        // mov rax, [r9]
        let code = _emit!(mov rax, [r9]);
        assert_eq!(code, vec![0x49, 0x8b, 0x01]);
    }

    #[test]
    fn it_should_emit_mov_with_reg_indirect_displacement_addressing_mode() {
        // mov rcx, [r8 + 0x78563412]
        let code = _emit!(mov rcx, [r8 + 0x78563412]);
        assert_eq!(code, vec![0x49, 0x8b, 0x88, 0x12, 0x34, 0x56, 0x78]);
        // mov r9, [rax + 0x78563412]
        let code = _emit!(mov r9, [rax + 0x78563412]);
        assert_eq!(code, vec![0x4c, 0x8b, 0x88, 0x12, 0x34, 0x56, 0x78]);
        // mov rcx, [rbx + 0x78563412]
        let code = _emit!(mov rcx, [rbx + 0x78563412]);
        assert_eq!(code, vec![0x48, 0x8b, 0x8b, 0x12, 0x34, 0x56, 0x78]);
        // mov r9, [r11 + 0x78563412]
        let code = _emit!(mov r9, [r11 + 0x78563412]);
        assert_eq!(code, vec![0x4d, 0x8b, 0x8b, 0x12, 0x34, 0x56, 0x78]);
        // mov rax, [rsp + 0x78563412]
        let code = _emit!(mov rax, [rsp + 0x78563412]);
        assert_eq!(code, vec![0x48, 0x8b, 0x84, 0x24, 0x12, 0x34, 0x56, 0x78]);
        // mov rax, [rsi + 0x78563412]
        let code = _emit!(mov rax, [rsi + 0x78563412]);
        assert_eq!(code, vec![0x48, 0x8b, 0x86, 0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn it_should_emit_mov_with_direct_reg_addressing_mode() {
        todo!()
    }
}
