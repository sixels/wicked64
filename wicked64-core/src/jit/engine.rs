use std::cell::RefCell;
use std::ops::Deref;
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
        Self { pc, state, len: 0 }
    }

    /// Compile the code
    // TODO: Take the number of cycles to compile as argument
    pub fn compile(mut self) -> (CompiledBlock, usize) {
        tracing::debug!("Generating compiled block");

        // initialize the code generation
        let mut code = RawBlock::new().unwrap();

        // generate the code
        // TODO: Change size to the actual number of cycles it should compile
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

    // TODO: Compile `n` number of cycles instead of `n` number of instructions
    fn compile_block(&mut self, code: &mut RawBlock, size: usize) -> usize {
        let pc = self.pc;

        for _ in 0..size {
            let instruction = {
                let state = self.state.borrow();
                let State {
                    ref cpu, ref mmu, ..
                } = state.deref();

                let instruction = cpu.fetch_instruction(mmu, self.pc).unwrap();
                println!("Compiling instruction: 0x{:08x}:{instruction:?}", self.pc);

                instruction
            };

            self.pc += 1;
            if self.compile_instruction(code, instruction) {
                break;
            }
        }
        (self.pc - pc) as usize
    }

    /// Compiles the given instruction and save the generated code into `buf`
    fn compile_instruction(&self, code: &mut RawBlock, instruction: Instruction) -> bool {
        // let state = self.state.borrow();
        // let cpu = &state.cpu;

        todo!()
    }
}
