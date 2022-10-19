use std::rc::Rc;

use hashbrown::HashMap;

use super::code::CompiledBlock;

pub struct JumpEntry {
    /// The address to jump
    pub jump_to: usize,
}

pub struct JumpTable {
    table: HashMap<usize, JumpEntry>,
}

impl JumpTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn get(&mut self, jump_addr: usize) -> &JumpEntry {
        self.table
            .entry(jump_addr)
            .or_insert(JumpEntry { jump_to: 0 })
    }

    pub(crate) fn resolve_to(&mut self, addr: usize, block: &Rc<CompiledBlock>) -> usize {
        if let Some(entry) = self.table.get_mut(&addr) {
            if entry.jump_to == 0 {
                entry.jump_to = block.ptr() as usize;
            }
            entry.jump_to
        } else {
            0
        }
    }
}
