pub(crate) mod cp0;
pub(crate) mod instruction;
pub mod signals;

/// CPU frequency in HZ
#[allow(dead_code)]
pub const CPU_FREQUENCY: u32 = 93_750_000; // 93.75MHz

use std::marker::PhantomData;

use bitvec::{field::BitField, order::Msb0, view::BitView};
use byteorder::ByteOrder;

use crate::mmu::{
    map::{addr_map, VirtualMemoryMap},
    MemoryUnit,
};

use self::{cp0::Cp0, instruction::Instruction, signals::ResetSignal};

/// The N64 CPU (VR4300).
///
/// The CPU has:
/// - 32 64-bit general purpose registers (GPR)
/// - 32 64-bit floating-point general purpose registers (FGR)
/// - 64-bit program counter (PC)
/// - 64-bit register containing the integer multiply and divide high-order
/// double-word result (HI)
/// - 64-bit register containing the integer multiply and divide low-order
/// double-word result (LO)
/// - 1-bit load/link register (LLbit)
/// - 32-bit floating-point Implementation/Revision register, FCR0
/// - 32-bit floating-point Control/Status register, FCR31
///
/// Two of the general purpose registers have assigned functions:
/// - r0 is hardwired to a value zero
/// - r31 is the link register used by 'JAL' and 'JALR' instructions
#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
pub struct Cpu<O: ByteOrder> {
    /// General Purpose Registers
    pub gpr: [u64; 32],
    /// Floating-point General purpose Registers
    pub fgr: [u64; 32],
    /// Program Counter. Always represents a VIRTUAL address
    pub pc: u64,
    /// Multiplication HI register
    pub multi_hi: u64,
    /// Multiplication LO register
    pub multi_lo: u64,
    /// Load/Link Register
    pub ll: u8,
    /// Floating-point Implementation/Revision Register
    pub fcr0: u32,
    /// Floating-point Control/Status Register
    pub fcr32: u32,

    /// Coprocessor 0
    pub cp0: Cp0,

    pub reset_signal: u8,
    pub cold_reset_clocks: u64,
    pub soft_reset_clocks: u64,

    /// Keep track of the total amount of clocks
    pub clocks: u64,

    pub _endianness: PhantomData<O>,
}

