use crate::cpu::instruction::Instruction;
use crate::jit::code::RawBlock;
use crate::jit::codegen::Emitter;
use crate::n64::SyncState;

use super::code::CompiledBlock;
use super::codegen::register::X64Gpr;

/// Wasm codegen engine
#[derive(Debug)]
pub struct JitEngine {}

impl JitEngine {
    pub fn new() -> JitEngine {
        Self {}
    }

    pub fn compile_block(&self, state: SyncState) -> CompiledBlock {
        Jit::new(state).compile()
    }
}

/// The JIT compiler
struct Jit {
    state: SyncState,
    pc: u64,
    len: usize,
}

impl Jit {
    /// Create a new Jit compiler
    pub fn new(state: SyncState) -> Self {
        let pc = state.lock().cpu.pc;
        tracing::info!("Creating a new JIT");

        Self { pc, state, len: 0 }
    }

    /// Compile the code
    pub fn compile(mut self) -> CompiledBlock {
        tracing::debug!("Generating compiled block");

        // initialize the code generation
        let mut code = RawBlock::new();
        tracing::debug!("Prelude generated with length: {}", code.len());

        // generate the code
        self.perform_codegen(&mut code);

        // compile the code
        let compiled = match code.compile(self.state.clone()) {
            Ok(compiled) => compiled,
            Err(error) => panic!("Could not compile the code: {error:?}"),
        };

        tracing::debug!(
            "Block compiled from pc: ({:08x}..{:08x})",
            self.state.lock().cpu.pc,
            self.pc
        );
        // tracing::debug!("Generated code: {compiled:?}");

        compiled
    }

    fn perform_codegen(&mut self, code: &mut RawBlock) {
        todo!()
    }

    /// Compiles the given instruction and save the generated code into `buf`
    fn compile_instruction(&self, buf: &mut Vec<u8>, instruction: Instruction) {
        let state = self.state.lock();
        let cpu = &state.cpu;

        todo!()
    }
}
