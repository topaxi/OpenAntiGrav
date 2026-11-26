//! The probe rise ([`super::super::probe`]) against fed sequences and against two modelled GPUs:
//! one whose clock follows its load, the way the Steam Deck's governor does, and one that
//! holds its clock.

use super::*;
use crate::drs::probe::{
    HOLD_SECONDS, LATE_PERIODS, MAX_BACKOFF, Outcome, RAMP_SECONDS, SAMPLE_SECONDS,
};

const PERIOD: f32 = 1.0 / 60.0;

/// A 3840x2160 allocation with a 25 % floor, aiming at 60: the Deck runs' settings.
fn deck_limits() -> Limits {
    Limits::new((3840, 2160), (960, 540), Target::OFFERED[2])
}

fn frames(seconds: f32) -> u32 {
    (seconds / PERIOD).ceil() as u32
}

/// What a modelled run did.
#[derive(Debug)]
struct Run {
    /// The scale held, sampled every frame over the last minute.
    mean_scale: f32,
    /// The scale at the end.
    scale: f32,
    /// How many times the scale moved over the last minute.
    changes: u32,
    /// Frames late on the wall clock over the last minute.
    late: u32,
    /// Probes started over the whole run.
    probes: u32,
}

/// A GPU at 60 Hz whose race costs `full` seconds at full scale and top clock, all per pixel,
/// for `minutes`, after a first two seconds half as heavy again (a race's opening, which is
/// what drives the scale down). With `governed`, the clock (a share of the top one, 0.6 to
/// 1.0) moves a fifth of the way a frame toward keeping the GPU busy `busy` of the period;
/// without, it stays at the top. The wall clock is the period or the GPU's frame, whichever is
/// longer. With `probe` off the controller is the loop without one.
fn run(full: f32, governed: bool, busy: f32, minutes: u32, probe: bool) -> Run {
    let limits = deck_limits();
    let mut drs = if probe {
        Controller::new()
    } else {
        Controller::new().without_probe()
    };
    let mut clock = 1.0f32;
    let total = minutes * 3600;
    let tail = total - 3600;
    let (mut sum, mut changes, mut late, mut probes) = (0.0f64, 0, 0, 0);
    for frame in 0..total {
        let scale = drs.scale();
        let heavy = if frame < 120 { 1.5 } else { 1.0 };
        let work = full * heavy * scale * scale;
        if governed {
            let wanted = (work / (busy * PERIOD)).clamp(0.6, 1.0);
            clock += (wanted - clock) * 0.2;
        }
        let gpu = work / clock;
        let frame_seconds = (gpu + 0.000_5).max(PERIOD);
        let was_probing = drs.probing();
        let moved = drs
            .record(
                Cost {
                    scalable: gpu,
                    fixed: 0.0,
                    blur: 0.0,
                },
                Some(frame_seconds),
                limits,
            )
            .is_some();
        if drs.probing() && !was_probing {
            probes += 1;
        }
        if frame >= tail {
            sum += f64::from(drs.scale());
            changes += u32::from(moved);
            late += u32::from(frame_seconds > PERIOD * LATE_PERIODS);
        }
    }
    Run {
        mean_scale: (sum / 3600.0) as f32,
        scale: drs.scale(),
        changes,
        late,
        probes,
    }
}

/// The report's shape: a race that fits at full scale on a top clock (12.5 ms of a 13.3 ms
/// budget) settles deep when the clock follows the load, because every scale from about 0.7
/// to 1.0 then reads the same milliseconds. The probe takes it back to where a held clock
/// settles.
#[test]
fn a_clock_that_follows_the_load_is_probed_back_up() {
    let full = 0.012_5;
    let held = run(full, false, 0.70, 4, false);
    let stuck = run(full, true, 0.70, 4, false);
    let probed = run(full, true, 0.70, 4, true);
    // The bug, so this cannot pass for a reason other than the probe.
    assert!(stuck.mean_scale <= 0.85, "no probe: {stuck:?}");
    assert!(
        probed.mean_scale >= held.mean_scale - STEP,
        "probed {probed:?}, held {held:?}"
    );
    assert!(probed.probes > 0);
    assert_eq!(probed.late, 0, "{probed:?}");
    assert!(probed.changes <= 2, "{probed:?}");
}

/// A GPU that holds its clock keeps the scale the normal rise chooses: every probe goes back,
/// and once the backoff has grown it costs at most one reverted pair of changes a minute.
#[test]
fn a_held_clock_keeps_its_settle() {
    for full in [0.014, 0.016, 0.020, 0.026] {
        let probed = run(full, false, 0.70, 10, true);
        let plain = run(full, false, 0.70, 10, false);
        assert_eq!(probed.scale, plain.scale, "at {full}: {probed:?}");
        assert!(
            (probed.mean_scale - plain.mean_scale).abs() < 0.01,
            "at {full}: {probed:?} against {plain:?}"
        );
        assert!(probed.changes <= 2, "at {full}: {probed:?}");
        assert_eq!(probed.late, 0, "at {full}: {probed:?}");
    }
}

