use proc_macro2::{Punct, Spacing, TokenStream};
use quote::{quote, TokenStreamExt};
use syn::{token::Paren, Ident};
use w64_codegen_types::register::Register;

use crate::{
    addressing::{AddrImmediate, AddrIndirect, AddrRegister, AddressingMode, Argument, CallArgs},
    instruction::Instruction,
    token::Slice,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Operation {
    Add = 0b000,
    Or = 0b001,
    Sub = 0b101,
}

pub fn emit(instruction: Instruction) -> TokenStream {
    match instruction {
        Instruction::Mov(dst, src) => emit_mov(dst, src),
        Instruction::Movabs(dst, src) => emit_movabs(dst, src),
        Instruction::Push(reg) => emit_push(reg),
        Instruction::Pop(reg) => emit_pop(reg),
        Instruction::Add(dst, src) => emit_op(Operation::Add, dst, src),
        Instruction::Or(dst, src) => emit_op(Operation::Or, dst, src),
        Instruction::Sub(dst, src) => emit_op(Operation::Sub, dst, src),
        // Instruction::Xor(_, _) => todo!(),
        Instruction::Call(addr) => emit_call(addr),
        Instruction::CallFn(funct, args) => emit_call_fn(funct, args),
        Instruction::Ret => quote!(buf.emit_byte(0xc3)),
        _ => todo!("Instruction not implemented yet: {}", instruction),
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
                let d = (#dst as u8) % 8;
                let base = d + 0xb8;

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
        (
            AddressingMode::Indirect(AddrIndirect { reg: dst, disp }),
            AddressingMode::Register(src),
        ) => {
            let (neg, disp) = match disp {
                Some((neg, disp)) => (neg, disp),
                None => (false, AddrImmediate::Lit(0)),
            };

            quote! {
                let base = (0b1001 << 3)
                    | (u8::from(#src >= Register::R8) << 2)
                    | (u8::from(#dst >= Register::R8) << 0);

                let mode = u8::from(#disp != 0) << 1;
                let s = (#src as u8) % 8;
                let d = (#dst as u8) % 8;
                let mod_rm = (mode << 6) | (s << 3) | (d << 0);

                buf.emit_raw(&[base, 0x89, mod_rm]);
                if #dst == Register::Rsp {
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
        AddressingMode::Register(AddrRegister::Var(dst)) => {
            quote! {
                let base = if #dst >= Register::R8 { 0x49 } else { 0x48 };
                let d = #dst as u8 % 8;

                buf.emit_raw(&[base, 0xb8 + d]);
                buf.emit_qword(#src as u64);
            }
        }
        AddressingMode::Register(AddrRegister::Lit(dst)) => {
            let base: u8 = if dst >= Register::R8 { 0x49 } else { 0x48 };
            let d = dst as u8 % 8;
            quote!(buf.emit_raw(&[#base, 0xb8 + #d]); buf.emit_qword(#src as u64);)
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
        buf.emit_byte(0x50 | r);
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

// TODO: ADD TESTS
fn emit_op(op: Operation, dst: AddrRegister, src: AddressingMode) -> TokenStream {
    let op = op as u8;
    let (op_code, sufix) = match src {
        AddressingMode::Immediate(imm) => (0b101, quote! { buf.emit_dword(#imm as u32); }),
        AddressingMode::Register(src) => (
            0b001,
            quote! { buf.emit_byte((0b11 << 6) | ((#src as u8) << 3) | ((#dst as u8) << 0)); },
        ),
        _ => unimplemented!(),
    };
    let op_code = (op << 3) | op_code;

    quote! {
        let base = 0x48 | u8::from(#dst >= Register::R8);
        if #dst == Register::Rax {
            buf.emit_raw(&[base, #op_code]);
        } else {
            let mod_rm = (0b11 << 6) | ((#op as u8) << 3) | (#dst as u8);
            buf.emit_raw(&[base, 0x81, mod_rm]);
        }
        #sufix
    }
}

fn emit_call(addr: AddressingMode) -> TokenStream {
    match &addr {
        AddressingMode::Register(reg)
        | AddressingMode::Indirect(AddrIndirect { reg, disp: None }) => {
            let mod_rm: u8 = match &addr {
                AddressingMode::Register(_) => 0b11010,
                AddressingMode::Indirect(_) => 0b00010,
                _ => unreachable!(),
            } << 3;
            quote! {
                if #reg >= Register::R8 { buf.emit_byte(0x41); }
                buf.emit_raw(&[0xff, #mod_rm | ((#reg as u8) & 0b111)]);
            }
        }
        _ => unimplemented!(),
    }
}

fn emit_call_fn(funct: Ident, args: CallArgs) -> TokenStream {
    static ARGS_REGS: &[Register] = &[
        Register::Rdi,
        Register::Rsi,
        Register::Rdx,
        Register::Rcx,
        Register::R8,
    ];
    assert!(
        args.len() <= ARGS_REGS.len(),
        "Argument list exceeds the limit of {} arguments.",
        ARGS_REGS.len() - 1
    );

    let mut ts = TokenStream::new();
    let mut tail = TokenStream::new();

    let reg_args = args
        .iter()
        .filter_map(|arg| match arg {
            Argument::Register(reg_arg) => Some(reg_arg),
            _ => None,
        })
        .collect::<Slice<_>>();

    let ptr_size = std::mem::size_of::<usize>();
    let stack_size = reg_args.len() * ptr_size;

    // save rax, as we will use it to call the function
    ts.extend(quote! {
        _emit_instructions! {
            push rax;
        };
    });
    // set the stack size
    if stack_size > 0 {
        ts.extend(quote! {
            let mut stack_index = #stack_size;
            let mut saved: [Option<usize>; 16] = [None; 16];
            _emit_instructions! {
                sub rsp, #stack_size;
            };
        });
        tail.extend(quote! {
            _emit_instructions! {
                add rsp, #stack_size;
            };
        });
    }

    if !reg_args.is_empty() {
        ts.extend(quote! {
            let reg_args: &[Register] = &#reg_args;
        });
    }

    macro_rules! save_reg {
        ($regs:expr, $cmp:expr) => {
            quote! {
                if $regs.iter().find(|&&r| r == dst && $cmp).is_some() && saved[dst as usize].is_none() {
                    stack_index -= #ptr_size;
                    saved[dst as usize] = Some(stack_index);
                    _emit_instructions! {
                        mov [rsp + $stack_index], %dst;
                    };
                }
            }
        };
        ($regs:expr) => {
            save_reg!($regs, true)
        };
    }

    for (dst, src) in ARGS_REGS.iter().zip(args.0.iter()) {
        match src {
            Argument::Register(src) => {
                ts.extend(quote! { let dst = #dst; let src = #src; });
                ts.extend(save_reg!(reg_args));
                ts.extend(quote! {
                    if let Some(index) = saved[src as usize] {
                        _emit_instructions!{
                            mov %dst, [rsp + $index];
                        };
                    } else {
                        _emit_instructions!{
                            mov %dst, %src;
                        };
                    }
                });
            }
            Argument::Immediate(_) | Argument::Ref(_) => {
                match src {
                    Argument::Immediate(src) => {
                        ts.extend(quote! { let dst = #dst; let src = #src; })
                    }
                    Argument::Ref(src) => ts.extend(quote! {
                        let dst = #dst;
                        let src = #src;

                        assert_sized(src);
                        let src = src as *const _ as *const u8 as usize;
                    }),
                    _ => unreachable!(),
                }
                if !reg_args.is_empty() {
                    ts.extend(save_reg!(reg_args));
                }
                ts.extend(quote! {
                    _emit_instructions!{
                        movabs %dst, $src;
                    };
                });
            }
        }
    }

    // (_,_,_,...)
    let mut cast_args = TokenStream::new();
    Paren::default().surround(&mut cast_args, |ts| {
        ts.append_separated(
            args.iter().map(|_| quote!(_)),
            Punct::new(',', Spacing::Joint),
        );
    });
    // call the function
    ts.extend(quote! {
        let funct = #funct as fn #cast_args -> _ as usize;
        _emit_instructions! {
            movabs rax, $funct;
            call rax;
        }
    });
    ts.extend(tail);
    ts.extend(quote! {
        _emit_instructions! {
            pop rax;
        };
    });

    ts
}
