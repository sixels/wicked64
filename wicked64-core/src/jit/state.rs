use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::n64::State;

pub struct EmulatorState(Rc<RefCell<State>>);

impl EmulatorState {
    pub fn new(state: Rc<RefCell<State>>) -> Self {
        Self(state)
    }

    pub fn offset_of<F, T>(&self, get_offset: F) -> usize
    where
        F: FnOnce(&State) -> &T,
    {
        let state = self.0.borrow();

        let data_addr = get_offset(&state) as *const T as usize;
        let state_addr = self.state_ptr() as usize;

        debug_assert!(state_addr <= data_addr);
        data_addr - state_addr
    }

    pub fn state_ptr(&self) -> *const State {
        &*self.0.borrow()
    }
}

impl Deref for EmulatorState {
    type Target = Rc<RefCell<State>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EmulatorState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
