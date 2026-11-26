use super::super::tests::{constant, effect};
use super::*;

fn camera() -> GuardBand {
    GuardBand {
        eye: Vec3::ZERO,
        right: Vec3::X,
        up: Vec3::Y,
        forward: Vec3::NEG_Z,
    }
}

fn eye() -> Option<NearEye> {
    Some(NearEye {
        eye: Vec3::ZERO,
        forward: Vec3::NEG_Z,
        draw_alpha: 1.0,
    })
}

/// The two measured frames: `shazzam`'s corners at depth 11.8 fall out of
/// range, the same quad at depth 71.2 does not.
#[test]
fn the_measured_shazzam_quad_is_dropped_near_and_kept_far() {
    let quad = |depth: f32, x0: f32, x1: f32, y0: f32, y1: f32| {
        [
            Vec3::new(x0, y0, -depth),
            Vec3::new(x1, y0, -depth),
            Vec3::new(x1, y1, -depth),
            Vec3::new(x0, y1, -depth),
        ]
    };
    assert!(camera().drops(quad(11.8, -109.7, 130.3, -22.9, 17.1)));
    assert!(!camera().drops(quad(71.2, -103.3, 136.7, -16.8, 23.2)));
}

/// The replayed bar's bracket: a vertex 2040 px off centre draws, one 2060 px off does not,
/// across and down.
#[test]
fn the_limit_is_2048_pixels_across_and_down() {
    let depth = 50.0;
    let across = |px: f32| Vec3::new(px * depth / PX_PER_DEPTH[0], 0.0, -depth);
    let down = |px: f32| Vec3::new(0.0, -px * depth / PX_PER_DEPTH[1], -depth);
    assert!(!camera().drops([across(2040.0), down(2040.0), across(-2040.0)]));
    assert!(camera().drops([across(2060.0)]));
    assert!(camera().drops([down(2060.0)]));
    assert!(camera().drops([across(-2060.0)]));
}

#[test]
fn a_vertex_behind_or_inside_the_near_plane_is_not_judged() {
    assert!(!camera().drops([Vec3::new(5000.0, 0.0, 5.0)]));
    assert!(!camera().drops([Vec3::new(5000.0, 0.0, -1.2)]));
    assert!(camera().drops([Vec3::new(5000.0, 0.0, -1.3)]));
}

/// Six vertices as [`super::super::quad`] orders them, `x` per corner, at depth 10.
fn quad_at(x: [f32; 4]) -> Vec<GpuVertex> {
    let corner = |at: usize, y: f32| GpuVertex {
        position: [x[at], y, -10.0],
        ..bytemuck::Zeroable::zeroed()
    };
    let [bl, br, tl, tr] = [
        corner(0, -1.0),
        corner(1, -1.0),
        corner(2, 1.0),
        corner(3, 1.0),
    ];
    vec![bl, br, tl, br, tr, tl]
}

fn dropped(effect: &Effect, template: bool, mut out: Vec<GpuVertex>) -> Vec<GpuVertex> {
    let mut spec = effect.emitters[0].clone();
    spec.template = template;
    drop_triangles(effect, &spec, eye(), (Vec3::X, Vec3::Y), &mut out, 0);
    out
}

fn clipped(name: &str) -> Effect {
    let mut effect = (*effect(name, false, 10.0)).clone();
    effect.with_psp_clip();
    effect
}

fn far_out(vertices: &[GpuVertex]) -> bool {
    camera().drops(vertices.iter().map(|v| Vec3::from(v.position)))
}

fn area(triangle: &[GpuVertex]) -> f32 {
    let [a, b, c] = [0, 1, 2].map(|at| Vec3::from(triangle[at].position));
    (b - a).cross(c - a).length()
}

