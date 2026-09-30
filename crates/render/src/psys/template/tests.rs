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

fn constant(value: f32) -> Channel {
    Channel {
        period: 0.0,
        mode: ChannelMode::Constant,
        lo: 0.0,
        hi: value,
        keys: Vec::new(),
    }
}

/// The `glow`'s roll channel as authored: nine keys over `0..2 pi`.
fn glow_roll() -> Channel {
    Channel {
        period: 0.0,
        mode: ChannelMode::Keyframed,
        lo: 0.0,
        hi: std::f32::consts::TAU,
        keys: vec![
            (0.0, 1.0),
            (0.174, 0.192),
            (0.372, 0.658),
            (0.431, 0.342),
            (0.503, 0.233),
            (0.692, 0.767),
            (0.697, 0.623),
            (0.949, 0.226),
            (1.0, 1.0),
        ],
    }
}

fn rotation(stretch: f32, roll: Channel, flags: u32) -> Rotation {
    Rotation {
        stretch: constant(stretch),
        roll,
        flags,
    }
}

/// `(width, height)` of the quad in the camera's own right and up.
fn extent(quad: &[GpuVertex; 6]) -> (f32, f32) {
    let span = |axis: usize| -> f32 {
        let values = quad.iter().map(|v| v.position[axis]);
        values.clone().fold(f32::MIN, f32::max) - values.fold(f32::MAX, f32::min)
    };
    (span(0), span(1))
}

/// Live: `shazam` drew aspect `1.5` at roll `0` (the `+0xf0` block's `0.5`),
/// `glow` `1.7` (`0.7`); `DrawRotatedSprite` is `aspect * size` across and
/// `size` tall.
#[test]
fn a_stretch_widens_the_quad_by_one_plus_its_value() {
    let particle = Particle::DEAD;
    for (stretch, width) in [(0.5, 1.5), (0.7, 1.7)] {
        let quad = rotation(stretch, constant(0.0), 0).quad(
            &particle,
            0.0,
            4.0,
            Vec3::X,
            Vec3::Y,
            [1.0; 3],
            1.0,
        );
        let (w, h) = extent(&quad);
        assert!((w - 2.0 * 4.0 * width).abs() < 1e-4, "{w}");
        assert!((h - 2.0 * 4.0).abs() < 1e-4, "{h}");
    }
}

/// A quarter turn puts the long axis up, whichever way it turns.
#[test]
fn a_quarter_turn_swaps_the_quads_axes() {
    let mut particle = Particle::DEAD;
    particle.roll = std::f32::consts::FRAC_PI_2;
    let quad =
        rotation(0.7, constant(0.0), 0).quad(&particle, 0.0, 1.0, Vec3::X, Vec3::Y, [1.0; 3], 1.0);
    let (w, h) = extent(&quad);
    assert!(
        (w - 2.0).abs() < 1e-4 && (h - 3.4).abs() < 1e-4,
        "{w} x {h}"
    );
}

/// A stretch at or below zero leaves the aspect `1 / (1 - v)` and grows the
/// size by `1 - v`, so the quad's area is unchanged in one direction.
#[test]
fn a_negative_stretch_grows_the_size_and_narrows_the_aspect() {
    let quad = rotation(-1.0, constant(0.0), 0).quad(
        &Particle::DEAD,
        0.0,
        1.0,
        Vec3::X,
        Vec3::Y,
        [1.0; 3],
        1.0,
    );
    let (w, h) = extent(&quad);
    // size 1 * (1 - -1) = 2 tall, aspect 1/2 wide.
    assert!(
        (h - 4.0).abs() < 1e-4 && (w - 2.0).abs() < 1e-4,
        "{w} x {h}"
    );
}

/// Live, the `glow`'s roll at ticks 0..7 of forty (flags `0x30`): `6.28, 5.60,
/// 4.81, 4.13, 3.35, 2.65, 1.91, 1.24`, on a variable timestep. The authored
/// channel at `tick / 40` gives the angle itself.
#[test]
fn an_absolute_roll_follows_the_authored_channel() {
    let rotation = rotation(0.7, glow_roll(), ABSOLUTE | RANDOM_START);
    let live = [
        std::f32::consts::TAU,
        5.60,
        4.81,
        4.13,
        3.35,
        2.65,
        1.91,
        1.24,
    ];
    for (tick, expected) in live.into_iter().enumerate() {
        let roll = rotation.roll_at(&Particle::DEAD, tick as f32 / 40.0, 1.0);
        assert!(
            (roll - expected).abs() < 0.12,
            "tick {tick}: {roll} vs {expected}"
        );
    }
}

/// Without the absolute flag the channel is a rate and the angle accumulates;
/// a particle the coin turned the other way unwinds it.
#[test]
fn a_rate_roll_accumulates_in_the_particles_sense() {
    let rotation = rotation(0.0, constant(0.5), 0);
    let mut particle = Particle::DEAD;
    particle.roll = 1.0;
    assert!((rotation.roll_at(&particle, 0.3, 1.0) - 1.5).abs() < 1e-6);
    particle.turn = -1.0;
    assert!((rotation.roll_at(&particle, 0.3, 2.0) - 0.0).abs() < 1e-6);
}

/// Only the flags that ask for a draw take one, so a still template leaves
/// the generator alone.
#[test]
fn a_template_draws_from_the_generator_only_when_its_flags_ask() {
    let mut a = Rng::new(7);
    let mut b = Rng::new(7);
    assert_eq!(rotation(0.5, constant(0.0), 0).start(&mut a), (0.0, 1.0));
    assert_eq!(a.next_u32(), b.next_u32());
    let (roll, turn) = rotation(0.5, constant(0.0), RANDOM_START | RANDOM_SENSE).start(&mut a);
    assert!((0.0..std::f32::consts::TAU).contains(&roll));
    assert!(turn == 1.0 || turn == -1.0);
    assert_ne!(a.next_u32(), b.next_u32());
}
