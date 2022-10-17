use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use hashbrown::HashMap;

use crate::n64::State;

use super::{compiler::ExecBuffer, JitEngine};

#[derive(PartialEq, Eq, Clone, Copy)]
enum CacheRange {
    Start,
    End,
}

#[derive(Clone)]
pub struct CompiledBlock {
    state: Rc<RefCell<State>>,
    exec_buf: Rc<ExecBuffer>,
    start: usize,
    len: usize,
}

impl CompiledBlock {
    pub fn new(state: Rc<RefCell<State>>, buf: ExecBuffer, start: usize, len: usize) -> Self {
        Self {
            state,
            exec_buf: Rc::new(buf),
            start,
            len,
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
    blocks: hashbrown::HashMap<usize, CompiledBlock>,
    compiled_addrs: BTreeMap<usize, CacheRange>,
}

impl Cache {
    /// Get a compiled block from the cache or create if no entries were found
    pub fn get_or_compile(&mut self, addr: usize, state: &Rc<RefCell<State>>) -> CompiledBlock {
        let block = self
            .blocks
            .entry(addr)
            .or_insert_with({
                let state = state.clone();
                || {
                    tracing::debug!("Generating cache for address '0x{addr:08x}'");

                    let cycles = 1024;

                    let (buf, len) = self.jit.compile_block(state.clone(), cycles);
                    // self.insert_range(addr, addr + len);
                    self.compiled_addrs.insert(addr, CacheRange::Start);
                    self.compiled_addrs.insert(addr + len, CacheRange::End);

                    tracing::debug!("Generated code: {:02x?}", buf.as_slice());
                    CompiledBlock::new(state, buf, addr, len)
                }
            })
            .clone();

        if let Some(invalidated_range) = state.borrow_mut().cache_invalidation.take() {
            self.blocks.retain(|_, block| {
                let start = block.start;
                let end = start + block.len;
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
            blocks: HashMap::default(),
            compiled_addrs: BTreeMap::default(),
        }
    }
}
