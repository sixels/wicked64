use byteorder::{LittleEndian, WriteBytesExt};

pub struct Emitter {
    buffer: Vec<u8>,
}

impl Emitter {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
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