//! The blur ramp ([`super::super::ramp`]) against steady rates, hitches, and
//! closed-loop cost models: strength is not free, so the factor feeds back.
//!
//! The models are labelled as such. The worst slope is a chain costing the
//! whole frame, which the docs' bound on the loop gain rests on; the measured
//! one is 8 ms per unit of factor at 4K on a Raphael iGPU
//! (`docs/rendering/motion-blur-half-rung.md`, "The quarter step").

use super::*;
use std::collections::VecDeque;

const SCALE: f32 = 1.0;

/// A frame time from `seconds` of everything but the chain, a chain of `flat`
/// that does not move with strength, and `slope` per unit of factor.
#[derive(Clone, Copy, Debug)]
struct Frame {
    seconds: f32,
    flat: f32,
    slope: f32,
}

impl Frame {
    fn at(self, factor: f32) -> (f32, f32) {
        let chain = if factor > 0.0 {
            self.flat + self.slope * factor
        } else {
            0.0
        };
        (self.seconds + chain, chain)
    }
}

/// What a run did.
#[derive(Default)]
struct Run {
    flips: u32,
    /// The factor over the last 120 frames, lowest and highest.
    last: (f32, f32),
}

/// `frames` frames of `frame`, each read two frames late as a GPU timer is, and
/// `noise` of the frame number's spread on top.
fn drive(ramp: &mut Ramp, frames: usize, frame: impl Fn(usize) -> Frame, noise: f32) -> Run {
    let mut run = Run {
        last: (1.0, 0.0),
        ..Run::default()
    };
    let mut late = VecDeque::from([(0.0f32, 0.0f32); 2]);
    let mut state = 12345u32;
    for index in 0..frames {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let wobble = (state >> 16) as f32 / 65535.0 * 2.0 - 1.0;
        late.push_back(frame(index).at(ramp.factor()));
        let (seconds, chain) = late.pop_front().expect("two");
        if seconds > 0.0 {
            ramp.chain(chain, SCALE);
            if ramp.frame(Some(seconds + noise * wobble), SCALE) {
                run.flips += 1;
            }
        }
        if index + 120 >= frames {
            run.last = (run.last.0.min(ramp.factor()), run.last.1.max(ramp.factor()));
        }
    }
    run
}

fn steady(seconds: f32) -> Frame {
    Frame {
        seconds,
        flat: 0.0,
        slope: 0.0,
    }
}

/// `ramp` after a settled run of a machine that pays nothing for the chain.
fn settled_at(fps: f32) -> f32 {
    let mut ramp = Ramp::default();
    drive(&mut ramp, 1200, |_| steady(fps.recip()), 0.0);
    ramp.factor()
}

#[test]
fn the_factor_is_one_at_60_and_falls_linearly_to_nothing_at_30() {
    assert_eq!(settled_at(144.0), 1.0);
    assert_eq!(settled_at(60.0), 1.0);
    for (fps, factor) in [
        (55.0, 25.0 / 30.0),
        (50.0, 2.0 / 3.0),
        (45.0, 0.5),
        (40.0, 1.0 / 3.0),
        (35.0, 1.0 / 6.0),
    ] {
        let read = settled_at(fps);
        assert!((read - factor).abs() < 0.002, "{fps} fps read {read}");
    }
    assert_eq!(settled_at(30.0), 0.0);
    assert_eq!(settled_at(20.0), 0.0);
}

#[test]
fn nothing_read_yet_draws_the_players_strength() {
    let mut ramp = Ramp::default();
    assert_eq!(ramp.factor(), 1.0);
    for _ in 0..6 {
        ramp.frame(Some(0.2), SCALE);
    }
    assert_eq!(ramp.factor(), 1.0, "a window of frames has not been read");
}

#[test]
fn a_hitch_of_up_to_three_frames_does_not_move_the_factor_at_all() {
    for fps in [60.0f32, 45.0, 35.0] {
        for length in 1..=3 {
            let mut ramp = Ramp::default();
            drive(&mut ramp, 1200, |_| steady(fps.recip()), 0.0);
            let before = ramp.factor();
            for _ in 0..length {
                ramp.frame(Some(0.25), SCALE);
            }
            for _ in 0..7 {
                ramp.frame(Some(fps.recip()), SCALE);
            }
            assert!(
                (ramp.factor() - before).abs() < 1e-4,
                "{fps} fps, {length} frames: {before} to {}",
                ramp.factor()
            );
        }
    }
}

