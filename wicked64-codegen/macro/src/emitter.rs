use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    addressing::{AddrImmediate, AddrIndirect, AddrRegister, AddressingMode},
    instruction::Instruction,
};

pub fn emit(instruction: Instruction) -> TokenStream {
    match instruction {
        Instruction::Mov(dst, src) => emit_mov(dst, src),
        Instruction::Movabs(dst, src) => emit_movabs(dst, src),
        Instruction::Push(reg) => emit_push(reg),
        Instruction::Pop(reg) => emit_pop(reg),
        // Instruction::Add(_, _) => todo!(),
        // Instruction::Or(_, _) => todo!(),
        // Instruction::Sub(_, _) => todo!(),
        // Instruction::Xor(_, _) => todo!(),
        // Instruction::Ret => todo!(),
        _ => todo!("Instruction not implemented yet"),
    }
}

fn emit_mov(dst: AddressingMode, src: AddressingMode) -> TokenStream {
    match (dst, src) {
        (AddressingMode::Immediate(_), _) => panic!("Invalid mov destination"),
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
                let base = (0b1001 << 3) | (u8::from(#dst >= Register::R8) << 2);

                let d = (#dst as u8) % 8;
                let mod_rm = (0b00 << 6) | (d << 3) | (0b100 << 0);

                buf.emit_raw(&[base, 0x8b, mod_rm, 0x25]);
                buf.emit_dword(#addr as i32 as u32);
            }
        }
        (AddressingMode::Register(dst), AddressingMode::Indirect(src)) => {
            let AddrIndirect { reg: src, disp } = src;

            let (neg, disp) = match disp {
                Some((neg, disp)) => (neg, disp),
                None => (false, AddrImmediate::Lit(0)),
            };

            quote! {
                let base = (0b1001 << 3)
                    | (u8::from(#dst >= Register::R8) << 2)
                    | (u8::from(#src >= Register::R8) << 0);

                let mode = u8::from(#disp != 0) << 1;
                let s = (#src as u8) % 8;
                let d = (#dst as u8) % 8;
                let mod_rm = (mode << 6) | (d << 3) | (s << 0);

                buf.emit_raw(&[base, 0x8b, mod_rm]);
                if #src == Register::Rsp {
                    buf.emit_byte(0x24);
                }
                if mode != 0 {
                    let disp = #disp as i32;
                    buf.emit_dword(if #neg { -disp } else { disp } as u32 );
                }
            }
        }
        (a, b) => todo!("mov {a}, {b}"),
    }
}

fn emit_movabs(dst: AddressingMode, src: AddrImmediate) -> TokenStream {
    match dst {
        AddressingMode::Immediate(_) => panic!("Invalid movabs destination"),
        AddressingMode::Register(dst) => {
            quote! {
                let d = #dst as u8 % 8;
                let base = if #dst >= X64Gpr::R8 { 0x49 } else { 0x48 };

                buf.emit_raw(&[base, 0xb8 + d]);
                buf.emit_qword(#src);
            }
        }
        a => todo!("movabs {a}, {src}"),
    }
}

fn emit_push(reg: AddrRegister) -> TokenStream {
    quote! {
        let r = #reg as u8 % 8;
        if #reg >= Register::R8 {
            buf.emit_byte(0x41);
        }
        buf.emit_byte(0x50 + r);
    }
}

fn emit_pop(reg: AddrRegister) -> TokenStream {
    quote! {
        let r = #reg as u8 % 8;
        if #reg >= Register::R8 {
            buf.emit_byte(0x41);
        }
        buf.emit_byte(0x58 + r);
    }
}
