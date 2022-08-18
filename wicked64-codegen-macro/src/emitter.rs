use proc_macro2::TokenStream;

use crate::{addressing::AddressingMode, instruction::Instruction, register::Register};

pub fn emit(instruction: Instruction) -> TokenStream {
    match instruction {
        Instruction::Mov(dst, src) => emit_mov(dst, src),
        _ => todo!("Instruction not implemented yet"),
    }
}

fn emit_mov(dst: AddressingMode, src: AddressingMode) -> TokenStream {
    match (dst, src) {
        (AddressingMode::Register(dst), AddressingMode::Register(src)) => {
            debug_assert!(dst != Register::Rsp);
            let base = (0b1001 << 3)
                | (u8::from(src >= Register::R8) << 2)
                | (u8::from(dst >= Register::R8) << 0);

            let s = (src as u8) % 8;
            let d = (dst as u8) % 8;
            let mod_rm = (0b11 << 6) | (s << 3) | (d << 0);

            // self.write(&[base, 0x89, mod_rm])?;
        }
        (AddressingMode::Register(dst), AddressingMode::Immediate(im)) => {
            debug_assert!(dst != Register::Rsp);
            let dst_n = (dst as u8) % 8;

            let base = dst_n + 0xb8;
            let im = im.0 as i32;

            // if dst >= Register::R8 {
            // self.write_u8(0x41)?;
            // }
            // self.write_u8(base)?;
            // self.write_i32::<LittleEndian>(im)?;
        }
        (AddressingMode::Register(dst), AddressingMode::Direct(addr)) => {
            debug_assert!(dst != Register::Rsp);
            let base = (0b1001 << 3) | (u8::from(dst >= Register::R8) << 2);

            let d = (dst as u8) % 8;
            let mod_rm = (0b00 << 6) | (d << 3) | (0b100 << 0);

            // self.write(&[base, 0x8b, mod_rm, 0x25])?;
            // self.write_i32::<LittleEndian>(addr)?;
        }
        (AddressingMode::Register(dst), AddressingMode::Indirect(src)) => {
            debug_assert!(dst != Register::Rsp);
            let base = (0b1001 << 3)
                | (u8::from(dst >= Register::R8) << 2)
                | (u8::from(src.reg >= Register::R8) << 0);

            let mode = u8::from(src.disp != 0) << 1;
            let s = (src.reg as u8) % 8;
            let d = (dst as u8) % 8;
            let mod_rm = (mode << 6) | (d << 3) | (s << 0);

            // self.write(&[base, 0x8b, mod_rm])?;
            // if src == Register::Rsp {
            //     self.write_u8(0x24)?;
            // }
            // if mode != 0 {
            //     self.write_i32::<LittleEndian>(disp)?;
            // }
        }
        _ => unimplemented!(),
    }

    todo!()
}
