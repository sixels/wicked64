use std::hash::Hash;

use super::codegen::register::X64Gpr;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[allow(dead_code)]
pub enum GuestRegister {
    Cpu(usize),
    Cp0(usize),
    Tmp(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct HostRegister {
    pub(crate) host_reg: X64Gpr,
    pub(crate) frequency: usize,
}

impl HostRegister {
    pub fn new(reg: X64Gpr) -> Self {
        Self {
            host_reg: reg,
            frequency: 1,
        }
    }
}

impl Hash for HostRegister {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.host_reg.hash(state);
    }
}
