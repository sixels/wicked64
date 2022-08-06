use std::{cell::RefCell, marker::PhantomData, path::Path, rc::Rc};

use byteorder::{BigEndian, ByteOrder};

use crate::{cpu::Cpu, io::Cartridge, jit::cache::Cache, mmu::MemoryManager};

/// N64 state
pub struct N64<O: ByteOrder> {
    state: Rc<RefCell<State>>,
    cache: Cache,
    clocks: usize,
    _marker: PhantomData<O>,
}

impl<O: ByteOrder> N64<O> {
    pub fn new<P: AsRef<Path>>(rom_path: P) -> anyhow::Result<Self> {
        tracing::info!("Creating a brand new N64!");

        let mut mmu = MemoryManager::new(Cartridge::open(rom_path)?);
        let cpu = Cpu::new(true, &mut mmu);

        let cache = Cache::default();
        let state = Rc::new(RefCell::new(State::new(mmu, cpu)));

        Ok(Self {
            state,
            clocks: 0,
            cache,
            _marker: Default::default(),
        })
    }

    pub fn state(&self) -> &Rc<RefCell<State>> {
        &self.state
    }

    /// Step the execution of the current running game
    pub fn step(&mut self) {
        let phys_pc = {
            let cpu = &self.state.borrow().cpu;
            cpu.translate_virtual(cpu.pc as usize)
        };

        let state = self.state.clone();
        let code = self.cache.get_or_compile(phys_pc, state);

        tracing::info!("Running generated code");
        let clocks = code.execute();

        self.clocks += clocks; // TODO
    }
}

#[derive(Debug)]
pub struct State {
    pub mmu: MemoryManager,
    pub cpu: Cpu<BigEndian>,
}

impl State {
    pub fn new(mmu: MemoryManager, cpu: Cpu<BigEndian>) -> Self {
        Self { mmu, cpu }
    }
}

#[cfg(test)]
mod tests {
    use byteorder::{BigEndian, ByteOrder};

    use crate::mmu::{map::addr_map, MemoryUnit};

    use super::*;

    /// Test Dillon's N64 tests basic.z64
    #[test]
    fn it_should_compile_dillonb_basic_test() {
        crate::tests::init_trace();

        let mut n64 = N64::<BigEndian>::new("../assets/test-roms/dillonb/basic.z64").unwrap();
        skip_boot_process(&n64);
        tracing::info!("Beginning the execution");

        n64.step();
    }

    fn skip_boot_process<O: ByteOrder>(n64: &N64<O>) {
        tracing::info!("Skipping the boot process");

        let mut state = n64.state().borrow_mut();

        let cart_rom_addr = *addr_map::phys::CART_D1A2_RANGE.start();
        let header_pc = state.mmu.read::<u32, O>(0x08 + cart_rom_addr);
        assert!(header_pc == 0x80001000);

        state.cpu.pc = header_pc as u64;

        state.mmu.copy_from(0x00001000, 0x10001000, 0x100000);
    }
}
