use std::rc::Rc;

use crate::utils::btree_range::BTreeRange;

use super::code::CompiledBlock;

pub struct Cache {
    blocks: BTreeRange<Rc<CompiledBlock>>,
}

impl Cache {
    /// Get a compiled block from the cache or create if no entries were found
    pub fn get_or_insert_with<F>(&mut self, addr: usize, mut f: F) -> Rc<CompiledBlock>
    where
        F: FnMut() -> CompiledBlock,
    {
        if let Some(block) = self.blocks.get_exact(addr) {
            return block.clone();
        }

        let block = Rc::new(f());
        self.blocks.insert(addr..=addr + block.len(), block.clone());
        block
    }

    pub fn invalidate_range(&mut self, inv_start: usize, inv_end: usize) {
        self.blocks
            .retain(|(start, end), _| !(inv_start <= end && inv_end >= start));
    }
}

impl Default for Cache {
    /// Creates a new cache manager for jitted instructions
    fn default() -> Cache {
        Self {
            blocks: BTreeRange::new(),
        }
    }
}
