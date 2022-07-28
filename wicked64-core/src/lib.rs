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
        tracing_subscriber::fmt::init();
    }
}