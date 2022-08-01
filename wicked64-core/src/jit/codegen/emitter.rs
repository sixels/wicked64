use std::io;

use byteorder::{LittleEndian, WriteBytesExt};

use super::{callable::Callable, register::X64Gpr};

// TODO: General MOV encoding
pub trait Emitter: io::Write {
    fn emit_raw(&mut self, raw_bytes: &[u8]) -> io::Result<()> {
        self.write(raw_bytes)?;
        Ok(())
    }
    fn emit_push_reg(&mut self, reg: X64Gpr) -> io::Result<()> {
        let reg_number = reg as u8 % 8;
        if reg >= X64Gpr::R8 {
            self.write_u8(0x41)?;
        }
        self.write_u8(0x50 + reg_number)
    }
    fn emit_pop_reg(&mut self, reg: X64Gpr) -> io::Result<()> {
        let reg_number = reg as u8 % 8;
        if reg >= X64Gpr::R8 {
            self.write_u8(0x41)?;
        }
        self.write_u8(0x58 + reg_number)
    }
    fn emit_movabs_reg(&mut self, reg: X64Gpr, immediate: u64) -> io::Result<()> {
        let reg_number = reg as u8 % 8;
        let base = if reg >= X64Gpr::R8 { 0x49 } else { 0x48 };

        self.write(&[base, 0xb8 + reg_number])?;
        self.write_u64::<LittleEndian>(immediate)
    }
    fn emit_mov_reg_reg(&mut self, dst: X64Gpr, src: X64Gpr) -> io::Result<()> {
        let base = match (dst < X64Gpr::R8, src < X64Gpr::R8) {
            (true, true) => 0x48,
            (true, false) => 0x4c,
            (false, true) => 0x4d,
            (false, false) => 0x49,
        };

        let s = (src as u8) % 8;
        let d = (dst as u8) % 8;
        self.write(&[base, 0x89, (0b11 << 6) | (s << 3) | (d << 0)])?;

        Ok(())
    }
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
    fn emit_mov_reg_qword_ptr(&mut self, reg: X64Gpr, offset: usize) -> io::Result<()> {
        let reg_number = reg as u8 % 8;

        let base = if reg >= X64Gpr::R8 { 0x4c } else { 0x48 };
        // TODO: Document MOD R/M
        let mod_rm = (0b10 << 6) | (reg_number << 3) | (0b110 << 0);

        self.write(&[base, 0x8b, mod_rm])?;
        self.write_u64::<LittleEndian>(offset as u64)
    }
    fn emit_ret(&mut self) -> io::Result<()> {
        self.write_u8(0xc3)
    }

    fn emit_call_safe<const N: usize, I, O, C: Callable<N, I, O>>(
        &mut self,
        funct: C,
        args: &[u64; N],
    ) -> io::Result<()> {
        self.emit_call(funct, args)
    }
    fn emit_call<const N: usize, I, O, C: Callable<N, I, O>>(
        &mut self,
        funct: C,
        args: &[u64],
    ) -> io::Result<()> {
        static ARGS_REGS: &[X64Gpr] = &[X64Gpr::Rdi, X64Gpr::Rsi, X64Gpr::Rdx, X64Gpr::Rcx];
        for (&reg, &arg) in ARGS_REGS.iter().zip(args) {
            self.emit_movabs_reg(reg, arg)?;
        }
        let funct_addr = funct.addr();

        self.emit_movabs_reg(X64Gpr::Rax, funct_addr as u64)?;
        self.write(&[0xff, 0xd0])?;

        Ok(())
    }

    //-------------- ALU

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

        self.emit_mov_reg_reg(X64Gpr::Rdx, reg)?;
        // pass only two arguments as we already passed the third one (on RDX)
        self.emit_call(assert_eq_wrapper as fn(_, _, _), &[reg as u64, expected])?;

        for reg in save_regs.iter().rev() {
            self.emit_pop_reg(*reg).unwrap();
        }

        Ok(())
    }
}

impl<T: io::Write> Emitter for T {}
