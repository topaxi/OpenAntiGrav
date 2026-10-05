//! The playback rate, the lifetime co-factor, the even ring, the sub-frame spread and
//! the ridden frame, each asserted on the law it plays.

use std::f32::consts::TAU;

use super::super::tests::{DT, constant, effect};
use super::*;
use crate::psys::{Direction, Spawn, TICK_HZ};
use oag_core::Rng;

fn live(system: &System) -> Vec<super::super::particle::Particle> {
    system
        .particles
        .iter()
        .copied()
        .filter(|p| p.alive())
        .collect()
}

/// `+0xc` is the frame's tick count times the rate, and a particle's life runs down
/// by it: ten ticks of life at rate `4` are gone in three frames.
#[test]
fn a_rate_of_four_spends_a_ten_tick_life_in_three_frames() {
    let mut built = (*effect("fast", false, 1.0)).clone();
    built.emitters[0].playback.rate = 4.0;
    let effect = std::sync::Arc::new(built);
    let mut rng = Rng::new(1);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    // The spawn frame draws it at age 0; then 4, 8 ticks spent; the fourth frame kills it.
    for frame in 0..3 {
        system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
        assert_eq!(system.alive_count(), 1, "frame {frame}");
    }
    system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    assert_eq!(system.alive_count(), 0, "rate 4 did not age the particle");
}

/// The emission countdown runs on the rate too: a two-tick interval at rate `2`
/// emits every frame.
#[test]
fn the_rate_drives_the_emission_schedule() {
    let mut built = (*effect("schedule", true, 10.0)).clone();
    built.emitters[0].interval_ticks = (2, 2);
    built.emitters[0].lifetime_ticks = (100.0, 0.0);
    built.emitters[0].playback.rate = 2.0;
    let effect = std::sync::Arc::new(built);
    let mut rng = Rng::new(2);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    for _ in 0..6 {
        system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    }
    assert_eq!(system.alive_count(), 6);
}

/// Selector 5 multiplies a newborn's life: `RandSpread(+0x5c, +0x60) * +0x54 / 60`.
#[test]
fn the_selector_five_record_lengthens_a_newborns_life() {
    let mut built = (*effect("long", false, 1.0)).clone();
    built.emitters[0].playback.lifetime_animation = Some(constant(1.5));
    let effect = std::sync::Arc::new(built);
    let mut rng = Rng::new(3);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    let born = live(&system);
    assert_eq!(born.len(), 1);
    assert!((born[0].max_life - 15.0 / TICK_HZ).abs() < 1e-6, "{born:?}");
}

/// `WO_REPULSER_BLAST`'s record, keyed `1.5 * (1, 0.69, 0.0068)` over the run: its one
/// burst is at age 0, so all fifty beads live `114 * 1.5` ticks.
#[test]
fn the_burst_reads_the_lifetime_record_at_the_emitters_age() {
    let mut built = (*effect("keyed", false, 10.0)).clone();
    built.emitters[0].playback.lifetime_animation = Some(Channel {
        period: 0.0,
        mode: pob::ChannelMode::Keyframed,
        lo: 0.0,
        hi: 1.5,
        keys: vec![(0.0, 1.0), (0.789_743_6, 0.691_780_8), (1.0, 0.006_849_315)],
    });
    let spec = &built.emitters[0];
    assert!((spec.burst(10.0, 1.0).lifetime - 1.5).abs() < 1e-6);
    assert!((spec.burst(0.0, 1.0).lifetime - 0.010_274).abs() < 1e-4);
}

/// Flag `0x200000`: one random start, then `2 pi / count` apart - fifty beads, every
/// neighbour `7.2` degrees on.
#[test]
fn an_even_ring_steps_its_angles_by_a_whole_turn_over_the_burst() {
    let mut built = (*effect("ring", false, 1.0)).clone();
    let spec = &mut built.emitters[0];
    spec.per_emission = (50, 50);
    spec.spawn = Spawn::Ring {
        extent: 10.0,
        spread: 0.0,
        mode: 0,
        arc: std::f32::consts::TAU,
    };
    spec.playback.even_ring = true;
    let effect = std::sync::Arc::new(built);
    let mut rng = Rng::new(4);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    let mut angles: Vec<f32> = live(&system)
        .iter()
        .map(|p| p.position.z.atan2(p.position.x).rem_euclid(TAU))
        .collect();
    assert_eq!(angles.len(), 50);
    angles.sort_by(f32::total_cmp);
    let step = TAU / 50.0;
    for pair in angles.windows(2) {
        assert!((pair[1] - pair[0] - step).abs() < 1e-3, "{angles:?}");
    }
}

