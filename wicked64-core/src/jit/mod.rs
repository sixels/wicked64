use std::{cell::RefCell, rc::Rc};

use crate::n64::State;

use self::compiler::{Compiler, ExecBuffer};

mod bridge;
pub(crate) mod cache;
pub mod compiler;
pub(crate) mod register;
pub mod state;

/// JIT codegen engine
#[derive(Debug, Default)]
pub struct JitEngine {}

impl JitEngine {
    pub fn new() -> JitEngine {
        Self::default()
    }

    pub fn compile_block(&self, state: Rc<RefCell<State>>, cycles: usize) -> (ExecBuffer, usize) {
        let pclock_size = 5;
        let pclocks = cycles * pclock_size;

        Compiler::new(state).compile(pclocks)
    }
}
