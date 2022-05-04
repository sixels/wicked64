use std::{collections::BTreeMap, ops::RangeInclusive};

use once_cell::sync::Lazy;

pub mod addr_map {
    use std::ops::RangeInclusive;
    type AddrRange = RangeInclusive<usize>;

    pub mod virt {
        use super::AddrRange;

        pub static KUSEG_RANGE: AddrRange = 0x00000000..=0x7FFFFFFF;
        pub static KSEG0_RANGE: AddrRange = 0x80000000..=0x9FFFFFFF;
        pub static KSEG1_RANGE: AddrRange = 0xA0000000..=0xBFFFFFFF;
        pub static KSSEG_RANGE: AddrRange = 0xC0000000..=0xDFFFFFFF;
        pub static KSEG3_RANGE: AddrRange = 0xE0000000..=0xFFFFFFFF;
    }

    #[rustfmt::skip]
    pub mod phys {
        use super::AddrRange;

        pub const RDRAM_RANGE: AddrRange          = 0x00000000..=0x007FFFFF;
        pub const RDRAM_REG_RANGE: AddrRange      = 0x03F00000..=0x03FFFFFF;
        pub const SP_DMEM_RANGE: AddrRange        = 0x04000000..=0x04000FFF;
        pub const SP_IMEM_RANGE: AddrRange        = 0x04001000..=0x04001FFF;
        pub const SP_REG_RANGE: AddrRange         = 0x04040000..=0x040FFFFF;
        pub const DP_CMD_REG_RANGE: AddrRange     = 0x04100000..=0x041FFFFF;
        pub const DP_SPAN_REG_RANGE: AddrRange    = 0x04200000..=0x042FFFFF;
        pub const MIPS_INT_RANGE: AddrRange       = 0x04300000..=0x043FFFFF;
        pub const VIDEO_INT_RANGE: AddrRange      = 0x04400000..=0x044FFFFF;
        pub const AUDIO_INT_RANGE: AddrRange      = 0x04500000..=0x045FFFFF;
        pub const PERIPHERAL_INT_RANGE: AddrRange = 0x04600000..=0x046FFFFF;
        pub const RDRAM_INT_RANGE: AddrRange      = 0x04700000..=0x047FFFFF;
        pub const SERIAL_INT_RANGE: AddrRange     = 0x04800000..=0x048FFFFF;
        pub const CART_D2A1_RANGE: AddrRange      = 0x05000000..=0x05FFFFFF;
        pub const CART_D1A1_RANGE: AddrRange      = 0x06000000..=0x07FFFFFF;
        pub const CART_D2A2_RANGE: AddrRange      = 0x08000000..=0x0FFFFFFF;
        pub const CART_D1A2_RANGE: AddrRange      = 0x10000000..=0x1FBFFFFF;
        pub const PIF_ROM_RANGE: AddrRange        = 0x1FC00000..=0x1FC007BF;
        pub const PIF_RAM_RANGE: AddrRange        = 0x1FC007C0..=0x1FC007FF;
        pub const RESERVED_RANGE: AddrRange       = 0x1FC00800..=0x1FCFFFFF;
        pub const CART_D1A3_RANGE: AddrRange      = 0x1FD00000..=0x7FFFFFFF;
        pub const UNKNOWN_RANGE: AddrRange        = 0x80000000..=0xFFFFFFFF;
    }
}

pub struct RangeMap<T> {
    inner: BTreeMap<usize, T>,
}

impl<T> RangeMap<T> {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }
    pub fn insert(&mut self, start: usize, value: T) {
        self.inner.insert(start, value);
    }

    pub fn insert_range_unchecked(&mut self, range: &RangeInclusive<usize>, value: T) {
        self.insert(*range.start(), value)
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        for start in self.inner.keys() {
            let diff = index - start;
            if diff > 0 {
                return self.inner.get(start);
            }
        }

        None
    }

    pub fn get_offset_value(&self, index: usize) -> Option<(usize, &T)> {
        for start in self.inner.keys() {
            let diff = index - start;
            if diff > 0 {
                return self.inner.get(start).map(|v| (diff, v));
            }
        }

        None
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        for start in self.inner.keys().copied() {
            let diff = index - start;
            if diff > 0 {
                return self.inner.get_mut(&start);
            }
        }

        None
    }
        None
    }
}

macro_rules! map_range {
    ($( $range:expr => $value:expr , )* ) => {{
        let mut map = RangeMap::new();

        $( map.insert_range_unchecked($range, $value); )*

        map
    }};
}

