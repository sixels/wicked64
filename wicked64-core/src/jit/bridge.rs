use std::arch::asm;

use crate::{mmu::MemoryUnit, n64::State};

use super::jump_table::JumpTable;

pub extern "C" fn mmu_store(state: &mut State, virt_addr: u64, rt: u64) {
    let State {
        cpu,
        mmu,
        cache_invalidation,
        ..
    } = state;

    let phys_addr = cpu.translate_virtual(virt_addr) as usize;

    // invalidate 8 bytes starting from `phys_addr`
    *cache_invalidation = Some((phys_addr, phys_addr + 8 + 1));

    mmu.store::<_, byteorder::BigEndian>(phys_addr, rt);
}

pub extern "C" fn get_host_jump_addr(state: &mut State, jump_table: &mut JumpTable, n64_addr: u64) {
    let _ = jump_table.get(state.cpu.translate_virtual(n64_addr));
}

#[naked]
pub extern "C" fn get_rip_value(disp: u32) -> u64 {
    unsafe {
        #[rustfmt::skip]
        asm!(
            "mov rax, [rsp]",
            "add rax, rdi",
            "ret",
            options(noreturn),
        );
    }
}
