use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use w64_codegen::ExecBuffer;

use crate::jit::engine::JitEngine;
use crate::n64::State;

#[derive(PartialEq, Eq, Clone, Copy)]
enum CacheRange {
    Start,
    End,
}

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
        let _ = self.state.borrow_mut();
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
    pub fn get_or_compile(&mut self, addr: usize, state: Rc<RefCell<State>>) -> &CompiledBlock {
        let block = self.blocks.entry(addr).or_insert_with(|| {
            tracing::debug!("Generating cache for address '0x{addr:08x}'");

            let (buf, len) = self.jit.compile_block(state.clone());
            // self.insert_range(addr, addr + len);
            self.compiled_addrs.insert(addr, CacheRange::Start);
            self.compiled_addrs.insert(addr + len, CacheRange::End);

            CompiledBlock::new(state, buf)
        });
        block
    }

    // fn insert_range(&mut self, start: usize, end: usize) {
    //     self.compiled_addrs.insert(start, CacheRange::Start);
    //     self.compiled_addrs.insert(end, CacheRange::End);
    // }

    // /// Check if an arbitrary address is cached
    // pub fn is_addr_compiled(&self, addr: usize) -> bool {
    //     match self.compiled_addrs.range(..=addr).last() {
    //         Some((_, CacheRange::Start)) => true,
    //         _ => false,
    //     }
    // }
}

impl Default for Cache {
    /// Creates a new cache manager for jitted instructions
    fn default() -> Cache {
        Self {
            jit: JitEngine::new(),
            blocks: Default::default(),
            compiled_addrs: Default::default(),
        }
    }
}
