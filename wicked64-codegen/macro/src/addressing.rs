use std::{fmt::Display, ops::Deref};

use proc_macro2::{Punct, Spacing};
use quote::{ToTokens, TokenStreamExt};
use syn::{
    bracketed, parenthesized,
    parse::{Parse, ParseStream},
    token::{Bracket, Comma},
    Expr, Ident, LitInt, Token,
};
use w64_codegen_types::register::Register;

#[derive(Clone)]
pub enum AddressingMode {
    Immediate(AddrImmediate),
    Register(AddrRegister),
    Direct(AddrDirect),
    Indirect(AddrIndirect),
}

pub struct CallArgs(pub Vec<Argument>);
pub enum Argument {
    Register(AddrRegister),
    Immediate(AddrImmediate),
    Ref(Expr),
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

impl Parse for CallArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        parenthesized!(content in input);

        let mut args = Vec::new();
        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if lookahead.peek(Token![ref]) {
                content.parse::<Token![ref]>()?;
                args.push(Argument::Ref(content.parse()?));
            } else if lookahead.peek(Token![$]) || lookahead.peek(LitInt) {
                args.push(Argument::Immediate(content.parse()?));
            } else if lookahead.peek(Token![%]) || lookahead.peek(Ident) {
                args.push(Argument::Register(content.parse()?));
            }

            if !content.is_empty() {
                content.parse::<Comma>()?;
            }
        }
        Ok(CallArgs(args))
    }
}

/// Immediate addressing mode
#[derive(Clone)]
pub enum AddrImmediate {
    /// Matches an immediate inside a variable (e.g `let val = 0x1234; mov rax, $val`).
    /// The variable's type must be u64
    Var(Ident),
    /// Matches a immediate literal (e.g: `mov rax, 0x1234`).
    /// The literal must be a valid u64
    Lit(u64),
}

/// Register addressing mode
#[derive(Clone)]
pub enum AddrRegister {
    /// Matches a register inside a variable (e.g: `let reg = Register::Rax; push %reg`).
    /// The variable must be of type `w64_codegen::register::Register`.
    Var(Ident),
    /// Matches a register literal (e.g: `push rax`).
    /// The literal must be a valid register name.
    Lit(Register),
}

/// Direct addressing mode
#[derive(Clone)]
pub struct AddrDirect {
    pub addr: AddrImmediate,
}

/// Indirect addressing mode
#[derive(Clone)]
pub struct AddrIndirect {
    pub reg: AddrRegister,
    pub disp: Option<(bool, AddrImmediate)>,
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
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        bracketed!(content in input);

        let reg = content.parse()?;
        let disp = if content.peek(Token![+]) || content.peek(Token![-]) {
            Some((content.parse::<Punct>()?.as_char() == '-', content.parse()?))
        } else {
            None
        };

        Ok(Self { reg, disp })
    }
}

impl ToTokens for CallArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append_separated(
            self.0.iter().map(|a| match a {
                Argument::Register(reg) => quote::quote!(#reg as _),
                Argument::Immediate(imm) => quote::quote!(#imm as _),
                Argument::Ref(ident) => quote::quote!(#ident as _),
            }),
            proc_macro2::Punct::new(',', Spacing::Joint),
        )
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

impl ToTokens for AddrDirect {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.addr.to_tokens(tokens)
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

impl Display for CallArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        for arg in self.0.iter().take(self.0.len() - 1) {
            write!(f, "{arg}, ")?;
        }
        if let Some(arg) = self.0.iter().last() {
            write!(f, "{arg}")?;
        }
        write!(f, ")")
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
        let Self { reg, disp } = self;

        match disp {
            Some((neg, disp)) => write!(f, "[{reg} {} {disp}]", if *neg { '-' } else { '+' }),
            None => write!(f, "[{reg}]"),
        }
    }
}

impl Display for Argument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Immediate(imm) => imm.fmt(f),
            Self::Register(reg) => reg.fmt(f),
            Self::Ref(_) => write!(f, "<reference>"),
        }
    }
}

impl Deref for CallArgs {
    type Target = Vec<Argument>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
