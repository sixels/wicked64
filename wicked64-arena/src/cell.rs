use std::{
    cell::Cell,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use sptr::Strict;

pub struct ArenaRef<'a, T: ?Sized> {
    data: &'a T,
    borrow: &'a Cell<isize>,
}
pub struct ArenaRefMut<'a, T: ?Sized> {
    data: &'a mut T,
    borrow: &'a Cell<isize>,
}

impl<'a, T: ?Sized> Drop for ArenaRef<'a, T> {
    fn drop(&mut self) {
        let borrow = self.borrow.get();
        self.borrow.set(borrow.wrapping_sub(1));
    }
}
impl<'a, T: ?Sized> Drop for ArenaRefMut<'a, T> {
    fn drop(&mut self) {
        let borrow = self.borrow.get();
        self.borrow.set(borrow.wrapping_add(1));
    }
}

impl<'a, T: ?Sized> ArenaRef<'a, T> {
    pub fn clone(self: &ArenaRef<'a, T>) -> ArenaRef<'a, T> {
        let borrow = self.borrow.get();
        self.borrow.set(borrow.wrapping_add(1));

        ArenaRef {
            data: self.data,
            borrow: self.borrow,
        }
    }
}

impl<'a, T: ?Sized> Deref for ArenaRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a, T: ?Sized> Deref for ArenaRefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a, T: ?Sized> DerefMut for ArenaRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

/// ArenaCell provides interior mutability to objects in our Arena
///
/// This is a simplified implementation of the `std::cell::RefCell`, so it's not
/// thread-safe
pub struct ArenaCell<T: ?Sized> {
    /// `ptr` will act like just like a regular UnsafeCell
    ptr: *mut T,
    /// The borrow state. Negative (<0) values means there is a mutable borrow
    /// of this cell. Positive values (>0) means there is 1 or more regular
    /// borrows.
    borrow: Cell<isize>,
}

unsafe impl<T: Send> Send for ArenaCell<T> where T: ?Sized {}

impl<T: ?Sized + Debug> Debug for ArenaCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <T as Debug>::fmt(self.borrow().deref(), f)
    }
}

impl<T: Sized> ArenaCell<T> {
    pub fn new(data: T) -> ArenaCell<T> {
        Self::new_ptr(crate::global_arena().alloc(data).unwrap())
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
}
impl<T: ?Sized> ArenaCell<T> {
    pub(crate) fn new_ptr(ptr: *mut T) -> ArenaCell<T> {
        Self {
            ptr,
            borrow: Cell::new(0),
        }
    }

    fn reference(borrow: &Cell<isize>) -> bool {
        let borrow_state = borrow.get().wrapping_add(1);
        if borrow_state > 0 {
            borrow.set(borrow_state);
            true
        } else {
            false
        }
    }
    fn reference_mut(borrow: &Cell<isize>) -> bool {
        match borrow.get() {
            0 => {
                borrow.set(-1);
                true
            }
            _ => false,
        }
    }

    pub fn borrow(&self) -> ArenaRef<'_, T> {
        self.try_borrow().unwrap()
    }
    pub fn try_borrow(&self) -> anyhow::Result<ArenaRef<'_, T>> {
        match Self::reference(&self.borrow) {
            true => {
                let data = unsafe { &*self.ptr };

                Ok(ArenaRef {
                    data,
                    borrow: &self.borrow,
                })
            }
            false => Err(anyhow::anyhow!("Already borrowed as mutable")),
        }
    }

    pub fn borrow_mut(&self) -> ArenaRefMut<'_, T> {
        self.try_borrow_mut().unwrap()
    }
    pub fn try_borrow_mut(&self) -> anyhow::Result<ArenaRefMut<'_, T>> {
        match Self::reference_mut(&self.borrow) {
            true => {
                let data = unsafe { &mut *self.ptr };

                Ok(ArenaRefMut {
                    data,
                    borrow: &self.borrow,
                })
            }
            false => Err(anyhow::anyhow!("Already borrowed")),
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *(self.ptr) }
    }
}

impl<T: Clone> Clone for ArenaCell<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.borrow().clone().deref() as *const T as *mut T,
            borrow: Cell::new(0),
        }
    }

    fn clone_from(&mut self, other: &Self) {
        self.get_mut().clone_from(&other.borrow())
    }
}

#[cfg(test)]
mod tests {

    use std::ops::Deref;

    use crate::{global_arena, init_arena, ArenaCell};

    #[test]
    fn arena_cell_borrows() {
        init_arena(1024);

        let x = ArenaCell::new("Lorem Ipsum");

        let y = x.borrow();
        let z = x.borrow();

        assert_eq!(*y, "Lorem Ipsum");
        assert_eq!(*y, *z);

        let a = x.try_borrow_mut();
        assert!(a.is_err());

        drop(y);
        drop(z);

        let a = x.borrow_mut();
        assert_eq!(*a, "Lorem Ipsum");

        let y = x.try_borrow();
        assert!(y.is_err());

        drop(a);

        let y = x.try_borrow();
        assert!(y.is_ok());
    }

    #[test]
    fn wasm_offset() {
        init_arena(1024);

        let foo = ArenaCell::new("Something");
        let bar = ArenaCell::new(420);
        let not_in_arena = 30;

        let arena = global_arena();

        assert!(arena.wasm_offset(foo.borrow().deref()).is_some());
        assert!(foo.wasm_offset() == 0x01);
        assert!(arena.wasm_offset(bar.borrow().deref()).is_some());
        assert!(arena.wasm_offset(&not_in_arena).is_none());
    }
}
