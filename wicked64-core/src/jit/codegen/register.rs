use num_enum::TryFromPrimitive;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, TryFromPrimitive)]
pub enum X64Gpr {
    Rax = 0x0,
    Rbx = 0x3,
    Rcx = 0x1,
    Rdx = 0x2,
    Rsi = 0x6,
    Rdi = 0x7,
    Rsp = 0x4,
    Rbp = 0x5,
    R8 = 0x8,
    R9 = 0x9,
    R10 = 0xA,
    R11 = 0xB,
    R12 = 0xC,
    R13 = 0xD,
    R14 = 0xE,
    R15 = 0xF,
}

impl X64Gpr {
    /// Check if register is one of: RSI, RDI, RSP, RBP
    pub fn is_reserved(&self) -> bool {
        match self {
            Self::Rsi | Self::Rdi | Self::Rsp | Self::Rbp => true,
            _ => false,
        }
    }
}

pub const CALLEE_SAVED_REGISTERS: [X64Gpr; 7] = [
    X64Gpr::Rbx,
    X64Gpr::Rsi,
    X64Gpr::Rbp,
    X64Gpr::R12,
    X64Gpr::R13,
    X64Gpr::R14,
    X64Gpr::R15,
];
