use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use parking_lot::{lock_api::RawMutex as RawMutexTrait, MutexGuard, RawMutex};

#[must_use = "if unused the Mutex will immediately unlock"]
pub struct ArenaMutexGuard<'a, T: ?Sized> {
    mutex: &'a ArenaMutex<T>,
}

impl<'a, T: ?Sized> ArenaMutexGuard<'a, T> {
    fn new(mutex: &'a ArenaMutex<T>) -> anyhow::Result<Self> {
        Ok(Self { mutex })
    }
}

impl<T: ?Sized> Deref for ArenaMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data }
    }
}

impl<T: ?Sized> DerefMut for ArenaMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data }
    }
}

impl<T: ?Sized> Drop for ArenaMutexGuard<'_, T> {
    fn drop(&mut self) {
        unsafe { self.mutex.mutex.unlock() }
    }
}

impl<T: ?Sized + Debug> Debug for ArenaMutexGuard<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <T as Debug>::fmt(self.deref(), f)
    }
}

/// A simple mutex implementation for wasm arena. Not 100% safe but usable for a
/// single threaded program (WasmerEnv requires Sync trait)
pub struct ArenaMutex<T: ?Sized> {
    mutex: RawMutex,
    data: *mut T,
}

impl<T: Sized> ArenaMutex<T> {
    pub fn new(data: T) -> Self {
        let data_ptr = crate::global_arena()
            .alloc(data)
            .expect("Could not allocate to the global arena.");

        Self {
            mutex: RawMutex::INIT,
            data: data_ptr,
        }
    }
}

impl<T: ?Sized> ArenaMutex<T> {
    pub fn lock(&self) -> ArenaMutexGuard<T> {
        self.try_lock().unwrap()
    }
    pub fn try_lock(&self) -> anyhow::Result<ArenaMutexGuard<T>> {
        match self.mutex.try_lock() {
            true => ArenaMutexGuard::new(self),
            false => anyhow::bail!("Could not lock the mutex."),
        }
    }

    pub fn unlock(guard: MutexGuard<'_, T>) {
        drop(guard);
    }
}

impl<T: ?Sized + Debug> Debug for ArenaMutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.try_lock() {
            Ok(guard) => guard.fmt(f),
            Err(_) => write!(f, "Locked mutex."),
        }
    }
}

unsafe impl<T: ?Sized + Send> Send for ArenaMutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for ArenaMutex<T> {}
