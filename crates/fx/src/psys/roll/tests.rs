use super::*;

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
        law: Law::Template {
            stretch: constant(stretch),
        },
        roll,
        flags,
    }
}

fn emitter(aspect: f32, roll: Channel, flags: u32) -> Rotation {
    Rotation::emitter(aspect, roll, flags)
}

fn started(rotation: &Rotation, rng: &mut Rng) -> Particle {
    let mut particle = Particle::DEAD;
    rotation.start(&mut particle, rng);
    particle
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
    let still = started(&rotation(0.5, constant(0.0), 0), &mut a);
    assert_eq!((still.roll, still.turn), (0.0, 1.0));
    assert_eq!(a.next_u32(), b.next_u32());
    let drawn = started(
        &rotation(0.5, constant(0.0), RANDOM_START | RANDOM_SENSE),
        &mut a,
    );
    assert!((0.0..std::f32::consts::TAU).contains(&drawn.roll));
    assert!(drawn.turn == 1.0 || drawn.turn == -1.0);
    assert_ne!(a.next_u32(), b.next_u32());
}

/// `ParticleSystem_DrawRolledQuads`: half-height `size`, half-width `aspect * size`, at roll 0
/// an axis-aligned rectangle. The corpus authors aspect `4` on the Shuriken's
/// two emitters and nothing else.
#[test]
fn an_emitter_quad_is_as_wide_as_its_aspect_says() {
    let quad = emitter(4.0, constant(0.0), 0).quad(
        &Particle::DEAD,
        0.0,
        1.5,
        Vec3::X,
        Vec3::Y,
        [1.0; 3],
        1.0,
    );
    let (w, h) = extent(&quad);
    assert!(
        (w - 12.0).abs() < 1e-4 && (h - 3.0).abs() < 1e-4,
        "{w} x {h}"
    );
}

/// The batched draw turns the *unit* square and scales the screen's x by the
/// aspect afterwards, so its edges are `(w cos, -h sin)` and `(w sin, h cos)`:
/// a parallelogram, not the template's rectangle, once the aspect is not 1.
#[test]
fn an_emitter_quad_scales_the_turned_square_not_the_turn() {
    let mut particle = Particle::DEAD;
    particle.roll = 0.5;
    let quad =
        emitter(4.0, constant(0.0), 0).quad(&particle, 0.0, 1.0, Vec3::X, Vec3::Y, [1.0; 3], 1.0);
    let at = |i: usize| Vec3::from_array(quad[i].position);
    // `[bl, br, tl, br, tr, tl]`: bl = -a - b, br = a - b, tl = b - a.
    let a = (at(1) - at(0)) * 0.5;
    let b = (at(2) - at(0)) * 0.5;
    let (sin, cos) = 0.5_f32.sin_cos();
    assert!((a - Vec3::new(4.0 * cos, -sin, 0.0)).length() < 1e-5, "{a}");
    assert!((b - Vec3::new(4.0 * sin, cos, 0.0)).length() < 1e-5, "{b}");
    assert!(a.dot(b).abs() > 0.5, "not a rectangle");
    // The template's quad, same aspect and roll, is one.
    let template =
        rotation(3.0, constant(0.0), 0).quad(&particle, 0.0, 1.0, Vec3::X, Vec3::Y, [1.0; 3], 1.0);
    let edge = |i: usize, j: usize| {
        Vec3::from_array(template[i].position) - Vec3::from_array(template[j].position)
    };
    assert!(edge(1, 0).dot(edge(2, 0)).abs() < 1e-4);
}

/// An unrotated, unstretched emitter quad is the plain billboard the port drew
/// before this law was played.
#[test]
fn a_unit_emitter_at_roll_zero_is_the_plain_billboard() {
    let particle = Particle::DEAD;
    let rotated =
        emitter(1.0, constant(0.0), 0).quad(&particle, 0.0, 2.0, Vec3::X, Vec3::Y, [1.0; 3], 1.0);
    let plain = quad(
        particle.position,
        Vec3::X * 2.0,
        Vec3::Y * 2.0,
        0.5,
        [1.0; 3],
        1.0,
    );
    for (a, b) in rotated.iter().zip(plain.iter()) {
        assert_eq!(a.position, b.position);
    }
}