#[test]
fn a_sustained_fall_does_fade_it() {
    let mut ramp = Ramp::default();
    drive(&mut ramp, 600, |_| steady(1.0 / 60.0), 0.0);
    drive(&mut ramp, 600, |_| steady(1.0 / 40.0), 0.0);
    assert!((ramp.factor() - 1.0 / 3.0).abs() < 0.002);
}

#[test]
fn a_stall_reading_is_no_reading() {
    let mut ramp = Ramp::default();
    drive(&mut ramp, 600, |_| steady(1.0 / 45.0), 0.0);
    let before = ramp.factor();
    ramp.reset();
    assert!(!ramp.frame(None, SCALE));
    assert_eq!(ramp.factor(), before);
}

/// **The feedback proof.** Every rate across the ramp and past both ends, a
/// chain from nothing to the whole frame's worth: the factor comes to rest. A
/// limit cycle would hold the last two seconds' range open.
#[test]
fn a_fixed_cost_model_settles_at_every_rate() {
    for base_ms in (2..=48).map(|ms| ms as f32) {
        for flat_ms in [0.0f32, 1.5, 3.0, 6.0] {
            for slope_ms in [0.0f32, 4.0, 8.0, 16.0, 26.0, 33.0] {
                let frame = Frame {
                    seconds: base_ms / 1000.0,
                    flat: flat_ms / 1000.0,
                    slope: slope_ms / 1000.0,
                };
                let mut ramp = Ramp::default();
                let run = drive(&mut ramp, 3600, |_| frame, 0.0);
                let (low, high) = run.last;
                assert!(
                    high - low < 0.002,
                    "{frame:?} still swings {low} to {high} after a minute"
                );
                // At rest it stays there: no flip in a further minute.
                let more = drive(&mut ramp, 3600, |_| frame, 0.0);
                assert_eq!(more.flips, 0, "{frame:?} still flips");
            }
        }
    }
}

/// The same machines on the rates the frame time really has: a spread of 1 ms
/// either way. The factor wanders by what the noise is worth, not more.
#[test]
fn noise_does_not_grow_into_a_swing() {
    for base_ms in (6..=30).map(|ms| ms as f32) {
        for slope_ms in [4.0f32, 8.0, 16.0] {
            let frame = Frame {
                seconds: base_ms / 1000.0,
                flat: 0.002,
                slope: slope_ms / 1000.0,
            };
            let mut ramp = Ramp::default();
            let run = drive(&mut ramp, 3600, |_| frame, 0.001);
            let (low, high) = run.last;
            if ramp.is_off() {
                continue;
            }
            assert!(high - low < 0.08, "{frame:?} swings {low} to {high}");
        }
    }
}

#[test]
fn the_chain_is_not_drawn_at_zero_and_stays_out_where_the_return_would_not_hold() {
    // 40 fps without the chain, 30 with it: the flip this exists to stop.
    let frame = Frame {
        seconds: 0.025,
        flat: 0.0085,
        slope: 0.004,
    };
    let mut ramp = Ramp::default();
    let run = drive(&mut ramp, 18000, |_| frame, 0.0003);
    assert!(ramp.is_off());
    assert_eq!(ramp.factor(), 0.0);
    assert_eq!(run.flips, 1, "five minutes at the edge");
}

#[test]
fn a_cheap_scene_brings_the_blur_back_after_its_patience_and_from_nothing() {
    let heavy = Frame {
        seconds: 0.040,
        flat: 0.004,
        slope: 0.004,
    };
    let light = Frame {
        seconds: 0.012,
        ..heavy
    };
    let mut ramp = Ramp::default();
    drive(&mut ramp, 1200, |_| heavy, 0.0);
    assert!(ramp.is_off());
    let mut frames = 0;
    let mut late = 0.0;
    while ramp.is_off() {
        ramp.frame(Some(light.seconds), SCALE);
        frames += 1;
        assert!(frames < 2000, "never came back");
        late = ramp.factor();
    }
    assert!(
        frames >= restore_frames(1.0 / TOP_FPS) as usize,
        "back after {frames}"
    );
    assert!(late < 0.1, "came back at {late}, not from about nothing");
    assert!(ramp.factor() < 0.1, "a step of {}", ramp.factor());
}

