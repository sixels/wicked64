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
    Add = 0,
    Or = 1,
    Sub = 5,
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
                let __base__ = (0b1001 << 3)
                    | (u8::from(#src >= Register::R8) << 2)
                    | (u8::from(#dst >= Register::R8) << 0);

                let __s__ = (#src as u8) % 8;
                let __d__ = (#dst as u8) % 8;
                let __mod_rm__ = (0b11 << 6) | (__s__ << 3) | (__d__ << 0);

                buf.emit_raw(&[__base__, 0x89, __mod_rm__]);
            }
        }
        (AddressingMode::Register(dst), AddressingMode::Immediate(imm)) => {
            quote! {
                let __d__ = (#dst as u8) % 8;
                let __base__ = __d__ + 0xb8;

                if #dst >= Register::R8 {
                    buf.emit_byte(0x41);
                }
                buf.emit_byte(__base__);
                buf.emit_dword(#imm as i32 as u32);
            }
        }
        (AddressingMode::Register(dst), AddressingMode::Direct(addr)) => {
            quote! {
                let __base__ = (0b1001 << 3) | (u8::from(#dst >= Register::R8) << 2);

                let __d__ = (#dst as u8) % 8;
                let __mod_rm__ = (0b00 << 6) | (__d__ << 3) | (0b100 << 0);

                buf.emit_raw(&[__base__, 0x8b, __mod_rm__, 0x25]);
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
                let __base__ = (0b1001 << 3)
                    | (u8::from(#dst >= Register::R8) << 2)
                    | (u8::from(#src >= Register::R8) << 0);

                let __s__ = (#src as u8) % 8;
                let __d__ = (#dst as u8) % 8;

                let __disp__ = if #neg { -(#disp as i32) } else { #disp as i32 };
                let __mode__ = u8::from(__disp__ != 0 || __s__ == Register::Rbp as u8) << u8::from(__disp__.abs() > i8::MAX as _);

                let __mod_rm__ = (__mode__ << 6) | (__d__ << 3) | (__s__ << 0);

                buf.emit_raw(&[__base__, 0x8b, __mod_rm__]);
                if __s__ == Register::Rsp as u8 {
                    buf.emit_byte(0x24);
                }

                if __mode__ != 0 {
                    if __disp__.abs() > i8::MAX as _ {
                        buf.emit_dword(__disp__ as u32);
                    } else {
                        buf.emit_byte(__disp__ as i8 as u8);
                    }
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
                let __base__ = (0b1001 << 3)
                    | (u8::from(#src >= Register::R8) << 2)
                    | (u8::from(#dst >= Register::R8) << 0);

                let __mode__ = u8::from(#disp != 0) << 1;
                let __s__ = (#src as u8) % 8;
                let __d__ = (#dst as u8) % 8;
                let __mod_rm__ = (__mode__ << 6) | (__s__ << 3) | (__d__ << 0);

                buf.emit_raw(&[__base__, 0x89, __mod_rm__]);
                if #dst == Register::Rsp {
                    buf.emit_byte(0x24);
                }
                if __mode__ != 0 {
                    let __disp__ = #disp as i32;
                    buf.emit_dword(if #neg { -__disp__ } else { __disp__ } as u32 );
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
                let __base__ = if #dst >= Register::R8 { 0x49 } else { 0x48 };
                let __d__ = #dst as u8 % 8;

                buf.emit_raw(&[__base__, 0xb8 + __d__]);
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
        if #reg >= Register::R8 {
            buf.emit_byte(0x41);
        }
        buf.emit_byte(0x50 | (#reg as u8 % 8));
    }
}

fn emit_pop(reg: AddrRegister) -> TokenStream {
    quote! {
        if #reg >= Register::R8 {
            buf.emit_byte(0x41);
        }
        buf.emit_byte(0x58 + (#reg as u8 % 8));
    }
}

// TODO: ADD TESTS
fn emit_op(op: Operation, dst: AddrRegister, src: AddressingMode) -> TokenStream {
    let id = op as u8;

    let mut ts = quote! {
        let __base__ = 0x48 | u8::from(#dst >= Register::R8);
        buf.emit_byte(__base__);
    };

    ts.extend(match src {
        AddressingMode::Immediate(imm) => {
            quote! {
                if #dst == Register::Rax {
                    buf.emit_byte((#id << 3) | 0b101);
                } else {
                    buf.emit_byte(0x81);
                }
                buf.emit_dword(#imm as u32);
            }
        }
        AddressingMode::Register(src) => {
            quote! {
                buf.emit_raw(&[
                    (#id << 3) | 0b001,
                    (0b11 << 6) | ((#src as u8) << 3) | ((#dst as u8) << 0)
                ]);
            }
        }

        _ => unimplemented!(),
    });

    ts
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
            let mut __stack_index__: usize = #stack_size;
            let mut __saved__: [Option<usize>; 16] = [None; 16];
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
            let __reg_args__: &[Register] = &#reg_args;
        });
    }

    macro_rules! save_reg {
        ($reg:expr, $cmp:expr) => {
            quote! {
                let __cmp_reg__: Register = $reg;
                if __reg_args__.iter().find(|&&r| r == __cmp_reg__ && $cmp).is_some() && __saved__[__cmp_reg__ as usize].is_none() {
                    __stack_index__ -= #ptr_size;
                    __saved__[__cmp_reg__ as usize] = Some(__stack_index__);
                    _emit_instructions! {
                        mov [rsp + $__stack_index__], %__cmp_reg__;
                    };
                }
            }
        };
        ($reg:expr) => {
            save_reg!($reg, true)
        };
    }

    for (dst, src) in ARGS_REGS.iter().zip(args.0.iter()) {
        match src {
            Argument::Register(src) => {
                ts.extend(quote! { let __dst__ = #dst; let __src__ = #src; });
                ts.extend(save_reg!(__dst__));
                ts.extend(quote! {
                    if let Some(__index__) = __saved__[__src__ as usize] {
                        _emit_instructions!{
                            mov %__dst__, [rsp + $__index__];
                        };
                    } else {
                        _emit_instructions!{
                            mov %__dst__, %__src__;
                        };
                    }
                });
            }
            Argument::Immediate(_) | Argument::Ref(_) => {
                match src {
                    Argument::Immediate(src) => {
                        ts.extend(quote! { let __dst__ = #dst; let __src__ = #src; })
                    }
                    Argument::Ref(src) => ts.extend(quote! {
                        let __dst__ = #dst;
                        let __src__ = #src;

                        assert_sized(__src__);
                        let __src__ = __src__ as *const _ as *const u8 as usize;
                    }),
                    _ => unreachable!(),
                }
                if !reg_args.is_empty() {
                    ts.extend(save_reg!(__dst__));
                }
                ts.extend(quote! {
                    _emit_instructions!{
                        movabs %__dst__, $__src__;
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
        let __funct__ = #funct as fn #cast_args -> _ as usize;
        _emit_instructions! {
            movabs rax, $__funct__;
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
