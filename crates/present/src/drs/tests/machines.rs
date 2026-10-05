//! Three whole machines, simulated frame by frame against the policy in
//! [`super::super::policy`].
//!
//! Its own file beside the assertions rather than among them, and not only for
//! the 1,000-line rule: everything in [`super`] feeds a sequence somebody
//! chose and reads what came back, and everything here feeds a sequence the
//! controller's own output produced. A closed loop is a different kind of test
//! from an open one - it can settle, hunt, or run away, and none of those is
//! expressible against a fixed list of costs.
//!
//! The machines are models of a report and are labelled as such. See
//! [ADR-0044](../../../../../docs/architecture/adr/0044-the-residual-is-a-learned-upper-bound.md).

use super::*;

/// Where one modelled machine ended up.
struct Settled {
    /// The scale it was holding after four thousand frames.
    scale: f32,
    /// How many times the scale moved over the last thousand of them, which is
    /// what a player sees rather than reads.
    changes: u32,
    /// What it was holding back, in seconds.
    reserved: f32,
}

/// A machine, simulated frame by frame, twice: once fed a wall clock and once
/// not.
///
/// `pace` is the shortest interval the loop may produce - the refresh under
/// vsync, the limiter's period otherwise - so a frame takes `max(pace, work)`
/// and the difference is sleep, exactly as ADR-0040 describes. The scene's
/// cost is `flat + per_pixel * scale^2`, because a scene pass is not purely
/// per-pixel: [`RISE_STEPS`]' own note records a real multiplier of 1.30 at
/// half scale where the quadratic model said 1.21.
///
/// Returns what the controller fed the constant settled on, and what the one
/// fed a wall clock did.
fn settle(target: Target, pace: f32, flat: f32, per_pixel: f32, truth: f32) -> (Settled, Settled) {
    let limits = limits(target);
    let period = target.period().expect("on");
    let run = |learning: bool| {
        let mut drs = Controller::new();
        let mut changes = 0;
        for frame_index in 0..4_000 {
            let scale = drs.scale();
            let scalable = flat + per_pixel * scale * scale;
            let frame = (scalable + truth).max(pace);
            let cost = Cost {
                scalable,
                fixed: 0.0,
            };
            let moved = drs
                .record(cost, learning.then_some(frame), limits)
                .is_some();
            if moved && frame_index >= 3_000 {
                changes += 1;
            }
        }
        Settled {
            scale: drs.scale(),
            changes,
            reserved: drs.residual().seconds(period),
        }
    };
    (run(false), run(true))
}

/// The reported machine keeps more pixels than the constant leaves it, and how
/// many more depends on what is pacing the loop.
///
/// **A model of a report rather than a measurement on the hardware**, and it
/// says so. What is modelled is what the report states: at 90 Hz with no
/// reconstruction the machine holds the rate at full scale, so its whole frame
/// fits inside 11.1 ms, and the controller shrinks it anyway. The 6 ms / 4 ms
/// split inside the scene pass is chosen to put the constant's settling point
/// where the report puts it; the 0.5 ms outside every timer is the number
/// under test.
///
/// The two halves are the same machine under the two pacings a player can be
/// in, and the gap between them is the honest limit of the whole mechanism: a
/// loop sleeping to the *same* rate the controller aims at has thrown the
/// evidence away before the controller sees it, and one sleeping to a faster
/// limiter has not.
#[test]
fn the_reported_machines_shape_keeps_more_pixels_than_the_constant_leaves_it() {
    let target = Target::OFFERED[3];
    let period = target.period().expect("on");
    let truth = 0.000_5;

    // Vsync at the target rate: every comfortable frame reads as a whole
    // period however little of it was work, so the only evidence arrives while
    // the controller is still falling.
    let (constant, learned) = settle(target, period, 0.006, 0.004, truth);
    assert_eq!(constant.scale, 0.80);
    assert_eq!(learned.scale, 0.90, "two grid steps, and 27 % more pixels");
    assert!(
        learned.reserved >= truth,
        "reserved {}, under the truth it cannot pass",
        learned.reserved
    );
    assert!(learned.reserved < RESIDUAL_SHARE * period);

    // A frame limiter above the target - the shipped default is 240 - leaves
    // the work itself on the clock, and the estimate converges on it.
    let (constant, learned) = settle(target, 1.0 / 240.0, 0.006, 0.004, truth);
    assert_eq!(constant.scale, 0.80);
    assert_eq!(learned.scale, 0.95);
    assert!(
        (learned.reserved - truth).abs() < truth * 0.05,
        "reserved {} against a truth of {truth}",
        learned.reserved
    );
}

/// The machine dynamic resolution exists for - one that cannot hold the rate
/// at full scale - and the one where a reserve that is too big costs the most,
/// because it is subtracted from a budget that had nothing to spare.
#[test]
fn a_machine_that_cannot_hold_the_rate_keeps_more_pixels_too() {
    let target = Target::OFFERED[3];
    let truth = 0.000_5;
    // 13 ms of scalable work in an 11.1 ms frame at full scale, holding the
    // rate from about two thirds down.
    let (constant, learned) = settle(target, target.period().expect("on"), 0.005, 0.008, truth);
    assert!(
        learned.scale > constant.scale,
        "learned {} against {} for the constant",
        learned.scale,
        constant.scale
    );
    assert!(learned.reserved >= truth);
}

/// Learning does not make the picture change size more often.
///
/// **The artefact a player actually notices**, and the one `RISE_PATIENCE`
/// exists for: a scale that settles is worth more than a scale that is right
/// on average. A budget that moves under the controller could in principle
/// hunt - the ratio is divided by a number that is itself being corrected -
/// so both modelled machines are asked how many times they moved over their
/// last thousand frames, with the constant's own answer as the bar.
#[test]
fn a_moving_budget_does_not_make_the_scale_hunt() {
    let target = Target::OFFERED[3];
    let period = target.period().expect("on");
    for (pace, flat, per_pixel) in [
        (period, 0.006, 0.004),
        (1.0 / 240.0, 0.006, 0.004),
        (period, 0.005, 0.008),
    ] {
        let (constant, learned) = settle(target, pace, flat, per_pixel, 0.000_5);
        assert_eq!(
            constant.changes, 0,
            "the constant itself did not settle; the comparison means nothing"
        );
        assert_eq!(
            learned.changes, 0,
            "the scale was still moving a thousand frames in"
        );
    }
}
