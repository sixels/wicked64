pub(crate) mod cache;
pub mod codegen;
pub mod imports;
pub mod wasm;

use std::ops::DerefMut;

use byteorder::{BigEndian, WriteBytesExt};
use wasmer::{Function, Instance, Pages};
use wicked64_arena as arena;

use crate::{
    cpu::instruction::Instruction,
    jit::{imports::EnvState, wasm::WasmSection},
    n64::SyncState,
};

use self::{
    codegen::emitter::Emitter,
    wasm::{CodeSection, ImportSection},
};

pub const WASM_VERSION: u32 = 0x01_00_00_00;
pub const WASM_STACK_OFFSET: u8 = 1;

/// Wasm codegen engine
#[derive(Debug)]
pub struct JitEngine {}

impl JitEngine {
    pub fn new() -> anyhow::Result<JitEngine> {
        // stack pointer (starts at index 1)
        let _ = arena::alloc!(1u32);
        // alloc the stack
        let _ = arena::alloc!([0u8; 1000]);

        Ok(Self {})
    }

    pub fn compile_block(&self, state: SyncState) -> CompiledBlock {
        Jit::new(state).compile()
    }
}

/// A compiled Wasm block
#[derive(Clone)]
pub struct CompiledBlock {
    instance: wasmer::Instance,
    len: usize,
}

