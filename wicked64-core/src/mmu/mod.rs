pub mod map;
pub mod memory;
pub mod num;

use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use byteorder::ByteOrder;
use enum_dispatch::enum_dispatch;

pub use memory::MemoryManager;

use self::num::MemInteger;
use crate::io::Cartridge;

#[enum_dispatch(MemoryUnit)]
#[derive(Debug)]
enum MemoryUnits {
    BoxedSlice(Box<[u8]>),
    Cartridge,
}

#[enum_dispatch]
pub trait MemoryUnit {
    fn read<I, O>(&self, addr: usize) -> I
    where
        I: MemInteger + Sized + Debug,
        O: ByteOrder + Sized;

    fn store<I, O>(&mut self, addr: usize, value: I)
    where
        I: MemInteger + Sized + Debug,
        O: ByteOrder + Sized;

    // Copy `n` bytes from `src` to `dst`
    fn copy_from(&mut self, dst: usize, src: usize, n: usize)
    where
        Self: Sized,
    {
        let _ = (dst, src, n);
        unimplemented!()
    }

    fn buffer(&self) -> &[u8] {
        unimplemented!()
    }
    fn buffer_mut(&mut self) -> &mut [u8] {
        unimplemented!()
    }
}

impl MemoryUnit for Box<[u8]> {
    fn read<I, O>(&self, addr: usize) -> I
    where
        Self: Sized,
        I: MemInteger + Sized,
        O: ByteOrder + Sized,
    {
        I::read_from::<O>(&self[addr..addr + I::SIZE])
    }

    fn store<I, O>(&mut self, addr: usize, value: I)
    where
        Self: Sized,
        I: MemInteger + Sized,
        O: ByteOrder + Sized,
    {
        I::write_to::<O>(&mut self[addr..addr + I::SIZE], value);
    }

    fn buffer(&self) -> &[u8] {
        self.deref()
    }
    fn buffer_mut(&mut self) -> &mut [u8] {
        self.deref_mut()
    }
}
