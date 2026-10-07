use super::*;

const RULE: &PreRace = &oag_pulse::pre_race::PRE_RACE;
const HOLD_TICKS: u32 = RULE.hold_ticks.value;
const LOCK_TICKS: u32 = RULE.lock_ticks.value;

const DT: f32 = 1.0 / 60.0;

/// Runs a clock until it ends, returning the tick it ended on, if it did within `limit`.
fn end_of(length: f32, held_from: Option<u32>, limit: u32) -> Option<u32> {
    let mut clock = Timeline::new(length, RULE);
    (0..limit).find(|&k| {
        let held = held_from.is_some_and(|from| k >= from);
        clock.step(held, DT) == Beat::Over
    })
}

#[test]
fn the_animation_holds_its_first_frame_then_plays() {
    let mut clock = Timeline::new(25.0, RULE);
    let shown: Vec<f32> = (0..HOLD_TICKS + 3)
        .map(|_| match clock.step(false, DT) {
            Beat::Show(seconds) => seconds,
            Beat::Over => panic!("over early"),
        })
        .collect();
    assert!(shown[..=HOLD_TICKS as usize].iter().all(|&s| s == 0.0));
    // The pose is one tick behind the clock: the clock leaves zero at HOLD_TICKS + 1, the pose
    // on the tick after.
    assert!((shown[HOLD_TICKS as usize + 1] - 0.0).abs() < f32::EPSILON);
    assert!((shown[HOLD_TICKS as usize + 2] - DT).abs() < 1e-6);
}

#[test]
fn it_ends_when_the_animation_reaches_its_end() {
    let end = end_of(25.0, None, 2000).expect("ends");
    // The clock reads `AnimEnd` at HOLD_TICKS + 1500.
    assert_eq!(end, HOLD_TICKS + 1500);
}

#[test]
fn a_shorter_circuit_ends_sooner() {
    assert_eq!(end_of(18.0, None, 2000), Some(HOLD_TICKS + 1080));
}

#[test]
fn a_held_button_cannot_end_it_before_the_lock_lifts() {
    // Held from the first tick: the animation has started by tick 29, but the lock holds to 60.
    assert_eq!(end_of(25.0, Some(0), 2000), Some(LOCK_TICKS));
}

#[test]
fn a_button_held_later_ends_it_on_the_tick_it_is_held() {
    assert_eq!(end_of(25.0, Some(300), 2000), Some(300));
}

#[test]
fn the_lock_outlasts_the_hold() {
    // So a button held from the start ends the flyby when the lock lifts, with the animation
    // already moving - the measured 61 ticks of the original's "dismissed by a held cross".
    const { assert!(HOLD_TICKS < LOCK_TICKS) };
}

#[test]
fn a_title_that_waits_for_a_press_loops_the_animation_and_never_runs_out() {
    let mut clock = Timeline::new(2.0, &oag_hd::pre_race::PRE_RACE);
    // 2 s is 120 ticks; a title ended by `AnimEnd` would be over by now.
    for _ in 0..1000 {
        assert!(matches!(clock.step(false, DT), Beat::Show(s) if s < 2.0));
    }
    assert_eq!(clock.step(true, DT), Beat::Over);
}
