use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    addressing::{AddrIndirect, AddressingMode},
    instruction::Instruction,
};

pub fn emit(instruction: Instruction) -> TokenStream {
    match instruction {
        Instruction::Mov(dst, src) => emit_mov(dst, src),
        _ => todo!("Instruction not implemented yet"),
    }
}

fn emit_mov(dst: AddressingMode, src: AddressingMode) -> TokenStream {
    match (dst, src) {
        (AddressingMode::Register(dst), AddressingMode::Register(src)) => {
            quote! {
                let base = (0b1001 << 3)
                    | (u8::from(#src >= Register::R8) << 2)
                    | (u8::from(#dst >= Register::R8) << 0);

                let s = (#src as u8) % 8;
                let d = (#dst as u8) % 8;
                let mod_rm = (0b11 << 6) | (s << 3) | (d << 0);

                buf.emit_raw(&[base, 0x89, mod_rm]);
            }
        }
        (AddressingMode::Register(dst), AddressingMode::Immediate(imm)) => {
            quote! {
                let dst_n = (#dst as u8) % 8;

                let base = dst_n + 0xb8;

                if #dst >= Register::R8 {
                    buf.emit_byte(0x41);
                }
                buf.emit_byte(base);
                buf.emit_dword(#imm as i32 as u32);
            }
        }
        (AddressingMode::Register(dst), AddressingMode::Direct(addr)) => {
            quote! {
                let base = (0b1001 << 3) | (u8::from(#dst >= wicked64_codegen_types::register::Register::R8) << 2);

                let d = (#dst as u8) % 8;
                let mod_rm = (0b00 << 6) | (d << 3) | (0b100 << 0);

                buf.emit_raw(&[base, 0x8b, mod_rm, 0x25]);
                buf.emit_dword(#addr as i32 as u32);
            }
        }
        (AddressingMode::Register(dst), AddressingMode::Indirect(src)) => {
            let AddrIndirect { reg: src, disp, .. } = src;

            quote! {
                let base = (0b1001 << 3)
                    | (u8::from(#dst >= Register::R8) << 2)
                    | (u8::from(#src >= Register::R8) << 0);

                let mode = u8::from(#disp != 0) << 1;
                let s = (#src as u8) % 8;
                let d = (#dst as u8) % 8;
                let mod_rm = (mode << 6) | (d << 3) | (s << 0);

                buf.emit_raw(&[base, 0x8b, mod_rm]);
                if #src == wicked64_codegen_types::register::Register::Rsp {
                    buf.emit_byte(0x24);
                }
                if mode != 0 {
                    buf.emit_dword(#disp);
                }
            }
        }
        _ => unimplemented!(),
    }
}
