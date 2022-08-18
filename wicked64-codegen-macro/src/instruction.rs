use syn::{
    parse::{Parse, ParseStream},
    Ident, Token,
};

use crate::{
    addressing::{AddrImmediate, AddressingMode},
    register::Register,
};

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
            _ => return Err(syn::Error::new(inst.span(), "Unimplemented instruction")),
        };
        input.parse::<Token![;]>()?;
        Ok(inst)
    }
}
