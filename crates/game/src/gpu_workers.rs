//! The threads that touch the GPU, and the one place the process waits for them on its way out.
//!
//! A worker holds clones of the `wgpu` device and queue (the EndRace prefetch, the race build,
//! the parked race's drop). Dropping a handle does not stop it, and `main` returning runs the
//! Vulkan driver's process-exit teardown under whatever thread is still inside the driver: a
//! SIGSEGV (exit 139) on a loaded machine, in the `--drive` tests and on a window close alike.
//! So the rule is: **no GPU-touching thread is alive when `main` returns.** Each such thread
//! holds a [`Guard`] for its whole life, detached or not, and [`finish`] asks them to stop and
//! waits (bounded) until none is left.
//!
//! Threads that never touch the GPU (music, circuit lengths, connects, the records writer) are
//! not here. The web build has no such thread to wait for: its race build runs inline and the
//! page closing ends everything (`docs/tools/web.md`, "Threads").

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// How long [`finish`] waits for the workers. **Chosen, not measured**: a debug build under
/// the validation layer on a loaded machine has taken minutes to finish a prefetch, and a
/// cooperative worker stops at its next [`stopping`] check, so this is the cap on a worker that
/// is inside one long call, not the usual wait.
pub const EXIT_WAIT: Duration = Duration::from_secs(60);

static LIVE: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
static IDLE: Condvar = Condvar::new();
static STOP: AtomicBool = AtomicBool::new(false);

/// Held by a worker thread from before it starts until it returns.
#[derive(Debug)]
#[must_use = "the guard is the registration: dropping it at once says the worker is already done"]
pub struct Guard {
    name: &'static str,
}

impl Guard {
    /// Registers a worker called `name`. Take it on the spawning thread and move it into the
    /// closure, so a worker that has not yet been scheduled is already counted.
    pub fn enter(name: &'static str) -> Self {
        if let Ok(mut live) = LIVE.lock() {
            live.push(name);
        }
        Self { name }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Ok(mut live) = LIVE.lock() {
            if let Some(at) = live.iter().position(|name| *name == self.name) {
                live.swap_remove(at);
            }
            if live.is_empty() {
                IDLE.notify_all();
            }
        }
    }
}

/// Whether the process is on its way out: a worker checks between its phases and returns early.
#[must_use]
pub fn stopping() -> bool {
    STOP.load(Ordering::Relaxed)
}

/// The workers still running, by name.
#[must_use]
pub fn live() -> Vec<&'static str> {
    LIVE.lock().map(|live| live.clone()).unwrap_or_default()
}

/// Asks every worker to stop and waits up to `wait` for them. `true` when none is left. When it
/// is `false` the caller still has threads inside the driver and says so; it never pretends.
/// Idempotent: a second call finds nothing to wait for. The stop request is withdrawn once
/// nothing is left, so a session dropped before the process ends (a test, a relaunch) does not
/// stop the workers of a later one.
pub fn finish_within(wait: Duration) -> bool {
    STOP.store(true, Ordering::Relaxed);
    let Ok(live) = LIVE.lock() else {
        return false;
    };
    let idle = match IDLE.wait_timeout_while(live, wait, |live| !live.is_empty()) {
        Ok((live, _)) => live.is_empty(),
        Err(_) => false,
    };
    if idle {
        STOP.store(false, Ordering::Relaxed);
    }
    idle
}

/// [`finish_within`] with [`EXIT_WAIT`], logging what it waited for. `main` calls it once the
/// session is dropped, on every route out.
pub fn finish() {
    let waiting = live();
    if waiting.is_empty() {
        return;
    }
    log::info!("waiting for GPU workers before exit: {waiting:?}");
    if !finish_within(EXIT_WAIT) {
        log::error!(
            "GPU workers still running after {EXIT_WAIT:?}: {:?}; exiting under them may crash \
             the driver",
            live()
        );
    }
}

#[cfg(test)]
mod tests;
