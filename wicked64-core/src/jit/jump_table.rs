use std::rc::Rc;

use hashbrown::HashMap;

use super::code::CompiledBlock;

pub struct JumpEntry {
    /// The address to jump
    pub target_block: usize,
}

pub struct JumpTable {
    /// Maps a n64 physical address to a `JumpEntry`
    table: HashMap<u64, Option<JumpEntry>>,
}

impl JumpTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn get(&mut self, phys_jump_addr: u64) -> Option<&JumpEntry> {
        self.table.entry(phys_jump_addr).or_default().as_ref()
    }

    pub(crate) fn resolve_with_block(
        &mut self,
        phys_addr: u64,
        block: &Rc<CompiledBlock>,
    ) -> Option<&JumpEntry> {
        self.table.get_mut(&phys_addr).map(|entry| {
            entry.get_or_insert(JumpEntry {
                target_block: block.ptr() as usize,
            }) as &_
        })
    }
}
