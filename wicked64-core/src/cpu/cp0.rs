#![allow(dead_code)]

pub mod config;
pub mod status;

pub use self::{config::ConfigRegister, status::StatusRegister};

/// MIPS' Coprocessor 0
///
/// | register | name           | function  | size |
/// | -------- | -------------- | --------  | ---- |
/// | 0        | `Index`        | TLB       | u32  |
/// | 1        | `Random`       | RNG       | u32  |
/// | 2        | `EntryLo0`     | TLB       | u64  |
/// | 3        | `EntryLo1`     | TLB       | u64  |
/// | 4        | `Context`      | TLB       | u64  |
/// | 5        | `PageMask`     | TLB       | u32  |
/// | 6        | `Wired`        | RNG       | u32  |
/// | 7        | -              | Unused    | -    |
/// | 8        | `BadVAddr`     | Exception | u64  |
/// | 9        | `Count`        | Timing    | u64  |
/// | 10       | `EntryHi`      | TLB       | u64  |
/// | 11       | `Compare`      | Timing    | u32  |
/// | 12       | `Status`       | Other     | u32  |
/// | 13       | `Cause`        | Exception | u32  |
/// | 14       | `EPC`          | Exception | u64  |
/// | 15       | `PRId`         | Other     | u32  |
/// | 16       | `Config`       | Other     | u32  |
/// | 17       | `LLAddr`       | Other     | u32  |
/// | 18       | `WatchLo`      | Exception | u32  |
/// | 19       | `WatchHi`      | Exception | u32  |
/// | 20       | `XContext`     | Exception | u64  |
/// | 21-25    | -              | Unused    | -    |
/// | 26       | `ParityError`  | Exception | u32  |
/// | 27       | `CacheError`   | Exception | u32  |
/// | 28       | `TagLo`        | Cache     | u32  |
/// | 29       | `TagHi`        | Cache     | u32  |
/// | 30       | `ErrorEPC`     | Exception | u64  |
/// | 31       | -              | Unused    | -    |
#[derive(Debug, Default, Clone)]
pub struct Cp0 {
    // TODO: Change to an array with all registers instead of separated variables
    pub index: u64,
    /// Holds a random number between `wired` and 0x1F.
    pub random: u64,
    pub entry_lo0: u64,
    pub entry_lo1: u64,
    pub context: u64,
    pub page_mask: u64,
    /// Provides the lower bound of the random number held in `random`.
    pub wired: u64,
    /// / When a TLB exception is thrown, this register is automatically loaded
    /// with the address of the failed translation.
    pub bad_vaddr: u64,
    /// This value is incremented every other cycle, and compared to the value
    /// in `compare`. As noted below, fire an interrupt when `count == compare`.
    pub count: u64,
    pub entry_hi: u64,
    /// Fire an interrupt when `count` equals this value. This interrupt sets
    /// the ip7 bit in `cause` to 1.
    pub compare: u64,
    pub status: StatusRegister,
    /// Contains details on the exception or interrupt that occurred. Only the
    /// low two bits of the Interrupt Pending field can be written to using
    /// MTC0, the rest are read-only and set by hardware when an exception is
    /// thrown. More information can be found in the interrupts section.
    pub cause: u64,
    pub epc: u64,
    pub prid: u64,
    pub config: ConfigRegister,
    pub ll_addr: u64,
    pub watch_lo: u64,
    pub watch_hi: u64,
    pub xcontext: u64,
    pub parity_error: u64,
    pub cache_error: u64,
    pub tag_lo: u64,
    pub tag_hi: u64,
    pub error_epc: u64,
}

impl Cp0 {
    pub fn get_register(&self, n: usize) -> &u64 {
        match n {
            0 => &self.index,
            1 => &self.random,
            2 => &self.entry_lo0,
            3 => &self.entry_lo1,
            4 => &self.context,
            5 => &self.page_mask,
            6 => &self.wired,
            8 => &self.bad_vaddr,
            9 => &self.count,
            10 => &self.entry_hi,
            11 => &self.compare,
            12 => &self.status.bits,
            13 => &self.cause,
            14 => &self.epc,
            15 => &self.prid,
            16 => &self.config.bits,
            17 => &self.ll_addr,
            18 => &self.watch_lo,
            19 => &self.watch_hi,
            20 => &self.xcontext,
            26 => &self.parity_error,
            27 => &self.cache_error,
            28 => &self.tag_lo,
            29 => &self.tag_hi,
            30 => &self.error_epc,
            _ => unreachable!("Invalid CP0 register: {n}"),
        }
    }
}
