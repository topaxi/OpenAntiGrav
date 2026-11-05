//! The blur rung ([`super::super::blur`]) against fed sequences and against
//! closed-loop machines.
//!
//! The machines are models, labelled as such: their proportions are taken
//! from the Steam Deck at 3840x2160 (`docs/rendering/motion-blur-half-rung.md`),
//! where the half gather saved about 45 % of the blur chain and a floor of the
//! chain's cost did not scale with the gather at all, scaled onto a 60 fps
//! budget.

use super::*;
use oag_display::display::{BlurResolution, MotionBlur};

const FULL: Option<BlurResolution> = Some(BlurResolution::Full);
const HALF: Option<BlurResolution> = Some(BlurResolution::Half);

/// The 60 fps limits every test here uses, with a blur ceiling.
fn blur_limits(blur: Option<BlurResolution>) -> Limits {
    limits(Target::OFFERED[2]).with_blur(blur)
}

/// The 60 fps target's period.
fn period() -> f32 {
    Target::OFFERED[2].period().expect("on")
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
            BlurResolution::Quarter => 0.0625,
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

/// `resolution`'s place in the order the rung sheds.
fn rank(resolution: BlurResolution) -> usize {
    BlurResolution::ALL
        .iter()
        .position(|&step| step == resolution)
        .expect("listed")
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
            if rank(after) > rank(before) {
                run.sheds.push((frame, drs.scale()));
            } else {
                run.restores.push((frame, drs.scale()));
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
    for _ in 0..relapse_frames(period()) * 2 {
        drs.record(blurred(marginal, half_blur), None, limits);
    }
    assert!(drs.blur_shed());
    // At 0.79 it fits: restored after exactly the patience, not before.
    let roomy = budget * 0.79 - full_blur + half_blur;
    for _ in 1..restore_frames(period()) {
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
    for _ in 0..relapse_frames(period()) {
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
    // The target turned off and on again does the same.
    for _ in 0..=COOLDOWN {
        drs.record(blurred(over, over * 0.3), None, blur_limits(FULL));
    }
    assert!(drs.blur_shed());
    drs.record(
        blurred(over, over * 0.3),
        None,
        limits(Target::OFF).with_blur(FULL),
    );
    assert!(!drs.blur_shed());
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
    for _ in 0..relapse_frames(period()) * 2 {
        drs.record(blurred(budget * 0.65, budget * 0.2), None, limits);
    }
    assert!(drs.blur_shed(), "restored on a remembered cost");
    // The same frame with the remembered estimate would have restored: 0.65 -
    // 0.2 + 0.1 is 0.55 of the budget. A little less scene and the predicted
    // 0.75 fits.
    for _ in 0..restore_frames(period()) {
        drs.record(blurred(budget * 0.55, budget * 0.2), None, limits);
    }
    assert!(!drs.blur_shed());
}

/// A machine whose restored frame fits on the median and spikes past the
/// shed line now and then: the shape a Raphael iGPU at 1280x800 aiming at 90
/// showed, which flipped the gather 21 times in 90 seconds while every
/// restore had been predicted to fit for the frames it was given.
///
/// The scene spikes 40 % every 400 frames: the full gather lands at 0.77 of
/// the budget and spikes to 0.96, past the shed line; the half one never
/// does. **A restore has to wait out the scene's spikes, not a few
/// frames between them**, so this sheds, may try once or twice more, and then
/// holds the half gather.
#[test]
fn a_restore_waits_out_the_scene_s_spikes() {
    let limits = blur_limits(FULL);
    let budget = budget();
    let mut drs = Controller::new();
    let mut flips = 0;
    for frame in 0..6_000 {
        let spike = if frame % 400 == 399 { 1.4 } else { 1.0 };
        let resolution = drs.blur_resolution(limits);
        let blur = match resolution {
            BlurResolution::Full => 0.3,
            BlurResolution::Half => 0.15,
            BlurResolution::Quarter => 0.1,
        };
        let cost = blurred((0.47 * spike + blur) * budget, blur * budget);
        drs.record(cost, None, limits);
        if drs.blur_resolution(limits) != resolution {
            flips += 1;
        }
    }
    assert!(flips <= 5, "{flips} flips in 6,000 frames");
    assert!(drs.blur_shed());
}

const QUARTER: Option<BlurResolution> = Some(BlurResolution::Quarter);

/// The 60 fps limits at 3840x2160, the first ceiling quarter is drawn at.
fn uhd_limits(blur: Option<BlurResolution>) -> Limits {
    Limits::new((3840, 2160), (960, 540), Target::OFFERED[2]).with_blur(blur)
}

/// How many frames the strength cap takes across its whole range at 60.
fn ease_frames() -> usize {
    (CAP_EASE_SECONDS / period()).ceil() as usize
}

/// A race the half gather cannot fit at 3840x2160 and the quarter gather can.
const UHD: Machine = Machine {
    scene_flat: 0.001,
    scene_pixels: 0.008,
    blur_flat: 0.003,
    blur_pixels: 0.012,
};

/// A race not even the quarter gather fits at the ceiling: the extent falls
/// with quarter still drawn.
const UHD_HEAVY: Machine = Machine {
    scene_flat: 0.001,
    scene_pixels: 0.012,
    blur_flat: 0.002,
    blur_pixels: 0.006,
};

#[test]
fn a_2160_line_ceiling_sheds_both_steps_before_the_extent_falls() {
    let limits = uhd_limits(FULL);
    let mut drs = Controller::new();
    let run = drive(&mut drs, 200, limits, |_| UHD);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Quarter);
    assert_eq!(run.sheds.len(), 2);
    assert_eq!(run.steps, 0, "the extent moved before the gather ran out");
    assert_eq!(drs.scale(), 1.0);
}

#[test]
fn under_2160_lines_the_ladder_ends_at_half_and_the_extent_falls_instead() {
    let limits = blur_limits(FULL);
    let mut drs = Controller::new();
    let run = drive(&mut drs, 200, limits, |_| UHD);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Half);
    assert_eq!(run.sheds.len(), 1);
    assert!(drs.scale() < 1.0);
}

#[test]
fn a_half_ceiling_sheds_only_the_second_step() {
    let limits = uhd_limits(HALF);
    let mut drs = Controller::new();
    let run = drive(&mut drs, 200, limits, |_| UHD);
    assert_eq!(run.sheds.len(), 1);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Quarter);

    let small = blur_limits(HALF);
    let mut drs = Controller::new();
    let run = drive(&mut drs, 200, small, |_| UHD);
    assert_eq!(run.flips, 0, "there is nothing to shed under 2160 lines");
    assert_eq!(drs.blur_resolution(small), BlurResolution::Half);
}

#[test]
fn a_quarter_ceiling_is_inert_and_under_2160_lines_it_draws_half() {
    let limits = uhd_limits(QUARTER);
    let mut drs = Controller::new();
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Quarter);
    let run = drive(&mut drs, 200, limits, |_| UHD);
    assert_eq!(run.flips, 0);
    assert!(!drs.blur_shed());
    assert_eq!(drs.blur_shutter_cap(limits), None);

    let small = blur_limits(QUARTER);
    assert_eq!(
        Controller::new().blur_resolution(small),
        BlurResolution::Half
    );
}

