use oag_title::width_throttle::Law;

use super::*;
use crate::drs::{Controller, Cost, Limits, Target};

/// The law as `docs/ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md` reads it.
const HD: Law = Law {
    max_level: 31,
    width_notches: 96,
    narrow_above_us: 16_111,
    widen_up_to_us: 15_555,
    frame_hz: 60,
};

const UHD: (u32, u32) = (1920, 1080);

fn cost(seconds: f32) -> Cost {
    Cost {
        scalable: seconds,
        fixed: 0.0,
        blur: 0.0,
    }
}

/// Seconds that truncate to `micros` microseconds.
fn micros(micros: u32) -> f32 {
    (micros as f32 + 0.5) * 1e-6
}

/// A throttle at `level`, reached by feeding it long frames.
fn at_level(level: u32, spec: Spec) -> Throttle {
    let mut throttle = Throttle::default();
    for _ in 0..level {
        throttle.offer(1.0);
        throttle.step(None, spec);
    }
    assert_eq!(throttle.level(), level);
    throttle
}

#[test]
fn the_width_is_one_less_a_ninety_sixth_a_level_down_to_1300_of_1920() {
    let spec = Spec::new(HD, Some(60));
    for level in 0..=31 {
        assert_eq!(spec.width(1920, level), 1920 - 20 * level, "level {level}");
        assert_eq!(spec.width(3840, level), 3840 - 40 * level, "level {level}");
    }
    assert_eq!(spec.narrowest(1920), 1300);
    // Past the last level there is no narrower width.
    assert_eq!(spec.width(1920, 90), 1300);
}

#[test]
fn the_thresholds_are_the_executables_own_microseconds_at_60_hz() {
    let spec = Spec::new(HD, Some(60));
    for (gpu, expected) in [
        (16_112, 3 + 1),
        (16_111, 3),
        (15_556, 3),
        (15_555, 3 - 1),
        (100, 3 - 1),
    ] {
        let mut throttle = at_level(3, spec);
        throttle.offer(micros(gpu));
        throttle.step(Some(1.0 / 60.0), spec);
        assert_eq!(throttle.level(), expected, "{gpu} us");
    }
}

#[test]
fn above_60_hz_the_thresholds_scale_with_the_frame_time() {
    let spec = Spec::new(HD, Some(120));
    assert_eq!(spec.threshold_hz(), 120);
    // 16111 / 2 = 8055.5 and 15555 / 2 = 7777.5.
    for (gpu, expected) in [(8_056, 4), (8_055, 3), (7_778, 3), (7_777, 2)] {
        let mut throttle = at_level(3, spec);
        throttle.offer(micros(gpu));
        throttle.step(Some(1.0 / 120.0), spec);
        assert_eq!(throttle.level(), expected, "{gpu} us at 120 Hz");
    }
}

#[test]
fn below_60_hz_the_thresholds_stay_the_originals_and_the_vblank_is_the_paced_one() {
    let spec = Spec::new(HD, Some(30));
    assert_eq!(spec.threshold_hz(), 60);
    let mut throttle = at_level(3, spec);
    // A frame on time at 30 Hz is not a missed vblank, so a quick GPU frame widens.
    throttle.offer(micros(5_000));
    throttle.step(Some(1.0 / 30.0), spec);
    assert_eq!(throttle.level(), 2);
}

#[test]
fn the_target_the_limits_carry_is_the_paced_frame() {
    let at_30 = Limits::original(UHD, Spec::new(HD, Some(30)));
    assert_eq!(at_30.target().hz(), Some(30));
    let at_120 = Limits::original(UHD, Spec::new(HD, Some(120)));
    assert_eq!(at_120.target().hz(), Some(120));
}

#[test]
fn with_no_rate_known_the_laws_own_rate_stands() {
    assert_eq!(Spec::new(HD, None), Spec::new(HD, Some(60)));
}

