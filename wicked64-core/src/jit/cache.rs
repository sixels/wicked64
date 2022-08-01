use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use crate::jit::{code::CompiledBlock, engine::JitEngine};
use crate::n64::State;

#[derive(PartialEq, Eq, Clone, Copy)]
enum CacheRange {
    Start,
    End,
}

pub struct Cache {
    jit_engine: JitEngine,
    blocks: hashbrown::HashMap<usize, CompiledBlock>,
    compiled_addrs: BTreeMap<usize, CacheRange>,
}

impl Cache {
    /// Get a compiled block from the cache or create if no entries were found
    pub fn get_or_compile(&mut self, addr: usize, state: Rc<RefCell<State>>) -> &CompiledBlock {
        let block = self.blocks.entry(addr).or_insert_with(|| {
            tracing::debug!("Generating cache for address '0x{addr:08x}'");

            let (block, len) = self.jit_engine.compile_block(state);

            self.compiled_addrs.insert(addr, CacheRange::Start);
            self.compiled_addrs.insert(addr + len, CacheRange::End);

            block
        });
        block
    }

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
            jit_engine: JitEngine::new(),
            blocks: Default::default(),
            compiled_addrs: Default::default(),
        }
    }
}
