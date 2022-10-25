use std::arch::asm;

use crate::{
    mmu::{num::MemInteger, MemoryUnit},
    n64::State,
};

use super::jump_table::JumpTable;

fn mmu_read<I: MemInteger>(state: &mut State, virt_addr: u64) -> I {
    let State { cpu, mmu, .. } = state;

    println!("{virt_addr:08x}");
    let phys_addr = cpu.translate_virtual(virt_addr) as usize;

    mmu.read::<I, byteorder::BigEndian>(phys_addr)
}
pub extern "C" fn mmu_read_byte(state: &mut State, virt_addr: u64) -> u8 {
    mmu_read(state, virt_addr)
}
pub extern "C" fn mmu_read_word(state: &mut State, virt_addr: u64) -> u16 {
    mmu_read(state, virt_addr)
}
pub extern "C" fn mmu_read_dword(state: &mut State, virt_addr: u64) -> u32 {
    mmu_read(state, virt_addr)
}

fn mmu_store<I: MemInteger>(state: &mut State, virt_addr: u64, value: I) {
    let State {
        cpu,
        mmu,
        cache_invalidation,
        ..
    } = state;

    println!("{virt_addr:08x}");
    dbg!(value);
    let phys_addr = cpu.translate_virtual(virt_addr) as usize;

    // invalidate I::SIZE bytes starting from `phys_addr`
    *cache_invalidation = Some(phys_addr..=phys_addr + I::SIZE);

    mmu.store::<I, byteorder::BigEndian>(phys_addr, value);
}
// pub extern "C" fn mmu_store_qword(state: &mut State, virt_addr: u64, value: u64) {
//     mmu_store(state, virt_addr, value);
// }
pub extern "C" fn mmu_store_dword(state: &mut State, virt_addr: u64, value: u32) {
    mmu_store(state, virt_addr, value);
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
