#[repr(C, u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interruption {
    None,
    PrepareJump(u64),
}

impl Interruption {
    #[must_use]
    pub fn take(&mut self) -> Interruption {
        let int = *self;
        *self = Interruption::None;
        int
    }
}
