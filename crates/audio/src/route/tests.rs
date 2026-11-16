use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::*;
use crate::spatial::{SpatialChoice, SpatialMode};

#[test]
fn headphones_resolve_to_the_hrtf_and_everything_else_to_the_rear_cue() {
    let auto = |kind| SpatialChoice::Auto.resolve(kind);
    assert_eq!(auto(RouteKind::Headphones), SpatialMode::Headphones);
    assert_eq!(auto(RouteKind::Speakers), SpatialMode::RearCue);
    assert_eq!(auto(RouteKind::Unknown), SpatialMode::RearCue);
}

#[test]
fn the_watcher_reports_a_change_once_and_never_waits() {
    let now = Arc::new(Mutex::new(Route {
        device: Some("a".into()),
        kind: RouteKind::Speakers,
    }));
    let seen = Arc::clone(&now);
    let mut watcher =
        RouteWatcher::with_probe(Arc::new(move || seen.lock().expect("lock").clone()), 1);
    let wait = |watcher: &mut RouteWatcher| {
        let start = Instant::now();
        loop {
            if let Some(route) = watcher.poll() {
                return Some(route);
            }
            if start.elapsed() > Duration::from_secs(5) {
                return None;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    };
    assert_eq!(wait(&mut watcher).expect("first").kind, RouteKind::Speakers);
    *now.lock().expect("lock") = Route {
        device: Some("b".into()),
        kind: RouteKind::Headphones,
    };
    let changed = wait(&mut watcher).expect("change");
    assert_eq!(changed.kind, RouteKind::Headphones);
    assert_eq!(changed.device.as_deref(), Some("b"));
    // Unchanged answers report nothing.
    for _ in 0..50 {
        assert!(watcher.poll().is_none());
        std::thread::sleep(Duration::from_millis(1));
    }
}
