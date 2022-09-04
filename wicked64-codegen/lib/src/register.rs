pub use w64_codegen_types::register::Register;

pub const CALLEE_SAVED_REGISTERS: [Register; 7] = [
    Register::Rbx,
    Register::Rsi,
    Register::Rbp,
    Register::R12,
    Register::R13,
    Register::R14,
    Register::R15,
];
