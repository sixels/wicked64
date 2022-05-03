pub mod arena;
pub mod r#box;
pub mod cell;
pub mod mutex;

use once_cell::unsync::OnceCell;

pub use self::{arena::Arena, cell::ArenaCell, r#box::ArenaBox, mutex::ArenaMutex};

pub static mut ARENA: OnceCell<Arena> = OnceCell::new();

/// Initialize the Arena. If it will overwrite the previous Arena state, if any.
pub fn init_arena(size: u32) {
    unsafe {
        ARENA.get_mut().map_or_else(
            || {
                let _ = ARENA.set(Arena::new(size).unwrap());
            },
            |arena| {
                *arena = Arena::new(size).unwrap();
            },
        );
    }
}

/// Get a reference to the global arena
pub fn global_arena() -> &'static Arena {
    unsafe {
        ARENA
            .get()
            .expect("Arena should already be initialized by `init_arena`")
    }
}

#[macro_export]
macro_rules! alloc {
    ($obj:tt) => {
        $crate::global_arena().alloc($obj).unwrap()
    };
}

/// Return the offset of the given object in the global arena.
///
/// # Panics:
///  It will panic if the object is not inside the arena
#[macro_export]
macro_rules! offset_of {
    ($obj:expr) => {
        $crate::global_arena().wasm_offset($obj).unwrap()
    };
}
