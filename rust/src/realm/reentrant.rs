//! A lock one thread holds at a time and takes again while it holds it, shaped
//! as std's `ReentrantLock`, which Rust 1.98 has not stabilised yet.

use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, ThreadId};

pub(super) struct ReentrantLock<T> {
    holder: Mutex<Holder>,
    released: Condvar,
    value: UnsafeCell<T>,
}

#[derive(Default)]
struct Holder {
    thread: Option<ThreadId>,
    depth: usize,
}

// SAFETY: only the thread holding the lock reaches `value`, and only through a
// guard that cannot leave that thread, so `T` is never used by two threads.
unsafe impl<T: Send> Sync for ReentrantLock<T> {}

impl<T> ReentrantLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            holder: Mutex::new(Holder {
                thread: None,
                depth: 0,
            }),
            released: Condvar::new(),
            value: UnsafeCell::new(value),
        }
    }

    /// Takes the lock, waiting while another thread holds it.
    pub fn lock(&self) -> ReentrantLockGuard<'_, T> {
        let current = thread::current().id();
        let mut holder = self.holder();
        while holder.thread.is_some_and(|thread| thread != current) {
            holder = self
                .released
                .wait(holder)
                .unwrap_or_else(PoisonError::into_inner);
        }
        holder.thread = Some(current);
        holder.depth += 1;
        ReentrantLockGuard {
            lock: self,
            not_send: PhantomData,
        }
    }

    // The holder is only ever changed whole, so a panic leaves it consistent.
    fn holder(&self) -> MutexGuard<'_, Holder> {
        self.holder.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub(super) struct ReentrantLockGuard<'a, T> {
    lock: &'a ReentrantLock<T>,
    not_send: PhantomData<*const ()>,
}

impl<T> ReentrantLockGuard<'_, T> {
    /// Whether this thread also holds the lock through another guard.
    pub fn is_nested(&self) -> bool {
        self.lock.holder().depth > 1
    }
}

impl<T> Deref for ReentrantLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: this thread holds the lock while the guard lives.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> Drop for ReentrantLockGuard<'_, T> {
    fn drop(&mut self) {
        let mut holder = self.lock.holder();
        holder.depth -= 1;
        if holder.depth == 0 {
            holder.thread = None;
            self.lock.released.notify_one();
        }
    }
}
