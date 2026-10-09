//! Threads on the web build: a closure run on a Web Worker that shares the
//! module's memory.
//!
//! `std::thread::spawn` is unsupported on `wasm32-unknown-unknown` even when
//! the module is built with the `atomics` target feature, so this asks the page
//! for a worker instead: `oagSpawnWorker(module, memory, entry)`, defined in
//! `web/main.js`, starts `web/worker.js`, which instantiates this same module
//! on the same shared memory and calls [`oag_worker_entry`] with `entry`, the
//! id the closure waits under. See docs/tools/web.md, "Threads".
//!
//! **The page's thread may not block.** `Atomics.wait`, and the
//! `memory.atomic.wait32` a contended `std::sync::Mutex` ends in, trap on it,
//! so whatever the main thread shares with a worker it reads with an atomic or
//! a `try_lock`, never a waiting `lock` ([`on_worker`] tells the two apart).
//! wgpu's web types are not `Send` here, so a worker never touches the GPU.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, TryLockError};

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

type Job = Box<dyn FnOnce() + Send + 'static>;

/// Closures waiting for their worker, by the id the page passes along.
static JOBS: Mutex<BTreeMap<u32, Job>> = Mutex::new(BTreeMap::new());
static NEXT: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static WORKER: Cell<bool> = const { Cell::new(false) };
}

/// Whether this thread is a worker [`spawn`] started, rather than the page's.
#[must_use]
pub fn on_worker() -> bool {
    WORKER.with(Cell::get)
}

/// Locks `mutex` without ever waiting on it: spins on `try_lock` instead, so
/// the page's thread cannot trap on a contended lock. Only for locks a worker
/// holds for moments.
pub fn lock_spinning<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    loop {
        match mutex.try_lock() {
            Ok(guard) => return guard,
            Err(TryLockError::Poisoned(poisoned)) => return poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => std::hint::spin_loop(),
        }
    }
}

/// Runs `job` on a new Web Worker.
///
/// # Errors
/// The page has no `oagSpawnWorker` (not this project's page), or the browser
/// refused the worker. `job` is dropped unrun.
pub fn spawn(job: impl FnOnce() + Send + 'static) -> Result<(), String> {
    let global = js_sys::global();
    let spawner = js_sys::Reflect::get(&global, &JsValue::from_str("oagSpawnWorker"))
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .ok_or_else(|| "the page offers no oagSpawnWorker".to_string())?;
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    lock_spinning(&JOBS).insert(id, Box::new(job));
    let args = js_sys::Array::of3(
        &wasm_bindgen::module(),
        &wasm_bindgen::memory(),
        &JsValue::from(id),
    );
    spawner.apply(&global, &args).map(|_| ()).map_err(|why| {
        lock_spinning(&JOBS).remove(&id);
        format!("could not start a worker: {why:?}")
    })
}

/// The worker's half of [`spawn`]: runs the closure `id` names. Called once
/// per [`spawn`] by `web/worker.js`; an id with no closure does nothing.
#[wasm_bindgen]
pub fn oag_worker_entry(id: u32) {
    WORKER.with(|worker| worker.set(true));
    let job = lock_spinning(&JOBS).remove(&id);
    if let Some(job) = job {
        job();
    }
}
