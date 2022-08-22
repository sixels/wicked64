use std::{cmp::Ordering, fmt::Display};

use proc_macro2::Punct;
use quote::ToTokens;
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    token::Bracket,
    Ident, LitInt, Token,
};

use crate::register::Register;

pub enum AddressingMode {
    Immediate(AddrImmediate),
    Register(AddrRegister),
    Direct(AddrDirect),
    Indirect(AddrIndirect),
}

impl Parse for AddressingMode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(LitInt) || lookahead.peek(Token![$]) {
            input.parse().map(Self::Immediate)
        } else if lookahead.peek(Ident) || lookahead.peek(Token![%]) {
            input.parse().map(Self::Register)
        } else if lookahead.peek(Bracket) {
            input.step(|cursor| {
                if let Some((tt, next)) = cursor.token_tree() {
                    let ts = tt.into_token_stream();
                    syn::parse2(ts.clone())
                        .map(Self::Direct)
                        .or_else(|_| syn::parse2(ts).map(Self::Indirect))
                        .map(|addressing| (addressing, next))
                } else {
                    Err(cursor.error("Invalid addressing mode"))
                }
            })
        } else {
            Err(lookahead.error())
        }
    }
}

/// Immediate addressing mode
pub enum AddrImmediate {
    /// Matches an immediate inside a variable (e.g `let val = 0x1234; mov rax, $val`).
    /// The variable's type must be u64
    Var(Ident),
    /// Matches a immediate literal (e.g: `mov rax, 0x1234`).
    /// The literal must be a valid u64
    Lit(u64),
}

/// Register addressing mode
pub enum AddrRegister {
    /// Matches a register inside a variable (e.g: `let reg = 0; push %reg`).
    /// The variable must be a valid integer.
    Var(Ident),
    /// Matches a register literal (e.g: `push rax`).
    /// The literal must be a valid register name.
    Lit(Register),
}

/// Direct addressing mode
pub struct AddrDirect {
    pub addr: AddrImmediate,
}

/// Indirect addressing mode
pub struct AddrIndirect {
    _bracket: Bracket,
    pub reg: Register,
    pub disp: i32,
}

impl Parse for AddrImmediate {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let look_ahead = input.lookahead1();

        if look_ahead.peek(Token![$]) {
            input.parse::<Token![$]>()?;
            input.parse().map(Self::Var)
        } else if look_ahead.peek(LitInt) {
            input
                .parse::<LitInt>()
                .and_then(|imm| imm.base10_parse())
                .map(Self::Lit)
        } else {
            Err(look_ahead.error())
        }
    }
}

impl Parse for AddrRegister {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let look_ahead = input.lookahead1();

        if look_ahead.peek(Token![%]) {
            input.parse::<Token![%]>()?;
            input.parse().map(Self::Var)
        } else if look_ahead.peek(Ident) {
            input.parse().map(Self::Lit)
        } else {
            Err(look_ahead.error())
        }
    }
}

impl Parse for AddrDirect {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        bracketed!(content in input);
        Ok(Self {
            addr: content.parse::<AddrImmediate>()?,
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

impl ToTokens for AddrRegister {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Var(var) => var.to_tokens(tokens),
            Self::Lit(reg) => reg.to_tokens(tokens),
        }
    }
}

impl ToTokens for AddrImmediate {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Var(var) => var.to_tokens(tokens),
            Self::Lit(imm) => imm.to_tokens(tokens),
        }
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
        match self {
            Self::Var(var) => var.fmt(f),
            Self::Lit(imm) => write!(f, "0x{imm:08x}"),
        }
    }
}

impl Display for AddrRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Var(var) => var.fmt(f),
            Self::Lit(reg) => reg.fmt(f),
        }
    }
}

impl Display for AddrDirect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let addr = &self.addr;
        write!(f, "[{addr}]")
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
