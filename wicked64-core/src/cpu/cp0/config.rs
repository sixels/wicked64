use std::ops::RangeInclusive;

use bitvec::{field::BitField, macros::internal::funty::Integral, order::Lsb0, view::BitView};

/// COP0 Config field
#[repr(transparent)]
#[derive(Debug, Default, Clone)]
pub struct ConfigRegister {
    /// (0..=2) K0 - Kseg0 coherency algorithm. This has the same format as the C field
    /// in EntryLo0 and EntryLo1. The only defined values for K0 for R4300i are
    /// 010 (cache is NOT used), other (cache is used).
    ///
    /// (3) CU - Reserved.
    ///
    /// (15) BE - Big Endian Memory.
    ///
    /// (24..=27) EP - Pattern for write-back data on SYSAD port
    ///
    /// (28..=30) EC - System Clock Ratio. Refer to `Config::clock_ratio` for the
    /// actual ratio values (ro)
    ///
    /// # Fixed value bits:
    ///
    /// | bits    | binary value |
    /// | ------- | ------------ |
    /// | 4..=14  | 11001000110  |
    /// | 16..=23 | 00000110     |
    /// | 31      | 0            |
    pub bits: u64,
}

impl ConfigRegister {
    // Define Config bit offsets and ranges
    pub const BIT_K0_RANGE: RangeInclusive<usize> = 0..=2;
    pub const BIT_CU_OFFSET: usize = 3;
    pub const BIT_BE_OFFSET: usize = 15;
    pub const BIT_EP_RANGE: RangeInclusive<usize> = 24..=27;
    pub const BIT_EC_RANGE: RangeInclusive<usize> = 28..=30;

    /// Get the clock ratio from Config bits (EC)
    ///
    /// | value | ratio |
    /// | ----- | ----- |
    /// | 0b110 | 1:1   |
    /// | 0b111 | 1.5:1 |
    /// | 0b000 | 2:1   |
    /// | 0b001 | 3:1   |
    pub fn get_clock_ratio(&self) -> f32 {
        let bits = self.get_bits(Self::BIT_EC_RANGE);

        intern_clock_ratio(bits).unwrap()
    }

    #[inline]
    pub fn get_bit(&self, bit: usize) -> bool {
        self.bits.view_bits::<Lsb0>()[bit]
    }
    #[inline]
    pub fn get_bits<T: Integral>(&self, bits: RangeInclusive<usize>) -> T {
        self.bits.view_bits::<Lsb0>()[bits].load::<T>()
    }
}

fn intern_clock_ratio(ratio: u8) -> anyhow::Result<f32> {
    match ratio {
        // 1:1
        0b110 => Ok(1.0),
        // 1.5:1
        0b111 => Ok(1.5),
        // 2:1
        0b000 => Ok(2.0),
        // 3:1
        0b001 => Ok(3.0),
        _ => anyhow::bail!("Invalid clock ratio: {}", ratio),
    }
}
