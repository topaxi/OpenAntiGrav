//! The blur rung ([`super::super::blur`]) against fed sequences and against
//! closed-loop machines.
//!
//! The machines are models, labelled as such: their proportions are taken
//! from the Steam Deck at 3840x2160 (`docs/rendering/motion-blur-half-rung.md`),
//! where the half gather saved about 45 % of the blur chain and a floor of the
//! chain's cost did not scale with the gather at all, scaled onto a 60 fps
//! budget.

use super::*;
use oag_display::display::BlurResolution;

const FULL: Option<BlurResolution> = Some(BlurResolution::Full);
const HALF: Option<BlurResolution> = Some(BlurResolution::Half);

/// The 60 fps limits every test here uses, with a blur ceiling.
fn blur_limits(blur: Option<BlurResolution>) -> Limits {
    limits(Target::OFFERED[2]).with_blur(blur)
}

/// The scalable budget at 60 with no fixed cost and the assumed residual.
fn budget() -> f32 {
    on_budget(Target::OFFERED[2])
}

/// A frame of `scalable` whose blur chain cost `blur` of it.
fn blurred(scalable: f32, blur: f32) -> Cost {
    Cost {
        scalable,
        fixed: 0.0,
        blur,
    }
}

/// A modelled frame: a scene and a blur chain, each part fixed and part
/// per-pixel, the blur's per-pixel part a quarter under the half gather.
#[derive(Clone, Copy)]
struct Machine {
    scene_flat: f32,
    scene_pixels: f32,
    blur_flat: f32,
    blur_pixels: f32,
}

impl Machine {
    /// Cost at `scale` of the ceiling under `resolution`.
    fn cost(self, scale: f32, resolution: BlurResolution) -> Cost {
        let pixels = scale * scale;
        let gather = match resolution {
            BlurResolution::Full => 1.0,
            BlurResolution::Half => 0.25,
        };
        let blur = self.blur_flat + self.blur_pixels * pixels * gather;
        blurred(self.scene_flat + self.scene_pixels * pixels + blur, blur)
    }
}

/// What one closed-loop run did.
#[derive(Default)]
struct Run {
    /// How many times the gather resolution changed.
    flips: u32,
    /// How many times the extent changed.
    steps: u32,
    /// The lowest scale it reached.
    lowest: f32,
    /// Every frame at which the gather went back to full, with the scale then.
    restores: Vec<(usize, f32)>,
    /// Every frame at which the gather went to half, with the scale then.
    sheds: Vec<(usize, f32)>,
}

/// Runs `drs` for `frames` frames, each frame's cost from `machine` at
/// the frame index, the scale and the gather in force.
fn drive(
    drs: &mut Controller,
    frames: usize,
    limits: Limits,
    machine: impl Fn(usize) -> Machine,
) -> Run {
    let mut run = Run {
        lowest: 1.0,
        ..Run::default()
    };
    for frame in 0..frames {
        let before = drs.blur_resolution(limits);
        let cost = machine(frame).cost(drs.scale(), before);
        if drs.record(cost, None, limits).is_some() {
            run.steps += 1;
        }
        let after = drs.blur_resolution(limits);
        if after != before {
            run.flips += 1;
            match after {
                BlurResolution::Half => run.sheds.push((frame, drs.scale())),
                BlurResolution::Full => run.restores.push((frame, drs.scale())),
            }
        }
        run.lowest = run.lowest.min(drs.scale());
    }
    run
}

/// A frame the full gather cannot fit and the half gather can, at full scale:
/// 8 ms of scene and 6 ms of blur against a 13.3 ms budget, 3 ms of that blur
/// a floor the gather does not move.
const DECK_4K: Machine = Machine {
    scene_flat: 0.001,
    scene_pixels: 0.007,
    blur_flat: 0.003,
    blur_pixels: 0.003,
};

#[test]
fn an_overrun_sheds_the_full_gather_before_the_extent_falls() {
    let limits = blur_limits(FULL);
    let mut drs = Controller::new();
    let over = budget() * 1.2;
    assert_eq!(drs.record(blurred(over, over * 0.3), None, limits), None);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Half);
    assert_eq!(drs.scale(), 1.0, "the extent did not move on the shed");
    // The readings in flight still carry the full gather, so they are ignored.
    for _ in 0..COOLDOWN {
        assert_eq!(drs.record(blurred(over, over * 0.3), None, limits), None);
    }
    // Still over budget once they are through: now the extent falls.
    assert!(
        drs.record(blurred(over, over * 0.15), None, limits)
            .is_some()
    );
    assert!(drs.scale() < 1.0);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Half);
}

