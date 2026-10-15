//! A lock one thread holds at a time and takes again while it holds it, shaped
//! as std's `ReentrantLock`, which Rust 1.98 has not stabilised yet.

use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

pub(super) struct ReentrantLock<T> {
    holder: Mutex<Holder>,
    release: Condvar,
    value: UnsafeCell<T>,
}

#[derive(Default)]
struct Holder {
    thread: Option<ThreadId>,
    name: Option<Arc<str>>,
    depth: usize,
}

/// How a thread waiting for the lock is told of: by the name it goes by,
/// which the lock keeps while the thread holds it, and, once it has waited
/// `patience`, by `report` given the holder's name and the time waited.
pub(super) struct Watch<'a> {
    pub name: &'a Arc<str>,
    pub patience: Duration,
    pub report: &'a dyn Fn(&str, Duration),
}

// SAFETY: only the thread holding the lock reaches `value`, and only through a
// guard that cannot leave that thread, so `T` is never used by two threads.
unsafe impl<T: Send> Sync for ReentrantLock<T> {}

impl<T> ReentrantLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            holder: Mutex::new(Holder {
                thread: None,
                name: None,
                depth: 0,
            }),
            release: Condvar::new(),
            value: UnsafeCell::new(value),
        }
    }

    /// Takes the lock, waiting while another thread holds it.
    pub fn lock(&self) -> ReentrantLockGuard<'_, T> {
        self.lock_watched(None)
    }

    /// Takes the lock as `lock` does, telling of a wait once it outlasts the
    /// watch's patience; the report runs while the holder is free to let go.
    pub fn lock_watched(&self, watch: Option<&Watch<'_>>) -> ReentrantLockGuard<'_, T> {
        let current = thread::current().id();
        let start = Instant::now();
        let mut patience = watch.map(|watch| watch.patience);
        let mut holder = self.holder();
        while holder.thread.is_some_and(|thread| thread != current) {
            let Some(left) = patience.map(|patience| patience.saturating_sub(start.elapsed()))
            else {
                holder = self
                    .release
                    .wait(holder)
                    .unwrap_or_else(PoisonError::into_inner);
                continue;
            };
            if left.is_zero() {
                let name = holder.name.clone();
                drop(holder);
                if let Some(watch) = watch {
                    (watch.report)(name.as_deref().unwrap_or("another thread"), start.elapsed());
                }
                patience = None;
                holder = self.holder();
                continue;
            }
            holder = self
                .release
                .wait_timeout(holder, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        if holder.depth == 0 {
            holder.name = watch.map(|watch| Arc::clone(watch.name));
        }
        holder.thread = Some(current);
        holder.depth += 1;
        ReentrantLockGuard {
            lock: self,
            not_send: PhantomData,
        }
    }

    /// Whether this thread holds the lock, which it answers without waiting
    /// for the thread that does.
    pub fn is_held_here(&self) -> bool {
        self.holder().thread == Some(thread::current().id())
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
        self.depth() > 1
    }

    /// How many guards this thread holds the lock through, this one included.
    pub fn depth(&self) -> usize {
        self.lock.holder().depth
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
            holder.name = None;
            self.lock.release.notify_one();
        }
    }
}
