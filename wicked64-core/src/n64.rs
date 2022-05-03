use std::{path::Path, sync::Arc};

use byteorder::BigEndian;
use parking_lot::Mutex;

use crate::{cpu::Cpu, hardware::Cartridge, jit::cache::Cache, mmu::MemoryManager};

/// 50MB
pub const DEFAULT_ARENA_SIZE_IN_BYTES: u32 = 50 * 1024 * 1024;
/// 10MB
pub const SMALL_ARENA_SIZE_IN_BYTES: u32 = 20 * 1024 * 1024;

pub type SyncState = Arc<Mutex<State>>;

/// N64 state
pub struct N64 {
    state: SyncState,
    cache: Arc<Cache>,
    clocks: u64,
}

impl N64 {
    pub fn new<P: AsRef<Path>>(rom_path: P) -> anyhow::Result<Self> {
        tracing::info!("Creating a brand new N64!");

        let mut mmu = MemoryManager::new(Cartridge::open(rom_path)?);
        let cpu = Cpu::new(true, &mut mmu);

        let cache = Arc::new(Default::default());
        let state = Arc::new(Mutex::new(State::new(mmu, cpu, Arc::clone(&cache))));

        Ok(Self {
            state,
            clocks: 0,
            cache,
        })
    }

    pub fn state(&self) -> &SyncState {
        &self.state
    }

    /// Step the execution of the current running game
    pub fn step(&mut self) {
        let phys_pc = {
            let cpu = &self.state.lock().cpu;
            cpu.translate_virtual(cpu.pc as usize) as u64
        };

        let cache = &mut self.cache;
        let code = cache.get_or_compile(phys_pc, self.state.clone());

        code.exec();

        self.clocks += 0; // TODO
    }
}

pub struct State {
    pub mmu: MemoryManager,
    pub cpu: Cpu<BigEndian>,
    pub cache: Arc<Cache>,
}

impl State {
    pub fn new(mmu: MemoryManager, cpu: Cpu<BigEndian>, cache: Arc<Cache>) -> Self {
        Self {
            mmu,
            cpu,
            cache,
        }
    }
}

#[cfg(test)]
mod tests {
    use byteorder::{BigEndian, ByteOrder};

    use crate::mmu::MemoryUnit;

    use super::*;

    fn skip_boot_process<O: ByteOrder>(n64: &N64) {
        tracing::info!("Skipping the boot process");

        let mut state = n64.state().lock();

        let cart = state.mmu.cartridge();
        let header_pc = cart.read::<u32, O>(0x08);
        assert!(header_pc == 0x80001000);

        state.cpu.pc = header_pc as u64;

        state.mmu.copy_from(0x00001000, 0x10001000, 0x100000);
    }

    /// Test Dillon's N64 tests basic.z64
    #[test]
    fn dillon_basic() {
        crate::tests::init_trace();

        let mut n64 = N64::new("../assets/test-roms/dillonb/basic.z64").unwrap();
        skip_boot_process::<BigEndian>(&n64);
        tracing::info!("Beginning the execution");

        n64.step();
        tracing::info!("Exiting");
    }
}
