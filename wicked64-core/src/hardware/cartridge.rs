use std::path::Path;

use byteorder::ByteOrder;

use crate::mmu::{num::MemInteger, MemoryUnit};

/// n64 cartridges may have more than 64 megabytes (ouch!).
/// 38 megabytes should be enough to play most games.
pub const CARTRIDGE_SIZE_IN_BYTES: usize = 38 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartridgeEndianness {
    /// Used by .z64 ROM
    Big,
    /// Used by .n64 ROM
    Little,
    /// Used by .v64 ROM
    ByteSwapped,
}

/// N64 Game Pak cartridge
#[derive(Debug)]
pub struct Cartridge {
    pub(crate) data: Box<[u8]>,
}

impl Cartridge {
    /// Create a new Cartridge from the given rom file
    pub fn open<P: AsRef<Path>>(rom_path: P) -> anyhow::Result<Cartridge> {
        let content = std::fs::read(rom_path)?;

        assert!(
            content.len() <= CARTRIDGE_SIZE_IN_BYTES,
            "Your cartridge is too large. The maximum cartridge size is {}MB",
            CARTRIDGE_SIZE_IN_BYTES / 1024 / 1024
        );

        let data = content.into_boxed_slice();

        Ok(Self { data })
    }

    pub fn endianness(&self) -> Result<CartridgeEndianness, ()> {
        match self.data[0] {
            0x80 => Ok(CartridgeEndianness::Big),
            0x40 => Ok(CartridgeEndianness::Little),
            0x37 => Ok(CartridgeEndianness::ByteSwapped),
            _ => Err(()),
        }
    }
}

impl MemoryUnit for Cartridge {
    fn read<I: MemInteger, O: ByteOrder>(&self, addr: usize) -> I {
        I::read_from::<O>(&self.data[addr..addr + I::SIZE])
    }
    fn store<I: MemInteger, O: ByteOrder>(&mut self, addr: usize, value: I) {
        I::write_to::<O>(&mut self.data[addr..addr + I::SIZE], value);
    }
    fn buffer(&self) -> &[u8] {
        &self.data
    }
    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_get_the_cartridge_endianness() {
        let cartridge = Cartridge::open("../assets/test-roms/dillonb/basic.z64").unwrap();
        assert_eq!(cartridge.endianness(), Ok(CartridgeEndianness::Big));
    }
}