#[test]
fn the_rung_is_inert_without_a_full_gather_to_shed() {
    let over = budget() * 1.2;
    // Blur off, the player's own half, and no target at all.
    for (limits, expected) in [
        (blur_limits(None), BlurResolution::Full),
        (blur_limits(HALF), BlurResolution::Half),
        (limits(Target::OFF).with_blur(FULL), BlurResolution::Full),
    ] {
        let mut with_blur = Controller::new();
        let mut without = Controller::new();
        for _ in 0..50 {
            let moved = with_blur.record(blurred(over, over * 0.3), None, limits);
            assert_eq!(
                moved,
                without.record(cost(over), None, limits.with_blur(None))
            );
            assert_eq!(with_blur.blur_resolution(limits), expected);
            assert!(!with_blur.blur_shed());
        }
        assert_eq!(with_blur.scale(), without.scale());
    }
}

#[test]
fn a_shed_needs_a_blur_that_was_measured_to_cost_something() {
    let limits = blur_limits(FULL);
    let mut drs = Controller::new();
    let over = budget() * 1.2;
    // No blur reading folded to zero: the extent falls as it always did.
    assert!(drs.record(blurred(over, 0.0), None, limits).is_some());
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Full);
}

#[test]
fn the_player_s_half_is_a_ceiling_the_rung_never_rises_above() {
    let limits = blur_limits(HALF);
    let mut drs = Controller::new();
    let run = drive(&mut drs, 2_000, limits, |frame| Machine {
        scene_flat: if frame % 400 < 200 { 0.012 } else { 0.001 },
        ..DECK_4K
    });
    assert_eq!(run.flips, 0);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Half);
}

#[test]
fn a_machine_the_half_gather_fits_settles_at_full_scale_on_it() {
    let limits = blur_limits(FULL);
    let mut drs = Controller::new();
    let run = drive(&mut drs, 4_000, limits, |_| DECK_4K);
    assert_eq!(run.sheds.len(), 1, "one shed, on the first frame over");
    assert!(run.restores.is_empty(), "the full gather never fits");
    assert_eq!(run.steps, 0, "no pixel was given up");
    assert_eq!(drs.scale(), 1.0);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Half);
}

/// Shed first, restored last: a heavy stretch takes the gather and then the
/// extent, and a light one gives the extent back before the gather.
#[test]
fn the_gather_is_shed_first_and_restored_last() {
    let limits = blur_limits(FULL);
    let mut drs = Controller::new();
    let heavy = Machine {
        scene_flat: 0.006,
        ..DECK_4K
    };
    let light = Machine {
        scene_flat: 0.0,
        scene_pixels: 0.003,
        ..DECK_4K
    };
    let run = drive(&mut drs, 3_000, limits, |frame| {
        if frame < 1_000 { heavy } else { light }
    });
    assert_eq!(run.sheds.first().map(|shed| shed.1), Some(1.0));
    assert!(run.lowest < 1.0, "the heavy stretch took pixels too");
    let (at, scale) = *run.restores.first().expect("the light stretch restores");
    assert!(at > 1_000);
    assert_eq!(scale, 1.0, "restored only once the extent was back");
    assert_eq!(run.flips, 2);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Full);
}

/// The full gather comes back only when it is predicted to land under the
/// deadband, not merely inside it, and only after a run of such frames.
#[test]
fn a_restore_needs_room_for_the_whole_saving_and_a_run_of_frames() {
    let limits = blur_limits(FULL);
    let budget = budget();
    let mut drs = Controller::new();
    let full_blur = budget * 0.3;
    drs.record(blurred(budget * 1.1, full_blur), None, limits);
    assert!(drs.blur_shed());
    for _ in 0..COOLDOWN {
        drs.record(blurred(budget * 0.5, full_blur / 2.0), None, limits);
    }
    let half_blur = full_blur / 2.0;
    // Restored, these would land at 0.81 of the budget: inside the band,
    // above its lower edge. Never restored however long it lasts.
    let marginal = budget * 0.81 - full_blur + half_blur;
    for _ in 0..(RESTORE_PATIENCE << MAX_BACKOFF) * 2 {
        drs.record(blurred(marginal, half_blur), None, limits);
    }
    assert!(drs.blur_shed());
    // At 0.79 it fits: restored after exactly the patience, not before.
    let roomy = budget * 0.79 - full_blur + half_blur;
    for _ in 1..RESTORE_PATIENCE {
        drs.record(blurred(roomy, half_blur), None, limits);
        assert!(drs.blur_shed());
    }
    drs.record(blurred(roomy, half_blur), None, limits);
    assert!(!drs.blur_shed());
}

