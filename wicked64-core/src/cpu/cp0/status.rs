use std::{mem, ops::RangeInclusive};

use bitvec::{field::BitField, macros::internal::funty::Integral, order::Lsb0, view::BitView};

#[derive(Debug, Default, Clone)]
pub struct StatusRegister {
    /// (0) IE - Global interrupt enable.
    ///
    /// (1) EXL - Exception Level.
    ///
    /// (2) ERL - Error level.
    ///
    /// (3..=4) KSU - Execution Mode. Refer to `ExecutionMode`.
    ///
    /// (5) UX - 64-bit addressing enabled in user mode.
    ///
    /// (6) SX - 64-bit addressing enabled in supervisor mode.
    ///
    /// (7) KX - 64-bit addressing enabled in kernel mode.
    ///
    /// (8..=15) IM - Interrupt mask.
    ///
    /// (16..=24) DS - Diagnostic Status.
    ///
    /// (16) DE - Unused (Included to maintain compatibility with MIPS' R4200).
    ///
    /// (17) CE - Unused (Included to maintain compatibility with MIPS' R4200).
    ///
    /// (18) CH - CP0 condition bit. Not set or cleared by hardware.
    ///
    /// (20) SR - Soft-reset or NMI has occurred.
    ///
    /// (21) TS - TLB Shutdown has occurred.
    ///
    /// (22) BEV - Controls location of TLB refill and general exception vectors.
    ///
    /// (23) rsvd - Reserved for future use.
    ///
    /// (24) ITS - Instruction Trace Support. Enables trace support.
    ///
    /// (25) RE - Reverse endianness to little-endian
    ///
    /// (26) FR - Additional floating-point registers (false -> 16 regs, true -> 32 regs).
    ///
    /// (27) RP - Reduced Power mode (run the CPU at 1/4th clock speed)
    ///
    /// (28..=31) CU - Coprocessor enabled if != 0
    ///
    /// # Fixed value bits:
    ///
    /// | bits     | binary value |
    /// | -------- | ------------ |
    /// | 19 (0)   | 0            |
    pub bits: u32,
}

impl StatusRegister {
    // Define Status bit offsets and ranges
    pub const BIT_IE_OFFSET: usize = 0;
    pub const BIT_EXL_OFFSET: usize = 1;
    pub const BIT_ERL_OFFSET: usize = 2;
    pub const BIT_KSU_RANGE: RangeInclusive<usize> = 3..=4;
    pub const BIT_UX_OFFSET: usize = 5;
    pub const BIT_SX_OFFSET: usize = 6;
    pub const BIT_KX_OFFSET: usize = 7;
    pub const BIT_IM_RANGE: RangeInclusive<usize> = 8..=15;
    pub const BIT_DS_RANGE: RangeInclusive<usize> = 16..=24;
    pub const BIT_DE_OFFSET: usize = 16;
    pub const BIT_CE_OFFSET: usize = 17;
    pub const BIT_CH_OFFSET: usize = 18;
    pub const BIT_ZERO_OFFSET: usize = 19;
    pub const BIT_SR_OFFSET: usize = 20;
    pub const BIT_TS_OFFSET: usize = 21;
    pub const BIT_BEV_OFFSET: usize = 22;
    pub const BIT_RSVD_OFFSET: usize = 23;
    pub const BIT_ITS_OFFSET: usize = 24;
    pub const BIT_RE_OFFSET: usize = 25;
    pub const BIT_FR_OFFSET: usize = 26;
    pub const BIT_RP_OFFSET: usize = 27;
    pub const BIT_CU_RANGE: RangeInclusive<usize> = 28..=31;
    pub const BIT_CU0_OFFSET: usize = 28;
    pub const BIT_CU1_OFFSET: usize = 29;
    pub const BIT_CU2_OFFSET: usize = 30;
    pub const BIT_CU3_OFFSET: usize = 31;

    /// Initialize a new Status register
    pub fn new() -> StatusRegister {
        let bits = 1 << 28;

        Self { bits }
    }

    /// Get the KSU value as an `ExecutionMode`
    pub fn get_execution_mode(&self) -> OperationMode {
        let ksu = self.get_execution_mode_raw();
        debug_assert!(ksu < 3);
        // we are reading only two bits (`BIT_KSU_RANGE.len() == 2`), `ksu` is guaranteed to be a valid ExecutionMode.
        unsafe { mem::transmute(ksu) }
    }
    /// Get the raw KSU value as u8
    #[inline]
    pub fn get_execution_mode_raw(&self) -> u8 {
        self.get_bits(Self::BIT_KSU_RANGE)
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

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationMode {
    Kernel = 0,
    Supervisor = 1,
    User = 2,
}

impl Default for OperationMode {
    fn default() -> Self {
        Self::Kernel
    }
}

impl From<OperationMode> for u8 {
    fn from(exec_mode: OperationMode) -> Self {
        // Safety: as ExecutionMode is `#[repr(u8)]`, its in-memory
        // representation is asserted to be the same as the primitive u8 type
        unsafe { mem::transmute(exec_mode) }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbVecLocation {
    Normal = 0,
    Bootstrap = 1,
}
