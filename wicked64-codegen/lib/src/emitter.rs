use crate::{self as w64_codegen, register::CALLEE_SAVED_REGISTERS};

use crate::emit;
use byteorder::{LittleEndian, WriteBytesExt};

pub struct Emitter {
    buffer: Vec<u8>,
}

impl Emitter {
    pub fn new() -> Self {
        let mut emitter = Self::default();

        emit!(emitter,
            push rax;
        );
        for reg in CALLEE_SAVED_REGISTERS {
            emit!(emitter,
                push %reg;
            );
        }

        emitter
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    pub fn finalize(self) -> region::Result<ExecBuffer> {
        let mut emitter = self;

        // retrieve callee-saved registers
        for reg in CALLEE_SAVED_REGISTERS.iter().copied().rev() {
            emit!(emitter,
                pop %reg;
            );
        }
        emit!(emitter,
            pop rcx;
            ret;
        );

        unsafe { emitter.make_exec() }
    }

    pub unsafe fn make_exec(self) -> region::Result<ExecBuffer> {
        ExecBuffer::new(self.buffer)
    }

    pub fn emit_raw(&mut self, raw_bytes: &[u8]) {
        self.buffer.extend_from_slice(raw_bytes);
    }
    pub fn emit_byte(&mut self, byte: u8) {
        self.buffer.push(byte);
    }
    pub fn emit_word(&mut self, word: u16) {
        self.buffer.write_u16::<LittleEndian>(word).unwrap();
    }
    pub fn emit_dword(&mut self, dword: u32) {
        self.buffer.write_u32::<LittleEndian>(dword).unwrap();
    }
    pub fn emit_qword(&mut self, qword: u64) {
        self.buffer.write_u64::<LittleEndian>(qword).unwrap();
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self { buffer: Vec::new() }
    }
}
pub struct ExecBuffer {
    ptr: *const u8,
    buf: Vec<u8>,
}

impl ExecBuffer {
    unsafe fn new(buffer: Vec<u8>) -> region::Result<Self> {
        let ptr = buffer.as_ptr();

        region::protect(ptr, buffer.len(), region::Protection::READ_WRITE_EXECUTE)?;

        Ok(Self { buf: buffer, ptr })
    }

    pub fn execute(&self) {
        unsafe {
            let f: unsafe extern "C" fn() = std::mem::transmute(self.ptr);
            f();
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.buf.as_slice()
    }
}
