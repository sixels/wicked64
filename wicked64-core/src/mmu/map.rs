#[rustfmt::skip]
pub mod addr_map {
    use std::ops::RangeInclusive;

    type AddrRange = RangeInclusive<usize>;

    pub static VIRT_KUSEG_RANGE: AddrRange = 0x00000000..=0x7FFFFFFF;
    pub static VIRT_KSEG0_RANGE: AddrRange = 0x80000000..=0x9FFFFFFF;
    pub static VIRT_KSEG1_RANGE: AddrRange = 0xA0000000..=0xBFFFFFFF;
    pub static VIRT_KSSEG_RANGE: AddrRange = 0xC0000000..=0xDFFFFFFF;
    pub static VIRT_KSEG3_RANGE: AddrRange = 0xE0000000..=0xFFFFFFFF;

    pub const PHYS_RDRAM_RANGE: AddrRange          = 0x00000000..=0x007FFFFF;
    pub const PHYS_RDRAM_REG_RANGE: AddrRange      = 0x03F00000..=0x03FFFFFF;
    pub const PHYS_SP_DMEM_RANGE: AddrRange        = 0x04000000..=0x04000FFF;
    pub const PHYS_SP_IMEM_RANGE: AddrRange        = 0x04001000..=0x04001FFF;
    pub const PHYS_SP_REG_RANGE: AddrRange         = 0x04040000..=0x040FFFFF;
    pub const PHYS_DP_CMD_REG_RANGE: AddrRange     = 0x04100000..=0x041FFFFF;
    pub const PHYS_DP_SPAN_REG_RANGE: AddrRange    = 0x04200000..=0x042FFFFF;
    pub const PHYS_MIPS_INT_RANGE: AddrRange       = 0x04300000..=0x043FFFFF;
    pub const PHYS_VIDEO_INT_RANGE: AddrRange      = 0x04400000..=0x044FFFFF;
    pub const PHYS_AUDIO_INT_RANGE: AddrRange      = 0x04500000..=0x045FFFFF;
    pub const PHYS_PERIPHERAL_INT_RANGE: AddrRange = 0x04600000..=0x046FFFFF;
    pub const PHYS_RDRAM_INT_RANGE: AddrRange      = 0x04700000..=0x047FFFFF;
    pub const PHYS_SERIAL_INT_RANGE: AddrRange     = 0x04800000..=0x048FFFFF;
    pub const PHYS_CART_D2A1_RANGE: AddrRange      = 0x05000000..=0x05FFFFFF;
    pub const PHYS_CART_D1A1_RANGE: AddrRange      = 0x06000000..=0x07FFFFFF;
    pub const PHYS_CART_D2A2_RANGE: AddrRange      = 0x08000000..=0x0FFFFFFF;
    pub const PHYS_CART_D1A2_RANGE: AddrRange      = 0x10000000..=0x1FBFFFFF;
    pub const PHYS_PIF_ROM_RANGE: AddrRange        = 0x1FC00000..=0x1FC007BF;
    pub const PHYS_PIF_RAM_RANGE: AddrRange        = 0x1FC007C0..=0x1FC007FF;
    pub const PHYS_RESERVED_RANGE: AddrRange       = 0x1FC00800..=0x1FCFFFFF;
    pub const PHYS_CART_D1A3_RANGE: AddrRange      = 0x1FD00000..=0x7FFFFFFF;
    pub const PHYS_UNKNOWN_RANGE: AddrRange        = 0x80000000..=0xFFFFFFFF;
}

/// Unfortunately, Rust [does not let we match const ranges](https://github.com/rust-lang/rust/issues/76191)
/// like this:
/// ```no_run
/// // does not compile
/// match addr {
///     PHYS_RESERVED_RANGE => Self::Reserved,
///     /* snip */
/// }
/// ```
/// This macro provides an **inefficient** workaround to keep things simple,
/// but apparently it will be optimized on release builds
/// (https://godbolt.org/z/7naWYqjf9).
macro_rules! map_range {
    ($addr:expr, { $( $range:expr => $value:expr , )* _ => $exhaust:expr } ) => {
        match $addr {
            $( i if $range.contains(&i) => $value, )*
            _ => $exhaust
        }
    };
}

