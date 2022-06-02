use std::io;

use byteorder::WriteBytesExt;

pub trait Emitter: io::Write {
    // TODO
}

impl<T: io::Write> Emitter for T {}
