use std::fmt::Debug;

use byteorder::ByteOrder;

use crate::{hardware::Cartridge, map_ranges};

use super::{map::RangeMap, num::MemInteger, MemoryUnit, MemoryUnits};

// 4 megabytes
pub const RDRAM_SIZE_IN_BYTES: usize = 4 * 1024 * 1024;

/// N64 Memory Management Unit
#[allow(dead_code)]
pub struct MemoryManager {
    units: RangeMap<MemoryUnits>,
    /// 9th bit from RDRAM bytes
    rdram9: Box<[u8]>,
}

impl MemoryManager {
    pub fn new(cartridge: Cartridge) -> MemoryManager {
        use crate::mmu::map::addr_map::phys;

        let rdram = std::iter::repeat(0)
            .take(2 * RDRAM_SIZE_IN_BYTES)
            .collect::<Box<[u8]>>();

        let units = map_ranges! {
            phys::RDRAM_RANGE => MemoryUnits::BoxedSlice(rdram),
            phys::SP_DMEM_RANGE => MemoryUnits::BoxedSlice(Box::new([0u8;0x1000]) as Box<[u8]>),
            phys::PIF_RAM_RANGE => MemoryUnits::BoxedSlice(Box::new([0u8;0x1000]) as Box<[u8]>),
            phys::CART_D1A2_RANGE => MemoryUnits::Cartridge(cartridge),
        };

        Self {
            units,
            rdram9: std::iter::repeat(0)
                .take(2 * RDRAM_SIZE_IN_BYTES)
                .collect::<Box<[u8]>>(),
        }
    }
}

impl MemoryUnit for MemoryManager {
    fn copy_from(&mut self, dst: usize, src: usize, n: usize)
    where
        Self: Sized,
    {
        let src = {
            let s = self.units.get(src).unwrap();
            s.buffer().as_ptr()
        };
        let dst = {
            let dst_unit = self.units.get_mut(dst).unwrap();
            dst_unit.buffer_mut().as_mut_ptr()
        };

        unsafe { std::ptr::copy_nonoverlapping(src, dst, n) };

        #[cfg(test)]
        {
            let src = unsafe { std::slice::from_raw_parts(src, n) };
            let dst = unsafe { std::slice::from_raw_parts(dst, n) };
            assert!(&src[..n] == &dst[..n]);
        }
    }

    fn read<I, O>(&self, addr: usize) -> I
    where
        Self: Sized,
        I: MemInteger + Sized + Debug,
        O: ByteOrder + Sized,
    {
        match self.units.get_offset_value(addr) {
            Some((offset, unit)) => {
                let value = unit.read::<I, O>(offset);
                value
            }
            None => {
                tracing::warn!(
                    "No modules are handling memory address 0x{addr:08x}. This might led to UB"
                );
                I::default()
            }
        }
    }

    fn store<I, O>(&mut self, addr: usize, value: I)
    where
        I: MemInteger + Sized + Debug,
        O: ByteOrder + Sized,
    {
        match self.units.get_mut_offset_value(addr) {
            Some((offset, unit)) => {
                unit.store::<I, O>(offset, value);
            }
            None => {
                tracing::warn!(
                    "No modules are handling memory address 0x{addr:08x}. This might led to UB"
                );
            }
        }
    }
}
