pub mod map;
pub mod memory;
pub mod num;

use byteorder::ByteOrder;
pub use memory::MemoryManager;

use self::num::MemInteger;

pub trait MemoryUnit {
    fn read<I, O>(&self, addr: usize) -> I
    where
        I: MemInteger + Sized,
        O: ByteOrder + Sized;

    fn store<I, O>(&mut self, addr: usize, value: I)
    where
        I: MemInteger + Sized,
        O: ByteOrder + Sized;

    // Copy `n` bytes from `src` to `dst`
    fn copy_from(&mut self, dst: usize, src: usize, n: usize)
    where
        Self: Sized,
    {
        let _ = (dst, src, n);
        unimplemented!()
    }
}

impl<const N: usize> MemoryUnit for [u8; N] {
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
}

impl<const N: usize> MemoryUnit for Box<[u8; N]> {
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
}
