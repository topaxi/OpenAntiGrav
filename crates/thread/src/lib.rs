//! A closure on another thread, on every target this project builds for.
//!
//! Natively that is `std::thread`. In the browser `std::thread::spawn` is
//! unsupported even when the module has atomics, so a thread is a Web Worker
//! that shares the module's memory ([`web`]). The two differ in one more way
//! that shapes everything here: **the page's thread may not block**. A
//! contended `std::sync::Mutex` ends in `memory.atomic.wait32`, which traps on
//! it, so [`lock`] spins there instead, and [`Task`] is polled through an
//! atomic and taken only once it is done. See docs/tools/web.md, "Threads".
//!
//! Below every crate that moves work off the frame loop (the race load, the
//! music fetch, the audio render-ahead loop) and above nothing of this
//! project's, so each of them reaches the same worker mechanism.

use std::sync::{LockResult, Mutex, MutexGuard};

#[cfg(target_arch = "wasm32")]
pub mod web;

/// Locks `mutex`, without ever waiting on the page's thread in the browser.
///
/// Natively and on a worker this is `Mutex::lock`. On the page's thread it
/// spins on `try_lock`, which is only fit for a lock the other side holds for
/// moments (a mixer pass, a counter): the alternative there is a trap.
///
/// # Errors
///
/// The lock is poisoned, as `Mutex::lock`.
pub fn lock<T>(mutex: &Mutex<T>) -> LockResult<MutexGuard<'_, T>> {
    #[cfg(target_arch = "wasm32")]
    if !web::on_worker() {
        return web::try_lock_spinning(mutex);
    }
    mutex.lock()
}

/// The closure behind a [`Task`] panicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panicked;

/// One closure running on its own thread, polled for its result.
///
/// The shape `oag_raceplay`'s race load and `oag_sound`'s music fetch share:
/// start it, ask [`Task::is_finished`] once a frame, [`Task::join`] once it
/// is. A task whose thread would not start reads as finished, so a poller
/// never waits on it, and [`Task::join`] then returns `None` (the start
/// failure is logged where it happened).
#[derive(Debug)]
pub struct Task<T> {
    #[cfg(not(target_arch = "wasm32"))]
    handle: Option<std::thread::JoinHandle<T>>,
    #[cfg(target_arch = "wasm32")]
    slot: Option<std::sync::Arc<web::Slot<T>>>,
}

impl<T: Send + 'static> Task<T> {
    /// Starts `job` on a new thread named `name` (natively; a worker has no
    /// name).
    #[must_use]
    pub fn spawn(name: &str, job: impl FnOnce() -> T + Send + 'static) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let handle = std::thread::Builder::new()
                .name(name.to_string())
                .spawn(job)
                .map_err(|why| log::error!("{name}: could not start a thread: {why}"))
                .ok();
            Self { handle }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let slot = std::sync::Arc::new(web::Slot::default());
            let spawned = web::spawn({
                let slot = std::sync::Arc::clone(&slot);
                move || slot.fill(job())
            });
            if let Err(why) = &spawned {
                log::error!("{name}: {why}");
            }
            Self {
                slot: spawned.ok().map(|()| slot),
            }
        }
    }

    /// Whether the closure has returned, or never started.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self
            .handle
            .as_ref()
            .is_none_or(std::thread::JoinHandle::is_finished);
        #[cfg(target_arch = "wasm32")]
        self.slot.as_ref().is_none_or(|slot| slot.is_done())
    }

    /// Takes the result, waiting for it natively if it is not in yet. A
    /// caller on the page's thread only calls this once [`Self::is_finished`]:
    /// in the browser an unfinished task returns `None` rather than wait, and
    /// keeps its result for a later call.
    ///
    /// `None` on a second call and for a task that never started.
    pub fn join(&mut self) -> Option<Result<T, Panicked>> {
        #[cfg(not(target_arch = "wasm32"))]
        return self
            .handle
            .take()
            .map(|handle| handle.join().map_err(|_| Panicked));
        #[cfg(target_arch = "wasm32")]
        {
            if !self.is_finished() {
                return None;
            }
            self.slot.take().and_then(|slot| slot.take()).map(Ok)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_task_hands_back_its_value_once() {
        let mut task = Task::spawn("test-task", || 6 * 7);
        assert_eq!(task.join(), Some(Ok(42)));
        assert!(
            task.is_finished(),
            "a joined task has nothing left to wait for"
        );
        assert_eq!(task.join(), None);
    }

    #[test]
    fn a_panicking_task_is_reported_rather_than_resumed() {
        let mut task = Task::spawn("test-panic", || -> u32 { panic!("on purpose") });
        assert_eq!(task.join(), Some(Err(Panicked)));
    }

    #[test]
    fn lock_is_the_plain_lock_off_the_browser() {
        let mutex = Mutex::new(1);
        *lock(&mutex).unwrap() += 1;
        assert_eq!(*mutex.lock().unwrap(), 2);
    }
}
