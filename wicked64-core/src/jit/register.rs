use std::hash::Hash;

use hashbrown::{HashMap, HashSet};
use iced_x86::code_asm::{registers::gpr64, AsmRegister64};

const REGISTER_SET: [AsmRegister64; 16] = [
    gpr64::rax,
    gpr64::rcx,
    gpr64::rdx,
    gpr64::rbx,
    gpr64::rsp,
    gpr64::rbp,
    gpr64::rsi,
    gpr64::rdi,
    gpr64::r8,
    gpr64::r9,
    gpr64::r10,
    gpr64::r11,
    gpr64::r12,
    gpr64::r13,
    gpr64::r14,
    gpr64::r15,
];

pub const CALLEE_SAVED_REGISTERS: [AsmRegister64; 7] = [
    gpr64::rbx,
    gpr64::rsi,
    gpr64::rbp,
    gpr64::r12,
    gpr64::r13,
    gpr64::r14,
    gpr64::r15,
];

pub const ARGS_REGS: &[AsmRegister64; 6] = &[
    gpr64::rdi,
    gpr64::rsi,
    gpr64::rdx,
    gpr64::rcx,
    gpr64::r8,
    gpr64::r9,
];

#[derive(Debug, Clone, Copy, Eq)]
pub struct Register(AsmRegister64);

impl PartialEq for Register {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Hash for Register {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        iced_x86::Register::from(self.0).hash(state);
    }
}

#[derive(Debug)]
pub enum InsertError {
    AlreadyReserved,
}

#[derive(Debug, Clone)]
pub struct Registers {
    regs: HashMap<GuestRegister, HostRegister>,
    free_regs: HashSet<Register>,
    borrow_index: usize,
}

impl Registers {
    pub fn new() -> Self {
        Self::with_registers(&REGISTER_SET)
    }
    pub fn with_registers(regs: &[AsmRegister64]) -> Self {
        let free_regs = regs
            .iter()
            .copied()
            .filter(|&r| !is_reserved(r))
            .map(Register)
            .collect();

        Self {
            regs: HashMap::new(),
            free_regs,
            borrow_index: 0,
        }
    }

    pub fn exclude_register(
        &mut self,
        host_reg: AsmRegister64,
    ) -> Option<(GuestRegister, AsmRegister64)> {
        let reg = Register(host_reg);
        if !self.free_regs.contains(&reg) {
            if let Some((guest_reg, _)) = self.find_by_host(host_reg) {
                return self.free(guest_reg);
            }
        }
        self.free_regs.remove(&reg);
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = (&GuestRegister, &AsmRegister64)> {
        self.regs
            .iter()
            .map(|(guest, HostRegister { register, .. })| (guest, &register.0))
    }

    /// Map a guest register with id `guest_reg` to a host register. If `lock`
    /// is true, the register will not be dropped automatically, and need to be
    /// manually dropped
    pub fn insert(
        &mut self,
        guest: GuestRegister,
    ) -> Result<(&AsmRegister64, Option<GuestRegister>), InsertError> {
        if self.regs.get(&guest).is_some() {
            return Err(InsertError::AlreadyReserved);
        }

        let (host_register, dropped_guest) = if let Some(&next) = self.free_regs.iter().next() {
            (next, None)
        } else {
            let mut regs = self.regs.iter().collect::<Vec<_>>();

            let (_, (&guest_reg, host_reg), _) =
                regs.select_nth_unstable_by_key(0, |(_, host)| host.borrow_index);
            self.free_regs.insert(host_reg.register);
            (host_reg.register, Some(guest_reg))
        };

        // At this point we already know the key does not exist, that's why we call `insert_unique_unchecked`
        self.free_regs.remove(&host_register);
        let (_, HostRegister { ref register, .. }) = self.regs.insert_unique_unchecked(
            guest,
            HostRegister {
                register: host_register,
                borrow_index: self.borrow_index,
            },
        );
        self.borrow_index += 1;

        Ok((&register.0, dropped_guest))
    }

    /// Get the host register mapped by the given guest register
    pub fn get(&mut self, guest_reg: GuestRegister) -> Option<&AsmRegister64> {
        self.regs.get_mut(&guest_reg).map(|host| {
            host.borrow_index = self.borrow_index;
            self.borrow_index += 1;
            &host.register.0
        })
    }

    /// Finds the guest register using the given host register
    pub fn find_by_host(&self, host_reg: AsmRegister64) -> Option<(GuestRegister, AsmRegister64)> {
        self.regs
            .iter()
            .map(|(&guest, HostRegister { register, .. })| (guest, *register))
            .find(|(_, r)| r.0 == host_reg)
            .map(|(guest, Register(host))| (guest, host))
    }

    /// Drops the given guest register, marking the host register as free to use
    pub fn free(&mut self, guest_reg: GuestRegister) -> Option<(GuestRegister, AsmRegister64)> {
        let mut dropped = None;

        if let Some(HostRegister {
            register,
            borrow_index,
            ..
        }) = self.regs.remove(&guest_reg)
        {
            tracing::info!("Dropping register {register:?} with borrow_index of {borrow_index}");
            self.free_regs.insert(register);
            dropped = Some((guest_reg, register.0));
        }

        // reset the borrow index if there are no registers mapped
        if self.regs.is_empty() {
            self.borrow_index = 0;
        }
        dropped
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum GuestRegister {
    /// CPU register
    Cpu(u8),
    Pc,
    // /// Coprocessor 0 register
    // Cop0,
}

impl GuestRegister {
    pub fn cpu(index: u8) -> Self {
        Self::Cpu(index as u8)
    }
    pub fn pc() -> Self {
        Self::Pc
    }
    // pub fn cop0(index: usize) -> Self {
    //     Self::new(index, GuestRegisterKind::Cop0)
    // }
}

#[derive(Debug, Clone)]
struct HostRegister {
    register: Register,
    borrow_index: usize,
}

impl Hash for HostRegister {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.register.hash(state);
    }
}

// Register that should not be mapped
fn is_reserved(register: AsmRegister64) -> bool {
    matches!(register, gpr64::rsi | gpr64::rsp | gpr64::rbp | gpr64::rbx)
}
