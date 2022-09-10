use proc_macro2::{Punct, Spacing, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
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

impl ToTokens for Operation {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            Operation::Add => 0u8.to_tokens(tokens),
            a => (a as u8).to_tokens(tokens),
        }
    }
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

fn emit_mov(addr_dst: AddressingMode, addr_src: AddressingMode) -> TokenStream {
    match (addr_dst.clone(), addr_src.clone()) {
        (AddressingMode::Immediate(_), _) => panic!("Invalid mov destination"),
        (AddressingMode::Register(dst), AddressingMode::Register(src)) => {
            quote! {
                let __rex__ = Rex(true, #src >= Register::R8, false, #dst >= Register::R8);
                let __mod_rm__ = ModRM(0b11, #src.value(), #dst.value());
                buf.encode_instruction(Some(__rex__), 0x89, Some(__mod_rm__), None, None, None);
            }
        }
        (AddressingMode::Register(dst), AddressingMode::Immediate(imm)) => {
            quote! {
                let __rex__ = if #dst >= Register::R8 { Some(Rex(false,false,false,true)) } else { None };
                buf.encode_instruction(__rex__, 0xb8 | #dst.value(), None, None, None, Some(#imm as i32 as u32));
            }
        }
        (AddressingMode::Register(dst), AddressingMode::Direct(addr)) => {
            quote! {
                let __rex__ = Rex(true, #dst >= Register::R8, false, false);
                let __mod_rm__ = ModRM(0b00, #dst.value(), 0b100);
                buf.encode_instruction(Some(__rex__), 0x8b, Some(__mod_rm__), Some(0x25u8), None, Some(#addr as i32 as u32));
            }
        }
        (
            AddressingMode::Register(dst),
            AddressingMode::Indirect(AddrIndirect { reg: src, disp }),
        )
        | (
            AddressingMode::Indirect(AddrIndirect { reg: src, disp }),
            AddressingMode::Register(dst),
        ) => {
            let opcode = if let AddressingMode::Indirect(_) = addr_dst {
                0x89u8
            } else {
                0x8bu8
            };

            let (neg, disp) = match disp {
                Some((neg, disp)) => (neg, disp),
                None => (false, AddrImmediate::Lit(0)),
            };

            quote! {
                let __rex__ = Rex(true, #dst >= Register::R8, false, #src >= Register::R8);

                let __disp__ = if #neg { -(#disp as i32) } else { #disp as i32 };

                let __mod__ = u8::from(__disp__ != 0 || #src.value() == Register::Rbp.value()) << u8::from(__disp__.abs() > i8::MAX as _);
                let __mod_rm__ = ModRM(__mod__, #dst.value(), #src.value());

                let __sib__ = (#src.value() == Register::Rsp.value()).then(|| 0x24);

                let __disp__ = (__mod__ != 0).then(|| __disp__);

                buf.encode_instruction(Some(__rex__), #opcode, Some(__mod_rm__), __sib__, __disp__, None);
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
                let __rex__ = if #dst >= Register::R8 { 0x49 } else { 0x48 };

                buf.emit_raw(&[__rex__, 0xb8 + #dst.value()]);
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

fn emit_op(op: Operation, dst: AddrRegister, src: AddressingMode) -> TokenStream {
    let mut ts = quote! {};

    ts.extend(match src {
        AddressingMode::Immediate(imm) => {
            quote! {
                let __rex__ = Rex(true, false, false, #dst >= Register::R8);
                let (__opcode__, __mod_rm__) = if #dst == Register::Rax {
                    ((#op << 3) | 0b101, None)
                } else {
                   (0x81, Some(ModRM(0b11, #op, #dst.value())))
                };
                buf.encode_instruction(Some(__rex__), __opcode__, __mod_rm__, None, None, Some(#imm as i32 as u32));
            }
        }
        AddressingMode::Register(src) => {
            quote! {
                let __rex__ = Rex(true, #src >= Register::R8, false, #dst >= Register::R8);
                let __mod_rm__ = ModRM(0b11, #src.value(), #dst.value());
                buf.encode_instruction(Some(__rex__), (#op << 3) | 0b001, Some(__mod_rm__), None, None, None);
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
