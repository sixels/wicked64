pub mod map;
pub mod memory;
pub mod num;

use std::fmt::Debug;

use byteorder::ByteOrder;
use enum_dispatch::enum_dispatch;

pub use memory::MemoryManager;

use self::num::MemInteger;
use crate::io::Cartridge;

#[enum_dispatch(MemoryUnit)]
#[derive(Debug)]
enum GenericMemoryUnit {
    BoxedSlice(Box<[u8]>),
    Cartridge,
}

#[enum_dispatch]
pub trait MemoryUnit {
    /// Read an integer `I` from address `addr`
    fn read<I, O>(&self, addr: usize) -> I
    where
        I: MemInteger,
        O: ByteOrder;

    /// Store an integer `value` of type `I` into address `addr`
    fn store<I, O>(&mut self, addr: usize, value: I)
    where
        I: MemInteger,
        O: ByteOrder;

    /// Copy `n` bytes from `src` to `dst`
    fn copy_from(&mut self, dst: usize, src: usize, n: usize) {
        self.buffer_mut().copy_within(src..src + n, dst);
    }
    /// Get a reference to the memory buffer
    fn buffer(&self) -> &[u8] {
        unimplemented!()
    }
    /// Get a mutable reference to the memory buffer
    fn buffer_mut(&mut self) -> &mut [u8] {
        unimplemented!()
    }
}

impl MemoryUnit for Box<[u8]> {
    fn read<I, O>(&self, addr: usize) -> I
    where
        I: MemInteger,
        O: ByteOrder,
    {
        I::read_from::<O>(&self[addr..addr + I::SIZE])
    }

    fn store<I, O>(&mut self, addr: usize, value: I)
    where
        I: MemInteger,
        O: ByteOrder,
    {
        I::write_to::<O>(&mut self[addr..addr + I::SIZE], value);
    }

    fn buffer(&self) -> &[u8] {
        self
    }
    fn buffer_mut(&mut self) -> &mut [u8] {
        self
    }
}
