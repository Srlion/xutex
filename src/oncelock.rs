/// A thread-safe cell which can be written to only once.
///
/// This is an implementation of a `OnceLock` for `no_std` environments.
/// It provides a way to initialize a value lazily and ensure that the
/// initialization happens exactly once, even when accessed from multiple
/// threads.
///
/// Unlike the standard library's `OnceLock`, this implementation does not
/// OS-level synchronization primitives, making it suitable for embedded systems
/// and other constrained environments.
use crate::backoff::Backoff;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

const EMPTY: u8 = 0;
const INITIALIZING: u8 = 1;
const INITIALIZED: u8 = 2;

pub(crate) struct OnceLock<T> {
    state: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T: Send + Sync> Sync for OnceLock<T> {}
unsafe impl<T: Send> Send for OnceLock<T> {}

impl<T> OnceLock<T> {
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(EMPTY),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn get(&self) -> Option<&T> {
        if self.state.load(Ordering::Acquire) == INITIALIZED {
            Some(unsafe { (*self.value.get()).assume_init_ref() })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if let Some(value) = self.get() {
            return value;
        }

        match self
            .state
            .compare_exchange(EMPTY, INITIALIZING, Ordering::Acquire, Ordering::Acquire)
        {
            Ok(_) => {
                let value = f();
                unsafe {
                    (*self.value.get()).write(value);
                }
                self.state.store(INITIALIZED, Ordering::Release);
                unsafe { (*self.value.get()).assume_init_ref() }
            }
            Err(_) => {
                // Spin until initialized
                let backoff = Backoff::new();
                while self.state.load(Ordering::Acquire) != INITIALIZED {
                    backoff.snooze();
                }
                unsafe { (*self.value.get()).assume_init_ref() }
            }
        }
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for OnceLock<T> {
    fn drop(&mut self) {
        if *self.state.get_mut() == INITIALIZED {
            unsafe {
                (*self.value.get()).assume_init_drop();
            }
        }
    }
}
