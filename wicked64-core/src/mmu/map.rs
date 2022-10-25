use once_cell::sync::Lazy;

use crate::{map_ranges, utils::btree_range::BTreeRange};

pub mod addr_map {
    use std::ops::RangeInclusive;
    type AddrRange = RangeInclusive<usize>;

    /// N64 virtual memory mapping
    ///
    /// | Address Range               | Name  | Description                                |
    /// | --------------------------- | ----- | ------------------------------------------ |
    /// | `0x0000_0000..=0x7FFF_FFFF` | KUSEG | User segment. TLB mapped                   |
    /// | `0x8000_0000..=0x9FFF_FFFF` | KSEG0 | Kernel segment 0. Direct mapped, cached.   |
    /// | `0xA000_0000..=0xBFFF_FFFF` | KSEG1 | Kernel segment 1. Direct mapped, no cache. |
    /// | `0xC000_0000..=0xDFFF_FFFF` | KSSEG | Kernel supervisor segment. TLB mapped.     |
    /// | `0xE000_0000..=0xFFFF_FFFF` | KSEG3 | Kernel segment 3. TLB mapped.              |
    pub mod virt {
        use super::AddrRange;

        pub static KUSEG_RANGE: AddrRange = 0x0000_0000..=0x7FFF_FFFF;
        pub static KSEG0_RANGE: AddrRange = 0x8000_0000..=0x9FFF_FFFF;
        pub static KSEG1_RANGE: AddrRange = 0xA000_0000..=0xBFFF_FFFF;
        pub static KSSEG_RANGE: AddrRange = 0xC000_0000..=0xDFFF_FFFF;
        pub static KSEG3_RANGE: AddrRange = 0xE000_0000..=0xFFFF_FFFF;
    }

    /// N64 physical memory mapping
    ///
    /// | Address Range               | Name                         | Description                                                               |
    /// | --------------------------- | ---------------------------- | ------------------------------------------------------------------------- |
    /// | `0x0000_0000..=0x003F_FFFF` | RDRAM                        | RDRAM - built in                                                          |
    /// | `0x0040_0000..=0x007F_FFFF` | RDRAM                        | RDRAM - expansion pak (available if inserted)                             |
    /// | `0x0080_0000..=0x03EF_FFFF` | Unused                       | Unused                                                                    |
    /// | `0x03F0_0000..=0x03FF_FFFF` | RDRAM Registers              | RDRAM MMIO, configures timings, etc. Irrelevant for emulation             |
    /// | `0x0400_0000..=0x0400_0FFF` | SP DMEM                      | RSP Data Memory                                                           |
    /// | `0x0400_1000..=0x0400_1FFF` | SP IMEM                      | RSP Instruction Memory                                                    |
    /// | `0x0400_2000..=0x0403_FFFF` | Unused                       | Unused                                                                    |
    /// | `0x0404_0000..=0x040F_FFFF` | SP Registers                 | Control RSP DMA engine, status, program counter                           |
    /// | `0x0410_0000..=0x041F_FFFF` | DP Command Registers         | Send commands to the RDP                                                  |
    /// | `0x0420_0000..=0x042F_FFFF` | DP Span Registers            | Unknown                                                                   |
    /// | `0x0430_0000..=0x043F_FFFF` | MIPS Interface (MI)          | System information, interrupts.                                           |
    /// | `0x0440_0000..=0x044F_FFFF` | Video Interface (VI)         | Screen resolution, framebuffer settings                                   |
    /// | `0x0450_0000..=0x045F_FFFF` | Audio Interface (AI)         | Control the audio subsystem                                               |
    /// | `0x0460_0000..=0x046F_FFFF` | Peripheral Interface (PI)    | Control the cartridge interface. Set up DMAs cart <==> RDRAM              |
    /// | `0x0470_0000..=0x047F_FFFF` | RDRAM Interface (RI)         | Control RDRAM settings (timings?) Irrelevant for emulation.               |
    /// | `0x0480_0000..=0x048F_FFFF` | Serial Interface (SI)        | Control PIF RAM <==> RDRAM DMA engine                                     |
    /// | `0x0490_0000..=0x04FF_FFFF` | Unused                       | Unused                                                                    |
    /// | `0x0500_0000..=0x05FF_FFFF` | Cartridge Domain 2 Address 1 | N64DD control registers - returns open bus (or all 0xFF) when not present |
    /// | `0x0600_0000..=0x07FF_FFFF` | Cartridge Domain 1 Address 1 | N64DD IPL ROM - returns open bus (or all 0xFF) when not present           |
    /// | `0x0800_0000..=0x0FFF_FFFF` | Cartridge Domain 2 Address 2 | SRAM is mapped here                                                       |
    /// | `0x1000_0000..=0x1FBF_FFFF` | Cartridge Domain 1 Address 2 | ROM is mapped here                                                        |
    /// | `0x1FC0_0000..=0x1FC0_07BF` | PIF Boot Rom                 | First code run on boot. Baked into hardware.                              |
    /// | `0x1FC0_07C0..=0x1FC0_07FF` | PIF RAM                      | Used to communicate with PIF chip (controllers, memory cards)             |
    /// | `0x1FC0_0800..=0x1FCF_FFFF` | Reserved                     |                                                                           |
    /// | `0x1FD0_0000..=0x7FFF_FFFF` | Cartridge Domain 1 Address 3 |                                                                           |
    /// | `0x8000_0000..=0xFFFF_FFFF` | External `SysAd`             |                                                                           |
    #[rustfmt::skip]
    pub mod phys {
        use super::AddrRange;

