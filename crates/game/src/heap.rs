//! Giving the freed heap back to the operating system after a race.
//!
//! A race load decodes and parses a few hundred megabytes of short-lived data on several
//! threads, and glibc's allocator keeps what is freed in the middle of an arena resident: it
//! only returns the top of a heap by itself. Measured with `mallinfo2` over four Race Box races
//! in one process, the bytes in use stayed at 140 to 180 MB while the arenas grew from 0.86 to
//! 1.8 GB, all of the growth sitting in free chunks. `malloc_trim(0)` hands those pages back
//! (`docs/architecture/memory.md`, "A second race in one session").
//!
//! [`release_freed`] is that call. It takes each arena's lock in turn, so it runs on the thread
//! that has just finished the work that freed the memory (the race build and the race drop),
//! never on the frame thread.

use std::sync::atomic::{AtomicU64, Ordering};

static RELEASES: AtomicU64 = AtomicU64::new(0);

/// How many times [`release_freed`] has run in this process, on every platform (a test that
/// checks the call is made does not depend on the allocator).
#[must_use]
pub fn releases() -> u64 {
    RELEASES.load(Ordering::Relaxed)
}

/// Returns the pages the allocator holds free to the operating system. Costs a pass over every
/// arena, so call it where a large amount was just freed, not per frame.
pub fn release_freed() {
    RELEASES.fetch_add(1, Ordering::Relaxed);
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        let start = web_time::Instant::now();
        trim();
        log::debug!("returned freed heap to the system in {:?}", start.elapsed());
    }
}

/// [`release_freed`] on a thread of its own, for a caller that is the frame thread. The thread
/// is detached: the trim is harmless to cut off at exit.
pub fn release_freed_off_thread() {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(e) = std::thread::Builder::new()
        .name("heap-trim".to_string())
        .spawn(release_freed)
    {
        log::debug!("could not trim the heap off the frame thread: {e}");
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[allow(unsafe_code)]
fn trim() {
    // SAFETY: `malloc_trim` takes only a pad size, may be called from any thread at any time
    // and touches nothing the program owns; it only returns free pages to the kernel.
    unsafe {
        libc::malloc_trim(0);
    }
}
