use crate::{mmu::MemoryUnit, n64::State};

pub extern "C" fn mmu_store(state: &mut State, virt_addr: usize, rt: u64) {
    let State {
        cpu,
        mmu,
        cache_invalidation,
    } = state;

    let phys_addr = cpu.translate_virtual(virt_addr);

    // invalidate 8 bytes starting from `phys_addr`
    *cache_invalidation = Some((phys_addr, phys_addr + 8 + 1));

    mmu.store::<_, byteorder::BigEndian>(phys_addr, rt);
}
