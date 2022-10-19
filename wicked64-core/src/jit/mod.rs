use std::{cell::RefCell, rc::Rc};

use crate::n64::State;

use self::{cache::Cache, code::CompiledBlock, compiler::Compiler, jump_table::JumpTable};

mod bridge;
mod cache;
mod code;
mod compiler;
mod interruption;
mod jump_table;
mod register;
mod state;

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

    pub fn compile(&mut self, virtual_pc: usize) -> Rc<CompiledBlock> {
        let physical_pc = self.state.borrow().translate_cpu_pc();

        self.cache.get_or_insert_with(physical_pc, || {
            let state = &self.state;
            let compiler = Compiler::new(state.clone(), &mut self.jump_table, virtual_pc);
            tracing::debug!("Compiling block at pc '0x{virtual_pc:08x}'");

            let cycles = 1024usize;

            let (buf, len) = compiler.compile(cycles);

            tracing::debug!("Generated code: {:02x?}", buf.as_slice());
            (CompiledBlock::new(buf), len)
        })
    }

    pub fn compile_current_pc(&mut self) -> Rc<CompiledBlock> {
        let pc = self.state.borrow().cpu.pc as usize;
        self.compile(pc)
    }

    pub fn invalidate_cache(&mut self) {
        if let Some((inv_start, inv_end)) = self.state.borrow_mut().cache_invalidation.take() {
            self.cache.invalidate_range(inv_start, inv_end);
        }
    }

    pub(crate) fn resolve_jump(&mut self, addr: usize) -> usize {
        let block = self.compile(addr);
        self.jump_table.resolve_to(addr, &block)
    }

    /// Resume the previous code execution passing `jump_to` information
    ///
    /// # Safety
    /// This function uses inline assembly to setup the stack frame before
    /// jumping into the memory containing the generated code.
    /// It is expected that the code jumps back to the address saved in `r13`
    /// register.
    pub fn resume_with(&self, jump_to: usize) {
        let resume_addr = self.state.borrow().resume_addr as usize;
        unsafe {
            code::resume(&self.state, resume_addr, jump_to);
        }
    }
}
