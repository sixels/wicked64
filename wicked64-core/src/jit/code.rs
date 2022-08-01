use std::cell::RefCell;
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
            let exec: unsafe extern "C" fn() -> usize = std::mem::transmute(self.mmap.as_ptr());
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
                code.emit_push_reg(reg)?;
            }

            Ok(code)
        }

        prelude().map(|code| Self(code))
    }

    /// Generate the compiled block ready to be executed
    pub fn compile(mut self, state: Rc<RefCell<State>>) -> std::io::Result<CompiledBlock> {
        fn map_memory(code: Vec<u8>) -> std::io::Result<Mmap> {
            let mut mmap = MmapMut::map_anon(code.len())?;
            mmap.as_mut().copy_from_slice(code.as_slice());

            mmap.make_exec()
        }
        fn postlude() -> std::io::Result<Vec<u8>> {
            let mut code = Vec::new();

            // retrieve callee-saved registers
            for reg in CALLEE_SAVED_REGISTERS.iter().rev() {
                code.emit_pop_reg(*reg)?;
            }
            // pop old rax into rcx
            code.emit_pop_reg(X64Gpr::Rcx)?;
            // return
            code.emit_ret()?;

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

#[cfg(test)]
mod tests {
    use std::io;

    use crate::{cpu::Cpu, io::Cartridge, mmu::MemoryManager};

    use super::*;

    #[test]
    fn it_should_move_a_value_to_a_register() -> io::Result<()> {
        let cart = Cartridge::open("../assets/test-roms/dillonb/basic.z64").unwrap();
        let mut mmu = MemoryManager::new(cart);
        let cpu = Cpu::new(false, &mut mmu);
        let state = Rc::new(RefCell::new(State::new(mmu, cpu)));

        let mut code = RawBlock::new()?;
        let mut bulk = Vec::with_capacity(1000);

        bulk.emit_mov_reg_immediate(X64Gpr::Rax, 1)?;
        bulk.emit_mov_reg_immediate(X64Gpr::Rbx, 2)?;
        bulk.emit_mov_reg_immediate(X64Gpr::Rcx, 3)?;
        bulk.emit_mov_reg_immediate(X64Gpr::Rdx, 4)?;
        bulk.emit_mov_reg_immediate(X64Gpr::Rdi, 5)?;
        bulk.emit_mov_reg_immediate(X64Gpr::Rsi, 6)?;
        bulk.emit_mov_reg_immediate(X64Gpr::R8, 7)?;
        bulk.emit_mov_reg_immediate(X64Gpr::R9, 8)?;
        bulk.emit_mov_reg_immediate(X64Gpr::R10, 9)?;
        bulk.emit_mov_reg_immediate(X64Gpr::R11, 10)?;
        bulk.emit_mov_reg_immediate(X64Gpr::R12, 11)?;
        bulk.emit_mov_reg_immediate(X64Gpr::R13, 12)?;
        bulk.emit_mov_reg_immediate(X64Gpr::R14, 13)?;
        bulk.emit_mov_reg_immediate(X64Gpr::R15, 14)?;

        bulk.emit_assert_reg_eq(X64Gpr::Rax, 1)?;
        bulk.emit_assert_reg_eq(X64Gpr::Rbx, 2)?;
        bulk.emit_assert_reg_eq(X64Gpr::Rcx, 3)?;
        bulk.emit_assert_reg_eq(X64Gpr::Rdx, 4)?;
        bulk.emit_assert_reg_eq(X64Gpr::Rdi, 5)?;
        bulk.emit_assert_reg_eq(X64Gpr::Rsi, 6)?;
        bulk.emit_assert_reg_eq(X64Gpr::R8, 7)?;
        bulk.emit_assert_reg_eq(X64Gpr::R9, 8)?;
        bulk.emit_assert_reg_eq(X64Gpr::R10, 9)?;
        bulk.emit_assert_reg_eq(X64Gpr::R11, 10)?;
        bulk.emit_assert_reg_eq(X64Gpr::R12, 11)?;
        bulk.emit_assert_reg_eq(X64Gpr::R13, 12)?;
        bulk.emit_assert_reg_eq(X64Gpr::R14, 13)?;
        bulk.emit_assert_reg_eq(X64Gpr::R15, 14)?;

        code.extend_from_slice(&bulk);

        let comp = code.compile(state).unwrap();
        comp.execute();

        Ok(())
    }
}
