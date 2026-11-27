//! The near law's cull, fade and cap, each on a single stationary billboard at a known depth.

use oag_core::Rng;
use oag_mesh::mesh::GpuVertex;

use super::*;
use crate::psys::tests::{constant, effect, run};
use crate::psys::{Blend, Render, System};

/// The law every wired title runs.
const LAW: NearLaw = NearLaw {
    cull_depth: 1.0,
    fade_depth: 4.0,
    template_cap: None,
};

/// The eye at the origin looking down `-Z`, drawn on the tick.
const EYE: NearEye = NearEye {
    eye: Vec3::ZERO,
    forward: Vec3::NEG_Z,
    draw_alpha: 1.0,
};

const CULL: Cull<'static> = Cull {
    guard: None,
    near: Some(EYE),
};

/// One billboard of half size `size` and alpha 1, under `law` with cap `cap`.
fn billboard_effect(size: f32, cap: Option<f32>, law: Option<NearLaw>) -> Effect {
    let mut built = (*effect("near", true, 100.0)).clone();
    built.emitters[0].size = constant(size);
    built.emitters[0].near_cap = cap;
    built.near = law;
    built
}

/// The additive quads of `built` after one tick of a pool lit `depth` ahead of the eye.
fn quads_at(built: &Effect, depth: f32, cull: Cull<'_>) -> Vec<GpuVertex> {
    let mut system = System::new();
    system.ignite(built, Vec3::new(0.0, 0.0, -depth), 1.0);
    let mut rng = Rng::new(7);
    for _ in 0..1 {
        system.advance(
            built,
            1.0 / 60.0,
            Vec3::new(0.0, 0.0, -depth),
            Vec3::Y,
            &mut rng,
        );
    }
    let (mut additive, mut alpha_over) = (Vec::new(), Vec::new());
    system.extend_vertices(
        &mut additive,
        &mut alpha_over,
        built,
        Vec3::X,
        Vec3::Y,
        cull,
    );
    assert!(alpha_over.is_empty());
    additive
}

/// Half the quad's width, which the square billboard's half size is.
fn half_of(quad: &[GpuVertex]) -> f32 {
    let xs = quad.iter().map(|v| v.position[0]);
    (xs.clone().fold(f32::MIN, f32::max) - xs.fold(f32::MAX, f32::min)) / 2.0
}

#[test]
fn a_billboard_nearer_than_the_cull_depth_draws_nothing() {
    let built = billboard_effect(0.1, None, Some(LAW));
    assert!(quads_at(&built, 0.99, CULL).is_empty());
    assert!(quads_at(&built, -2.0, CULL).is_empty(), "behind the eye");
    assert!(!quads_at(&built, 1.01, CULL).is_empty());
}

#[test]
fn the_alpha_fades_from_the_cull_depth_to_the_fade_depth() {
    let built = billboard_effect(0.1, None, Some(LAW));
    for (depth, want) in [(1.0, 0.0), (2.5, 0.5), (4.0, 1.0), (9.0, 1.0)] {
        let quad = quads_at(&built, depth, CULL);
        assert!(!quad.is_empty(), "depth {depth}");
        assert!(
            (quad[0].colour[3] - want).abs() < 1e-5,
            "depth {depth}: alpha {}, wanted {want}",
            quad[0].colour[3]
        );
    }
}

#[test]
fn a_billboard_is_never_larger_than_its_depth_times_the_cap() {
    let built = billboard_effect(8.0, Some(0.5), Some(LAW));
    // 8 / 10 is past the cap of 0.5: capped to 5.
    assert!((half_of(&quads_at(&built, 10.0, CULL)) - 5.0).abs() < 1e-4);
    // 8 / 20 is under it: its own size.
    assert!((half_of(&quads_at(&built, 20.0, CULL)) - 8.0).abs() < 1e-4);
    // An emitter with no cap keeps its size at any depth.
    let uncapped = billboard_effect(8.0, None, Some(LAW));
    assert!((half_of(&quads_at(&uncapped, 10.0, CULL)) - 8.0).abs() < 1e-4);
}