/// A prediction the scene keeps fooling: the remembered chain is far cheaper
/// than the chain turns out to be, every time. The flips are bounded by the
/// backoff, as the rung's are.
#[test]
fn a_return_that_does_not_hold_backs_off() {
    let mut ramp = Ramp::default();
    let frame = Frame {
        seconds: 0.028,
        flat: 0.010,
        slope: 0.0,
    };
    let mut flips = 0;
    for _ in 0..(120 * 40) {
        // The chain's timer lies low, so every prediction says it fits.
        ramp.chain(0.0001, SCALE);
        ramp.chain(0.0001, SCALE);
        let (seconds, _) = frame.at(ramp.factor());
        if ramp.frame(Some(seconds), SCALE) {
            flips += 1;
        }
    }
    // About 150 seconds. Unbacked, a return every three is 92 flips; backed
    // off, returns wait 3, 6, 12, then 24 seconds.
    assert!(flips <= 16, "{flips} flips");
}

#[test]
fn the_return_prediction_follows_the_extent() {
    // A chain measured at half scale costs four times that at the ceiling.
    let mut ramp = Ramp::default();
    ramp.chain(0.002, 0.5);
    drive_off(&mut ramp);
    for _ in 0..4000 {
        ramp.frame(Some(0.027), 1.0);
    }
    assert!(ramp.is_off(), "8 ms on 27 misses");
    for _ in 0..1000 {
        ramp.frame(Some(0.027), 0.5);
    }
    assert!(!ramp.is_off(), "2 ms on 27 fits");
}

fn drive_off(ramp: &mut Ramp) {
    for _ in 0..600 {
        ramp.frame(Some(0.05), 1.0);
    }
    assert!(ramp.is_off());
}

/// One machine for the controller and the ramp together: a scene and a chain,
/// each scaling with pixels, strength scaling the chain's gather.
fn closed_loop(target: Target, scene_ms: f32, chain_ms: f32) -> (Run, Controller, Ramp, Limits) {
    let limits = Limits::new((3840, 2160), (960, 540), target)
        .with_blur(Some(oag_display::display::BlurResolution::Full));
    let mut drs = Controller::new();
    let mut ramp = Ramp::default();
    let mut run = Run {
        last: (1.0, 0.0),
        ..Run::default()
    };
    let frames = 7200;
    let mut extents = 0;
    let mut late = VecDeque::from([(0.0f32, 0.0f32, 0.0f32); 2]);
    for index in 0..frames {
        let scale = drs.scale();
        let pixels = scale * scale;
        let gather = match drs.blur_resolution(limits) {
            oag_display::display::BlurResolution::Full => 1.0,
            oag_display::display::BlurResolution::Half => 0.45,
            oag_display::display::BlurResolution::Quarter => 0.3,
        };
        let factor = ramp.factor();
        let chain = if factor > 0.0 {
            chain_ms / 1000.0 * pixels * gather * (0.5 + 0.5 * factor)
        } else {
            0.0
        };
        let scene = 0.001 + scene_ms / 1000.0 * pixels + chain;
        late.push_back((scene, chain, scene * 1.1));
        let (scene, chain, seconds) = late.pop_front().expect("two");
        if seconds <= 0.0 {
            continue;
        }
        let cost = Cost {
            scalable: scene,
            fixed: 0.0,
            blur: chain,
        };
        if drs.record(cost, Some(seconds), limits).is_some() {
            extents += 1;
        }
        ramp.chain(chain, drs.scale());
        if ramp.frame(Some(seconds), drs.scale()) {
            run.flips += 1;
            drs.reset();
        }
        if index + 600 >= frames {
            run.last = (run.last.0.min(ramp.factor()), run.last.1.max(ramp.factor()));
        }
    }
    assert!(
        extents <= 40,
        "{scene_ms} + {chain_ms}: {extents} extent steps"
    );
    (run, drs, ramp, limits)
}

