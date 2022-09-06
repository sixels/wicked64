use std::hash::Hash;

use hashbrown::{HashMap, HashSet};
use w64_codegen::register::Register;

#[derive(Debug)]
pub enum InsertError {
    AlreadyReserved,
    NoRegistersAvailable,
}

#[derive(Debug, Clone)]
pub struct Registers {
    regs: HashMap<GuestRegister, HostRegister>,
    free_regs: HashSet<Register>,
    borrow_index: usize,
}

impl Registers {
    pub fn new() -> Self {
        let free_regs = (0..16)
            .map(|r| Register::try_from(r).unwrap())
            .filter(|r| !is_reserved(r))
            .collect();

        Self {
            regs: HashMap::new(),
            free_regs,
            borrow_index: 0,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&GuestRegister, &Register)> {
        self.regs
            .iter()
            .map(|(guest, HostRegister { register, .. })| (guest, register))
    }

    /// Map a guest register with id `guest_reg` to a host register. If `lock`
    /// is true, the register will not be dropped automatically, and need to be
    /// manually dropped
    pub fn insert(
        &mut self,
        guest: GuestRegister,
        locked: bool,
    ) -> Result<(&Register, Option<GuestRegister>), InsertError> {
        if let Some(_) = self.regs.get(&guest) {
            return Err(InsertError::AlreadyReserved);
        }

        let mut dropped = None;
        let host_register = if let Some(&next) = self.free_regs.iter().next() {
            next
        } else {
            let mut unused_regs = self.regs.iter().collect::<Vec<_>>();
            unused_regs.sort_by_key(|(_, HostRegister { borrow_index, .. })| borrow_index);

            for (&guest_register, host_register) in unused_regs.into_iter() {
                // We need to know if we can drop this register safely, as
                // tmp registers are intended to be dropped manually.
                if !host_register.locked {
                    self.free_regs.insert(host_register.register);
                    dropped = Some((guest_register, host_register.register));
                    break;
                }
            }

            match dropped {
                Some((_, reg)) => reg,
                None => return Err(InsertError::NoRegistersAvailable),
            }
        };

        // At this point we already know the key does not exist, that's why we call `insert_unique_unchecked`
        self.free_regs.remove(&host_register);
        let (_, HostRegister { ref register, .. }) = self.regs.insert_unique_unchecked(
            guest,
            HostRegister {
                register: host_register,
                borrow_index: self.borrow_index,
                locked,
            },
        );
        self.borrow_index += 1;

        let dropped = dropped.map(|(guest, _)| guest);
        Ok((register, dropped))
    }

    /// Get the host register mapped by the given guest register
    pub fn get(&mut self, guest_reg: &GuestRegister) -> Option<&Register> {
        self.regs.get_mut(&guest_reg).map(|host| {
            host.borrow_index = self.borrow_index;
            self.borrow_index += 1;
            &host.register
        })
    }

    /// Finds the guest register using the given host register
    pub fn find_by_host(&self, host_reg: Register) -> Option<(GuestRegister, Register)> {
        self.regs
            .iter()
            .map(|(&guest, HostRegister { register, .. })| (guest, *register))
            .find(|(_, r)| *r == host_reg)
    }

    /// Drops the given guest register, marking the host register as free to use
    pub fn drop(&mut self, guest_reg: GuestRegister) {
        self.regs
            .remove(&guest_reg)
            .map(|HostRegister { register, .. }| {
                self.free_regs.insert(register);
            });

        // reset the borrow index if there are no register mapped
        if self.regs.len() == 0 {
            self.borrow_index = 0;
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GuestRegister(u16);

#[repr(u16)]
pub enum GuestRegisterKind {
    /// CPU register
    Cpu,
    // /// Coprocessor 0 register
    // Cop0,
    /// Temporary register
    Temporary,
}

impl GuestRegister {
    pub fn new(index: usize, kind: GuestRegisterKind) -> Self {
        Self(((kind as u16) << 8) | (index as u16))
    }
    pub fn cpu(index: usize) -> Self {
        Self::new(index, GuestRegisterKind::Cpu)
    }
    // pub fn cop0(index: usize) -> Self {
    //     Self::new(index, GuestRegisterKind::Cop0)
    // }
    pub fn tmp(id: usize) -> Self {
        Self::new(id, GuestRegisterKind::Temporary)
    }

    pub fn kind(&self) -> GuestRegisterKind {
        unsafe { std::mem::transmute(self.0 >> 8) }
    }

    pub(crate) fn id(&self) -> usize {
        (self.0 & 0xff00) as usize
    }
}

#[derive(Debug, Clone)]
struct HostRegister {
    register: Register,
    borrow_index: usize,
    locked: bool,
}

impl Hash for HostRegister {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.register.hash(state);
    }
}

// Register that should not be mapped
fn is_reserved(register: &Register) -> bool {
    match register {
        Register::Rsi | Register::Rsp | Register::Rbp => true,
        _ => false,
    }
}
