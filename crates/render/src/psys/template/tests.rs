use super::*;

/// The `glow`'s size channel as authored: six keys, looping every ten ticks.
fn glow_size() -> Channel {
    Channel {
        period: 10.0,
        mode: ChannelMode::Keyframed,
        lo: 0.3,
        hi: 2.0,
        keys: vec![
            (0.0, 0.0068),
            (0.1667, 1.0),
            (0.3154, 1.0),
            (0.4256, 0.0068),
            (0.6615, 0.6301),
            (1.0, 0.0068),
        ],
    }
}

/// Live, on PPSSPP: the `glow` particle's `v` at ticks 0..14 of its forty-tick
/// life ran `0.007, 0.60, 1, 1, 0.22, 0.21, 0.47, 0.55, 0.37, 0.19, 0.02,
/// 0.61, 1, 1, 0.22` - two peaks, ten ticks apart. The unrolled channel over
/// the normalised life must reproduce that.
#[test]
fn a_looping_channel_repeats_over_the_particles_life() {
    let unrolled = unroll(&glow_size(), 40.0);
    assert_eq!(unrolled.period, 0.0);
    let v = |tick: f32| unrolled.value_at(tick / 40.0);
    let live = [
        (0.0, 0.007),
        (1.0, 0.60),
        (2.0, 1.0),
        (4.0, 0.24),
        (5.0, 0.20),
        (6.0, 0.47),
        (10.0, 0.007),
        (11.0, 0.60),
        (12.0, 1.0),
        (14.0, 0.24),
        (30.0, 0.007),
        (32.0, 1.0),
    ];
    for (tick, expected) in live {
        assert!(
            (v(tick) - expected).abs() < 0.05,
            "tick {tick}: {} against {expected}",
            v(tick)
        );
    }
    assert!(unrolled.keys.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(unrolled.keys.last().map(|key| key.0), Some(1.0));
}

/// A channel with no period, or one that is not a keyframed ramp, is left as
/// authored: the unrolling is for the looping ones alone.
#[test]
fn a_channel_without_a_period_is_left_alone() {
    let mut still = glow_size();
    still.period = 0.0;
    assert_eq!(unroll(&still, 40.0), still);
    let mut constant = glow_size();
    constant.mode = ChannelMode::Constant;
    assert_eq!(unroll(&constant, 40.0), constant);
}

/// A life shorter than one period is a stretch of the first cycle, not a
/// whole one squeezed in.
#[test]
fn a_life_shorter_than_the_period_takes_part_of_the_first_cycle() {
    let unrolled = unroll(&glow_size(), 5.0);
    // Half of the authored cycle: the normalised end is the authored 0.5.
    let end = glow_size().value_at(0.5);
    assert!((unrolled.value_at(1.0) - end).abs() < 1.0e-4);
    assert!((unrolled.value_at(0.0) - glow_size().value_at(0.0)).abs() < 1.0e-4);
}