        pub const RDRAM_RANGE: AddrRange          = 0x0000_0000..=0x007F_FFFF;
        pub const RDRAM_REG_RANGE: AddrRange      = 0x03F0_0000..=0x03FF_FFFF;
        pub const SP_DMEM_RANGE: AddrRange        = 0x0400_0000..=0x0400_0FFF;
        pub const SP_IMEM_RANGE: AddrRange        = 0x0400_1000..=0x0400_1FFF;
        pub const SP_REG_RANGE: AddrRange         = 0x0404_0000..=0x040F_FFFF;
        pub const DP_CMD_REG_RANGE: AddrRange     = 0x0410_0000..=0x041F_FFFF;
        pub const DP_SPAN_REG_RANGE: AddrRange    = 0x0420_0000..=0x042F_FFFF;
        pub const MIPS_INT_RANGE: AddrRange       = 0x0430_0000..=0x043F_FFFF;
        pub const VIDEO_INT_RANGE: AddrRange      = 0x0440_0000..=0x044F_FFFF;
        pub const AUDIO_INT_RANGE: AddrRange      = 0x0450_0000..=0x045F_FFFF;
        pub const PERIPHERAL_INT_RANGE: AddrRange = 0x0460_0000..=0x046F_FFFF;
        pub const RDRAM_INT_RANGE: AddrRange      = 0x0470_0000..=0x047F_FFFF;
        pub const SERIAL_INT_RANGE: AddrRange     = 0x0480_0000..=0x048F_FFFF;
        pub const CART_D2A1_RANGE: AddrRange      = 0x0500_0000..=0x05FF_FFFF;
        pub const CART_D1A1_RANGE: AddrRange      = 0x0600_0000..=0x07FF_FFFF;
        pub const CART_D2A2_RANGE: AddrRange      = 0x0800_0000..=0x0FFF_FFFF;
        pub const CART_D1A2_RANGE: AddrRange      = 0x1000_0000..=0x1FBF_FFFF;
        pub const PIF_ROM_RANGE: AddrRange        = 0x1FC0_0000..=0x1FC0_07BF;
        pub const PIF_RAM_RANGE: AddrRange        = 0x1FC0_07C0..=0x1FC0_07FF;
        pub const RESERVED_RANGE: AddrRange       = 0x1FC0_0800..=0x1FCF_FFFF;
        pub const CART_D1A3_RANGE: AddrRange      = 0x1FD0_0000..=0x7FFF_FFFF;
        pub const UNKNOWN_RANGE: AddrRange        = 0x8000_0000..=0xFFFF_FFFF;
    }
}

static VIRT_MAP: Lazy<BTreeRange<VirtualMemoryMap>> = Lazy::new(|| {
    use addr_map::virt;

    map_ranges! [
        virt::KUSEG_RANGE => VirtualMemoryMap::KUSEG,
        virt::KSEG0_RANGE => VirtualMemoryMap::KSEG0,
        virt::KSEG1_RANGE => VirtualMemoryMap::KSEG1,
        virt::KSSEG_RANGE => VirtualMemoryMap::KSSEG,
        virt::KSEG3_RANGE => VirtualMemoryMap::KSEG3
    ]
});

#[derive(Debug, Clone, Copy)]
pub enum VirtualMemoryMap {
    KUSEG,
    KSEG0,
    KSEG1,
    KSSEG,
    KSEG3,
}

impl From<u64> for VirtualMemoryMap {
    fn from(addr: u64) -> Self {
        VIRT_MAP.get(addr as usize).copied().unwrap()
    }
}
impl From<u32> for VirtualMemoryMap {
    fn from(addr: u32) -> Self {
        (addr as u64).into()
    }
}