/// A restore whose remembered cost was stale sheds again, and the next
/// restore has to wait twice as long - so a scene that keeps fooling the
/// prediction costs a bounded, shrinking number of flips.
#[test]
fn a_restore_that_does_not_hold_backs_off() {
    let limits = blur_limits(FULL);
    let budget = budget();
    let mut drs = Controller::new();
    // The machine flips between two scenes the prediction cannot tell apart:
    // a cheap blur when the shed is measured, and a dear one by the time the
    // restore lands. Shed at a full gather of 0.2 budget, restored predicting
    // it, and paying 0.5.
    let cheap_shed = blurred(budget * 1.1, budget * 0.2);
    let half = budget * 0.1;
    let room = blurred(budget * 0.5, half);
    let dear_full = blurred(budget * 1.1, budget * 0.5);
    let mut waits = Vec::new();
    for _ in 0..4 {
        drs.record(cheap_shed, None, limits);
        assert!(drs.blur_shed());
        let mut frames = 0;
        while drs.blur_shed() {
            drs.record(room, None, limits);
            frames += 1;
            assert!(frames < 10_000, "never restored");
        }
        waits.push(frames);
        // The restore's cooldown ignores these; the cheap shed that follows
        // is the relapse, and it remembers the cheap cost again, so the next
        // prediction is as stale as this one was.
        for _ in 0..COOLDOWN {
            drs.record(dear_full, None, limits);
        }
    }
    for pair in waits.windows(2) {
        assert!(
            pair[1] >= 2 * pair[0] - COOLDOWN,
            "patience did not double: {waits:?}"
        );
    }
}

/// A restore that holds for the relapse window pays its backoff back.
#[test]
fn a_restore_that_holds_forgets_its_backoff() {
    let limits = blur_limits(FULL);
    let budget = budget();
    let mut drs = Controller::new();
    let shed = blurred(budget * 1.1, budget * 0.2);
    let room = blurred(budget * 0.5, budget * 0.1);
    let wait = |drs: &mut Controller| {
        // Past the cooldown a restore starts, so the shed is a decision.
        for _ in 0..COOLDOWN {
            drs.record(room, None, limits);
        }
        drs.record(shed, None, limits);
        assert!(drs.blur_shed());
        let mut frames = 0;
        while drs.blur_shed() {
            drs.record(room, None, limits);
            frames += 1;
        }
        frames
    };
    let first = wait(&mut drs);
    // Relapse at once: the next restore waits twice as long.
    let second = wait(&mut drs);
    assert!(second > first);
    // Hold for the whole window, then relapse: back to the first wait.
    for _ in 0..RELAPSE_WINDOW {
        drs.record(room, None, limits);
    }
    assert_eq!(wait(&mut drs), first);
}

#[test]
fn turning_the_blur_off_and_on_starts_the_rung_clean() {
    let mut drs = Controller::new();
    let over = budget() * 1.2;
    drs.record(blurred(over, over * 0.3), None, blur_limits(FULL));
    assert!(drs.blur_shed());
    drs.record(blurred(over, 0.0), None, blur_limits(None));
    assert!(!drs.blur_shed());
    assert_eq!(drs.blur_resolution(blur_limits(FULL)), BlurResolution::Full);
}

/// Noise on every reading does not make the gather flicker: the closed loop
/// of [`DECK_4K`] with each frame's cost moved by up to 10 % either way.
#[test]
fn noisy_readings_do_not_make_the_gather_flicker() {
    let limits = blur_limits(FULL);
    for seed in 1..=8u32 {
        let mut drs = Controller::new();
        let mut state = seed.wrapping_mul(2_654_435_761);
        let mut noise = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            0.9 + 0.2 * (state as f32 / u32::MAX as f32)
        };
        let mut flips = 0;
        for _ in 0..6_000 {
            let resolution = drs.blur_resolution(limits);
            let clean = DECK_4K.cost(drs.scale(), resolution);
            let k = noise();
            drs.record(blurred(clean.scalable * k, clean.blur * k), None, limits);
            if drs.blur_resolution(limits) != resolution {
                flips += 1;
            }
        }
        assert_eq!(flips, 1, "seed {seed}: the gather flickered");
    }
}

/// The full gather a restore would bring back is predicted from the half
/// gather measured now, at the saving the shed measured, rather than
/// remembered from the moment of the shed: a scene whose blur has grown since
/// does not restore on a cost that is no longer true.
#[test]
fn a_restore_predicts_the_full_gather_from_the_half_one_measured_now() {
    let limits = blur_limits(FULL);
    let budget = budget();
    let mut drs = Controller::new();
    // Shed on a cheap blur (0.1 budget), then read its half at 0.05: the
    // machine's saving is a half.
    drs.record(blurred(budget * 1.1, budget * 0.1), None, limits);
    for _ in 0..COOLDOWN {
        drs.record(blurred(budget * 0.5, budget * 0.05), None, limits);
    }
    drs.record(blurred(budget * 0.5, budget * 0.05), None, limits);
    // Now the blur grows: a half gather of 0.2 stands for a full one of 0.4,
    // so 0.65 restored would be 0.85 of the budget. Never restored.
    for _ in 0..(RESTORE_PATIENCE << MAX_BACKOFF) * 2 {
        drs.record(blurred(budget * 0.65, budget * 0.2), None, limits);
    }
    assert!(drs.blur_shed(), "restored on a remembered cost");
    // The same frame with the remembered estimate would have restored: 0.65 -
    // 0.2 + 0.1 is 0.55 of the budget. A little less scene and the predicted
    // 0.75 fits.
    for _ in 0..RESTORE_PATIENCE {
        drs.record(blurred(budget * 0.55, budget * 0.2), None, limits);
    }
    assert!(!drs.blur_shed());
}
