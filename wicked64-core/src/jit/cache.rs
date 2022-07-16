use std::{
    ops::Range,
    sync::{Arc, Mutex, RwLock},
};

use crate::n64::SyncState;

use super::{code::CompiledBlock, engine::JitEngine};

pub struct Cache {
    jit_engine: JitEngine,
    blocks: Mutex<hashbrown::HashMap<u64, Arc<CompiledBlock>>>,
    compiled_ranges: RwLock<hashbrown::HashSet<Range<u64>>>,
}

impl Cache {
    /// Get a compiled block from the cache or create if no entries were found
    pub fn get_or_compile(&self, addr: u64, state: SyncState) -> Arc<CompiledBlock> {
        let mut blocks = self.blocks.lock().unwrap();

        let block = blocks.entry(addr).or_insert_with(|| {
            tracing::debug!("Generating cache for address '0x{addr:08x}'");

            let block = self.jit_engine.compile_block(state);
            let mut ranges = self.compiled_ranges.write().unwrap();
            ranges.insert(addr..addr + (block.len as u64));

            Arc::new(block)
        });
        block.clone()
    }

    /// Check if an arbitrary address is cached
    pub fn is_addr_compiled(&self, addr: u64) -> Option<u64> {
        let ranges = self.compiled_ranges.read().unwrap();
        ranges
            .iter()
            .find_map(|range| range.contains(&addr).then(|| range.start))
    }
}

impl Default for Cache {
    /// Creates a new cache manager for jitted instructions
    fn default() -> Cache {
        Self {
            jit_engine: JitEngine::new(),
            blocks: Default::default(),
            compiled_ranges: Default::default(),
        }
    }
}