impl CompiledBlock {
    fn new(state: SyncState, bytes: &[u8], len: usize) -> anyhow::Result<CompiledBlock> {
        let arena = arena::global_arena();
        let store = arena.store();

        // create the module
        let module = match wasmer::Module::from_binary(store, bytes) {
            Ok(module) => {
                tracing::debug!("Generated code: {bytes:?}");
                module
            }
            Err(e) => {
                tracing::error!("Generated code: {bytes:?}");
                panic!("{e:?}");
            }
        };
        // use the line below instead, when all instructions get implemented and tested.
        // let module = unsafe { wasmer::Module::from_binary_unchecked(store, bytes).unwrap_unchecked() };

        let state_env = EnvState::new(state);
        let imports = wasmer::imports! {
            "env" => {
                "translate_virtual_addr" => Function::new_native_with_env(
                    store,
                    state_env.clone(),
                    self::imports::translate_virtual_addr,
                ),
                "read_word" => Function::new_native_with_env(
                    store,
                    state_env.clone(),
                    self::imports::read_word
                ),
                "store_word" => Function::new_native_with_env(
                    store,
                    state_env.clone(),
                    self::imports::store_word
                ),
                "memory" => arena.memory_cloned(),
            }
        };

        let instance = Instance::new(&module, &imports).unwrap();

        Ok(Self { instance, len })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Execute the generated code
    pub fn exec(&self) {
        // `exec` function must exist
        let exec = self.instance.exports.get_function("exec").unwrap();
        tracing::debug!("Executing code");

        tracing::info!("Executing WASM");
        exec.call(&[]).unwrap();
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
        let mut code_section = self.emit_prelude();

        let mut code_buffer = Vec::with_capacity(0x10);
        code_buffer.push(0x00);

        for _ in 0..3 {
            let instruction = {
                let state = self.state.lock();
                let cpu = &state.cpu;
                let mmu = &state.mmu;
                cpu.fetch_instruction(mmu, self.pc).unwrap()
            };
            tracing::debug!("Compiling instruction {instruction:?} from {:08x}", self.pc);

            self.compile_instruction(&mut code_buffer, instruction);

            self.pc += 4;
            self.len += 1;
        }

        code_buffer.push(0x0b);
        debug_assert!(code_buffer.len() <= 0xFFFFFFFF);
        wasm::leb128::signed::write_i32(code_section.deref_mut(), code_buffer.len() as i32)
            .unwrap();
        code_section.extend(&code_buffer);
        self.compiled_block.extend(code_section.build());
    }

    /// Compiles the given instruction and save the generated code into `buf`
    fn compile_instruction(&self, buf: &mut Vec<u8>, instruction: Instruction) {
        let state = self.state.lock();
        let cpu = &state.cpu;

        match instruction {
            // Load Upper Immediate
            Instruction::LUI(itype) => {
                let src = if ((itype.immediate as u64) & 0x8000) != 0 {
                    0xFFFFFFFF_0000_0000
                } else {
                    0
                } | (itype.immediate as u64) << 16;
                buf.emit_i64_store_const(&cpu.gpr[itype.rt as usize], src);
            }
            // OR Immediate
            Instruction::ORI(itype) => {
                buf.emit_push_ptr_offset(&cpu.gpr[itype.rt as usize]);
                buf.emit_i64_load(&cpu.gpr[itype.rs as usize]);
                buf.emit_i64_const(itype.immediate as u64);
                buf.extend(&[0x84, 0x37, 0x03, 0x00]); // i64.or then i64.store
            }
            // Store Word
            Instruction::SW(itype) => {
                let vaddr_upper = if ((itype.immediate as u32) & 0x8000) != 0 {
                    0xFF
                } else {
                    0
                } | (itype.immediate as u32);
                buf.emit_i32_const(vaddr_upper);
                buf.emit_i32_from_i64(&cpu.gpr[itype.rs as usize]);
                buf.push(0x6a); // i32.add ;; will push vAddr
                buf.extend(&[0x10, 0x00]); // call $translate_virtual_addr ;; will push pAddr
                buf.emit_i32_from_i64(&cpu.gpr[itype.rt as usize]); // will push DATA

                buf.extend(&[0x10, 0x02]); // $call store_word
            }
            Instruction::Cop0MTC0(rtype) => {
                buf.reserve(0x1c);
                // cp0.gpr[rd] <- gpr[rt]

                buf.emit_i64_store(
                    &cpu.cp0.gpr[rtype.rd as usize],
                    &cpu.cp0.gpr[rtype.rt as usize],
                );
            }
            _ => todo!("Instruction not implemented: {instruction:?}"),
        }
    }

    fn emit_prelude(&mut self) -> CodeSection {
        // write the wasm header
        self.compiled_block.extend(b"\0asm");
        self.compiled_block
            .write_u32::<BigEndian>(WASM_VERSION)
            .unwrap();

        // type section
        // define a function type with no return
        self.compiled_block.extend([
            0x01, 0x0e, 0x03, // type section; _ bytes; _ types
            0x60, 0x00, 0x00, // [] -> []
            0x60, 0x01, 0x7f, 0x01, 0x7f, // [i32] -> [i32]
            0x60, 0x02, 0x7f, 0x7f, 0x00, // [i32, i32] -> []
        ]);

        let imports = {
            let Pages(mem_pages) = arena::global_arena().pages();
            let mut imports = ImportSection::new(4);

            // import memory
            imports.extend(b"\x03env\x06memory\x02\x00");
            wasm::leb128::signed::write_i32(imports.deref_mut(), mem_pages as i32).unwrap();
            // import translate_virtual_addr
            imports.extend(b"\x03env\x16translate_virtual_addr\x00\x01");
            imports.extend(b"\x03env\x09read_word\x00\x01");
            imports.extend(b"\x03env\x0astore_word\x00\x02");

            imports.build()
        };
        self.compiled_block.extend(imports);

        // function section + export section
        self.compiled_block.extend([
            0x03, 0x02, 0x01, // function section; 0x02 bytes; 0x01 functions
            0x00, // $exec
            0x07, 0x08, 0x01, // export section; 0x08 bytes; 0x01 exports
            0x04, b'e', b'x', b'e', b'c', 0x00, 0x03, // 0x04 bytes; name; function; id _
        ]);

        CodeSection::new(1)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::{Deref, DerefMut};

    use wicked64_arena::{init_arena, ArenaBox};

    use crate::{
        jit::{codegen::emitter::Emitter, wasm::WasmSection},
        n64::N64,
    };

    use super::{wasm, Jit};

    #[test]
    fn transmute_integers() {
        crate::tests::init_trace();
        init_arena(18 * 1024 * 1024);

        let n64 = N64::new("../assets/test-roms/dillonb/basic.z64").unwrap();
        let mut jit = Jit::new(n64.state().clone());

        let mut cs = jit.emit_prelude();

        let src64 = ArenaBox::new(0xFFFFFFFF_77A2FF69u64);
        let dst16 = ArenaBox::new(0u16);
        let dst8 = ArenaBox::new(0u8);
        let dst64 = ArenaBox::new(0u64);
        let dst32 = ArenaBox::new(0u32);

        let mut buf = Vec::<u8>::new();

        {
            let wb = &mut buf;
            wb.push(0x00);

            // i64 -> i64
            wb.emit_i64_store(dst64.deref(), src64.deref());
            // i64 -> i32
            wb.emit_i64_store32(dst32.deref(), src64.deref(), false);
            // i64 -> i16
            wb.emit_i64_store16(dst16.deref(), src64.deref(), false);
            // i64 -> i8
            wb.emit_i64_store8(dst8.deref(), src64.deref(), false);

            wb.push(0x0b);
        }

        // finalize
        wasm::leb128::signed::write_i32(cs.deref_mut(), buf.len() as i32).unwrap();
        cs.extend(&buf);
        jit.compiled_block.extend(cs.build());

        let cb = jit.create_compiled_block();
        cb.exec();

        assert_eq!(*dst8, *src64 as u8, "u8 failed");
        assert_eq!(*dst16, *src64 as u16, "u16 failed");
        assert_eq!(*dst32, *src64 as u32, "u32 failed");
        assert_eq!(*dst64, *src64, "u64 failed");
    }
}
