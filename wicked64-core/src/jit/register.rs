use std::hash::Hash;

use hashbrown::{HashMap, HashSet};
use w64_codegen::register::Register;

// use super::codegen::register::Register;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone, Copy)]
#[allow(dead_code)]
pub enum GuestRegister {
    Cpu(usize),
    Cp0(usize),
    Tmp(usize),
}

#[derive(Debug, Clone, Copy)]
struct HostRegister(Register, usize);

impl Hash for HostRegister {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

type RegMap = HashMap<GuestRegister, HostRegister>;

#[derive(Debug, Clone)]
pub struct Registers {
    regs: RegMap,
    free_regs: HashSet<Register>,
    borrow_count: usize,
}

pub fn is_reserved(register: &Register) -> bool {
    match register {
        Register::Rsi | Register::Rdi | Register::Rsp | Register::Rbp => true,
        _ => false,
    }
}

impl Registers {
    pub fn new() -> Self {
        let free_regs = (0..16)
            .map(|r| Register::try_from(r).unwrap())
            .filter(|r| !is_reserved(r))
            .collect();

        Self {
            regs: RegMap::new(),
            free_regs,
            borrow_count: 0,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (GuestRegister, Register)> {
        self.regs
            .clone()
            .into_iter()
            .map(|(g, HostRegister(h, _))| (g, h))
    }

    pub fn get_mapped_register<F>(
        &mut self,
        guest_reg: GuestRegister,
        mut drop_unused: F,
    ) -> Register
    where
        F: FnMut(GuestRegister, Register) -> bool,
    {
        if let Some(HostRegister(host_reg, borrowed)) = self.regs.get_mut(&guest_reg) {
            *borrowed = self.borrow_count;
            self.borrow_count += 1;
            return *host_reg;
        }

        let host_reg = if let Some(next) = self.free_regs.iter().next() {
            // we still have free registers
            *next
        } else {
            let mut unused_regs = self.regs.iter().collect::<Vec<_>>();
            unused_regs.sort_by_key(|(_, HostRegister(_, n))| *n);

            let mut unused = None;
            for (&guest_reg, &HostRegister(host_reg, _)) in unused_regs.into_iter() {
                // We need to know if we can drop this register safely, as
                // tmp registers are intended to be dropped manually.
                if drop_unused(guest_reg, host_reg) {
                    self.free_regs.insert(host_reg);
                    unused = Some(host_reg);
                    break;
                }
            }

            unused.expect("Could not drop any register")
        };

        // At this point we already know the key does not exist, that's why we call `insert_unique_unchecked`
        self.regs
            .insert_unique_unchecked(guest_reg, HostRegister(host_reg, self.borrow_count));
        self.free_regs.remove(&host_reg);
        self.borrow_count += 1;
        host_reg
    }

    pub fn find_host_register(&self, host_reg: Register) -> Option<(GuestRegister, Register)> {
        self.regs
            .iter()
            .map(|(g, HostRegister(r, _))| (*g, *r))
            .find(|(_, r)| *r == host_reg)
    }

    pub fn drop_guest_register(&mut self, guest_reg: GuestRegister) {
        self.regs
            .remove(&guest_reg)
            .map(|HostRegister(host_reg, _)| {
                self.free_regs.insert(host_reg);
            });

        if self.regs.len() == 0 {
            debug_assert_eq!(
                self.free_regs.len(),
                (0..16)
                    .map(|r| Register::try_from(r).unwrap())
                    .filter(|r| !is_reserved(r))
                    .collect::<Vec<_>>()
                    .len()
            );
            self.borrow_count = 0;
        }
    }
}
