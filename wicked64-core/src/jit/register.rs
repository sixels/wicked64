use std::hash::Hash;

use hashbrown::{HashMap, HashSet};

use super::codegen::register::X64Gpr;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone, Copy)]
#[allow(dead_code)]
pub enum GuestRegister {
    Cpu(usize),
    Cp0(usize),
    Tmp(usize),
}

#[derive(Debug, Clone, Copy)]
struct HostRegister(X64Gpr, usize);

impl Hash for HostRegister {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

type RegMap = HashMap<GuestRegister, HostRegister>;

#[derive(Debug)]
pub struct Registers {
    regs: RegMap,
    free_regs: HashSet<X64Gpr>,
    borrow_count: usize,
}

impl Registers {
    pub fn new() -> Self {
        let free_regs = (0..16)
            .map(|r| X64Gpr::try_from(r).unwrap())
            .filter(|r| !r.is_reserved())
            .collect();

        Self {
            regs: RegMap::new(),
            free_regs,
            borrow_count: 0,
        }
    }

    pub fn get_mapped_register<F>(&mut self, guest_reg: GuestRegister, drop_unused: F) -> X64Gpr
    where
        F: FnOnce(GuestRegister, X64Gpr),
    {
        if let Some(HostRegister(host_reg, borrowed)) = self.regs.get_mut(&guest_reg) {
            *borrowed = self.borrow_count;
            self.borrow_count += 1;
            return *host_reg;
        }

        let host_reg = if let Some(next) = self.free_regs.iter().next().clone() {
            // we still have free registers
            *next
        } else {
            let (guest_reg, host_reg) = self.find_unused().unwrap();
            drop_unused(guest_reg, host_reg);
            host_reg
        };

        // At this point we already know the key does not exist, that's why we call `insert_unique_unchecked`
        self.regs
            .insert_unique_unchecked(guest_reg, HostRegister(host_reg, self.borrow_count));
        self.free_regs.remove(&host_reg);
        self.borrow_count += 1;
        host_reg
    }

    pub fn find_unused(&self) -> Option<(GuestRegister, X64Gpr)> {
        self.regs
            .iter()
            .min_by_key(|(_, HostRegister(_, n))| *n)
            .map(|(guest_reg, HostRegister(host_reg, _))| (*guest_reg, *host_reg))
    }

    pub fn drop_host_register(&mut self, reg: X64Gpr) {
        self.free_regs.insert(reg);
    }
}
