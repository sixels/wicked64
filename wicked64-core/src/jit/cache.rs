use std::{cell::RefCell, rc::Rc};

use crate::{n64::State, utils::btree_range::BTreeRange};

use super::{compiler::ExecBuffer, JitEngine};

#[derive(Clone)]
pub struct CompiledBlock {
    state: Rc<RefCell<State>>,
    exec_buf: ExecBuffer,
}

impl CompiledBlock {
    pub fn new(state: Rc<RefCell<State>>, buf: ExecBuffer) -> Self {
        Self {
            state,
            exec_buf: buf,
        }
    }

    pub(crate) fn execute(&self) -> usize {
        let _state = self.state.borrow_mut();
        self.exec_buf.execute();
        0
    }
}

pub struct Cache {
    jit: JitEngine,
    blocks: BTreeRange<Rc<CompiledBlock>>,
}

impl Cache {
    /// Get a compiled block from the cache or create if no entries were found
    pub fn get_or_compile(&mut self, addr: usize, state: &Rc<RefCell<State>>) -> Rc<CompiledBlock> {
        if let Some(block) = self.blocks.get_exact(addr) {
            return block.clone();
        }

        // .or_insert_with({
        let (block, len) = {
            let state = state.clone();
            tracing::debug!("Generating cache for address '0x{addr:08x}'");

            let cycles = 1024;

            let (buf, len) = self.jit.compile_block(state.clone(), cycles);

            tracing::debug!("Generated code: {:02x?}", buf.as_slice());
            (Rc::new(CompiledBlock::new(state, buf)), len)
        };
        self.blocks.insert(addr..=addr + len, block.clone());

        if let Some(invalidated_range) = state.borrow_mut().cache_invalidation.take() {
            self.blocks.retain(|(start, end), _| {
                !(invalidated_range.0 <= end && invalidated_range.1 >= start)
            });
        }

        block
    }
}

impl Default for Cache {
    /// Creates a new cache manager for jitted instructions
    fn default() -> Cache {
        Self {
            jit: JitEngine::new(),
            blocks: BTreeRange::new(),
        }
    }
}
