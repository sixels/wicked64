use byteorder::ByteOrder;

use crate::{hardware::Cartridge, mmu::map::PhysicalMemoryMap};

use super::{num::MemInteger, MemoryUnit};

// 4 megabytes
pub const RDRAM_SIZE_IN_BYTES: usize = 4 * 1024 * 1024;

/// N64 Memory Management Unit
#[allow(dead_code)]
pub struct MemoryManager {
    /// Up to 8 megabytes internal memory size.
    rdram: Box<[u8]>,
    /// 9th bit from `rdram` bytes
    rdram9: Box<[u8]>,

    /// dummy SP DMEM
    spdmem: Box<[u8; 0x1000]>,
    // dummy PIF RAM
    pifram: Box<[u8; 0x1000]>,
    cartridge: Cartridge,
}

impl MemoryManager {
    pub fn new(cartridge: Cartridge) -> MemoryManager {
        let rdram = std::iter::repeat(0)
            .take(2 * RDRAM_SIZE_IN_BYTES)
            .collect::<Vec<u8>>();
        let rdram9 = std::iter::repeat(0)
            .take(2 * RDRAM_SIZE_IN_BYTES)
            .collect::<Vec<u8>>();

        Self {
            rdram: rdram.into_boxed_slice(),
            rdram9: rdram9.into_boxed_slice(),
            spdmem: Box::new([0; 0x1000]),
            pifram: Box::new([0; 0x1000]),
            cartridge,
        }
    }

    pub fn cartridge(&self) -> &Cartridge {
        &self.cartridge
    }
}

impl MemoryUnit for MemoryManager {
    fn copy_from(&mut self, dst: usize, src: usize, n: usize)
    where
        Self: Sized,
    {
        let src: *const [u8] = match PhysicalMemoryMap::from(src) {
            PhysicalMemoryMap::RDRAM => &self.rdram[src..src + n] as *const _,
            PhysicalMemoryMap::CartridgeD1A2(a) => &self.cartridge.data[a..a + n] as *const _,
            PhysicalMemoryMap::SPDMEM(a) => &self.spdmem[a..a + n] as *const _,
            PhysicalMemoryMap::PIFRAM(a) => &self.pifram[a..a + n] as *const _,
            other => panic!("Copy not supported for src region '{other:?}'"),
        };
        let dst = match PhysicalMemoryMap::from(dst) {
            PhysicalMemoryMap::RDRAM => &mut self.rdram[dst..dst + n],
            PhysicalMemoryMap::CartridgeD1A2(a) => &mut self.cartridge.data[a..a + n],
            PhysicalMemoryMap::SPDMEM(a) => &mut self.spdmem[a..a + n],
            PhysicalMemoryMap::PIFRAM(a) => &mut self.pifram[a..a + n],
            other => panic!("Copy not supported for dst region '{other:?}'"),
        };

        debug_assert!(std::ptr::eq(dst as *const _, src) == false);
        dst.copy_from_slice(unsafe { &*src });
    }

    fn read<I, O>(&self, addr: usize) -> I
    where
        Self: Sized,
        I: MemInteger + Sized,
        O: ByteOrder + Sized,
    {
        match PhysicalMemoryMap::from(addr) {
            PhysicalMemoryMap::RDRAM => I::read_from::<O>(&self.rdram[addr..addr + I::SIZE]),
            PhysicalMemoryMap::SPDMEM(offset) => {
                I::read_from::<O>(&self.spdmem[offset..offset + I::SIZE])
            }
            PhysicalMemoryMap::CartridgeD1A2(offset) => self.cartridge.read::<I, O>(offset),
            other => {
                tracing::warn!("Unhandled `read` region: '{other:?}'");
                I::default()
            }
        }
    }
    fn store<I: MemInteger, O: ByteOrder>(&mut self, addr: usize, value: I) {
        match PhysicalMemoryMap::from(addr) {
            PhysicalMemoryMap::RDRAM => {
                I::write_to::<O>(&mut self.rdram[addr..addr + I::SIZE], value)
            }
            PhysicalMemoryMap::SPDMEM(offset) => {
                I::write_to::<O>(&mut self.spdmem[offset..offset + I::SIZE], value)
            }
            PhysicalMemoryMap::PIFRAM(offset) => {
                I::write_to::<O>(&mut self.pifram[offset..offset + I::SIZE], value)
            }
            other => tracing::warn!("Unhandled `store` region: '{other:?}'"),
        }
    }
}