/// A sprite template has no cap of its own: the law's template cap bounds it, and its spec's
/// `near_cap` is not read.
#[test]
fn a_template_is_capped_by_the_laws_template_cap() {
    let mut built = billboard_effect(
        8.0,
        Some(0.5),
        Some(NearLaw {
            template_cap: Some(2.0),
            ..LAW
        }),
    );
    built.emitters[0].template = true;
    // 8 / 3 is past 2.0: capped to 6, not to the spec's 1.5.
    assert!((half_of(&quads_at(&built, 3.0, CULL)) - 6.0).abs() < 1e-4);
    assert!((half_of(&quads_at(&built, 5.0, CULL)) - 8.0).abs() < 1e-4);
    // Without a template cap the template keeps its size.
    built.near = Some(LAW);
    assert!((half_of(&quads_at(&built, 3.0, CULL)) - 8.0).abs() < 1e-4);
}

#[test]
fn an_effect_without_the_law_and_a_view_without_an_eye_draw_as_authored() {
    let unlawful = billboard_effect(8.0, Some(0.5), None);
    let lawful = billboard_effect(8.0, Some(0.5), Some(LAW));
    for quad in [
        quads_at(&unlawful, 0.5, CULL),
        quads_at(&lawful, 0.5, Cull::NONE),
    ] {
        assert!((half_of(&quad) - 8.0).abs() < 1e-4);
        assert_eq!(quad[0].colour[3], 1.0);
    }
}

/// Past the fade depth and under the cap the law changes no byte.
#[test]
fn a_far_billboard_under_its_cap_is_byte_identical() {
    let built = billboard_effect(2.0, Some(0.5), Some(LAW));
    let with = quads_at(&built, 4.5, CULL);
    let without = quads_at(&built, 4.5, Cull::NONE);
    assert!(!with.is_empty());
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&with),
        bytemuck::cast_slice::<_, u8>(&without)
    );
}

/// The depth is taken where the shader draws the particle, `shift * (draw_alpha - 1)` back
/// along the last tick's move.
#[test]
fn the_depth_is_the_drawn_position_between_two_ticks() {
    let particle = Particle {
        position: Vec3::new(0.0, 0.0, -2.0),
        before: Vec3::new(0.0, 0.0, -0.5),
        ..Particle::DEAD
    };
    let half_way = NearEye {
        draw_alpha: 0.5,
        ..EYE
    };
    assert!((half_way.depth(&particle) - 1.25).abs() < 1e-5);
    assert!((EYE.depth(&particle) - 2.0).abs() < 1e-5);
}

#[test]
fn a_streak_keeps_its_own_size_near_the_eye() {
    let mut built = billboard_effect(8.0, Some(0.5), Some(LAW));
    built.emitters[0].render = Render::Streak { from_spawn: false };
    let with = quads_at(&built, 0.5, CULL);
    let without = quads_at(&built, 0.5, Cull::NONE);
    assert!(!with.is_empty());
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&with),
        bytemuck::cast_slice::<_, u8>(&without)
    );
}

#[test]
fn the_heat_haze_quads_follow_the_law_too() {
    let mut built = billboard_effect(8.0, Some(0.5), Some(LAW));
    built.emitters[0].blend = Blend::Distort;
    built.emitters[0].distort_strength = 1.0;
    built.emitters[0].sheet_rect = Some([0.0, 0.0, 1.0, 1.0]);
    let haze = |depth: f32| {
        let mut system = System::new();
        system.ignite(&built, Vec3::new(0.0, 0.0, -depth), 1.0);
        run(&mut system, &built, 1, &mut Rng::new(7));
        let mut out = Vec::new();
        system.extend_distort_vertices(&mut out, &built, Vec3::X, Vec3::Y, CULL);
        out
    };
    assert!(haze(0.5).is_empty());
}
