#![feature(naked_functions)]
#![deny(clippy::pedantic)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::similar_names)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_bool)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::borrow_as_ptr)]

#[cfg(not(target_pointer_width = "64"))]
compile_error!("Your CPU does not supports 64-bit integers");

pub mod cpu;
pub mod io;
pub mod jit;
pub mod mmu;
pub mod n64;
mod utils;

#[cfg(test)]
mod tests {
    pub(crate) fn init_trace() {
        std::env::set_var("RUST_LOG", "debug");
        tracing_subscriber::fmt::init();
    }
}