#[test]
fn a_missed_vblank_narrows_however_quick_the_gpu_frame_was() {
    let spec = Spec::new(HD, Some(60));
    let mut throttle = at_level(3, spec);
    throttle.offer(micros(1_000));
    throttle.step(Some(2.0 / 60.0), spec);
    assert_eq!(throttle.level(), 4);
    // A frame a little long is not two vblanks.
    throttle.offer(micros(1_000));
    throttle.step(Some(1.3 / 60.0), spec);
    assert_eq!(throttle.level(), 3);
}

#[test]
fn the_level_stops_at_both_ends() {
    let spec = Spec::new(HD, Some(60));
    let mut throttle = Throttle::default();
    for _ in 0..80 {
        throttle.offer(1.0);
        throttle.step(None, spec);
    }
    assert_eq!(throttle.level(), 31);
    for _ in 0..80 {
        throttle.offer(micros(100));
        throttle.step(None, spec);
    }
    assert_eq!(throttle.level(), 0);
}

#[test]
fn one_frame_moves_one_level_however_many_readings_it_drained() {
    let limits = Limits::original(UHD, Spec::new(HD, Some(60)));
    let mut drs = Controller::new();
    // A backlog of three long frames drained in one, the way `PassTimer::drain` hands them.
    for _ in 0..3 {
        assert_eq!(drs.record(cost(0.030), None, limits), None);
    }
    assert_eq!(
        drs.finish_frame(Some(1.0 / 60.0), limits),
        Some((1900, 1080))
    );
    assert_eq!(drs.width_level(), 1);
    // And a frame with no reading holds.
    assert_eq!(drs.finish_frame(Some(1.0), limits), None);
    assert_eq!(drs.width_level(), 1);
}

#[test]
fn the_height_never_moves_and_the_players_floor_does_not_apply() {
    let limits = Limits::original(UHD, Spec::new(HD, Some(60)));
    assert_eq!(limits.floor(), (1300, 1080));
    assert_eq!(limits.target(), Target::OFFERED[2]);
    let mut drs = Controller::new();
    for frame in 0..40 {
        drs.record(cost(0.5), Some(0.5), limits);
        drs.finish_frame(Some(0.5), limits);
        let (width, height) = drs.extent(limits);
        assert_eq!(height, 1080, "frame {frame}");
        assert_eq!(width, 1920 - 20 * (frame + 1).min(31));
    }
    // A load or a changed row puts the race back at full width.
    drs.reset();
    assert_eq!(drs.extent(limits), UHD);
}

#[test]
fn a_cost_the_budget_loop_would_answer_leaves_its_scale_alone() {
    let limits = Limits::original(UHD, Spec::new(HD, Some(60)));
    let mut drs = Controller::new();
    for _ in 0..100 {
        drs.record(cost(0.5), Some(0.5), limits);
        drs.finish_frame(Some(0.5), limits);
    }
    assert_eq!(drs.scale(), 1.0);
    assert!(!drs.unreachable());
    // And the budget loop has nothing to finish.
    let budget = Limits::new(UHD, (960, 540), Target::OFFERED[2]);
    assert_eq!(drs.finish_frame(Some(0.5), budget), None);
}

#[test]
fn the_squeeze_is_the_share_of_the_width_drawn_and_one_under_the_budget_loop() {
    let limits = Limits::original(UHD, Spec::new(HD, Some(60)));
    assert_eq!(limits.squeeze((1920, 1080)), 1.0);
    assert_eq!(limits.squeeze((1300, 1080)), 1300.0 / 1920.0);
    let budget = Limits::new(UHD, (960, 540), Target::OFFERED[2]);
    assert_eq!(budget.squeeze((960, 540)), 1.0);
}

#[test]
fn the_original_reads_and_writes_as_the_word_and_is_no_rate() {
    let target: Target = "original".parse().unwrap();
    assert_eq!(target, Target::ORIGINAL);
    assert_eq!(target.to_string(), "original");
    assert!(target.is_original() && !target.is_on() && !target.follows_display());
    assert_eq!(target.resolve(Some(144)), Target::ORIGINAL);
}
