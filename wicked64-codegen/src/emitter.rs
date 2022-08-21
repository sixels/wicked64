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

// #[cfg(test)]
// mod tests {
//     use super::*;

//     macro_rules! _emit {
//         (mov $dst:tt, $src:tt) => {{
//             let mut buf = Vec::new();
//             buf.emit_mov(_addressing!($dst), _addressing!($src))
//                 .unwrap();
//             buf
//         }};
//     }
//     macro_rules! _register {
//         (rax) => {
//             X64Gpr::Rax
//         };
//         (rbx) => {
//             X64Gpr::Rbx
//         };
//         (rcx) => {
//             X64Gpr::Rcx
//         };
//         (rdx) => {
//             X64Gpr::Rdx
//         };
//         (rsi) => {
//             X64Gpr::Rsi
//         };
//         (rdi) => {
//             X64Gpr::Rdi
//         };
//         (rsp) => {
//             X64Gpr::Rsp
//         };
//         (rbp) => {
//             X64Gpr::Rbp
//         };
//         (r8) => {
//             X64Gpr::R8
//         };
//         (r9) => {
//             X64Gpr::R9
//         };
//         (r10) => {
//             X64Gpr::R10
//         };
//         (r11) => {
//             X64Gpr::R11
//         };
//         (r12) => {
//             X64Gpr::R12
//         };
//         (r13) => {
//             X64Gpr::R13
//         };
//         (r14) => {
//             X64Gpr::R14
//         };
//         (r15) => {
//             X64Gpr::R15
//         };
//     }
//     macro_rules! _addressing {
//         ([$reg:tt + $displacement:literal]) => {
//             AddressingMode::IndirectDisplacement(_register!($reg), $displacement)
//         };
//         ([$reg:tt - $displacement:literal]) => {
//             AddressingMode::IndirectDisplacement(_register!($reg), -$displacement)
//         };
//         ([$addr:literal]) => {
//             AddressingMode::Direct($addr)
//         };
//         ([$reg:tt]) => {
//             AddressingMode::Indirect(_register!($reg))
//         };
//         ($im:literal) => {
//             AddressingMode::Immediate($im)
//         };
//         ($reg:tt) => {
//             AddressingMode::Register(_register!($reg))
//         };
//     }

//     #[test]
//     fn it_should_emit_mov_with_reg_reg_addressing_mode() {
//         // mov rcx,r8
//         let code = _emit!(mov rcx, r8);
//         assert_eq!(code, vec![0x4c, 0x89, 0xc1]);
//         // mov r9, rax
//         let code = _emit!(mov r9, rax);
//         assert_eq!(code, vec![0x49, 0x89, 0xc1]);
//         // mov rcx, rbx
//         let code = _emit!(mov rcx, rbx);
//         assert_eq!(code, vec![0x48, 0x89, 0xd9]);
//         // mov r9, r11
//         let code = _emit!(mov r9, r11);
//         assert_eq!(code, vec![0x4d, 0x89, 0xd9]);
//     }

//     #[test]
//     fn it_should_emit_mov_with_reg_immediate_addressing_mode() {
//         //mov rcx, 0x3412
//         let code = _emit!(mov rcx, 0x3412);
//         assert_eq!(code, vec![0xb9, 0x12, 0x34, 0x00, 0x00]);
//         //mov rbx, 0x3412
//         let code = _emit!(mov rbx, 0x3412);
//         assert_eq!(code, vec![0xbb, 0x12, 0x34, 0x00, 0x00]);
//         //mov r9, 0x3412
//         let code = _emit!(mov r9, 0x3412);
//         assert_eq!(code, vec![0x41, 0xb9, 0x12, 0x34, 0x00, 0x00]);
//         //mov r11, 0x3412
//         let code = _emit!(mov r11, 0x3412);
//         assert_eq!(code, vec![0x41, 0xbb, 0x12, 0x34, 0x00, 0x00]);
//         //mov rax, 0x3412
//         let code = _emit!(mov rax, 0x3412);
//         assert_eq!(code, vec![0xb8, 0x12, 0x34, 0x00, 0x00]);
//         //mov r8, 0x3412
//         let code = _emit!(mov r8, 0x3412);
//         assert_eq!(code, vec![0x41, 0xb8, 0x12, 0x34, 0x00, 0x00]);
//     }

