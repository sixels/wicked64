use std::fmt::{Debug, Display};

use byteorder::ByteOrder;

pub trait MemInteger:
    Copy + Clone + Default + Ord + PartialOrd + Eq + PartialEq + Send + Sized + Display + Debug
{
    const SIZE: usize;

    fn truncate_u64(n: u64) -> Self;
    fn read_from<O: ByteOrder>(buf: &[u8]) -> Self;
    fn write_to<O: ByteOrder>(buf: &mut [u8], value: Self);
}

impl MemInteger for u8 {
    const SIZE: usize = 1;

    fn truncate_u64(n: u64) -> Self {
        n as u8
    }
    fn read_from<O: ByteOrder>(buf: &[u8]) -> Self {
        buf[0]
    }
    fn write_to<O: ByteOrder>(buf: &mut [u8], value: Self) {
        buf[0] = value;
    }
}

impl MemInteger for u16 {
    const SIZE: usize = 1;

    fn truncate_u64(n: u64) -> Self {
        n as u16
    }
    fn read_from<O: ByteOrder>(buf: &[u8]) -> Self {
        O::read_u16(buf)
    }
    fn write_to<O: ByteOrder>(buf: &mut [u8], value: Self) {
        O::write_u16(buf, value);
    }
}

impl MemInteger for u32 {
    const SIZE: usize = 4;

    fn truncate_u64(n: u64) -> Self {
        n as u32
    }
    fn read_from<O: ByteOrder>(buf: &[u8]) -> Self {
        O::read_u32(buf)
    }
    fn write_to<O: ByteOrder>(buf: &mut [u8], value: Self) {
        O::write_u32(buf, value);
    }
}

impl MemInteger for u64 {
    // type Type = Self;

    const SIZE: usize = 8;

    fn truncate_u64(n: u64) -> Self {
        n as u64
    }
    fn read_from<O: ByteOrder>(buf: &[u8]) -> Self {
        O::read_u64(buf)
    }
    fn write_to<O: ByteOrder>(buf: &mut [u8], value: Self) {
        O::write_u64(buf, value);
    }
}
