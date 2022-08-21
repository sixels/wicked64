use std::{cmp::Ordering, fmt::Display};

use proc_macro2::Punct;
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    token::Bracket,
    Ident, LitInt, Token,
};

use crate::register::Register;

pub enum AddressingMode {
    Immediate(AddrImmediate),
    Register(Register),
    Direct(AddrDirect),
    Indirect(AddrIndirect),
}

impl Parse for AddressingMode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(LitInt) {
            input.parse().map(Self::Immediate)
        } else if lookahead.peek(Ident) {
            input.parse().map(Self::Register)
        } else if lookahead.peek(Bracket) {
            input
                .parse()
                .map(Self::Indirect)
                .or_else(|_| input.parse().map(Self::Direct))
        } else {
            Err(lookahead.error())
        }
    }
}

#[repr(transparent)]
pub struct AddrImmediate(pub u64);

pub struct AddrDirect {
    _bracket: Bracket,
    pub addr: i32,
}

pub struct AddrIndirect {
    _bracket: Bracket,
    pub reg: Register,
    pub disp: i32,
}

impl Parse for AddrImmediate {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse::<LitInt>()?.base10_parse()?))
    }
}

impl Parse for AddrDirect {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            _bracket: bracketed!(content in input),
            addr: content.parse::<LitInt>()?.base10_parse()?,
        })
    }
}

impl Parse for AddrIndirect {
    /// Match `[register + displacement] || [registers]`.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            _bracket: bracketed!(content in input),
            reg: content.parse()?,
            disp: {
                if content.peek(Token![+]) || content.peek(Token![-]) {
                    let sign = i32::from(content.parse::<Punct>()?.as_char() == '-') * -1;
                    content.parse::<LitInt>()?.base10_parse::<i32>()? * sign
                } else {
                    0
                }
            },
        })
    }
}

impl Display for AddressingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddressingMode::Immediate(imm) => imm.fmt(f),
            AddressingMode::Register(reg) => reg.fmt(f),
            AddressingMode::Direct(dir) => dir.fmt(f),
            AddressingMode::Indirect(ind) => ind.fmt(f),
        }
    }
}

impl Display for AddrImmediate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let imm = self.0;
        write!(f, "0x{imm:08x}")
    }
}

impl Display for AddrDirect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let addr = self.addr;
        write!(f, "[0x{addr:04x}]")
    }
}

impl Display for AddrIndirect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { reg, disp, .. } = self;

        match disp.cmp(&0) {
            Ordering::Greater => write!(f, "[{reg} + 0x{disp:04x}]"),
            Ordering::Less => write!(f, "[{reg} - 0x{disp:04x}]"),
            Ordering::Equal => write!(f, "[{reg}]"),
        }
    }
}
