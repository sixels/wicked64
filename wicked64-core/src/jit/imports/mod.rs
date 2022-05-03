use std::ops::{Deref, DerefMut};

use byteorder::BigEndian;
use wasmer::WasmerEnv;

use crate::{cpu::Cpu, mmu::MemoryUnit, n64::SyncState};

#[derive(WasmerEnv, Clone)]
pub struct EnvState {
    state: SyncState,
}

impl EnvState {
    pub fn new(state: SyncState) -> Self {
        Self { state }
    }
}

impl Deref for EnvState {
    type Target = SyncState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

/// Translates a virtual address into a physical one
pub fn translate_virtual_addr(env: &EnvState, virt_addr: u32) -> u32 {
    let cpu = &env.state.lock().cpu;
    Cpu::translate_virtual(cpu, virt_addr as usize) as u32
}

pub fn read_word(env: &EnvState, phys_addr: u32) -> u32 {
    let mmu = &env.state.lock().mmu;
    mmu.read::<u32, BigEndian>(phys_addr as usize)
}
pub fn store_word(env: &EnvState, phys_addr: u32, value: u32) {
    let mut state_guard = env.state.lock();
    
    let state = state_guard.deref_mut();
    let mmu = &mut state.mmu;
    let cache = &state.cache;

    if let Some(k) = cache.is_addr_compiled(phys_addr as u64) {
        tracing::warn!("TODO: Invalidate cache containing '0x{phys_addr:08x}' (key = 0x{k:08x})");
    }
    
    mmu.store::<u32, BigEndian>(phys_addr as usize, value);

}
