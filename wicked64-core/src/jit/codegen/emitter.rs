use std::io;

use byteorder::WriteBytesExt;

use super::register::X64Gpr;

pub trait Emitter: io::Write {
    fn emit_push_reg(&mut self, reg: X64Gpr) -> io::Result<()> {
        self.write_u8(0x50 + reg as u8)
    }
    fn emit_pop_reg(&mut self, reg: X64Gpr) -> io::Result<()> {
        self.write_u8(0x58 + reg as u8)
    }
}

impl<T: io::Write> Emitter for T {}