#[test]
fn the_steps_come_back_one_at_a_time_and_only_at_the_ceiling() {
    let limits = uhd_limits(FULL);
    let mut drs = Controller::new();
    let quiet = Machine {
        scene_flat: 0.001,
        scene_pixels: 0.001,
        blur_flat: 0.0005,
        blur_pixels: 0.001,
    };
    // Heavy long enough for both sheds, quiet from then on.
    let run = drive(&mut drs, 1500, limits, |frame| {
        if frame < 200 { UHD } else { quiet }
    });
    assert_eq!(run.sheds.len(), 2);
    assert_eq!(run.restores.len(), 2);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Full);
    let gap = run.restores[1].0 - run.restores[0].0;
    assert!(
        gap >= restore_frames(period()) as usize,
        "the second step did not wait out its own patience: {gap}"
    );
    assert!(run.restores.iter().all(|&(_, scale)| scale == 1.0));
}

#[test]
fn a_shed_straight_after_a_shed_still_learns_the_step_above() {
    let limits = uhd_limits(FULL);
    let mut drs = Controller::new();
    let over = budget() * 1.2;
    // Full gather, 6 of blur: shed. The next reading past the cooldown is the
    // half gather's 3 and over budget, so it sheds again on that one reading.
    assert!(
        drs.record(blurred(over, over * 0.5), None, limits)
            .is_none()
    );
    for _ in 0..COOLDOWN {
        drs.record(blurred(over, over * 0.25), None, limits);
    }
    drs.record(blurred(over, over * 0.25), None, limits);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Quarter);
    // Settle the cap, then let a cheap frame restore quarter to half; were the
    // first step's ratio unread the half gather would predict the shed's cost.
    let quiet = budget() * 0.3;
    let mut frames = 0;
    while drs.blur_resolution(limits) == BlurResolution::Quarter && frames < 2000 {
        drs.record(blurred(quiet, quiet * 0.1), None, limits);
        frames += 1;
    }
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Half);
    assert!(
        drs.record(blurred(quiet, quiet * 0.2), None, limits)
            .is_none()
    );
}

#[test]
fn the_cap_eases_in_and_out_in_no_single_visible_step() {
    let limits = uhd_limits(FULL);
    let mut drs = Controller::new();
    assert_eq!(
        drs.blur_shutter_cap(limits),
        Some(MotionBlur::High.shutter())
    );
    drive(&mut drs, 40, limits, |_| UHD);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Quarter);

    let widest = (MotionBlur::High.shutter() - MotionBlur::Low.shutter()) / ease_frames() as f32;
    let mut drs = Controller::new();
    let mut last = drs.blur_shutter_cap(limits).expect("running");
    let mut frames = Vec::new();
    for frame in 0..300 {
        let cost = UHD.cost(drs.scale(), drs.blur_resolution(limits));
        drs.record(cost, None, limits);
        let cap = drs.blur_shutter_cap(limits).expect("running");
        assert!(cap <= last, "the cap rose under a held quarter gather");
        assert!(last - cap <= widest * 1.01, "a step of {}", last - cap);
        if cap != last {
            frames.push(frame);
        }
        last = cap;
    }
    assert_eq!(last, MotionBlur::Low.shutter());
    assert_eq!(frames.len(), ease_frames());
}

