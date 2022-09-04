mod emitter;

pub use emitter::Emitter;

pub use w64_codegen_macro::emit;
pub use w64_codegen_types::register;

pub mod prelude {
    pub use super::register::Register;
}

pub mod macro_internals {
    pub use w64_codegen_macro::_emit_instructions;
}
