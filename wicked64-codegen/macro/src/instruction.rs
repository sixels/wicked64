use std::fmt::Display;

use syn::{
    parse::{Parse, ParseStream},
    Ident, Token,
};
use w64_codegen_types::register::Register;

use crate::addressing::{AddrImmediate, AddressingMode};

pub enum Instruction {
    Mov(AddressingMode, AddressingMode),
    Movabs(AddressingMode, AddrImmediate),
    Push(Register),
    Pop(Register),
    Add(Register, AddressingMode),
    Or(Register, AddressingMode),
    Sub(Register, AddressingMode),
    Xor(Register, AddressingMode),
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
            Instruction::Ret => w!("ret"),
        }
    }
}
