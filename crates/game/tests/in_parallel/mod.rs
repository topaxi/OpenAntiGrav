//! Independent races, raced side by side inside one test.
//!
//! A ground-truth test that sums or compares over many races cannot always be
//! split into one `#[test]` per race: `ram_ground_truth`'s bound is a ratio,
//! and `ai_roll_ground_truth`'s is a comparison of two tiers' totals over every
//! circuit. Run in series, such a test is the slowest thing in `just
//! test-data` and sets its wall clock alone, on one core.
//!
//! The races in it share nothing: each loads its own track and steps its own
//! `Race` from its own seed. So [`map`] runs each on its own thread and hands
//! the results back **in input order**, which is all a sum, a ratio or a
//! printed table depends on. No race's state can see another's, so every
//! number a test asserts is the one the serial loop produced; what changes is
//! only how many cores the test can use.
//!
//! `tests/<dir>/mod.rs` is not a test target of its own, which is why this
//! is a directory.

#![allow(dead_code)]

/// `items.iter().map(f).collect()`, with each call on a thread of its own, at
/// most `width` at a time, results in the order of `items`.
///
/// A panic in any call is re-raised here, so an assertion inside `f` still
/// fails the test that called this.
pub fn map<T: Sync, R: Send>(items: &[T], width: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let width = width.max(1);
    let f = &f;
    let mut out = Vec::with_capacity(items.len());
    for chunk in items.chunks(width) {
        std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|item| scope.spawn(move || f(item)))
                .collect();
            for handle in handles {
                match handle.join() {
                    Ok(result) => out.push(result),
                    Err(panic) => std::panic::resume_unwind(panic),
                }
            }
        });
    }
    out
}