/// One vertex out takes only the triangle that uses it: the bottom left is in the first
/// triangle alone, the top right in the second alone. The shared diagonal takes both.
#[test]
fn only_the_triangle_with_a_vertex_out_is_dropped() {
    let effect = clipped("WO_ANY");
    let far = 1000.0;
    for (x, degenerate) in [
        ([-far, 1.0, -1.0, 1.0], [true, false]),
        ([-1.0, 1.0, -1.0, far], [false, true]),
    ] {
        let out = dropped(&effect, false, quad_at(x));
        assert_eq!(out.len(), 6, "{x:?}");
        assert!(!far_out(&out), "{x:?}");
        assert_eq!(
            [area(&out[..3]) == 0.0, area(&out[3..]) == 0.0],
            degenerate,
            "{x:?}"
        );
    }
    for x in [[-1.0, far, -1.0, 1.0], [-1.0, 1.0, -far, 1.0]] {
        assert!(dropped(&effect, false, quad_at(x)).is_empty(), "{x:?}");
    }
    let inside = [-1.0, 1.0, -1.0, 1.0];
    let positions = |out: Vec<GpuVertex>| out.iter().map(|v| v.position).collect::<Vec<_>>();
    assert_eq!(
        positions(dropped(&effect, false, quad_at(inside))),
        positions(quad_at(inside))
    );
}

/// A source without the law keeps every triangle, and so does a template that is not the
/// Repulser's.
#[test]
fn the_drop_needs_the_law_and_a_template_needs_the_measured_effect() {
    let quad = || quad_at([-1000.0, 1000.0, -1000.0, 1000.0]);
    let unclipped = (*effect("WO_ANY", false, 10.0)).clone();
    assert_eq!(dropped(&unclipped, false, quad()).len(), 6);
    assert!(dropped(&clipped("WO_ANY"), false, quad()).is_empty());
    assert_eq!(dropped(&clipped("WO_QUAKE"), true, quad()).len(), 6);
    assert!(dropped(&clipped("WO_REPULSER"), true, quad()).is_empty());
}

/// A particle `depth` ahead of the eye, whose other end is `far_end` ahead.
fn at_depth(depth: f32, far_end: f32) -> Particle {
    let position = Vec3::new(0.0, 0.0, -depth);
    Particle {
        position,
        before: position,
        origin: Vec3::new(0.0, 0.0, -far_end),
        life: 1.0,
        max_life: 1.0,
        ..Particle::DEAD
    }
}

fn alpha_of(class: DrawClass, template: bool, depth: f32, far_end: f32) -> Option<f32> {
    let effect = clipped("WO_ANY");
    let mut spec = effect.emitters[0].clone();
    spec.draw_class = class;
    spec.template = template;
    spec.size = constant(1.0);
    depth_law(&effect, &spec, &at_depth(depth, far_end), eye(), 1.0)
}

#[test]
fn the_near_tests_go_by_draw_class() {
    use DrawClass::{Bar, Rolled, Square};
    // The near plane takes every screen-parallel quad.
    for class in [Square, Rolled] {
        assert_eq!(alpha_of(class, false, 1.1, 1.1), None, "{class:?}");
    }
    // A square fades to 2 units; a rolled quad and a template do not.
    assert_eq!(alpha_of(Square, false, 1.5, 1.5), Some(0.5));
    assert_eq!(alpha_of(Square, false, 2.0, 2.0), Some(1.0));
    assert_eq!(alpha_of(Rolled, false, 1.5, 1.5), Some(1.0));
    assert_eq!(alpha_of(Square, true, 1.5, 1.5), Some(1.0));
    // A bar goes when either end is nearer than 1 unit, or their mean is inside the near
    // plane.
    assert_eq!(alpha_of(Bar, false, 0.9, 40.0), None);
    assert_eq!(alpha_of(Bar, false, 40.0, 0.9), None);
    assert_eq!(alpha_of(Bar, false, 1.1, 1.2), None);
    assert_eq!(alpha_of(Bar, false, 1.1, 1.5), Some(1.0));
}

/// The record's class picks the routine.
#[test]
fn draw_classes_map_to_their_routines() {
    assert_eq!(DrawClass::of(Some(1)), DrawClass::Square);
    assert_eq!(DrawClass::of(Some(2)), DrawClass::Square);
    assert_eq!(DrawClass::of(Some(3)), DrawClass::Rolled);
    assert_eq!(DrawClass::of(Some(6)), DrawClass::Bar);
    assert_eq!(DrawClass::of(Some(7)), DrawClass::Bar);
}
