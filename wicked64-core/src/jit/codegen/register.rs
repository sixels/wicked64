#[repr(u8)]
#[derive(Clone, Copy)]
pub enum X64Gpr {
    Rax,
    Rbx,
    Rcx,
    Rdx,
    Rsi,
    Rdi,
    Rsp,
    Rbp,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

pub const CALLEE_SAVED_REGISTERS: &[X64Gpr] = &[
    X64Gpr::Rbx,
    X64Gpr::Rsi,
    X64Gpr::Rbp,
    X64Gpr::R12,
    X64Gpr::R13,
    X64Gpr::R14,
    X64Gpr::R15,
];
