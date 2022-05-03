#![allow(dead_code)]

pub mod config;
pub mod status;

pub use self::{config::Config, status::Status};

/// MIPS' Coprocessor 0
///
/// | register | name           | function  | size |
/// | -------- | -------------- | --------  | ---- |
/// | 0        | Index          | TLB       | u32  |
/// | 1        | Random         | RNG       | u32  |
/// | 2        | EntryLo0       | TLB       | u64  |
/// | 3        | EntryLo1       | TLB       | u64  |
/// | 4        | Context        | TLB       | u64  |
/// | 5        | PageMask       | TLB       | u32  |
/// | 6        | Wired          | RNG       | u32  |
/// | 7        | -              | Unused    | -    |
/// | 8        | BadVAddr       | Exception | u64  |
/// | 9        | Count          | Timing    | u64  |
/// | 10       | EntryHi        | TLB       | u64  |
/// | 11       | Compare        | Timing    | u32  |
/// | 12       | Status         | Other     | u32  |
/// | 13       | Cause          | Exception | u32  |
/// | 14       | EPC            | Exception | u64  |
/// | 15       | PRId           | Other     | u32  |
/// | 16       | Config         | Other     | u32  |
/// | 17       | LLAddr         | Other     | u32  |
/// | 18       | WatchLo        | Exception | u32  |
/// | 19       | WatchHi        | Exception | u32  |
/// | 20       | XContext       | Exception | u64  |
/// | 21-25    | -              | Unused    | -    |
/// | 26       | ParityError    | Exception | u32  |
/// | 27       | CacheError     | Exception | u32  |
/// | 28       | TagLo          | Cache     | u32  |
/// | 29       | TagHi          | Cache     | u32  |
/// | 30       | ErrorEPC       | Exception | u64  |
/// | 31       | -              | Unused    | -    |
#[derive(Debug, Default, Clone)]
pub struct Cp0 {
    pub gpr: [u64; 32],

    pub index: u32,
    /// Holds a random number between `wired` and 0x1F.
    pub random: u32,
    pub entry_lo0: u64,
    pub entry_lo1: u64,
    pub context: u64,
    pub page_mask: u32,
    /// Provides the lower bound of the random number held in `random`.
    pub wired: u32,
    /// / When a TLB exception is thrown, this register is automatically loaded
    /// with the address of the failed translation.
    pub bad_vaddr: u64,
    /// This value is incremented every other cycle, and compared to the value
    /// in `compare`. As noted below, fire an interrupt when `count == compare`.
    pub count: u64,
    pub entry_hi: u64,
    /// Fire an interrupt when `count` equals this value. This interrupt sets
    /// the ip7 bit in `cause` to 1.
    pub compare: u32,
    pub status: Status,
    /// Contains details on the exception or interrupt that occurred. Only the
    /// low two bits of the Interrupt Pending field can be written to using
    /// MTC0, the rest are read-only and set by hardware when an exception is
    /// thrown. More information can be found in the interrupts section.
    pub cause: u32,
    pub epc: u64,
    pub prid: u32,
    pub config: Config,
    pub ll_addr: u32,
    pub watch_lo: u32,
    pub watch_hi: u32,
    pub xcontext: u64,
    pub parity_error: u32,
    pub cache_error: u32,
    pub tag_lo: u32,
    pub tag_hi: u32,
    pub error_epc: u64,
}

impl Cp0 {}