macro_rules! map_range_tup {
    ($( $range:expr => $value:expr , )* ) => {{
        let mut map = RangeMap::new();

        $( map.insert_range_unchecked($range, ($value, *$range.start())); )*

        map
    }};
}

static VIRT_MAP: Lazy<RangeMap<VirtualMemoryMap>> = Lazy::new(|| {
    use addr_map::virt;

    map_range! {
        virt::KUSEG_RANGE => VirtualMemoryMap::KUSEG,
        virt::KSEG0_RANGE => VirtualMemoryMap::KSEG0,
        virt::KSEG1_RANGE => VirtualMemoryMap::KSEG1,
        virt::KSSEG_RANGE => VirtualMemoryMap::KSSEG,
        virt::KSEG3_RANGE => VirtualMemoryMap::KSEG3,
    }
});

static PHYS_MAP: Lazy<RangeMap<(PhysicalMemoryMap, usize)>> = Lazy::new(|| {
    use addr_map::phys;

    map_range_tup! {
        phys::RDRAM_RANGE => PhysicalMemoryMap::RDRAM,
        phys::RDRAM_REG_RANGE => PhysicalMemoryMap::RDRAMReg,
        phys::SP_DMEM_RANGE => PhysicalMemoryMap::SPDMEM,
        phys::SP_IMEM_RANGE => PhysicalMemoryMap::SPIMEM,
        phys::SP_REG_RANGE => PhysicalMemoryMap::SPReg,
        phys::DP_CMD_REG_RANGE => PhysicalMemoryMap::DPCmdReg,
        phys::DP_SPAN_REG_RANGE => PhysicalMemoryMap::DPSpanReg,
        phys::MIPS_INT_RANGE => PhysicalMemoryMap::MIPSInterface,
        phys::VIDEO_INT_RANGE => PhysicalMemoryMap::VideoInterface,
        phys::AUDIO_INT_RANGE => PhysicalMemoryMap::AudioInterface,
        phys::PERIPHERAL_INT_RANGE => PhysicalMemoryMap::PeripheralInterface,
        phys::RDRAM_INT_RANGE => PhysicalMemoryMap::RDRAMInterface,
        phys::SERIAL_INT_RANGE => PhysicalMemoryMap::SerialInterface,
        phys::CART_D2A1_RANGE => PhysicalMemoryMap::CartridgeD2A1,
        phys::CART_D1A1_RANGE => PhysicalMemoryMap::CartridgeD1A1,
        phys::CART_D2A2_RANGE => PhysicalMemoryMap::CartridgeD2A2,
        phys::CART_D1A2_RANGE => PhysicalMemoryMap::CartridgeD1A2,
        phys::PIF_ROM_RANGE => PhysicalMemoryMap::PIFROM,
        phys::PIF_RAM_RANGE => PhysicalMemoryMap::PIFRAM,
        phys::RESERVED_RANGE => PhysicalMemoryMap::Reserved,
        phys::CART_D1A3_RANGE => PhysicalMemoryMap::CartridgeD1A3,
        phys::UNKNOWN_RANGE => PhysicalMemoryMap::ExternSysAD,

        // unused ranges
        0x00800000..=0x03EFFFFF => PhysicalMemoryMap::Unused,
        0x04002000..=0x0403FFFF => PhysicalMemoryMap::Unused,
        0x04900000..=0x04FFFFFF => PhysicalMemoryMap::Unused,
    }
});

/// N64 virtual memory mapping
///
/// | Address Range           | Name  | Description                                |
/// | ----------------------- | ----- | ------------------------------------------ |
/// | 0x00000000..=0x7FFFFFFF | KUSEG | User segment. TLB mapped                   |
/// | 0x80000000..=0x9FFFFFFF | KSEG0 | Kernel segment 0. Direct mapped, cached.   |
/// | 0xA0000000..=0xBFFFFFFF | KSEG1 | Kernel segment 1. Direct mapped, no cache. |
/// | 0xC0000000..=0xDFFFFFFF | KSSEG | Kernel supervisor segment. TLB mapped.     |
/// | 0xE0000000..=0xFFFFFFFF | KSEG3 | Kernel segment 3. TLB mapped.              |
#[derive(Debug, Clone, Copy)]
pub enum VirtualMemoryMap {
    KUSEG,
    KSEG0,
    KSEG1,
    KSSEG,
    KSEG3,
}