#[allow(dead_code)]
impl<O: ByteOrder> Cpu<O> {
    /// Create a new CPU
    pub fn new<M: 'static + MemoryUnit + Sized>(simulate_pif: bool, mmu: &mut M) -> Self {
        let mut cpu = Self::default().power_on();
        simulate_pif.then(|| cpu.simulate_pif(mmu));
        cpu
    }

    /// Fetch a instructions at virtual address `addr`
    pub fn fetch_instruction<M: MemoryUnit + Sized>(
        &self,
        mmu: &M,
        addr: u64,
    ) -> anyhow::Result<Instruction> {
        let phys_pc = self.translate_virtual(addr as usize);
        Instruction::try_from(mmu.read::<u32, O>(phys_pc))
    }

    /// Translates a virtual address into a physical address
    pub fn translate_virtual(&self, addr: usize) -> usize {
        match VirtualMemoryMap::from(addr) {
            VirtualMemoryMap::KSEG0 => addr - (*addr_map::virt::KSEG0_RANGE.start()),
            VirtualMemoryMap::KSEG1 => addr - (*addr_map::virt::KSEG1_RANGE.start()),
            mm => panic!("Unhandled Virtual Memory segment: {mm:?} (0x{addr:08x})."),
        }
    }

    /// Perform a Power-On-Reset procedure.
    fn power_on(mut self) -> Self {
        self.reset_signal = ResetSignal::PowerOnReset;
        self.handle_reset_signal();

        self.pc = 0xBFC00000;

        self
    }

    /// Simulates the PIF ROM.
    ///
    /// The side effects of this procedure
    /// [can be found in more details here](https://n64.readthedocs.io/#boot-process).
    fn simulate_pif<M: 'static + MemoryUnit + Sized>(&mut self, mmu: &mut M) {
        self.gpr = {
            let mut gpr = [0; 32];

            gpr[11] = 0xffffffffa4000040;
            gpr[20] = 0x0000000000000001;
            gpr[22] = 0x000000000000003f;
            gpr[29] = 0xffffffffa4001ff0;

            gpr
        };

        self.cp0 = Cp0 {
            random: 0x1f,
            // ERL, BEV -> 1
            // CU -> 01111
            // KSU -> kernel (0)
            // the rest is 0 or fixed value.
            status: cp0::Status { bits: 0x70400004 },
            prid: 0x00000B00,
            // K0 -> cache
            // the rest is 0 or fixed value.
            config: cp0::Config { bits: 0x0006e463 },

            ..Cp0::default()
        };

        // The first 0x1000 bytes from the cartridge are then copied to SP DMEM.
        // This is implemented as a copy of 0x1000 bytes from 0xB0000000 (VIRTUAL) to
        // 0xA4000000 (VIRTUAL).
        mmu.copy_from(
            self.translate_virtual(0xa4000000),
            self.translate_virtual(0xb0000000),
            0x1000,
        );

        // The program counter is then set to 0xA4000040 (VIRTUAL). Note that this skips
        // the first 0x40 bytes of the ROM, as this is where the header is
        // stored. Also note that execution begins with the CPU executing out of
        // SP DMEM.
        self.pc = 0xa4000040;

        self.reset_signal = ResetSignal::None;
    }

    /// VR4300's Reset handler.
    fn handle_reset_signal(&mut self) {
        match self.reset_signal {
            ResetSignal::PowerOnReset => {
                self.perform_power_on_reset();
            }
            // Cold Reset
            ResetSignal::ColdReset => {
                self.perform_cold_reset();
            }
            // Soft Reset:
            // Restarts processor, but does not affect clocks. The major
            // part of the initial status of the processor can be retained
            // by using soft reset.
            ResetSignal::Reset => {
                self.reset_signal = unsafe { std::mem::transmute(0u8) };
                todo!()
            }
            // Resets already performed.
            ResetSignal::ColdResetActive | ResetSignal::ResetActive => {}
            // No resets to perform. Do nothing.
            ResetSignal::None => {}
            _ => unreachable!(),
        }
    }

    /// Perform a Power-On-Reset.
    ///
    /// When the ColdReset signal is asserted active after the power
    /// is applied and has become stable all clocks are restarted. A
    /// Power-ON Reset completely initializes the internal state of
    /// the processor without saving any state information.
    ///
    /// # Procedure Effect
    ///
    /// `cp0.status.{TS, SR, RP} = 0` and `cp0.config.EP[3:0] = 0`
    ///
    /// `cp0.status.{ERL, BEV} = 1` and `cp0.config.{BE} = 1`
    ///
    /// `cp0.random = 0x1f` (upper-limit value)
    ///
    /// `cp0.config.EP[2:0] = div_mode[1:0]`
    ///
    /// all other registers are undefined.
    ///
    /// # NOTE
    ///
    /// The official CPU documentation may have made a typo when it
    /// refers to the REV bit of the Status register. It should be BEV
    /// instead (the same for the cold-reset procedure).
    fn perform_power_on_reset(&mut self) {
        // the only difference between cold-reset and power-on-reset is that the
        // second set bits 0..=2 of the cp0.config.EP bit
        self.perform_cold_reset();
        // TODO: cp0.config.EP[2:0] = div_mode[1:0]
    }

    /// Perform a Cold-Reset.
    ///
    /// When the ColdReset signal is asserted active while the
    /// processor is operating, all clocks are restarted. A Cold Reset
    /// completely initializes the internal state of the processor
    /// without saving any state information.
    ///
    /// # Procedure effect
    ///
    /// `cp0.status.{TS, SR, RP} = 0` and `cp0.config.EP[3:0] = 0`
    ///
    /// `cp0.status.{ERL, BEV} = 1` and `cp0.config.{BE} = 1`
    ///
    /// `cp0.random = 0x1f` (upper-limit value)
    ///
    /// all other registers are undefined.
    fn perform_cold_reset(&mut self) {
        // cp0.status
        {
            let bits = self.cp0.status.bits.view_bits_mut::<Msb0>();

            // set ts,sr and rp
            bits.set(cp0::Status::BIT_TS_OFFSET, false);
            bits.set(cp0::Status::BIT_SR_OFFSET, false);
            bits.set(cp0::Status::BIT_RP_OFFSET, false);

            // set erl and bev
            bits.set(cp0::Status::BIT_ERL_OFFSET, true);
            bits.set(cp0::Status::BIT_BEV_OFFSET, true);
            // part of the initial status of the processor can be retained
        };

        // cp0.config
        {
            let bits = self.cp0.config.bits.view_bits_mut::<Msb0>();
            bits[cp0::Config::BIT_EP_RANGE].store_be(0u32);

            bits.set(cp0::Config::BIT_BE_OFFSET, true);
        };

        self.cp0.random = 0x1f;

        // disable (ColdReset | PowerOnReset) and enable ColdResetActive
        self.reset_signal = self::signals::disable_signal(
            self.reset_signal,
            ResetSignal::ColdReset | ResetSignal::PowerOnReset,
        ) | ResetSignal::ColdResetActive;

        self.cold_reset_clocks = 64000;
        self.clocks = 0;
    }

    /// Update the currently active reset signal by the given amount of clocks
    fn update_reset_signal(&mut self, clocks: u64) {
        // decrement Cold-Reset clocks
        let dec_cr_clocks = |cpu: &mut Cpu<O>| {
            cpu.cold_reset_clocks = cpu.cold_reset_clocks.saturating_sub(clocks);
            cpu.cold_reset_clocks
        };
        // decrement Reset clocks
        let dec_sr_clocks = |cpu: &mut Cpu<O>| {
            cpu.soft_reset_clocks = cpu.soft_reset_clocks.saturating_sub(clocks);
            cpu.soft_reset_clocks
        };

        // (is Cold-Reset active?, is Reset active?)
        match (
            self.reset_signal & ResetSignal::ColdResetActive != 0,
            self.reset_signal & ResetSignal::ResetActive != 0,
        ) {
            // Both signals are active. Await Cold-Reset to complete, then keep Reset active for 16 clocks.
            (true, true) => {
                let rem = dec_cr_clocks(self);
                if rem == 0 {
                    self.soft_reset_clocks = 16;
                }
            }
            // Cold-Reset active.
            (true, false) => {
                let rem = dec_cr_clocks(self);
                // remaining clocks reached 0, disable Cold-Reset
                if rem == 0 {
                    self.reset_signal = ResetSignal::disable_cold_reset(self.reset_signal);
                }
            }
            // Reset active.
            (false, true) => {
                let rem = dec_sr_clocks(self);
                // remaining clocks reached 0, disable cold-reset
                if rem == 0 {
                    self.reset_signal = ResetSignal::disable_soft_reset(self.reset_signal);
                }
            }
            // No signals active.
            (false, false) => {}
        };
    }
}

