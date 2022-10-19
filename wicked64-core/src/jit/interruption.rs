#[repr(C, u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interruption {
    None,
    PrepareJump(u64),
}
