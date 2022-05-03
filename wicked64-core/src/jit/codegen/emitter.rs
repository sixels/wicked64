use std::io;

use byteorder::WriteBytesExt;
use wicked64_arena as arena;

use crate::jit::wasm;

pub trait Emitter: io::Write {
    /// Load `ptr` value as 8-bit.
    fn emit_i64_load8(&mut self, ptr: &u64, signed: bool) {
        self.emit_push_ptr_offset(ptr);
        self.write_all(&[0x30 | !signed as u8, 0x00, 0x00]).unwrap()
    }
    /// Load `ptr` value as 16-bit.
    fn emit_i64_load16(&mut self, ptr: &u64, signed: bool) {
        self.emit_push_ptr_offset(ptr);
        self.write_all(&[0x32 | !signed as u8, 0x01, 0x00]).unwrap()
    }
    /// Load `ptr` value as 32-bit.
    fn emit_i64_load32(&mut self, ptr: &u64, signed: bool) {
        self.emit_push_ptr_offset(ptr);
        self.write_all(&[0x34 | !signed as u8, 0x02, 0x00]).unwrap()
    }

    /// Store `src` value as 8-bit into `dst`.
    fn emit_i64_store8(&mut self, dst: &u8, src: &u64, signed: bool) {
        self.emit_push_ptr_offset(dst);
        self.emit_i64_load8(src, signed);
        self.write_all(&[0x3c, 0x00, 0x00]).unwrap()
    }
    /// Store `src` value as 16-bit into `dst`.
    fn emit_i64_store16(&mut self, dst: &u16, src: &u64, signed: bool) {
        self.emit_push_ptr_offset(dst);
        self.emit_i64_load16(src, signed);
        self.write_all(&[0x3d, 0x01, 0x00]).unwrap()
    }
    /// Store `src` value as 32-bit into `dst`.
    fn emit_i64_store32(&mut self, dst: &u32, src: &u64, signed: bool) {
        self.emit_push_ptr_offset(dst);
        self.emit_i64_load32(src, signed);
        self.write_all(&[0x3e, 0x02, 0x00]).unwrap()
    }

    /// Store `src` value as 8-bit into `dst`.
    fn emit_i32_store8(&mut self, dst: &u8, src: &u32, signed: bool) {
        self.emit_push_ptr_offset(dst);
        self.emit_i32_load8(src, signed);
        self.write_all(&[0x3a, 0x00, 0x00]).unwrap()
    }
    /// Store `src` value as 16-bit into `dst`.
    fn emit_i32_store16(&mut self, dst: &u16, src: &u32, signed: bool) {
        self.emit_push_ptr_offset(dst);
        self.emit_i32_load16(src, signed);
        self.write_all(&[0x3b, 0x01, 0x00]).unwrap()
    }

    /// Load `ptr` value as 8-bit.
    fn emit_i32_load8(&mut self, ptr: &u32, signed: bool) {
        self.emit_push_ptr_offset(ptr);
        self.write_all(&[0x2c | (!signed) as u8, 0x00, 0x00]).unwrap()
    }
    /// Load `ptr` value as 16-bit.
    fn emit_i32_load16(&mut self, ptr: &u32, signed: bool) {
        self.emit_push_ptr_offset(ptr);
        self.write_all(&[0x2e | (!signed) as u8, 0x01, 0x00]).unwrap()
    }
    fn emit_i32_from_i64(&mut self, ptr: &u64) {
        self.emit_push_ptr_offset(ptr);
        self.write_all(&[0x28, 0x02, 0x00]).unwrap();
    }

    /// Load the value of pointer `ptr` and push into the stack.
    /// This is equivalent to `i64.load ptr`.
    fn emit_i64_load(&mut self, ptr: &u64) {
        self.emit_push_ptr_offset(ptr);
        self.write_all(&[0x29, 0x03, 0x00]).unwrap(); // i64.load 0x03 0x00
    }

    /// Load the value of pointer `ptr` and push into the stack.
    /// This is equivalent to `i32.load ptr`.
    fn emit_i32_load(&mut self, ptr: &u32) {
        self.emit_push_ptr_offset(ptr);
        self.write_all(&[0x28, 0x02, 0x00]).unwrap(); // i64.load 0x03 0x00
    }

    /// Store the value of `src` into `dst` dynamically.
    /// This is equivalent to `i64.store (dst) (src)`.
    fn emit_i64_store(&mut self, dst: &u64, src: &u64) {
        self.emit_push_ptr_offset(dst);
        self.emit_i64_load(src);
        self.write_all(&[0x37, 0x03, 0x00]).unwrap();
    }
    /// Store `src` into `dst`.
    /// This is equivalent to `i64.store (dst) (src)`.
    fn emit_i64_store_const(&mut self, dst: &u64, src: u64) {
        self.emit_push_ptr_offset(dst);
        self.emit_i64_const(src);
        self.write_all(&[0x37, 0x03, 0x00]).unwrap();
    }

    /// Store the value of `src` into `dst` dynamically.
    /// This is equivalent to `i32.store (dst) (src)`.
    fn emit_i32_store(&mut self, dst: &u32, src: &u32) {
        self.emit_push_ptr_offset(dst);
        self.emit_i32_load(src);
        self.write_all(&[0x36, 0x02, 0x00]).unwrap();
    }
    /// Store `src` into `dst`.
    /// This is equivalent to `i64.store (dst) (src)`.
    fn emit_i32_store_const(&mut self, dst: &u64, src: u32) {
        self.emit_push_ptr_offset(dst);
        self.emit_i32_const(src);
        self.write_all(&[0x36, 0x02, 0x00]).unwrap();
    }

    /// Push the offset of `ptr` into the stack.
    /// This is equivalent to `i32.const *ptr`.
    fn emit_push_ptr_offset<T>(&mut self, ptr: &T) {
        self.emit_i32_const(arena::offset_of!(ptr) as u32);
    }

    /// Equivalent to WASM instruction `i64.const v`.
    fn emit_i64_const(&mut self, v: u64) {
        self.write_u8(0x42).unwrap();
        wasm::leb128::signed::write_i64(self, v as i64).unwrap();
    }

    /// Equivalent to WASM instruction `i32.const v`.
    fn emit_i32_const(&mut self, v: u32) {
        self.write_u8(0x41).unwrap();
        wasm::leb128::signed::write_i32(self, v as i32).unwrap();
    }
}

impl<T: io::Write> Emitter for T {}