/// A constant or random channel is a rate that gains `rate * dt` a tick; the
/// coin negates it on the particles it flips.
#[test]
fn an_emitter_roll_is_a_rate_the_coin_negates() {
    let rotation = emitter(1.0, constant(0.1), 0);
    let mut particle = started(&rotation, &mut Rng::new(1));
    assert_eq!((particle.roll, particle.turn), (0.0, 1.0));
    particle.roll = rotation.advance(&particle, 0.0, 0.1, 2.0);
    assert!((particle.roll - 0.2).abs() < 1e-6);

    let coin = emitter(1.0, constant(0.1), flags::RANDOM_ROTATION_SIGN);
    let turns: Vec<f32> = (0..32)
        .map(|seed| started(&coin, &mut Rng::new(seed)).turn)
        .collect();
    assert!(turns.contains(&1.0) && turns.contains(&-1.0), "{turns:?}");
}

/// A keyframed channel is read at the age the tick *began* with, and is
/// subtracted on a particle the coin left alone, added on one it flipped:
/// the other way round from the constant and random channels.
#[test]
fn a_keyframed_emitter_roll_turns_against_the_other_channels() {
    let ramp = Channel {
        period: 0.0,
        mode: ChannelMode::Keyframed,
        lo: 0.0,
        hi: 1.0,
        keys: vec![(0.0, 0.0), (1.0, 1.0)],
    };
    let plain = emitter(1.0, ramp.clone(), 0);
    let mut particle = started(&plain, &mut Rng::new(1));
    assert_eq!(particle.turn, -1.0);
    // Age 0.5 at the start of the tick: the ramp reads 0.5, not its end.
    particle.roll = plain.advance(&particle, 0.5, 0.9, 1.0);
    assert!((particle.roll + 0.5).abs() < 1e-6, "{}", particle.roll);

    let coin = emitter(1.0, ramp, flags::RANDOM_ROTATION_SIGN);
    let turns: Vec<f32> = (0..32)
        .map(|seed| started(&coin, &mut Rng::new(seed)).turn)
        .collect();
    assert!(turns.contains(&1.0) && turns.contains(&-1.0), "{turns:?}");
}

/// Flag `0x4` starts the roll at `U(-pi, pi)` - the template's flag `0x10` is
/// `U(0, 2 pi)` - and a random channel draws its sample once, at spawn.
#[test]
fn an_emitter_starts_at_a_random_roll_and_draws_a_random_rate_once() {
    let random = Channel {
        period: 0.0,
        mode: ChannelMode::Random,
        lo: 0.0,
        hi: 0.1,
        keys: Vec::new(),
    };
    let rotation = emitter(1.0, random, flags::RANDOM_ROLL);
    let rolls: Vec<f32> = (0..32)
        .map(|seed| started(&rotation, &mut Rng::new(seed)).roll)
        .collect();
    let pi = std::f32::consts::PI;
    assert!(rolls.iter().all(|r| (-pi..pi).contains(r)));
    assert!(rolls.iter().any(|&r| r < 0.0) && rolls.iter().any(|&r| r > 0.0));
    let particle = started(&rotation, &mut Rng::new(9));
    let first = rotation.advance(&particle, 0.1, 0.2, 1.0) - particle.roll;
    let second = rotation.advance(&particle, 0.7, 0.8, 1.0) - particle.roll;
    assert!(
        (first - second).abs() < 1e-7 && first > 0.0,
        "{first} {second}"
    );

    // A constant channel with no flag draws nothing from the generator.
    let (mut a, mut b) = (Rng::new(4), Rng::new(4));
    started(&emitter(1.0, constant(0.1), 0), &mut a);
    assert_eq!(a.next_u32(), b.next_u32());
}
