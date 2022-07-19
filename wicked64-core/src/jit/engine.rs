use std::cell::RefCell;
use std::ops::DerefMut;
use std::rc::Rc;

use crate::cpu::instruction::Instruction;
use crate::jit::code::RawBlock;
use crate::n64::State;

use super::code::CompiledBlock;

/// Wasm codegen engine
#[derive(Debug)]
pub struct JitEngine {}

impl JitEngine {
    pub fn new() -> JitEngine {
        Self {}
    }

    pub fn compile_block(&self, state: Rc<RefCell<State>>) -> (CompiledBlock, usize) {
        Jit::new(state).compile()
    }
}

/// The JIT compiler
struct Jit {
    state: Rc<RefCell<State>>,
    pc: u64,
    len: usize,
}

impl Jit {
    /// Create a new Jit compiler
    pub fn new(state: Rc<RefCell<State>>) -> Self {
        let pc = state.borrow().cpu.pc;
        tracing::info!("Creating a new JIT");

        Self { pc, state, len: 0 }
    }

    /// Compile the code
    pub fn compile(mut self) -> (CompiledBlock, usize) {
        tracing::debug!("Generating compiled block");

        // initialize the code generation
        let mut code = RawBlock::new().unwrap();

        // generate the code
        let compiled_size = self.compile_block(&mut code, 2);

        // compile the code
        let compiled = match code.compile(self.state.clone()) {
            Ok(compiled) => compiled,
            Err(error) => panic!("Could not compile the code: {error:?}"),
        };

        tracing::debug!(
            "Block compiled from pc: ({:08x}..{:08x})",
            self.state.borrow().cpu.pc,
            self.pc
        );
        // tracing::debug!("Generated code: {compiled:?}");

        (compiled, (self.pc - self.state.borrow().cpu.pc) as usize)
    }

    fn compile_block(&mut self, code: &mut RawBlock, size: usize) -> usize {
        let pc = self.pc;

        let mut state = self.state.borrow_mut();
        let State {
            ref mut cpu,
            ref mut mmu,
            ..
        } = state.deref_mut();

        // let cpu = &mut state.cpu;
        // let mmu = &mut state.mmu;

        loop {
            let phys_pc = cpu.translate_virtual(self.pc as usize) as u64;

            let instruction = cpu.fetch_instruction(mmu, phys_pc).unwrap();

            tracing::debug!("Compiling instruction: {instruction:?} @ 0x{:08x}", self.pc);

            self.pc += 1;
            if self.compile_instruction(code, instruction) {
                break (self.pc - pc) as usize;
            }
        }
    }

    /// Compiles the given instruction and save the generated code into `buf`
    fn compile_instruction(&self, code: &mut RawBlock, instruction: Instruction) -> bool {
        let state = self.state.borrow();
        let cpu = &state.cpu;

        todo!()
    }
}