/// The two loops together, every race weight, a target at each end of the ramp
/// and one past it: the extent and the factor both come to rest, and the
/// factor's last ten seconds are flat.
#[test]
fn the_ramp_and_the_extent_settle_together() {
    for target in [Target::OFFERED[1], Target::OFFERED[2], Target::OFFERED[3]] {
        for scene_ms in [4.0f32, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0] {
            for chain_ms in [2.0f32, 6.0, 12.0, 20.0, 40.0] {
                let (run, ..) = closed_loop(target, scene_ms, chain_ms);
                let (low, high) = run.last;
                assert!(
                    high - low < 0.02,
                    "{target} {scene_ms}+{chain_ms}: {low} to {high}, {} flips",
                    run.flips
                );
                assert!(
                    run.flips <= 6,
                    "{target} {scene_ms}+{chain_ms}: {} flips",
                    run.flips
                );
            }
        }
    }
}

/// The ramp moving must not hold the rung off: a race that eases climbs the
/// gather back to the player's while the factor is still on its way up.
#[test]
fn the_ramp_does_not_starve_the_rung() {
    let limits = Limits::new((3840, 2160), (960, 540), Target::OFFERED[2])
        .with_blur(Some(oag_display::display::BlurResolution::Full));
    let mut drs = Controller::new();
    let mut ramp = Ramp::default();
    // Heavy, then a scene the player's gather fits again.
    for index in 0..2400 {
        let (scene_ms, chain_ms) = if index < 300 {
            (30.0, 16.0)
        } else {
            (2.0, 4.0)
        };
        let factor = ramp.factor();
        let chain = chain_ms / 1000.0 * (0.5 + 0.5 * factor);
        let scene = scene_ms / 1000.0 + chain;
        let cost = Cost {
            scalable: scene,
            fixed: 0.0,
            blur: chain,
        };
        drs.record(cost, Some(scene), limits);
        ramp.chain(chain, drs.scale());
        ramp.frame(Some(scene), drs.scale());
    }
    assert_eq!(
        drs.blur_resolution(limits),
        oag_display::display::BlurResolution::Full
    );
    assert_eq!(ramp.factor(), 1.0);
}

#[test]
fn a_chain_not_drawn_gives_the_rung_no_reading() {
    let limits = Limits::new((3840, 2160), (960, 540), Target::OFFERED[2])
        .with_blur(Some(oag_display::display::BlurResolution::Full));
    let mut drs = Controller::new();
    let over = on_budget(Target::OFFERED[2]) * 1.2;
    let shed = Cost {
        scalable: over,
        fixed: 0.0,
        blur: over * 0.5,
    };
    drs.record(shed, None, limits);
    assert!(drs.blur_shed());
    let quiet = Cost {
        scalable: over * 0.1,
        fixed: 0.0,
        blur: 0.0,
    };
    for _ in 0..3000 {
        drs.record(quiet, None, limits);
    }
    assert!(drs.blur_shed(), "a restore was judged on a chain not drawn");
}

/// One function shapes the strength, and with a factor of 1 it is the plain
/// `min` a capture draws with, bit for bit.
#[test]
fn the_shutter_reduces_to_the_capture_rule() {
    use oag_display::display::{BlurResolution, MotionBlur};
    let drs = Controller::new();
    for resolution in BlurResolution::ALL {
        let limits = Limits::new((3840, 2160), (960, 540), Target::OFF).with_blur(Some(resolution));
        for strength in MotionBlur::ALL {
            let chosen = strength.shutter();
            assert_eq!(
                drs.blur_shutter(chosen, limits, 1.0).to_bits(),
                chosen.min(resolution.shutter_cap()).to_bits(),
            );
            assert_eq!(
                drs.blur_shutter(chosen, limits, 0.5).to_bits(),
                (chosen.min(resolution.shutter_cap()) * 0.5).to_bits(),
            );
        }
    }
}
