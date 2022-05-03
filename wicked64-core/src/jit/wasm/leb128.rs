pub mod signed {
    use std::io;

    use byteorder::WriteBytesExt;

    pub fn write_i64<W>(buf: &mut W, value: i64) -> Result<usize, io::Error>
    where
        W: ?Sized + io::Write,
    {
        let mut value = value;
        let mut n_bytes = 0;

        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0) {
                buf.write_u8(byte)?;
                break;
            }
            buf.write_u8(byte | 0x80)?;
            n_bytes += 1;
        }

        Ok(n_bytes + 1)
    }

    pub fn write_i32<W>(buf: &mut W, value: i32) -> Result<usize, io::Error>
    where
        W: ?Sized + io::Write,
    {
        let mut value = value;
        let mut n_bytes = 0;

        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0) {
                buf.write_u8(byte)?;
                break;
            }
            buf.write_u8(byte | 0x80)?;
            n_bytes += 1;
        }

        Ok(n_bytes + 1)
    }
}

pub mod unsigned {
    use std::io;

    use byteorder::WriteBytesExt;

    pub fn write_u64<W>(buf: &mut W, value: u64) -> Result<usize, io::Error>
    where
        W: ?Sized + io::Write,
    {
        let mut value = value;
        let mut n_bytes = 0;

        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                buf.write_u8(byte)?;
                break;
            }
            buf.write_u8(byte | 0x80)?;
            n_bytes += 1;
        }

        Ok(n_bytes + 1)
    }

    pub fn write_u32<W>(buf: &mut W, value: u32) -> Result<usize, io::Error>
    where
        W: ?Sized + io::Write,
    {
        self::write_u64(buf, value as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::signed;
    use super::unsigned;

    #[test]
    fn signed() {
        let mut buf = Vec::new();
        assert_eq!(signed::write_i32(&mut buf, 0x9FFFFFFFu32 as i32).unwrap(), 5);
        assert_eq!(buf, b"\xff\xff\xff\xff\x79");
    }

    #[test]
    fn unsigned() {
        let mut buf = Vec::new();
        assert_eq!(unsigned::write_u32(&mut buf, 0x9FFFFFFF).unwrap(), 5);
        assert_eq!(buf, b"\xff\xff\xff\xff\x09");
    }
}