//     #[test]
//     fn it_should_emit_mov_with_reg_direct_addressing_mode() {
//         // mov rcx, [0x78563412]
//         let code = _emit!(mov rcx, [0x78563412]);
//         assert_eq!(code, vec![0x48, 0x8b, 0x0c, 0x25, 0x12, 0x34, 0x56, 0x78]);
//         // mov rbx, [0x78563412]
//         let code = _emit!(mov rbx, [0x78563412]);
//         assert_eq!(code, vec![0x48, 0x8b, 0x1c, 0x25, 0x12, 0x34, 0x56, 0x78]);
//         // mov r9, [0x78563412]
//         let code = _emit!(mov r9, [0x78563412]);
//         assert_eq!(code, vec![0x4c, 0x8b, 0x0c, 0x25, 0x12, 0x34, 0x56, 0x78]);
//         // mov r11, [0x78563412]
//         let code = _emit!(mov r11, [0x78563412]);
//         assert_eq!(code, vec![0x4c, 0x8b, 0x1c, 0x25, 0x12, 0x34, 0x56, 0x78]);
//         // mov rax, [0x78563412]
//         let code = _emit!(mov rax, [0x78563412]);
//         assert_eq!(code, vec![0x48, 0x8b, 0x04, 0x25, 0x12, 0x34, 0x56, 0x78]);
//         // mov r8, [0x78563412]
//         let code = _emit!(mov r8, [0x78563412]);
//         assert_eq!(code, vec![0x4c, 0x8b, 0x04, 0x25, 0x12, 0x34, 0x56, 0x78]);
//     }

//     #[test]
//     fn it_should_emit_mov_with_reg_indirect_addressing_mode() {
//         // mov rcx, [r8]
//         let code = _emit!(mov rcx, [r8]);
//         assert_eq!(code, vec![0x49, 0x8b, 0x08]);
//         // mov r9, [rax]
//         let code = _emit!(mov r9, [rax]);
//         assert_eq!(code, vec![0x4c, 0x8b, 0x08]);
//         // mov rcx, [rbx]
//         let code = _emit!(mov rcx, [rbx]);
//         assert_eq!(code, vec![0x48, 0x8b, 0x0b]);
//         // mov r9, [r11]
//         let code = _emit!(mov r9, [r11]);
//         assert_eq!(code, vec![0x4d, 0x8b, 0x0b]);
//         // mov rax, [r9]
//         let code = _emit!(mov rax, [r9]);
//         assert_eq!(code, vec![0x49, 0x8b, 0x01]);
//     }

//     #[test]
//     fn it_should_emit_mov_with_reg_indirect_displacement_addressing_mode() {
//         // mov rcx, [r8 + 0x78563412]
//         let code = _emit!(mov rcx, [r8 + 0x78563412]);
//         assert_eq!(code, vec![0x49, 0x8b, 0x88, 0x12, 0x34, 0x56, 0x78]);
//         // mov r9, [rax + 0x78563412]
//         let code = _emit!(mov r9, [rax + 0x78563412]);
//         assert_eq!(code, vec![0x4c, 0x8b, 0x88, 0x12, 0x34, 0x56, 0x78]);
//         // mov rcx, [rbx + 0x78563412]
//         let code = _emit!(mov rcx, [rbx + 0x78563412]);
//         assert_eq!(code, vec![0x48, 0x8b, 0x8b, 0x12, 0x34, 0x56, 0x78]);
//         // mov r9, [r11 + 0x78563412]
//         let code = _emit!(mov r9, [r11 + 0x78563412]);
//         assert_eq!(code, vec![0x4d, 0x8b, 0x8b, 0x12, 0x34, 0x56, 0x78]);
//         // mov rax, [rsp + 0x78563412]
//         let code = _emit!(mov rax, [rsp + 0x78563412]);
//         assert_eq!(code, vec![0x48, 0x8b, 0x84, 0x24, 0x12, 0x34, 0x56, 0x78]);
//         // mov rax, [rsi + 0x78563412]
//         let code = _emit!(mov rax, [rsi + 0x78563412]);
//         assert_eq!(code, vec![0x48, 0x8b, 0x86, 0x12, 0x34, 0x56, 0x78]);
//     }

//     #[test]
//     fn it_should_emit_mov_with_direct_reg_addressing_mode() {
//         todo!()
//     }
// }
