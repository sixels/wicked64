#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModRM(pub u8, pub u8, pub u8);

impl ModRM {
    pub const fn md(&self) -> u8 {
        self.0 & 0b11
    }
    pub const fn reg(&self) -> u8 {
        self.1 & 0b111
    }
    pub const fn rm(&self) -> u8 {
        self.2 & 0b111
    }
    pub const fn value(&self) -> u8 {
        (self.md() << 6) | (self.reg() << 3) | self.rm()
    }
}

pub struct Rex(pub bool, pub bool, pub bool, pub bool);

impl Rex {
    pub const fn w(&self) -> u8 {
        self.0 as u8
    }
    pub const fn r(&self) -> u8 {
        self.1 as u8
    }
    pub const fn x(&self) -> u8 {
        self.2 as u8
    }
    pub const fn b(&self) -> u8 {
        self.3 as u8
    }
    pub const fn value(&self) -> u8 {
        (0b0100 << 4) | (self.w() << 3) | (self.r() << 2) | (self.x() << 1) | self.b()
    }
}

#[cfg(feature = "macro")]
impl quote::ToTokens for ModRM {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.value().to_tokens(tokens)
    }
}
#[cfg(feature = "macro")]
impl quote::ToTokens for Rex {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.value().to_tokens(tokens)
    }
}