#[test]
fn the_cap_is_uncapped_until_a_quarter_gather_is_drawn() {
    let limits = blur_limits(FULL);
    let mut drs = Controller::new();
    drive(&mut drs, 200, limits, |_| UHD);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Half);
    assert_eq!(
        drs.blur_shutter_cap(limits),
        Some(MotionBlur::High.shutter())
    );
}

#[test]
fn no_restore_is_read_off_a_gather_the_cap_is_still_moving_under() {
    let limits = uhd_limits(HALF);
    let mut drs = Controller::new();
    let over = budget() * 1.2;
    drs.record(blurred(over, over * 0.5), None, limits);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Quarter);
    // A reading cheap enough to restore on from the very first frame: the
    // patience counts from when the cap settles, not from the shed.
    let quiet = budget() * 0.1;
    let mut frames = 0;
    while drs.blur_shed() {
        drs.record(blurred(quiet, quiet * 0.2), None, limits);
        frames += 1;
    }
    assert!(
        frames >= ease_frames() + restore_frames(period()) as usize - 1,
        "restored after {frames} frames"
    );
}

/// Quarter stays drawn while the extent falls beneath 2160 lines: the shed
/// exists to come before pixels, so a fall never draws a dearer gather.
#[test]
fn quarter_stays_drawn_as_the_extent_falls_and_settles() {
    let limits = uhd_limits(FULL);
    let mut drs = Controller::new();
    let run = drive(&mut drs, 3600, limits, |_| UHD_HEAVY);
    assert!(drs.scale() < 1.0);
    assert!(drs.extent(limits).1 / 4 < oag_display::display::QUARTER_MIN_LINES);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Quarter);
    assert_eq!(
        run.sheds.len(),
        2,
        "the gather came back as the extent fell"
    );
    assert_eq!(run.flips, 2, "{} gather flips", run.flips);
    assert!(run.steps <= 4, "{} extent steps", run.steps);
    let settled = drive(&mut drs, 1800, limits, |_| UHD_HEAVY);
    assert_eq!((settled.flips, settled.steps), (0, 0));
    // Nothing was written anywhere: the player's choice is still in force the
    // moment the blur rung is told to forget.
    assert_eq!(limits.blur(), FULL);
}

#[test]
fn a_ceiling_that_can_no_longer_draw_quarter_drops_the_step() {
    let limits = uhd_limits(FULL);
    let mut drs = Controller::new();
    drive(&mut drs, 40, limits, |_| UHD);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Quarter);
    // The window shrinks: the ladder ends at half now, for good.
    let small = blur_limits(FULL);
    drs.record(blurred(0.001, 0.0001), None, small);
    assert_eq!(drs.blur_resolution(small), BlurResolution::Half);
    drs.record(blurred(0.001, 0.0001), None, limits);
    assert_eq!(drs.blur_resolution(limits), BlurResolution::Half);
}

#[test]
fn a_changed_ceiling_starts_the_rung_over() {
    let limits = uhd_limits(FULL);
    let mut drs = Controller::new();
    drive(&mut drs, 40, limits, |_| UHD);
    assert!(drs.blur_shed());
    let half = uhd_limits(HALF);
    drs.record(blurred(0.001, 0.0001), None, half);
    assert!(!drs.blur_shed(), "a depth was reread against a new ceiling");
    assert_eq!(drs.blur_resolution(half), BlurResolution::Half);
}

/// Every race weight between "half fits" and "nothing fits", with the scene
/// swinging every few seconds, the shape that made the first rung flip 21
/// times in 90 seconds: no weight may go round.
#[test]
fn no_race_weight_at_a_2160_line_ceiling_goes_round() {
    let limits = uhd_limits(FULL);
    for weight in 0..40 {
        for swing in [0.0f32, 0.1, 0.25] {
            let machine = move |frame: usize| {
                let pulse = if (frame / 150).is_multiple_of(2) {
                    1.0
                } else {
                    1.0 - swing
                };
                Machine {
                    scene_flat: 0.001,
                    scene_pixels: (0.004 + 0.0003 * weight as f32) * pulse,
                    blur_flat: 0.0025,
                    blur_pixels: 0.008 * pulse,
                }
            };
            let mut drs = Controller::new();
            let run = drive(&mut drs, 5400, limits, machine);
            assert!(
                run.flips <= 4 && run.steps <= 4,
                "weight {weight} swing {swing}: {} flips, {} steps",
                run.flips,
                run.steps
            );
        }
    }
}
