use std::{cell::RefCell, rc::Rc};

use crate::n64::State;

use self::{
    cache::Cache,
    code::CompiledBlock,
    compiler::Compiler,
    jump_table::{JumpEntry, JumpTable},
};

mod bridge;
mod cache;
mod code;
mod compiler;
mod interruption;
mod jump_table;

pub use interruption::Interruption;

/// JIT codegen engine
pub struct JitEngine {
    cache: Cache,
    state: Rc<RefCell<State>>,
    jump_table: JumpTable,
}

impl JitEngine {
    pub fn new(state: Rc<RefCell<State>>) -> Self {
        Self {
            cache: Cache::default(),
            state,
            jump_table: JumpTable::new(),
        }
    }

    pub fn compile(&mut self, virtual_pc: u64) -> Rc<CompiledBlock> {
        let physical_pc = self.state.borrow().translate_cpu_pc();

        let block = self.cache.get_or_insert_with(physical_pc as usize, || {
            tracing::debug!("Compiling a block at addr '{virtual_pc:08x}'");

            let state = &self.state;
            let compiler = Compiler::new(state.clone(), &mut self.jump_table, virtual_pc as usize);

            let cycles = 1024usize;

            let (buf, len) = compiler.compile(cycles);

            CompiledBlock::new(buf, virtual_pc, len)
        });

        tracing::debug!(
            "Getting block at addr '0x{virtual_pc:08x}' with id: {:p}",
            block.ptr()
        );

        block
    }

    pub fn compile_current_pc(&mut self) -> Rc<CompiledBlock> {
        let pc = self.state.borrow().cpu.pc;
        self.compile(pc)
    }

    pub fn invalidate_cache(&mut self) {
        // ! TODO: delete entries from jump table too
        if let Some(inv_range) = self.state.borrow_mut().cache_invalidation.take() {
            self.cache.invalidate_range(inv_range);
        }
    }

    pub(crate) fn resolve_jump(&mut self, addr: u64) -> Option<&JumpEntry> {
        let block = self.compile(addr);
        self.jump_table
            .resolve_with_block(self.state.borrow().cpu.translate_virtual(addr), &block)
    }

    pub fn resume_from(&self, resume_block: usize) {
        let resume_addr = self.state.borrow().resume_addr as usize;
        tracing::debug!(
            "Resuming execution at 0x{resume_addr:08x} and jumping to 0x{:08x}",
            resume_block
        );
        unsafe {
            code::resume(&self.state, resume_addr, resume_block);
        }
    }
}
