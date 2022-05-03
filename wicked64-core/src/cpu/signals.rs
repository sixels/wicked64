pub struct ResetSignal;

#[allow(non_upper_case_globals)]
impl ResetSignal {
    pub const None: u8 = 0;

    pub const PowerOnReset: u8 = 1 << 0;
    pub const ColdReset: u8 = 1 << 1;
    pub const Reset: u8 = 1 << 2;

    pub const ColdResetActive: u8 = 1 << 3;
    pub const ResetActive: u8 = 1 << 4;

    #[inline]
    pub fn disable_cold_reset(signals: u8) -> u8 {
        self::disable_signal(signals, Self::ColdResetActive)
    }
    #[inline]
    pub fn disable_soft_reset(signals: u8) -> u8 {
        self::disable_signal(signals, Self::ResetActive)
    }
}

#[inline]
pub fn disable_signal(signals: u8, signal: u8) -> u8 {
    signals & !signal
}
