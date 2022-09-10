mod emitter;
pub mod register;

pub use w64_codegen_macro::emit;
pub use w64_codegen_types::encoding;

pub use emitter::{Emitter, ExecBuffer};

pub mod prelude {
    pub use super::encoding::{ModRM, Rex};
    pub use super::register::Register;
}

pub mod macro_internals {
    pub use w64_codegen_macro::_emit_instructions;
}
