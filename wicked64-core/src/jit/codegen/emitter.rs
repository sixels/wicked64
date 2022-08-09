use std::io;

use byteorder::{LittleEndian, WriteBytesExt};

use super::{callable::Callable, register::X64Gpr};

// TODO: General MOV encoding

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum CallArg {
    Val(u64),
    Reg(X64Gpr),
}

pub trait Emitter: io::Write + Sized {
    fn emit_raw(&mut self, raw_bytes: &[u8]) -> io::Result<()> {
        self.write(raw_bytes)?;
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
    /// mov `dst`, `src`
    fn emit_mov_reg_reg(&mut self, dst: X64Gpr, src: X64Gpr) -> io::Result<()> {
        let base = match (dst < X64Gpr::R8, src < X64Gpr::R8) {
            (true, true) => 0x48,
            (true, false) => 0x4c,
            (false, true) => 0x49,
            (false, false) => 0x4d,
        };

        let s = (src as u8) % 8;
        let d = (dst as u8) % 8;
        self.write(&[base, 0x89, (0b11 << 6) | (s << 3) | (d << 0)])?;

        Ok(())
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
    /// mov qword ptr \[`dst`\], `src`
    fn emit_mov_qword_ptr_reg_reg(&mut self, dst: X64Gpr, src: X64Gpr) -> io::Result<()> {
        let base = match (dst < X64Gpr::R8, src < X64Gpr::R8) {
            (true, true) => 0x48,
            (true, false) => 0x4c,
            (false, true) => 0x49,
            (false, false) => 0x4d,
        };

        let s = src as u8 % 8;
        let d = dst as u8 % 8;

        self.write(&[base, 0x8b, (0b00 << 6) | (s << 3) | (d << 0)])?;
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
        static AUX_REGS: &[X64Gpr] = &[X64Gpr::Rax, X64Gpr::Rbx];

        // organize the registers so we don't write to a register before reading its value
        let reg_has_deps = |reg: X64Gpr| args.contains(&CallArg::Reg(reg)).then(|| reg);
        let arg_has_deps = |arg: &CallArg| match *arg {
            CallArg::Val(_) => None,
            CallArg::Reg(r) => reg_has_deps(r),
        };

        let mut save = Vec::new();
        let mut aux_iter = AUX_REGS.iter();
        for (dependency, dependent) in ARGS_REGS.iter().zip(args) {
            match arg_has_deps(dependent) {
                Some(reg) => {
                    // TODO: Save the original register value to the stack
                    let aux = *aux_iter.next().unwrap();
                    save.push((dependency, aux));
                    self.emit_push_reg(aux)?;
                    self.emit_mov_reg_reg(aux, reg)?;
                    self.emit_mov_reg_reg(*dependency, reg)?;
                }
                _ => {}
            }
        }

        for (&reg, &arg) in ARGS_REGS.iter().zip(args).rev() {
            match arg {
                CallArg::Val(val) => self.emit_movabs_reg(reg, val)?,
                CallArg::Reg(src) => self.emit_mov_reg_reg(reg, src)?,
            }
        }
        let funct_addr = funct.addr();

        self.emit_movabs_reg(X64Gpr::Rax, funct_addr as u64)?;
        self.write(&[0xff, 0xd0])?;

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
        let mod_rm = (0b11110 << 3) | (reg as u8) % 8;

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
