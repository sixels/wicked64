use std::{cell::RefCell, rc::Rc};

use crate::n64::State;

use self::{
    cache::{Cache, CompiledBlock},
    compiler::Compiler,
};

mod bridge;
pub(crate) mod cache;
pub mod compiler;
pub(crate) mod register;
pub mod state;

/// JIT codegen engine
pub struct JitEngine {
    cache: Cache,
    state: Rc<RefCell<State>>,
}

impl JitEngine {
    pub fn new(state: Rc<RefCell<State>>) -> Self {
        Self {
            cache: Cache::default(),
            state,
        }
    }

    pub fn compile_current_pc(&mut self) -> Rc<CompiledBlock> {
        let pc = {
            let cpu = &self.state.borrow().cpu;
            cpu.translate_virtual(cpu.pc as usize)
        };

        let block = self.cache.get_or_insert_with(pc, || {
            let state = &self.state;
            let compiler = Compiler::new(state.clone());
            tracing::debug!("Compiling block at pc '0x{pc:08x}'");
            let cycles = 1024usize;

            let (buf, len) = compiler.compile(cycles);

            tracing::debug!("Generated code: {:02x?}", buf.as_slice());
            (CompiledBlock::new(buf), len)
        });

        self.invalidate_cache();

        block
    }

    pub fn invalidate_cache(&mut self) {
        if let Some((inv_start, inv_end)) = self.state.borrow_mut().cache_invalidation.take() {
            self.cache.invalidate_range(inv_start, inv_end);
        }
    }
}