/// Shape 8 under flag `0x200000` steps by `pi / count` from a start in `U(0, pi)`
/// (`0x088fcb88`): ten beads, `18` degrees apart, all on the `+Z` half.
#[test]
fn an_even_half_ring_steps_by_half_a_turn_over_the_burst() {
    let mut built = (*effect("half", false, 1.0)).clone();
    let spec = &mut built.emitters[0];
    spec.per_emission = (10, 10);
    spec.spawn = Spawn::Ring {
        extent: 10.0,
        spread: 0.0,
        mode: 0,
        arc: std::f32::consts::PI,
    };
    spec.playback.even_ring = true;
    spec.playback.arc = std::f32::consts::PI;
    let effect = std::sync::Arc::new(built);
    let mut rng = Rng::new(4);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    let mut angles: Vec<f32> = live(&system)
        .iter()
        .map(|p| p.position.z.atan2(p.position.x).rem_euclid(TAU))
        .collect();
    assert_eq!(angles.len(), 10);
    angles.sort_by(f32::total_cmp);
    let step = std::f32::consts::PI / 10.0;
    for pair in angles.windows(2) {
        assert!((pair[1] - pair[0] - step).abs() < 1e-3, "{angles:?}");
    }
}

/// Flag `0x20`: particle `i` of `n` is pulled back by `i / n` of the frame's motion.
#[test]
fn a_subframe_burst_strings_back_along_the_emitters_motion() {
    let mut built = (*effect("spread", true, 10.0)).clone();
    built.emitters[0].per_emission = (4, 4);
    built.emitters[0].playback.subframe_spread = true;
    let effect = std::sync::Arc::new(built);
    let mut rng = Rng::new(5);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    let first: Vec<f32> = live(&system).iter().map(|p| p.position.x).collect();
    assert!(first.iter().all(|x| x.abs() < 1e-6), "{first:?}");
    system.advance(&effect, DT, Vec3::new(8.0, 0.0, 0.0), Vec3::Y, &mut rng);
    let mut second: Vec<f32> = live(&system)
        .iter()
        .filter(|p| p.position.x > 0.5)
        .map(|p| p.position.x)
        .collect();
    second.sort_by(f32::total_cmp);
    assert_eq!(second, [2.0, 4.0, 6.0, 8.0]);
}

/// Flag `0x2` under a frame that moves and turns: the particle keeps its place in
/// the frame, and its velocity turns with it. Off, it stays where it was born.
#[test]
fn a_local_space_particle_rides_a_moving_frame_only_when_asked() {
    let mut built = (*effect("local", false, 1.0)).clone();
    let spec = &mut built.emitters[0];
    spec.world_space = true;
    spec.lifetime_ticks = (100.0, 0.0);
    spec.speed_per_tick = (0.0, 0.0);
    spec.direction = Direction::Cone { half_angle: 0.0 };
    spec.spawn = Spawn::Ring {
        extent: 5.0,
        spread: 0.0,
        mode: 0,
        arc: std::f32::consts::TAU,
    };
    spec.playback.even_ring = true;
    let effect = std::sync::Arc::new(built);
    for rides in [false, true] {
        let mut rng = Rng::new(6);
        let mut system = System::new();
        system.ignite(&effect, Vec3::ZERO, 1.0);
        system.set_rides_frame(rides);
        system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
        let born = live(&system)[0].position;
        // A quarter turn about `Y` and ten units along `Z`.
        system.stretch(1.0, Vec3::Z);
        system.advance(&effect, DT, Vec3::new(0.0, 0.0, 10.0), Vec3::Y, &mut rng);
        let now = live(&system)[0].position;
        if rides {
            let turned = Vec3::new(-born.z, born.y, born.x) + Vec3::new(0.0, 0.0, 10.0);
            assert!(
                (now - turned).length() < 1e-4,
                "{born} -> {now}, want {turned}"
            );
        } else {
            assert!((now - born).length() < 1e-6, "{born} -> {now}");
        }
    }
}
