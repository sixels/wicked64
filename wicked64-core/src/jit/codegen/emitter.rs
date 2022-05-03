use std::io;

use byteorder::WriteBytesExt;
// use wicked64_arena as arena;

pub trait Emitter: io::Write {
    // TODO
}

impl<T: io::Write> Emitter for T {}
