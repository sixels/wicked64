use byteorder::{LittleEndian, WriteBytesExt};
use w64_codegen_types::encoding::{ModRM, Rex};

pub struct Emitter {
    buffer: Vec<u8>,
}

impl Emitter {
    pub fn new() -> Emitter {
        Default::default()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
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

    pub fn encode_instruction(
        &mut self,
        rex: Option<Rex>,
        opcode: u8,
        mod_rm: Option<ModRM>,
        sib: Option<u8>,
        disp: Option<i32>,
        imm: Option<u32>,
    ) {
        if let Some(rex) = rex {
            self.emit_byte(rex.value());
        }

        self.emit_byte(opcode);

        if let Some(mod_rm) = mod_rm {
            self.emit_byte(mod_rm.value());
        }

        if let Some(sib) = sib {
            self.emit_byte(sib);
        }

        if let Some(disp) = disp {
            if disp.abs() > i8::MAX as i32 {
                self.emit_dword(disp as u32);
            } else {
                self.emit_byte(disp as u8);
            }
        }

        if let Some(imm) = imm {
            self.emit_dword(imm);
        }
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
