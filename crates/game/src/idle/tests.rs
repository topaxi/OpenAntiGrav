use std::cell::RefCell;
use std::rc::Rc;

use super::*;

fn at(secs: u64, base: Instant) -> Instant {
    base + Duration::from_secs(secs)
}

#[test]
fn a_running_race_holds_and_a_paused_or_ended_one_does_not() {
    let rule = Rule::default();
    let now = Instant::now();
    assert!(rule.wanted(true, now));
    assert!(!rule.wanted(false, now), "paused, or the race ended");
}

#[test]
fn pad_input_holds_for_sixty_seconds_and_not_after() {
    let t0 = Instant::now();
    let mut rule = Rule::default();
    rule.pad_spoke(t0);
    assert!(rule.wanted(false, t0), "just now");
    assert!(rule.wanted(false, at(59, t0)));
    assert!(!rule.wanted(false, at(61, t0)));
}

#[test]
fn a_race_ending_inside_the_grace_keeps_holding_until_it_runs_out() {
    let t0 = Instant::now();
    let mut rule = Rule::default();
    rule.pad_spoke(t0);
    assert!(rule.wanted(true, at(300, t0)), "racing, pad untouched");
    assert!(!rule.wanted(false, at(300, t0)), "race over, pad untouched");
}

struct Log(Rc<RefCell<Vec<bool>>>);

impl Backend for Log {
    fn hold(&mut self, on: bool) {
        self.0.borrow_mut().push(on);
    }
}

fn idle_with_log() -> (Idle, Rc<RefCell<Vec<bool>>>) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let idle = Idle {
        backend: Some(Box::new(Log(calls.clone()))),
        bound: Some(WindowId::dummy()),
        ..Idle::default()
    };
    (idle, calls)
}

#[test]
fn the_backend_is_told_on_edges_only() {
    let t0 = Instant::now();
    let (mut idle, calls) = idle_with_log();
    idle.update(false, false, t0);
    idle.update(true, false, at(1, t0));
    idle.update(true, false, at(2, t0));
    idle.update(false, false, at(3, t0));
    idle.update(false, false, at(4, t0));
    assert_eq!(*calls.borrow(), [true, false]);
}

#[test]
fn a_hidden_window_does_not_hold_and_lets_go() {
    let t0 = Instant::now();
    let (mut idle, calls) = idle_with_log();
    idle.update(true, false, t0);
    idle.set_hidden(true);
    idle.update(true, false, at(1, t0));
    assert!(!idle.held());
    idle.set_hidden(false);
    idle.update(true, false, at(2, t0));
    assert_eq!(*calls.borrow(), [true, false, true]);
}

#[test]
fn a_window_with_no_backend_is_not_retried_every_frame() {
    let mut idle = Idle::default();
    let mut opened = 0;
    for _ in 0..3 {
        idle.bind(WindowId::dummy(), || {
            opened += 1;
            None
        });
    }
    assert_eq!(opened, 1);
}
