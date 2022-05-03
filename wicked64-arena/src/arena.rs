use std::{cell::Cell, mem};

use sptr::Strict;
use wasmer::{Memory, MemoryType};

use crate::ArenaBox;

pub const PAGE_SIZE_IN_BYTES: u32 = 65536;

/// An arena to manage memory in wasm.
pub struct Arena {
    store: wasmer::Store,
    engine: wasmer::UniversalEngine,
    pub(crate) memory: Memory,
    index: Cell<usize>,
}

impl Arena {
    /// Create a new arena with the given size in bytes.
    ///
    /// The actual memory size may be bigger than the given size if `size` is
    /// not divisible by 65536.
    pub fn new(size: u32) -> anyhow::Result<Arena> {
        let engine = {
            let universal = wasmer::Universal::new(wasmer::Singlepass::default());
            universal.engine()
        };
        let store = wasmer::Store::new(&engine);

        // get the number of pages for the given size
        // also ceil the result
        let pages = size / PAGE_SIZE_IN_BYTES + u32::from((size % PAGE_SIZE_IN_BYTES) > 0);
        assert!(pages > 0);

        let host_memory = Memory::new(&store, MemoryType::new(pages, None, false))?;

        Ok(Self {
            store,
            engine,
            memory: host_memory,
            index: Cell::new(1),
        })
    }

    pub fn store(&self) -> &wasmer::Store {
        &self.store
    }
    pub fn engine(&self) -> &wasmer::UniversalEngine {
        &self.engine
    }

    /// Returns the number of pages the memory has.
    /// The size in bytes of each page is 65536.
    pub fn pages(&self) -> wasmer::Pages {
        self.memory.size()
    }

    pub fn memory_cloned(&self) -> Memory {
        self.memory.clone()
    }

    /// Allocate T to the wasm memory and return its reference.
    ///
    /// If the data size is greater than the remaining arena size, an error will
    /// be returned.
    ///
    /// # SAFETY
    ///
    /// - The arena is already initialized and its address is guaranteed to be
    /// valid by Rust's lifetime system.
    /// - The data size is validated before any unsafe operation.
    /// - The pointer to data is a value in the stack, so it won't overlap with
    /// the arena memory.
    /// - The pointer created in the arena is always valid since it lies within
    /// the memory boundaries.
    pub fn alloc<T: Sized>(&self, data: T) -> anyhow::Result<*mut T> {
        let data_size = mem::size_of::<T>();
        let data_offset = self.index.get();

        if (data_offset + data_size) as u64 > self.memory.data_size() {
            anyhow::bail!("BUFFER OVERFLOW");
        }

        let arena_ptr = self.memory.data_ptr();
        let data_ptr = unsafe {
            let data_ptr = arena_ptr.add(data_offset);
            data_ptr.copy_from_nonoverlapping(&data as *const T as *const u8, data_size);
            data_ptr
        };

        self.index.set(self.index.get() + data_size);

        Ok(data_ptr as *mut T)
    }

    /// Clone an slice into an ArenaBox
    pub fn boxed_slice<T: Sized>(&self, slice: &[T]) -> anyhow::Result<ArenaBox<[T]>> {
        let len = slice.len();
        let clone_len = mem::size_of::<T>() * len;
        let data_offset = self.index.get();

        if (data_offset + clone_len) as u64 > self.memory.data_size() {
            anyhow::bail!("BUFFER OVERFLOW");
        }
        let arena_ptr = self.memory.data_ptr();

        let data_ptr = unsafe {
            let data_ptr = arena_ptr.add(data_offset);
            data_ptr.copy_from_nonoverlapping(slice.as_ptr() as *const u8, clone_len);
            data_ptr
        };
        self.index.set(self.index.get() + clone_len);

        let new_slice = std::ptr::slice_from_raw_parts_mut(data_ptr as *mut T, len);

        Ok(ArenaBox::new_raw(new_slice))
    }

    /// Returns the offset of `data` in the wasm offset.
    ///
    /// If data is not in the arena, this function returns None
    pub fn wasm_offset<T>(&self, data: &T) -> Option<u32> {
        let base_offset = self.memory.data_ptr().addr();

        let (end_offset, overflow) = base_offset.overflowing_add(self.memory.data_size() as usize);
        debug_assert!(overflow == false);

        let data_addr = (data as *const T).addr();

        if data_addr < base_offset || data_addr >= end_offset {
            None
        } else {
            let offset = data_addr - base_offset;
            debug_assert!(offset <= 0xFFFFFFFF);
            Some(offset as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{global_arena, init_arena, ArenaCell};

    use super::*;

    #[test]
    fn arena_size() {
        let arena = Arena::new(0x1_400_000).unwrap();
        assert_eq!(arena.memory.size(), wasmer::Pages::from(320));
        arena.alloc(0).unwrap();
    }

    #[test]
    fn arena_alloc() {
        #[derive(Debug, PartialEq, Eq)]
        #[allow(dead_code)]
        enum TestEnum {
            Error(String),
            Success,
        }
        struct TestStruct {
            name: String,
            code: usize,
            status: TestEnum,
        }
        impl Drop for TestStruct {
            fn drop(&mut self) {
                println!("{} is dropping", self.name);
            }
        }

        init_arena(1024);

        let arena = global_arena();
        let mem_addr = arena.memory.data_ptr();

        let foo = ArenaCell::new(10u64);

        // check if the arena index is being set as expected
        let mut new_index = std::mem::size_of::<u64>() + 1;
        assert_eq!(arena.index.get(), new_index);

        let bar = {
            ArenaCell::new(TestStruct {
                name: String::from(
                    "Never gonna give you up\r\nNever gonna let you down\r\n...\r\n",
                ),
                code: 69,
                status: TestEnum::Success,
            })
        };

        new_index += std::mem::size_of::<TestStruct>();
        assert_eq!(arena.index.get(), new_index);

        {
            let foo = foo.try_borrow().unwrap();
            let bar = bar.try_borrow().unwrap();

            assert_eq!(*foo, 10);
            assert_eq!(
                bar.name,
                String::from("Never gonna give you up\r\nNever gonna let you down\r\n...\r\n")
            );
            assert_eq!(bar.code, 69);
            assert_eq!(bar.status, TestEnum::Success);
        }

        arena.memory.grow(2000).unwrap();

        // values should remain the same after growing the memory
        assert_eq!(arena.memory.data_ptr(), mem_addr);
        {
            let foo = foo.try_borrow().unwrap();
            let bar = bar.try_borrow().unwrap();

            assert_eq!(*foo, 10);
            assert_eq!(
                bar.name,
                String::from("Never gonna give you up\r\nNever gonna let you down\r\n...\r\n")
            );
            assert_eq!(bar.code, 69);
            assert_eq!(bar.status, TestEnum::Success);
        }
    }
}
