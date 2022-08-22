mod emitter;

pub use emitter::Emitter;

pub use wicked64_codegen_macro::emit;
pub use wicked64_codegen_types::register;

pub mod prelude {
    pub use wicked64_codegen_types::register::Register;
}
