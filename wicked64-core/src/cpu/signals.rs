pub mod reset_signal {
    pub const NONE: u8 = 0;

    pub const POWER_ON_RESET: u8 = 1 << 0;
    pub const COLD_RESET: u8 = 1 << 1;
    pub const RESET: u8 = 1 << 2;

    pub const COLD_RESET_ACTIVE: u8 = 1 << 3;
    pub const RESET_ACTIVE: u8 = 1 << 4;

    #[inline]
    pub fn disable_cold_reset(signals: u8) -> u8 {
        super::disable_signal(signals, self::COLD_RESET_ACTIVE)
    }
    #[inline]
    pub fn disable_soft_reset(signals: u8) -> u8 {
        super::disable_signal(signals, self::RESET_ACTIVE)
    }
}

#[inline]
pub fn disable_signal(signals: u8, signal: u8) -> u8 {
    signals & !signal
}