/// N64 virtual memory mapping
///
/// | Address Range           | Name  | Description                                |
/// | ----------------------- | ----- | ------------------------------------------ |
/// | 0x00000000..=0x7FFFFFFF | KUSEG | User segment. TLB mapped                   |
/// | 0x80000000..=0x9FFFFFFF | KSEG0 | Kernel segment 0. Direct mapped, cached.   |
/// | 0xA0000000..=0xBFFFFFFF | KSEG1 | Kernel segment 1. Direct mapped, no cache. |
/// | 0xC0000000..=0xDFFFFFFF | KSSEG | Kernel supervisor segment. TLB mapped.     |
/// | 0xE0000000..=0xFFFFFFFF | KSEG3 | Kernel segment 3. TLB mapped.              |
#[derive(Debug)]
pub enum VirtualMemoryMap {
    KUSEG,
    KSEG0,
    KSEG1,
    KSSEG,
    KSEG3,
}

impl From<usize> for VirtualMemoryMap {
    fn from(addr: usize) -> Self {
        use addr_map::*;

        map_range!(addr, {
            VIRT_KUSEG_RANGE => Self::KUSEG,
            VIRT_KSEG0_RANGE => Self::KSEG0,
            VIRT_KSEG1_RANGE => Self::KSEG1,
            VIRT_KSSEG_RANGE => Self::KSSEG,
            VIRT_KSEG3_RANGE => Self::KSEG3,
            0x100000000..=usize::MAX => panic!("Memory offset '0x{:08x}' out of bound", addr),
            _ => unreachable!()
        })
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
    SPDMEM(usize),
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
    CartridgeD1A2(usize),
    CartridgeD1A3,
    PIFROM,
    PIFRAM(usize),
    Reserved,
    ExternSysAD,
    Unused(usize),
}

impl From<usize> for PhysicalMemoryMap {
    fn from(addr: usize) -> Self {
        use addr_map::*;

        map_range!(addr, {
            PHYS_RDRAM_RANGE => Self::RDRAM,
            PHYS_RDRAM_REG_RANGE => Self::RDRAMReg,
            PHYS_SP_DMEM_RANGE => Self::SPDMEM(addr - *PHYS_SP_DMEM_RANGE.start()),
            PHYS_SP_IMEM_RANGE => Self::SPIMEM,
            PHYS_SP_REG_RANGE => Self::SPReg,
            PHYS_DP_CMD_REG_RANGE => Self::DPCmdReg,
            PHYS_DP_SPAN_REG_RANGE => Self::DPSpanReg,
            PHYS_MIPS_INT_RANGE => Self::MIPSInterface,
            PHYS_VIDEO_INT_RANGE => Self::VideoInterface,
            PHYS_AUDIO_INT_RANGE => Self::AudioInterface,
            PHYS_PERIPHERAL_INT_RANGE => Self::PeripheralInterface,
            PHYS_RDRAM_INT_RANGE => Self::RDRAMInterface,
            PHYS_SERIAL_INT_RANGE => Self::SerialInterface,
            PHYS_CART_D2A1_RANGE => Self::CartridgeD2A1,
            PHYS_CART_D1A1_RANGE => Self::CartridgeD1A1,
            PHYS_CART_D2A2_RANGE => Self::CartridgeD2A2,
            PHYS_CART_D1A2_RANGE => Self::CartridgeD1A2(addr - *PHYS_CART_D1A2_RANGE.start()),
            PHYS_PIF_ROM_RANGE => Self::PIFROM,
            PHYS_PIF_RAM_RANGE => Self::PIFRAM(addr - *PHYS_PIF_RAM_RANGE.start()),
            PHYS_RESERVED_RANGE => Self::Reserved,
            PHYS_CART_D1A3_RANGE => Self::CartridgeD1A3,
            PHYS_UNKNOWN_RANGE => Self::ExternSysAD,

            // unused ranges
            0x00800000..=0x03EFFFFF => Self::Unused(addr),
            0x04002000..=0x0403FFFF => Self::Unused(addr),
            0x04900000..=0x04FFFFFF => Self::Unused(addr),

            0x100000000..=usize::MAX => panic!("Memory offset '0x{:08x}' out of bound", addr),
            _ => unreachable!()
        })
    }
}
impl From<u32> for PhysicalMemoryMap {
    fn from(addr: u32) -> Self {
        (addr as usize).into()
    }
}
