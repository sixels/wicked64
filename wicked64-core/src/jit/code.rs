use std::{arch::asm, cell::RefCell, rc::Rc};

use crate::n64::State;

#[derive(Clone)]
pub struct CompiledBlock {
    exec_buf: ExecBuffer,
    start_pc: u64,
    len: usize,
}

impl CompiledBlock {
    pub fn new(buf: ExecBuffer, start_pc: u64, len: usize) -> Self {
        Self {
            exec_buf: buf,
            start_pc,
            len,
        }
    }

    pub fn execute(&self) {
        unsafe { self.exec_buf.execute() };
    }

    pub fn ptr(&self) -> *const u8 {
        self.exec_buf.ptr()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn start_pc(&self) -> u64 {
        self.start_pc
    }
}

#[derive(Clone)]
pub struct ExecBuffer {
    ptr: *const u8,
    buf: Vec<u8>,
    state: Rc<RefCell<State>>,
}

impl ExecBuffer {
    pub unsafe fn new(buffer: Vec<u8>, state: Rc<RefCell<State>>) -> region::Result<Self> {
        let ptr = buffer.as_ptr();

        region::protect(ptr, buffer.len(), region::Protection::READ_WRITE_EXECUTE)?;

        Ok(Self {
            buf: buffer,
            ptr,
            state,
        })
    }

    pub unsafe fn execute(&self) {
        let fn_ptr = self.ptr;
        let state = self.state.borrow_mut();
        execute((&*state) as *const _ as usize, fn_ptr as usize);
    }

    pub fn ptr(&self) -> *const u8 {
        self.ptr
    }

    pub fn as_slice(&self) -> &[u8] {
        self.buf.as_slice()
    }
}

/// Executes the generated code
///
/// # Safety
/// This function relies on inline assembly to setup the stack frame before
/// jumping into the memory containing the generated code.
/// It is expected that the code jumps back to the address saved in `r13`
/// register.
pub unsafe fn execute(state_addr: usize, resume_addr: usize) {
    asm!(
        "lea r13, [rip+3]", // save the address of the instruction after `jmp` as a return address
        "jmp r14",

        in("r14") resume_addr,
        in("rsi") state_addr,
        out("r13") _,
        out("r15") _,
    );
}
/// Resumes the generated code
///
/// # Safety
/// This function relies on inline assembly to setup the stack frame before
/// jumping into the memory containing the generated code.
/// It is expected that the code jumps back to the address saved in `r13`
/// register.
pub unsafe fn resume(state: &Rc<RefCell<State>>, resume_addr: usize, jump_to: usize) {
    let state = state.borrow_mut();
    let state_addr = (&*state) as *const _ as u64;

    std::arch::asm!(
        "lea r13, [rip+3]", // save the address of the instruction after `jmp` as a return address
        "jmp r14",

        in("r14") resume_addr,
        in("r15") jump_to,
        in("rsi") state_addr,
        out("r13") _,
    );
}
