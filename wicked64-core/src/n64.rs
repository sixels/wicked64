use std::{cell::RefCell, marker::PhantomData, ops::RangeInclusive, path::Path, rc::Rc};

use byteorder::{BigEndian, ByteOrder};

use crate::{
    cpu::Cpu,
    io::Cartridge,
    jit::{Interruption, JitEngine},
    mmu::MemoryManager,
};

/// N64 state
pub struct N64<O: ByteOrder> {
    state: Rc<RefCell<State>>,
    jit: JitEngine,
    #[allow(unused)]
    clocks: usize,
    _marker: PhantomData<O>,
}

impl<O: ByteOrder> N64<O> {
    /// Create a new N64 virtual machine
    ///
    /// # Errors
    /// Any
    pub fn new<P: AsRef<Path>>(rom_path: P) -> anyhow::Result<Self> {
        tracing::info!("Creating a brand new N64!");

        let mut mmu = MemoryManager::new(Cartridge::open(rom_path)?);
        let cpu = Cpu::new(true, &mut mmu);

        let state = Rc::new(RefCell::new(State::new(mmu, cpu)));

        Ok(Self {
            state: state.clone(),
            clocks: 0,
            jit: JitEngine::new(state),
            _marker: PhantomData::default(),
        })
    }

    pub fn state(&self) -> &Rc<RefCell<State>> {
        &self.state
    }

    /// Step the execution of the current running game
    pub fn cycle(&mut self) {
        loop {
            self.jit.invalidate_cache();

            // handle interruptions
            let mut resume_jump = None;
            {
                let interruption = self.state.borrow_mut().interruption.take();
                match interruption {
                    Interruption::PrepareJump(addr) => {
                        tracing::debug!("Resolving jump to: 0x{addr:08x}");
                        self.state.borrow_mut().cpu.pc = addr;
                        resume_jump = Some(self.jit.resolve_jump(addr));
                    }
                    Interruption::None => {}
                }
            }

            tracing::debug!("CPU PC: {:08x}", self.state.borrow().cpu.pc);

            if let Some(jump_entry) = resume_jump.take().flatten() {
                let target = jump_entry.target_block;
                self.jit.resume_from(target);
            } else {
                let code = self.jit.compile_current_pc();
                tracing::debug!("Executing code at {:p}", code.ptr());
                code.execute();
            }
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub mmu: MemoryManager,
    pub cpu: Cpu<BigEndian>,
    pub cache_invalidation: Option<RangeInclusive<usize>>,
    pub interruption: Interruption,
    pub resume_addr: u64,
}

impl State {
    pub fn new(mmu: MemoryManager, cpu: Cpu<BigEndian>) -> Self {
        Self {
            mmu,
            cpu,
            cache_invalidation: None,
            interruption: Interruption::None,
            resume_addr: 0,
        }
    }
    pub fn translate_cpu_pc(&self) -> u64 {
        self.cpu.translate_virtual(self.cpu.pc)
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

        n64.cycle();
    }

    fn skip_boot_process<O: ByteOrder>(n64: &N64<O>) {
        tracing::info!("Skipping the boot process");

        let mut state = n64.state().borrow_mut();

        let cart_rom_addr = *addr_map::phys::CART_D1A2_RANGE.start();
        let header_pc = state.mmu.read::<u32, O>(0x08 + cart_rom_addr);
        assert!(header_pc == 0x8000_1000);

        state.cpu.pc = header_pc as u64;

        state.mmu.copy_from(0x0000_1000, 0x1000_1000, 0x10_0000);
    }
}
