//! The frame scale and the run law - `System::set_frame_scale`, `EmitterSpec::short_run`.
//!
//! The numbers they pin are read off the running original (`WO_SHIP_EXPLOSION`,
//! `docs/ghidra/functions/psp-pulse-usa/particle-system.md`, 2026-10-01).

use super::tests::{DT, effect, run};
use super::*;

/// An effect whose one emitter throws a particle a tick straight up at `speed` units a tick,
/// each living a long time.
fn rising(speed: f32, short_run: bool) -> std::sync::Arc<Effect> {
    let mut built = effect("rising", false, 3.0);
    let spec = std::sync::Arc::get_mut(&mut built)
        .unwrap()
        .emitters
        .first_mut()
        .unwrap();
    spec.speed_per_tick = (speed, 0.0);
    spec.lifetime_ticks = (200.0, 0.0);
    spec.short_run = short_run;
    built
}

fn first_height(system: &System) -> f32 {
    system
        .particles
        .iter()
        .filter(|p| p.alive())
        .map(|p| p.position.y)
        .fold(f32::MIN, f32::max)
}

#[test]
fn a_frame_scale_scales_the_ejection_velocity_and_not_the_size() {
    let effect = rising(1.0, false);
    let mut rng = Rng::new(5);
    let (mut plain, mut scaled) = (System::new(), System::new());
    plain.ignite(&effect, Vec3::ZERO, 1.0);
    scaled.ignite(&effect, Vec3::ZERO, 1.0);
    scaled.set_frame_scale(0.75);
    run(&mut plain, &effect, 10, &mut rng);
    let mut rng = Rng::new(5);
    run(&mut scaled, &effect, 10, &mut rng);
    let ratio = first_height(&scaled) / first_height(&plain);
    assert!((ratio - 0.75).abs() < 1e-4, "{ratio}");
    let width = |system: &System| {
        let (additive, _) = system.vertices(&effect, Vec3::X, Vec3::Y);
        let v = &additive[..6];
        (Vec3::from(v[1].position) - Vec3::from(v[0].position)).length()
    };
    assert_eq!(width(&plain), width(&scaled), "a size is not a position");
}

#[test]
fn a_world_space_emitter_skips_the_matrix() {
    let mut built = rising(1.0, false);
    std::sync::Arc::get_mut(&mut built).unwrap().emitters[0].world_space = true;
    let mut rng = Rng::new(5);
    let (mut plain, mut scaled) = (System::new(), System::new());
    plain.ignite(&built, Vec3::ZERO, 1.0);
    scaled.ignite(&built, Vec3::ZERO, 1.0);
    scaled.set_frame_scale(0.75);
    run(&mut plain, &built, 10, &mut rng);
    let mut rng = Rng::new(5);
    run(&mut scaled, &built, 10, &mut rng);
    assert_eq!(first_height(&plain), first_height(&scaled));
}

#[test]
fn a_child_instance_takes_a_matrix_of_unit_rows() {
    let mut built = effect("parent", false, 1.0);
    {
        let effect = std::sync::Arc::get_mut(&mut built).unwrap();
        let mut child = effect.emitters[0].clone();
        child.name = "child".to_string();
        child.duration_ticks = 3.0;
        child.speed_per_tick = (1.0, 0.0);
        child.lifetime_ticks = (200.0, 0.0);
        child.short_run = false;
        let parent = &mut effect.emitters[0];
        parent.speed_per_tick = (0.0, 0.0);
        parent.lifetime_ticks = (200.0, 0.0);
        parent.short_run = false;
        parent.particle_child = Some(1);
        effect.emitters.push(child);
    }
    let mut rng = Rng::new(9);
    let mut system = System::new();
    system.ignite(&built, Vec3::ZERO, 1.0);
    system.set_frame_scale(0.75);
    run(&mut system, &built, 10, &mut rng);
    let child_height = system
        .particles
        .iter()
        .filter(|p| p.alive() && p.spec == 1)
        .map(|p| p.position.y)
        .fold(f32::MIN, f32::max);
    // The first child particle left at 1.0 a tick, born on the tick after the parent.
    assert!(child_height > 7.5, "{child_height}: the child was scaled");
}

/// Particles alive a long time after a run of `duration` ticks, one thrown per tick.
fn thrown(duration: f32, short_run: bool) -> usize {
    let mut built = effect("run", false, duration);
    {
        let spec = &mut std::sync::Arc::get_mut(&mut built).unwrap().emitters[0];
        spec.lifetime_ticks = (200.0, 0.0);
        spec.short_run = short_run;
    }
    let mut system = System::new();
    system.ignite(&built, Vec3::ZERO, 1.0);
    run(&mut system, &built, 30, &mut Rng::new(1));
    system.alive_count()
}

#[test]
fn a_run_emits_one_tick_fewer_than_its_duration_and_a_one_tick_run_emits_once() {
    assert_eq!(thrown(10.0, true), 9, "smoke: duration 10, 18 of 2 a tick");
    assert_eq!(thrown(4.0, true), 3, "spikes: duration 4, 9 of 3 a tick");
    assert_eq!(thrown(1.0, true), 1, "a burst emits on its first update");
    assert_eq!(
        thrown(10.0, false),
        10,
        "every other source keeps the old run"
    );
}

#[test]
fn a_template_is_drawn_for_one_tick_less_than_its_life() {
    let drawn = |short_run: bool| {
        let mut built = effect("glow", false, 1.0);
        {
            let spec = &mut std::sync::Arc::get_mut(&mut built).unwrap().emitters[0];
            spec.template = true;
            spec.lifetime_ticks = (6.0, 0.0);
            spec.short_run = short_run;
        }
        let mut system = System::new();
        system.ignite(&built, Vec3::ZERO, 1.0);
        let mut rng = Rng::new(1);
        (0..12)
            .filter(|_| {
                system.advance(&built, DT, Vec3::ZERO, Vec3::Y, &mut rng);
                system.alive_count() > 0
            })
            .count()
    };
    assert_eq!(drawn(true), 5, "the glow's draws at ages 0 to 4");
    assert_eq!(drawn(false), 6);
}
