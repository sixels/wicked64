pub(crate) mod cache;
pub mod codegen;

use std::ops::DerefMut;

use byteorder::{BigEndian, WriteBytesExt};
use wasmer::{Function, Instance, Pages};

use crate::{cpu::instruction::Instruction, n64::SyncState};

use self::codegen::emitter::Emitter;

pub const WASM_VERSION: u32 = 0x01_00_00_00;
pub const WASM_STACK_OFFSET: u8 = 1;

/// Wasm codegen engine
#[derive(Debug)]
pub struct JitEngine {}

impl JitEngine {
    pub fn new() -> anyhow::Result<JitEngine> {
        Ok(Self {})
    }

    pub fn compile_block(&self, state: SyncState) -> CompiledBlock {
        Jit::new(state).compile()
    }
}

/// A compiled Wasm block
#[derive(Clone)]
pub struct CompiledBlock {
    len: usize,
}

impl CompiledBlock {
    fn new(state: SyncState, bytes: &[u8], len: usize) -> anyhow::Result<CompiledBlock> {
        todo!();
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Execute the generated code
    pub fn exec(&self) {
        todo!()
    }
}

/// The JIT compiler
struct Jit {
    state: SyncState,
    compiled_block: Vec<u8>,
    pc: u64,
    len: usize,
}

impl Jit {
    /// Create a new Jit compiler
    pub fn new(state: SyncState) -> Self {
        let pc = state.lock().cpu.pc;
        tracing::info!("Creating a new JIT");

        Self {
            pc,
            state,
            compiled_block: Vec::with_capacity(57),
            len: 0,
        }
    }

    /// Compile a code
    pub fn compile(mut self) -> CompiledBlock {
        self.start_code_generation();

        tracing::info!("Generating compiled block");
        self.create_compiled_block()
    }

    pub(crate) fn create_compiled_block(self) -> CompiledBlock {
        CompiledBlock::new(self.state, &self.compiled_block, self.len).unwrap()
    }

    fn start_code_generation(&mut self) {
        todo!()

        // for _ in 0..3 {
        //     let instruction = {
        //         let state = self.state.lock();
        //         let cpu = &state.cpu;
        //         let mmu = &state.mmu;
        //         cpu.fetch_instruction(mmu, self.pc).unwrap()
        //     };
        //     tracing::debug!("Compiling instruction {instruction:?} from {:08x}", self.pc);

        //     self.compile_instruction(&mut code_buffer, instruction);

        //     self.pc += 4;
        //     self.len += 1;
        // }
    }

    /// Compiles the given instruction and save the generated code into `buf`
    fn compile_instruction(&self, buf: &mut Vec<u8>, instruction: Instruction) {
        let state = self.state.lock();
        let cpu = &state.cpu;

        todo!()
    }

    fn emit_prelude(&mut self) {
        todo!()
    }
}
