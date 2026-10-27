//! A particle drawn between two ticks sweeps its own last tick's move.

use oag_core::Rng;

use super::super::tests::{DT, effect};
use super::super::*;
use super::*;

/// The centre of each live particle's quad as the shader places it at
/// `draw_alpha`: a quad's six vertices average to its centre.
fn centres(system: &System, effect: &Effect, draw_alpha: f32) -> Vec<Vec3> {
    let (additive, _) = system.vertices(effect, Vec3::X, Vec3::Y);
    additive
        .chunks(6)
        .map(|quad| quad.iter().map(|v| drawn_at(v, draw_alpha)).sum::<Vec3>() / 6.0)
        .collect()
}

fn close(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-4
}

/// One particle, fired up at `speed` units a tick.
fn one_shot(speed: f32) -> std::sync::Arc<Effect> {
    let mut built = (*effect("shot", false, 1.0)).clone();
    built.emitters[0].speed_per_tick = (speed, 0.0);
    std::sync::Arc::new(built)
}

#[test]
fn a_moving_particle_draws_between_its_last_two_ticks() {
    let effect = one_shot(2.0);
    let (mut system, mut rng) = (System::new(), Rng::new(1));
    system.ignite(&effect, Vec3::ZERO, 1.0);
    system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    let previous = centres(&system, &effect, 1.0);
    assert_eq!(previous.len(), 1);
    system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    let current = centres(&system, &effect, 1.0)[0];
    assert!(!close(previous[0], current), "the particle did not move");

    // At 0 it is where the last tick left it, at 1 where this one did, and
    // between them on the line, evenly.
    assert!(close(centres(&system, &effect, 0.0)[0], previous[0]));
    for alpha in [0.25f32, 0.5, 0.75] {
        let expected = previous[0] + (current - previous[0]) * alpha;
        assert!(
            close(centres(&system, &effect, alpha)[0], expected),
            "{alpha}"
        );
    }
}

/// At a tick exactly the vertex is the CPU's, bit for bit: what keeps every
/// capture byte-identical.
#[test]
fn at_alpha_one_a_vertex_is_where_the_cpu_put_it() {
    let effect = one_shot(2.0);
    let (mut system, mut rng) = (System::new(), Rng::new(1));
    system.ignite(&effect, Vec3::ZERO, 1.0);
    for _ in 0..3 {
        system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    }
    let (additive, _) = system.vertices(&effect, Vec3::X, Vec3::Y);
    assert!(!additive.is_empty());
    for vertex in &additive {
        assert_ne!(shift_of(vertex), Vec3::ZERO);
        assert_eq!(drawn_at(vertex, 1.0).to_array(), vertex.position);
    }
}

/// A particle born this tick has no previous position: it draws at its spawn
/// point at every alpha.
#[test]
fn a_particle_born_this_tick_holds_its_spawn_point() {
    let effect = one_shot(2.0);
    let (mut system, mut rng) = (System::new(), Rng::new(1));
    system.ignite(&effect, Vec3::new(3.0, 4.0, 5.0), 1.0);
    system.advance(&effect, DT, Vec3::new(3.0, 4.0, 5.0), Vec3::Y, &mut rng);
    let (additive, _) = system.vertices(&effect, Vec3::X, Vec3::Y);
    assert_eq!(additive.len(), 6);
    assert!(additive.iter().all(|v| shift_of(v) == Vec3::ZERO));
}

/// A pool nobody advanced since the last tick path started draws still: no
/// stale move replayed by every frame's alpha.
#[test]
fn a_pool_held_still_draws_no_move() {
    let effect = one_shot(2.0);
    let (mut system, mut rng) = (System::new(), Rng::new(1));
    system.ignite(&effect, Vec3::ZERO, 1.0);
    for _ in 0..3 {
        system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    }
    system.hold_still();
    let (additive, _) = system.vertices(&effect, Vec3::X, Vec3::Y);
    assert!(additive.iter().all(|v| shift_of(v) == Vec3::ZERO));
}

#[test]
fn a_move_past_the_snap_distance_draws_where_it_lands() {
    let mut particle = Particle {
        position: Vec3::new(SNAP_DISTANCE + 1.0, 0.0, 0.0),
        life: 1.0,
        max_life: 1.0,
        ..Particle::DEAD
    };
    assert_eq!(particle.shift(), Vec3::ZERO);
    particle.before = Vec3::new(2.0, 0.0, 0.0);
    assert_eq!(particle.shift(), Vec3::new(SNAP_DISTANCE - 1.0, 0.0, 0.0));
}

/// A template rides its instance's anchor, so between ticks it follows the
/// anchor's move rather than sliding off it.
#[test]
fn a_riding_template_sweeps_its_anchors_move() {
    let mut built = (*effect("head", true, 10.0)).clone();
    let mut template = built.emitters[0].clone();
    template.template = true;
    template.looping = false;
    template.duration_ticks = 1.0;
    template.lifetime_ticks = (3600.0, 0.0);
    built.roots.push(built.emitters.len());
    built.emitters.push(template);
    let effect = std::sync::Arc::new(built);
    let (mut stage, mut rng) = (Stage::new(), Rng::new(1));
    let playing = stage.attach(&effect, Vec3::ZERO, 1.0).expect("attach");
    for tick in 1..=4 {
        stage.follow(playing, Vec3::new(tick as f32 * 10.0, 0.0, 0.0));
        stage.advance(DT, &mut rng);
    }
    let template = stage.instances[usize::from(playing.index)]
        .system
        .particles
        .iter()
        .find(|p| p.alive() && effect.emitters[usize::from(p.spec)].template)
        .copied()
        .expect("the template");
    assert_eq!(template.position, Vec3::new(40.0, 0.0, 0.0));
    assert_eq!(template.shift(), Vec3::new(10.0, 0.0, 0.0));
}

/// A `from_spawn` streak's tail is its spawn point and stays put; its head
/// sweeps. An ordinary streak moves whole.
#[test]
fn a_from_spawn_streaks_tail_stays_put() {
    let mut spec = effect("streak", false, 10.0).emitters[0].clone();
    let particle = Particle {
        position: Vec3::new(10.0, 0.0, 0.0),
        before: Vec3::new(8.0, 0.0, 0.0),
        origin: Vec3::ZERO,
        life: 1.0,
        max_life: 1.0,
        ..Particle::DEAD
    };
    let shifts = |spec: &EmitterSpec| {
        let mut out = Vec::new();
        streak::extend(&mut out, spec, &particle, 0.5, [1.0; 4], Vec3::Y, Vec3::Z);
        out.iter()
            .map(|v| (Vec3::from(v.position).x, shift_of(v).x))
            .collect::<Vec<_>>()
    };
    spec.render = Render::Streak { from_spawn: false };
    assert!(shifts(&spec).iter().all(|&(_, shift)| shift == 2.0));
    spec.render = Render::Streak { from_spawn: true };
    for (x, shift) in shifts(&spec) {
        if x <= 0.0 {
            assert_eq!(shift, 0.0, "the tail moved");
        } else if x >= 10.0 {
            assert_eq!(shift, 2.0, "the head stayed");
        }
    }
}
