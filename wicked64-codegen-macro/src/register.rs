use std::fmt::Display;

use proc_macro2::Ident;
use syn::parse::Parse;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Register {
    Rax = 0,
    Rcx = 1,
    Rdx = 2,
    Rbx = 3,
    Rsp = 4,
    Rbp = 5,
    Rsi = 6,
    Rdi = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
}

impl TryFrom<&str> for Register {
    type Error = ();
    fn try_from(reg_str: &str) -> Result<Self, Self::Error> {
        let reg = match reg_str {
            "rax" => Self::Rax,
            "rcx" => Self::Rcx,
            "rdx" => Self::Rdx,
            "rbx" => Self::Rbx,
            "rsp" => Self::Rsp,
            "rbp" => Self::Rbp,
            "rsi" => Self::Rsi,
            "rdi" => Self::Rdi,
            "r8" => Self::R8,
            "r9" => Self::R9,
            "r10" => Self::R10,
            "r11" => Self::R11,
            "r12" => Self::R12,
            "r13" => Self::R13,
            "r14" => Self::R14,
            "r15" => Self::R15,
            _ => return Err(()),
        };
        Ok(reg)
    }
}

impl Parse for Register {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let reg = input.parse::<Ident>()?;
        reg.to_string()
            .as_str()
            .try_into()
            .map_err(|_| syn::Error::new(reg.span(), "Invalid register name"))
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reg = match self {
            Register::Rax => "rax",
            Register::Rcx => "rcx",
            Register::Rdx => "rdx",
            Register::Rbx => "rbx",
            Register::Rsp => "rsp",
            Register::Rbp => "rbp",
            Register::Rsi => "rsi",
            Register::Rdi => "rdi",
            Register::R8 => "r8",
            Register::R9 => "r9",
            Register::R10 => "r10",
            Register::R11 => "r11",
            Register::R12 => "r12",
            Register::R13 => "r13",
            Register::R14 => "r14",
            Register::R15 => "r15",
        };
        write!(f, "{reg}")
    }
}