impl From<usize> for VirtualMemoryMap {
    fn from(addr: usize) -> Self {
        VIRT_MAP.get(addr).copied().unwrap()
    }
}
impl From<u32> for VirtualMemoryMap {
    fn from(addr: u32) -> Self {
        (addr as usize).into()
    }
}

/// N64 physical memory mapping
///
/// | Address Range           | Name                         | Description                                                               |
/// | ----------------------- | ---------------------------- | ------------------------------------------------------------------------- |
/// | 0x00000000..=0x003FFFFF | RDRAM                        | RDRAM - built in                                                          |
/// | 0x00400000..=0x007FFFFF | RDRAM                        | RDRAM - expansion pak (available if inserted)                             |
/// | 0x00800000..=0x03EFFFFF | Unused                       | Unused                                                                    |
/// | 0x03F00000..=0x03FFFFFF | RDRAM Registers              | RDRAM MMIO, configures timings, etc. Irrelevant for emulation             |
/// | 0x04000000..=0x04000FFF | SP DMEM                      | RSP Data Memory                                                           |
/// | 0x04001000..=0x04001FFF | SP IMEM                      | RSP Instruction Memory                                                    |
/// | 0x04002000..=0x0403FFFF | Unused                       | Unused                                                                    |
/// | 0x04040000..=0x040FFFFF | SP Registers                 | Control RSP DMA engine, status, program counter                           |
/// | 0x04100000..=0x041FFFFF | DP Command Registers         | Send commands to the RDP                                                  |
/// | 0x04200000..=0x042FFFFF | DP Span Registers            | Unknown                                                                   |
/// | 0x04300000..=0x043FFFFF | MIPS Interface (MI)          | System information, interrupts.                                           |
/// | 0x04400000..=0x044FFFFF | Video Interface (VI)         | Screen resolution, framebuffer settings                                   |
/// | 0x04500000..=0x045FFFFF | Audio Interface (AI)         | Control the audio subsystem                                               |
/// | 0x04600000..=0x046FFFFF | Peripheral Interface (PI)    | Control the cartridge interface. Set up DMAs cart <==> RDRAM              |
/// | 0x04700000..=0x047FFFFF | RDRAM Interface (RI)         | Control RDRAM settings (timings?) Irrelevant for emulation.               |
/// | 0x04800000..=0x048FFFFF | Serial Interface (SI)        | Control PIF RAM <==> RDRAM DMA engine                                     |
/// | 0x04900000..=0x04FFFFFF | Unused                       | Unused                                                                    |
/// | 0x05000000..=0x05FFFFFF | Cartridge Domain 2 Address 1 | N64DD control registers - returns open bus (or all 0xFF) when not present |
/// | 0x06000000..=0x07FFFFFF | Cartridge Domain 1 Address 1 | N64DD IPL ROM - returns open bus (or all 0xFF) when not present           |
/// | 0x08000000..=0x0FFFFFFF | Cartridge Domain 2 Address 2 | SRAM is mapped here                                                       |
/// | 0x10000000..=0x1FBFFFFF | Cartridge Domain 1 Address 2 | ROM is mapped here                                                        |
/// | 0x1FC00000..=0x1FC007BF | PIF Boot Rom                 | First code run on boot. Baked into hardware.                              |
/// | 0x1FC007C0..=0x1FC007FF | PIF RAM                      | Used to communicate with PIF chip (controllers, memory cards)             |
/// | 0x1FC00800..=0x1FCFFFFF | Reserved                     |                                                                           |
/// | 0x1FD00000..=0x7FFFFFFF | Cartridge Domain 1 Address 3 |                                                                           |
/// | 0x80000000..=0xFFFFFFFF | External SysAd               |                                                                           |
#[derive(Debug)]
pub enum PhysicalMemoryMap {
    RDRAM,
    RDRAMReg,
    SPDMEM,
    SPIMEM,
    SPReg,
    DPCmdReg,
    DPSpanReg,
    MIPSInterface,
    VideoInterface,
    AudioInterface,
    PeripheralInterface,
    RDRAMInterface,
    SerialInterface,
    CartridgeD2A1,
    CartridgeD1A1,
    CartridgeD2A2,
    CartridgeD1A2,
    CartridgeD1A3,
    PIFROM,
    PIFRAM,
    Reserved,
    ExternSysAD,
    Unused,
}

impl From<usize> for PhysicalMemoryMap {
    fn from(addr: usize) -> Self {
        PHYS_MAP.get(addr).unwrap().0
    }
}
impl From<u32> for PhysicalMemoryMap {
    fn from(addr: u32) -> Self {
        (addr as usize).into()
    }
}
