use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::*;

/// The registry is process-wide and `STOP` is one-way, so one test walks the whole contract.
#[test]
fn finish_waits_for_a_detached_worker_and_tells_it_to_stop() {
    let done = Arc::new(AtomicBool::new(false));
    let guard = Guard::enter("test-worker");
    let flag = Arc::clone(&done);
    // Detached: the handle is dropped, as a `Task` in an abandoned stage is.
    drop(std::thread::spawn(move || {
        let _guard = guard;
        while !stopping() {
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(50));
        flag.store(true, Ordering::SeqCst);
    }));
    assert_eq!(live(), vec!["test-worker"]);
    assert!(finish_within(Duration::from_secs(10)));
    assert!(
        done.load(Ordering::SeqCst),
        "finish returned before the worker did"
    );
    assert!(live().is_empty());
    assert!(finish_within(Duration::from_millis(1)), "idempotent");
}

#[test]
fn a_worker_that_will_not_stop_is_reported_not_waited_for_forever() {
    // Runs after the other test sets STOP or before it; either way the guard below is held by
    // this test alone for its duration, so use a distinct name and a private release flag.
    let release = Arc::new(AtomicBool::new(false));
    let guard = Guard::enter("stubborn");
    let flag = Arc::clone(&release);
    let handle = std::thread::spawn(move || {
        let _guard = guard;
        while !flag.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
        }
    });
    assert!(!finish_within(Duration::from_millis(50)));
    assert!(live().contains(&"stubborn"));
    release.store(true, Ordering::SeqCst);
    handle.join().unwrap();
    assert!(!live().contains(&"stubborn"));
}
