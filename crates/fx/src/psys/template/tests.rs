use super::*;
use crate::psys::{Direction, MAX_PARTICLES, Particle};
use oag_core::Rng;

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

/// `WO_BLUE_WELDER`'s `GLOW` as authored: a twenty-tick loop whose duplicate
/// key times are jumps - four flashes a cycle, each a jump to full size and a
/// fall. Read off `WO_BLUE_WELDER.POB`; the original's frames show the halo
/// re-flashing every few frames on Basilico Black
/// (`docs/ghidra/functions/psp-pulse-usa/placed-particle-systems.md`).
fn welder_glow_size() -> Channel {
    Channel {
        period: 20.0,
        mode: ChannelMode::Keyframed,
        lo: 0.0,
        hi: 30.0,
        keys: vec![
            (0.0, 1.0),
            (0.107692, 0.0068),
            (0.107692, 1.0),
            (0.305128, 0.0068),
            (0.484615, 0.0068),
            (0.484615, 1.0),
            (0.710256, 0.0068),
            (0.866667, 0.0068),
            (0.866667, 1.0),
            (1.0, 0.0137),
        ],
    }
}

#[test]
fn a_jump_inside_a_cycle_survives_the_unrolling() {
    let life = 200.0;
    let unrolled = unroll(&welder_glow_size(), life);
    let v = |tick: f32| unrolled.value_at(tick / life);
    // Just after each authored jump, in the first cycle and in the sixth.
    for cycle in [0.0, 5.0] {
        for jump in [0.107692, 0.484615, 0.866667] {
            let tick = (cycle + jump) * 20.0 + 0.01;
            assert!(v(tick) > 0.95, "cycle {cycle} jump {jump}: {}", v(tick));
            let before = (cycle + jump) * 20.0 - 0.01;
            assert!(
                v(before) < 0.05,
                "cycle {cycle} before {jump}: {}",
                v(before)
            );
        }
        // And the cycle's own start, which follows the previous cycle's end.
        let start = cycle * 20.0 + 0.01;
        assert!(v(start) > 0.95, "cycle {cycle} start: {}", v(start));
    }
}

/// `crate::psys::tests::effect`'s emitter, its particles thrown at a random speed and
/// direction so they draw from the stream, `per_tick` of them a tick for 30 ticks,
/// optionally with one sprite template beside it as a second root.
fn spray(per_tick: u32, with_template: bool) -> Effect {
    let mut built = (*crate::psys::tests::effect("spray", false, 30.0)).clone();
    let emitter = &mut built.emitters[0];
    emitter.per_emission = (per_tick, per_tick);
    emitter.lifetime_ticks = (100.0, 20.0);
    emitter.speed_per_tick = (1.0, 0.5);
    emitter.direction = Direction::Cone { half_angle: 1.0 };
    emitter.live_cap = 0;
    if with_template {
        let mut template = built.emitters[0].clone();
        template.name = "glow".to_string();
        template.template = true;
        template.duration_ticks = 1.0;
        template.per_emission = (1, 1);
        template.lifetime_ticks = (30.0, 0.0);
        built.roots.push(built.emitters.len());
        built.emitters.push(template);
    }
    built
}

fn after_ticks(effect: &Effect, ticks: usize) -> System {
    let mut system = System::new();
    system.ignite(effect, Vec3::ZERO, 1.0);
    crate::psys::tests::run(&mut system, effect, ticks, &mut Rng::new(5));
    system
}

/// A template draws from a copy of the shared stream: with it playing, every emitted particle
/// is where it would be without it.
#[test]
fn a_template_leaves_the_rest_of_its_effect_unchanged() {
    let without = after_ticks(&spray(3, false), 10);
    let with = after_ticks(&spray(3, true), 10);
    let emitted = |system: &System| -> Vec<Particle> {
        system.particles[..MAX_PARTICLES]
            .iter()
            .filter(|p| p.alive())
            .copied()
            .collect()
    };
    assert!(!emitted(&without).is_empty());
    assert_eq!(emitted(&with), emitted(&without));
    assert_eq!(with.alive_count(), without.alive_count() + 1);
}

/// A template takes none of the emitted pool's slots: in a full pool every emitted particle
/// still lives beside it.
#[test]
fn a_template_lives_beside_a_full_pool() {
    let system = after_ticks(&spray(40, true), 10);
    let emitted = system.particles[..MAX_PARTICLES]
        .iter()
        .filter(|p| p.alive())
        .count();
    assert_eq!(emitted, MAX_PARTICLES);
    assert_eq!(system.alive_count(), MAX_PARTICLES + 1);
}
