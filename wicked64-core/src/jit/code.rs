use std::cell::RefCell;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use memmap2::{Mmap, MmapMut};

use crate::jit::codegen::register::CALLEE_SAVED_REGISTERS;
use crate::jit::codegen::{register::X64Gpr, Emitter};
use crate::n64::State;

/// A compiled x64 code block
pub struct CompiledBlock {
    state: Rc<RefCell<State>>,
    mmap: Mmap,
}

impl CompiledBlock {
    /// Execute the generated code
    pub fn execute(&self) -> usize {
        let _state = self.state.borrow_mut();

        unsafe {
            let exec = std::mem::transmute::<_, extern "C" fn() -> usize>(self.mmap.as_ptr());
            exec()
        }
    }
}

/// Raw block represents a block of code with instructions in hexadecimal format
pub struct RawBlock(Vec<u8>);

impl RawBlock {
    pub fn new() -> std::io::Result<Self> {
        /// push callee-saved registers into the stack so we can recover them
        /// later
        fn prelude() -> std::io::Result<Vec<u8>> {
            let mut code = Vec::new();

            // push rax to align the stack
            code.emit_push_reg(X64Gpr::Rax)?;
            // push callee-saved registers
            for reg in CALLEE_SAVED_REGISTERS {
                code.emit_push_reg(*reg)?;
            }

            Ok(code)
        }

        prelude().map(|code| Self(code))
    }

    /// Generate the compiled block ready to be executed
    pub fn compile(mut self, state: Rc<RefCell<State>>) -> std::io::Result<CompiledBlock> {
        fn map_memory(code: Vec<u8>) -> std::io::Result<Mmap> {
            let code_len = code.len();

            let mut mmap = MmapMut::map_anon(code_len)?;
            mmap.as_mut().write(&code)?;

            mmap.make_exec()
        }
        fn postlude() -> std::io::Result<Vec<u8>> {
            let mut code = Vec::new();

            // retrieve callee-saved registers
            for reg in CALLEE_SAVED_REGISTERS.iter().rev() {
                code.emit_pop_reg(*reg)?;
            }
            // pop rax into rcx
            code.emit_pop_reg(X64Gpr::Rcx)?;

            Ok(code)
        }

        postlude().map(|code| {
            self.extend_from_slice(&code);

            let mmap = map_memory(self.0).unwrap();
            CompiledBlock { state, mmap }
        })
    }
}

impl Deref for RawBlock {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RawBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
