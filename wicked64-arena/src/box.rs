use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use sptr::Strict;

pub struct ArenaBox<T: ?Sized> {
    ptr: *mut T,
}
unsafe impl<T> Sync for ArenaBox<T> where T: ?Sized + Sync {}
unsafe impl<T> Send for ArenaBox<T> where T: ?Sized + Send {}

impl<T: Sized> ArenaBox<T> {
    pub fn new(data: T) -> ArenaBox<T> {
        Self::new_raw(crate::global_arena().alloc(data).unwrap())
    }
    /// Returns the offset of `data` in the wasm offset.
    ///
    /// If data is not in the arena, this function returns None
    pub fn wasm_offset(&self) -> usize {
        let arena = crate::global_arena();

        let mem_ptr = arena.memory.data_ptr().addr();
        let data_ptr = self.ptr.addr();

        debug_assert!(data_ptr > mem_ptr); // arena offset begins at index 1
        debug_assert!(data_ptr < (mem_ptr + arena.memory.data_size() as usize));
        debug_assert!(data_ptr > mem_ptr);

        data_ptr - mem_ptr
    }

    pub fn boxed_slice(slice: &[T]) -> ArenaBox<[T]> {
        crate::global_arena().boxed_slice(slice).unwrap()
    }
}

impl<T: ?Sized> ArenaBox<T> {
    pub(crate) fn new_raw(ptr: *mut T) -> ArenaBox<T> {
        Self { ptr }
    }
}

impl<T: ?Sized> AsRef<T> for ArenaBox<T> {
    fn as_ref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T: ?Sized> AsMut<T> for ArenaBox<T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }
}

impl<T: ?Sized> Deref for ArenaBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: ?Sized + Default> Default for ArenaBox<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: ?Sized> DerefMut for ArenaBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T: ?Sized + Debug> Debug for ArenaBox<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (*self.deref()).fmt(f)
    }
}