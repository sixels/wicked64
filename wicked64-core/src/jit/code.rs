use std::ops::{Deref, DerefMut};

use crate::jit::codegen::Emitter;
use crate::n64::SyncState;

use super::codegen::register::X64Gpr;

/// A compiled Wasm block
#[derive(Clone)]
pub struct CompiledBlock {
    pub len: usize,
}

impl CompiledBlock {
    pub fn new(state: SyncState, bytes: &[u8], len: usize) -> anyhow::Result<CompiledBlock> {
        todo!();
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Execute the generated code
    pub fn exec(&self) {
        todo!()
    }
}

pub struct RawBlock(Vec<u8>);

impl RawBlock {
    pub fn new() -> Self {
        let prelude = || -> std::io::Result<Vec<u8>> {
            let mut code = Vec::new();

            code.emit_push_reg(X64Gpr::Rax)?;

            Ok(code)
        };

        let code = match prelude() {
            Ok(code) => code,
            Err(error) => panic!("Could not generate the prelude instructions: {error:?}"),
        };

        Self(code)
    }
    pub fn compile(mut self, state: SyncState) -> anyhow::Result<CompiledBlock> {
        // pop rax
        self.emit_pop_reg(X64Gpr::Rax)?;

        CompiledBlock::new(state, self.as_slice(), self.len())
    }
}

impl Deref for RawBlock {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RawBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
