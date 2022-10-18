use std::rc::Rc;

use crate::utils::btree_range::BTreeRange;

use super::compiler::ExecBuffer;

#[derive(Clone)]
pub struct CompiledBlock {
    exec_buf: ExecBuffer,
}

impl CompiledBlock {
    pub fn new(buf: ExecBuffer) -> Self {
        Self { exec_buf: buf }
    }

    pub fn execute(&self) {
        unsafe { self.exec_buf.execute() };
    }
}

pub struct Cache {
    blocks: BTreeRange<Rc<CompiledBlock>>,
}

impl Cache {
    pub fn new() -> Self {
        todo!()
    }

    /// Get a compiled block from the cache or create if no entries were found
    pub fn get_or_insert_with<F>(&mut self, addr: usize, mut f: F) -> Rc<CompiledBlock>
    where
        F: FnMut() -> (CompiledBlock, usize),
    {
        if let Some(block) = self.blocks.get_exact(addr) {
            return block.clone();
        }

        // let (block, len) = {
        //     tracing::debug!("Generating cache for address '0x{addr:08x}'");

        //     let cycles = 1024usize;

        //     let (buf, len) = compiler.compile(cycles);

        //     tracing::debug!("Generated code: {:02x?}", buf.as_slice());
        //     (Rc::new(CompiledBlock::new(buf)), len)
        // };
        let (block, len) = f();
        let block = Rc::new(block);
        self.blocks.insert(addr..=addr + len, block.clone());
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
