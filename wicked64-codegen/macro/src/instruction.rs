use std::fmt::Display;

use syn::{
    parse::{Parse, ParseStream},
    Ident, Token,
};
use w64_codegen_types::register::Register;

use crate::addressing::{AddrImmediate, AddrRegister, AddressingMode, CallArgs};

pub enum Instruction {
    Mov(AddressingMode, AddressingMode),
    Movabs(AddressingMode, AddrImmediate),
    Push(AddrRegister),
    Pop(AddrRegister),
    Add(AddrRegister, AddressingMode),
    Or(Register, AddressingMode),
    Sub(AddrRegister, AddressingMode),
    Xor(Register, AddressingMode),
    Call(AddressingMode),
    CallFn(Ident, CallArgs),
    Ret,
}

impl Parse for Instruction {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        macro_rules! parse {
            (1, $variant:expr) => {{
                let a = input.parse()?;
                $variant(a)
            }};
            (2, $variant:expr) => {{
                let a = input.parse()?;
                input.parse::<Token![,]>()?;
                let b = input.parse()?;
                $variant(a, b)
            }};
            (2d, $variant:expr) => {{
                let a = input.parse()?;
                let b = input.parse()?;
                $variant(a, b)
            }};
        }

        let inst: Ident = input.parse()?;
        let inst = match inst.to_string().as_str() {
            "mov" => parse!(2, Self::Mov),
            "movabs" => parse!(2, Self::Movabs),
            "push" => parse!(1, Self::Push),
            "pop" => parse!(1, Self::Pop),
            "add" => parse!(2, Self::Add),
            "or" => parse!(2, Self::Or),
            "sub" => parse!(2, Self::Sub),
            "xor" => parse!(2, Self::Xor),
            "call" => parse!(1, Self::Call),
            "call_fn" => parse!(2d, Self::CallFn),
            "ret" => Self::Ret,
            _ => {
                return Err(syn::Error::new(
                    inst.span(),
                    format!("Unimplemented instruction: {}", inst),
                ))
            }
        };
        input.parse::<Token![;]>()?;
        Ok(inst)
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        macro_rules! w {
            ($inst:literal) => {
                write!(f, "{}", $inst)
            };
            ($inst:literal, $a:expr) => {
                write!(f, "{} {}", $inst, $a)
            };
            ($inst:literal, $a:expr, $b:expr) => {
                write!(f, "{} {}, {}", $inst, $a, $b)
            };
        }

        match self {
            Instruction::Mov(dst, src) => w!("mov", dst, src),
            Instruction::Movabs(dst, src) => w!("movabs", dst, src),
            Instruction::Push(reg) => w!("push", reg),
            Instruction::Pop(reg) => w!("pop", reg),
            Instruction::Add(a, b) => w!("add", a, b),
            Instruction::Or(a, b) => w!("or", a, b),
            Instruction::Sub(a, b) => w!("sub", a, b),
            Instruction::Xor(a, b) => w!("xor", a, b),
            Instruction::CallFn(a, b) => write!(f, "call_fn {a}({b})"),
            Instruction::Call(a) => w!("call", a),
            Instruction::Ret => w!("ret"),
        }
    }
}