/// Settles a controller at 0.80 of a 60 Hz target on a steady in-band cost, fed a wall clock.
fn settled_at_80() -> (Controller, f32) {
    let limits = deck_limits();
    let budget = on_budget(Target::OFFERED[2]);
    let mut drs = Controller::new();
    // A steady clock whose cost is 0.9 of the budget at 0.80: one fall lands there.
    for _ in 0..60 {
        let scale = drs.scale();
        drs.record(
            cost(budget * 0.9 * (scale / 0.80).powi(2)),
            Some(PERIOD),
            limits,
        );
        if drs.scale() < 1.0 {
            break;
        }
    }
    assert_eq!(drs.scale(), 0.80);
    assert!(!drs.probing());
    (drs, budget)
}

/// Feeds `reading` at period-long frames until a probe starts, and returns how many frames
/// that took.
fn until_probe(drs: &mut Controller, reading: f32) -> u32 {
    let limits = deck_limits();
    for frame in 1..=frames(HOLD_SECONDS) << (MAX_BACKOFF + 1) {
        drs.record(cost(reading), Some(PERIOD), limits);
        if drs.probing() {
            return frame;
        }
    }
    panic!("no probe within the longest hold");
}

/// A probe whose cost came in at the old milliseconds (the clock rose) is kept.
#[test]
fn a_probe_the_clock_pays_for_is_kept() {
    let (mut drs, budget) = settled_at_80();
    let reading = budget * 0.9;
    let waited = until_probe(&mut drs, reading);
    assert!(waited >= frames(HOLD_SECONDS), "probed after {waited}");
    assert_eq!(drs.scale(), 0.85);
    for _ in 0..frames(RAMP_SECONDS) + frames(SAMPLE_SECONDS) {
        drs.record(cost(reading), Some(PERIOD), deck_limits());
    }
    assert!(!drs.probing());
    assert_eq!(drs.scale(), 0.85, "the step was not kept");
}

/// A probe whose cost grew with its pixels (the clock held) goes back, and the next hold is
/// twice as long.
#[test]
fn a_probe_that_only_paid_for_pixels_backs_off() {
    let (mut drs, budget) = settled_at_80();
    let reading = budget * 0.85;
    let first = until_probe(&mut drs, reading);
    let grown = reading * (0.85f32 / 0.80).powi(2);
    for _ in 0..frames(RAMP_SECONDS) + frames(SAMPLE_SECONDS) {
        drs.record(cost(grown), Some(PERIOD), deck_limits());
    }
    assert!(!drs.probing());
    assert_eq!(
        drs.scale(),
        0.80,
        "a step the clock did not pay for was kept"
    );
    let second = until_probe(&mut drs, reading);
    assert!(
        second >= 2 * frames(HOLD_SECONDS) && second > first,
        "the hold did not double: {first} then {second}"
    );
}

/// A late frame on the wall clock reverts a probe at once, ramp or not, once the step's own
/// cooldown has passed.
#[test]
fn a_late_frame_ends_a_probe_at_once() {
    let (mut drs, budget) = settled_at_80();
    until_probe(&mut drs, budget * 0.9);
    for _ in 0..COOLDOWN {
        drs.record(cost(budget * 0.9), Some(PERIOD), deck_limits());
    }
    let back = drs.record(
        cost(budget * 0.9),
        Some(PERIOD * (LATE_PERIODS + 0.1)),
        deck_limits(),
    );
    assert_eq!(back, Some(deck_limits().pixels(0.80)));
    assert!(!drs.probing());
    assert_eq!(drs.probe_outcome(), Some(Outcome::Late));
}

/// The long frame the step itself costs, inside its cooldown, does not end the probe.
#[test]
fn the_steps_own_long_frame_does_not_end_a_probe() {
    let (mut drs, budget) = settled_at_80();
    until_probe(&mut drs, budget * 0.9);
    for _ in 0..COOLDOWN {
        assert_eq!(
            drs.record(cost(budget * 0.9), Some(PERIOD * 2.0), deck_limits()),
            None
        );
    }
    assert!(drs.probing());
    assert_eq!(drs.scale(), 0.85);
}

/// During the ramp an over-budget reading does not fall: it was taken at the old clock.
#[test]
fn the_ramp_holds_off_a_fall() {
    let (mut drs, budget) = settled_at_80();
    until_probe(&mut drs, budget * 0.9);
    for _ in 0..frames(RAMP_SECONDS) {
        assert_eq!(
            drs.record(cost(budget * 1.1), Some(PERIOD), deck_limits()),
            None
        );
    }
    assert_eq!(drs.scale(), 0.85);
}

/// No wall clock, no probe: a caller with no frame time keeps the loop it had.
#[test]
fn a_frame_with_no_wall_clock_never_probes() {
    let (mut drs, budget) = settled_at_80();
    for _ in 0..frames(HOLD_SECONDS) * 4 {
        drs.record(cost(budget * 0.9), None, deck_limits());
    }
    assert!(!drs.probing());
    assert_eq!(drs.scale(), 0.80);
}
