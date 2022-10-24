use std::fmt::Debug;

use byteorder::ByteOrder;

use crate::{io::Cartridge, map_ranges, utils::btree_range::BTreeRange};

use super::{num::MemInteger, GenericMemoryUnit, MemoryUnit};

// 4 megabytes
pub const RDRAM_SIZE_IN_BYTES: usize = 4 * 1024 * 1024;

/// N64 Memory Management Unit
#[derive(Debug)]
#[allow(dead_code)]
pub struct MemoryManager {
    units: BTreeRange<GenericMemoryUnit>,
    /// 9th bit from RDRAM bytes
    rdram9: Box<[u8]>,
}

impl MemoryManager {
    pub fn new(cartridge: Cartridge) -> MemoryManager {
        use crate::mmu::map::addr_map;

        let rdram = std::iter::repeat(0)
            .take(2 * RDRAM_SIZE_IN_BYTES)
            .collect::<Box<[u8]>>();

        let units = map_ranges! {
            addr_map::phys::RDRAM_RANGE => GenericMemoryUnit::BoxedSlice(rdram),
            addr_map::phys::SP_DMEM_RANGE => GenericMemoryUnit::BoxedSlice(Box::new([0u8;0x1000]) as Box<[u8]>),
            addr_map::phys::PIF_RAM_RANGE => GenericMemoryUnit::BoxedSlice(Box::new([0u8;0x1000]) as Box<[u8]>),
            addr_map::phys::CART_D1A2_RANGE => GenericMemoryUnit::Cartridge(cartridge),
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
    fn copy_from(&mut self, dst: usize, src: usize, n: usize) {
        let src = {
            let s = self.units.get(src).unwrap();
            s.buffer().as_ptr()
        };
        let dst = {
            let dst_unit = self.units.get_mut(dst).unwrap();
            dst_unit.buffer_mut().as_mut_ptr()
        };

        unsafe { std::ptr::copy_nonoverlapping(src, dst, n) };
    }

    fn read<I, O>(&self, addr: usize) -> I
    where
        I: MemInteger,
        O: ByteOrder,
    {
        if let Some((offset, unit)) = self.units.get_offset_and_value(addr) {
            let value = unit.read::<I, O>(offset);
            return value;
        }
        tracing::warn!("No modules are handling memory address 0x{addr:08x}. This might led to UB");
        I::default()
    }

    fn store<I, O>(&mut self, addr: usize, value: I)
    where
        I: MemInteger,
        O: ByteOrder,
    {
        match self.units.get_offset_and_value_mut(addr) {
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
