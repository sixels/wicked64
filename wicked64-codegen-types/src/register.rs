use std::fmt::Display;

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

impl Register {
    pub fn as_str(&self) -> &'static str {
        match self {
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
        }
    }
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

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(feature = "macro")]
impl syn::parse::Parse for Register {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let reg = input.parse::<proc_macro2::Ident>()?;
        reg.to_string()
            .as_str()
            .try_into()
            .map_err(|_| syn::Error::new(reg.span(), "Invalid register name"))
    }
}

#[cfg(feature = "macro")]
impl quote::ToTokens for Register {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use quote::quote;

        match self {
            Register::Rax => quote! { Register::Rax },
            Register::Rcx => quote! { Register::Rcx },
            Register::Rdx => quote! { Register::Rdx },
            Register::Rbx => quote! { Register::Rbx },
            Register::Rsp => quote! { Register::Rsp },
            Register::Rbp => quote! { Register::Rbp },
            Register::Rsi => quote! { Register::Rsi },
            Register::Rdi => quote! { Register::Rdi },
            Register::R8 => quote! { Register::R8 },
            Register::R9 => quote! { Register::R9 },
            Register::R10 => quote! { Register::R10 },
            Register::R11 => quote! { Register::R11 },
            Register::R12 => quote! { Register::R12 },
            Register::R13 => quote! { Register::R13 },
            Register::R14 => quote! { Register::R14 },
            Register::R15 => quote! { Register::R15 },
        }
        .to_tokens(tokens)
    }
}
