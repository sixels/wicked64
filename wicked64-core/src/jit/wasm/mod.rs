pub mod leb128;

pub use self::sections::*;

#[allow(dead_code)]
mod sections {
    use std::ops::{Deref, DerefMut};

    use crate::jit::wasm;

    pub trait WasmSection: Sized {
        type Buffer;

        fn new(n: u32) -> Self;
        fn id() -> u8;
        fn buffer(&self) -> &Self::Buffer;
        fn cont(&self) -> u32;

        fn build(self) -> Vec<u8>
        where
            Self: Sized,
            Self::Buffer: AsRef<[u8]>,
        {
            let buffer = self.buffer().as_ref();

            let mut final_block = Vec::with_capacity(buffer.len() + 5);

            final_block.push(Self::id());

            let (leb_block_len, cont_section) = {
                let mut cont_section = Vec::new();
                let block_len = buffer.len()
                    + wasm::leb128::signed::write_i32(&mut cont_section, self.cont() as i32)
                        .unwrap();

                let mut buf = Vec::new();
                wasm::leb128::signed::write_i32(&mut buf, block_len as i32).unwrap();
                (buf, cont_section)
            };

            final_block.extend(leb_block_len);
            final_block.extend(cont_section);

            final_block.extend(buffer);

            final_block
        }
    }

    macro_rules! define_section {
        ($name:ident, $id: literal) => {
            pub struct $name {
                cont: u32,
                block: Vec<u8>,
            }

            impl WasmSection for $name {
                type Buffer = Vec<u8>;

                fn new(n: u32) -> Self {
                    Self {
                        cont: n,
                        block: Vec::new(),
                    }
                }
                fn id() -> u8 {
                    $id
                }
                fn cont(&self) -> u32 {
                    self.cont
                }
                fn buffer(&self) -> &Self::Buffer {
                    &self.block
                }
            }

            impl Deref for $name {
                type Target = Vec<u8>;

                fn deref(&self) -> &Self::Target {
                    &self.block
                }
            }
            impl DerefMut for $name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.block
                }
            }
        };
    }

    define_section!(TypeSection, 0x01);
    define_section!(ImportSection, 0x02);
    define_section!(FunctionSection, 0x03);
    define_section!(GlobalSection, 0x06);
    define_section!(ExportSection, 0x07);
    define_section!(CodeSection, 0x0A);

    impl CodeSection {
        pub fn push_function(&mut self, bytes: &[u8]) {
            wasm::leb128::signed::write_i32(self.deref_mut(), bytes.len() as i32).unwrap();
            self.extend(bytes);
        }
    }
}