#[cfg(test)]
mod tests {
    use bitvec::bits;
    use byteorder::BigEndian;

    use crate::cpu::cp0::status::ExecutionMode;
    use crate::cpu::cp0::Config;
    use crate::hardware::Cartridge;
    use crate::mmu::MemoryManager;

    use super::cp0::Status;
    use super::*;

    /// Checks CPU registers after a Power-On reset
    #[test]
    fn power_on_procedure() {
        let mut dummy = Box::new([0u8; 100]) as Box<[u8]>;

        // prevent PIF boot side effects
        let cpu = Cpu::<BigEndian>::new(false, &mut dummy);

        let cp0 = &cpu.cp0;
        // cp0.random = 0x1f (upper-limit value)
        assert!(cp0.random == 0x1f);

        // cp0.status.{TS, SR, RP} = 0
        // cp0.status.{ERL, BEV} = 1
        let status_bits = cp0.status.bits.view_bits::<Msb0>();
        assert_eq!(
            status_bits.get(Status::BIT_TS_OFFSET).as_deref(),
            Some(&false)
        );
        assert_eq!(
            status_bits.get(Status::BIT_SR_OFFSET).as_deref(),
            Some(&false)
        );
        assert_eq!(
            status_bits.get(Status::BIT_RP_OFFSET).as_deref(),
            Some(&false)
        );

        assert_eq!(
            status_bits.get(Status::BIT_ERL_OFFSET).as_deref(),
            Some(&true)
        );
        assert_eq!(
            status_bits.get(Status::BIT_BEV_OFFSET).as_deref(),
            Some(&true)
        );

        // cp0.config.EP[3:0] = 0
        // cp0.config.{BE} = 1
        // cp0.config.EC[2:0] = div_mode[1:0]
        use bitvec::prelude::*;
        let config_bits = cp0.config.bits.view_bits::<Msb0>();
        assert_eq!(
            config_bits.get(Config::BIT_EP_RANGE),
            Some(bits![u32, Msb0; 0, 0, 0, 0])
        );
        assert_eq!(
            config_bits.get(Config::BIT_BE_OFFSET).as_deref(),
            Some(&true)
        );
        // TODO: cp0.config.EC[2:0] = div_mode[1:0]

        assert!((cpu.reset_signal | ResetSignal::ColdResetActive) == ResetSignal::ColdResetActive)
    }

    #[test]
    fn pif_behavior() {
        crate::tests::init_trace();

        let mut mmu = {
            let cartridge = Cartridge::open("../assets/test-roms/dillonb/basic.z64").unwrap();
            MemoryManager::new(cartridge)
        };

        let cpu = Cpu::<BigEndian>::new(true, &mut mmu);

        assert_eq!(cpu.gpr[11], 0xffffffffa4000040);
        assert_eq!(cpu.gpr[20], 0x0000000000000001);
        assert_eq!(cpu.gpr[22], 0x000000000000003f);
        assert_eq!(cpu.gpr[29], 0xffffffffa4001ff0);
        assert_eq!(cpu.cp0.random, 0x1f);
        assert_eq!(cpu.cp0.prid, 0x00000B00);

        let status = &cpu.cp0.status;
        assert!(status.get_bit(cp0::Status::BIT_ERL_OFFSET));
        assert!(status.get_bit(cp0::Status::BIT_BEV_OFFSET));
        assert!(status.get_bits::<u8>(cp0::Status::BIT_CU_RANGE) == 0b0111);
        assert_eq!(status.get_execution_mode(), ExecutionMode::Kernel);

        let config = &cpu.cp0.config;
        assert!(config.get_bits::<u16>(4..=14) == 0b11001000110);
        assert!(config.get_bits::<u8>(16..=23) == 0b00000110);
        assert!(config.get_bit(31) == false);
        assert!(config.get_bits::<u8>(cp0::Config::BIT_K0_RANGE) == 0b011);

        let mut rom_title = Vec::with_capacity(20);
        for i in 0x04000020..0x04000034 {
            rom_title.push(mmu.read::<u8, BigEndian>(i));
        }

        assert_eq!(b"Dillon's N64 Tests\x20\x20".as_slice(), &rom_title);
    }
}
