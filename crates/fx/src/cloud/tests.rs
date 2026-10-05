use super::*;
use oag_core::math::vec3;

/// A sprite with every field set by hand.
fn sprite(position: Vec3, phase: f32, rate: f32) -> Sprite {
    Sprite {
        position,
        half_size: 4.0,
        colour: [1.0, 1.0, 1.0, 1.0],
        cell: 6,
        phase,
        rate,
    }
}

/// [`Sprite::advance`] adds the rate with no `dt` scaling, matching
/// `CloudGroup_Draw`'s own `phase += rate` per call.
#[test]
fn advancing_adds_the_rate_with_no_dt() {
    let mut s = sprite(Vec3::ZERO, 1.25, 0.0015);
    s.advance();
    assert_eq!(s.phase, 1.25 + 0.0015);
}

/// At phase zero the rotated basis is the identity, so the quad's corners
/// land exactly at `centre +/- right * half_size +/- up * half_size` - the
/// un-rotated case a camera-facing billboard degenerates to.
#[test]
fn at_phase_zero_the_quad_is_axis_aligned() {
    let sprite = Sprite {
        position: vec3(1.0, 2.0, 3.0),
        half_size: 2.0,
        colour: [1.0, 0.5, 0.25, 0.8],
        cell: 6,
        phase: 0.0,
        rate: 0.0,
    };
    let verts = sprite.vertices(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), 0.0);
    // bl, br, tl (the first triangle) at phase zero: centre (1, 2, 3),
    // half_size 2, so the corners are +/- 2 along each camera axis.
    assert_eq!(verts[0].position, [-1.0, 0.0, 3.0]);
    assert_eq!(verts[1].position, [3.0, 0.0, 3.0]);
    assert_eq!(verts[2].position, [-1.0, 4.0, 3.0]);
    for v in &verts {
        assert_eq!(v.colour, [1.0, 0.5, 0.25, 0.8]);
    }
}

/// A quarter turn of phase swaps which camera axis the quad's extent runs
/// along, turning from up towards right: `CloudGroup_Draw` rotates its corners
/// by `roll - phase`, see the module doc.
#[test]
fn a_quarter_turn_swaps_the_quad_s_axes() {
    let sprite = Sprite {
        position: Vec3::ZERO,
        half_size: 1.0,
        colour: [1.0, 1.0, 1.0, 1.0],
        cell: 6,
        phase: std::f32::consts::FRAC_PI_2,
        rate: 0.0,
    };
    let verts = sprite.vertices(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), 0.0);
    // At an angle of minus a quarter turn (cos=0, sin=-1):
    // r = -up; u = right. bl = -r - u = up - right.
    let bl = verts[0].position;
    assert!((bl[0] - -1.0).abs() < 1e-5, "{bl:?}");
    assert!((bl[1] - 1.0).abs() < 1e-5, "{bl:?}");
}

/// [`Layer::is_empty`] and [`Layer::advance`]/[`Layer::extend_vertices`] on a
/// hand-built layer - `Layer::from_groups` itself is exercised against real
/// disc data by `oag_vex::cloud`'s own ground-truth tests and by the
/// `--screenshot` check `clouds.md` records; this crate depends on nothing
/// but the vertices those positions and colours produce.
#[test]
fn a_layer_advances_every_sprite_and_emits_six_vertices_each() {
    let mut layer = Layer {
        sprites: vec![
            sprite(vec3(0.0, 0.0, 0.0), 0.5, 0.001),
            sprite(vec3(10.0, 0.0, 0.0), 2.0, -0.002),
        ],
    };
    assert!(!layer.is_empty());
    let phases_before: Vec<f32> = layer.sprites.iter().map(|s| s.phase).collect();
    layer.advance();
    for (sprite, before) in layer.sprites.iter().zip(phases_before) {
        assert_eq!(sprite.phase, before + sprite.rate);
    }

    let mut out = Vec::new();
    layer.extend_vertices(&mut out, &Frame::IDENTITY);
    assert_eq!(out.len(), 2 * 6);
}

/// An empty layer is what every Pulse circuit but `05_Track` gets - see
/// `oag_vex::cloud`'s census - and it must draw nothing rather than a
/// degenerate quad.
#[test]
fn an_empty_layer_is_empty_and_draws_nothing() {
    let layer = Layer::default();
    assert!(layer.is_empty());
    let mut out = Vec::new();
    layer.extend_vertices(&mut out, &Frame::IDENTITY);
    assert!(out.is_empty());
}

/// The camera rolled `angle` about its back axis, `Z`.
fn rolled(angle: f32) -> Frame {
    let (sin, cos) = angle.sin_cos();
    Frame {
        position: Vec3::ZERO,
        right: vec3(cos, sin, 0.0),
        up: vec3(-sin, cos, 0.0),
        back: Vec3::Z,
    }
}

/// `CloudGroup_Draw` subtracts `g_camera_roll` from the phase before it
/// rotates the corners, so a sprite keeps its orientation in the world while
/// the camera banks: the same sprite drawn by a level camera and by cameras
/// rolled either way lands on the same world corners. Dropping the roll, or
/// flipping its sign, turns the quad by twice the roll and fails this.
#[test]
fn a_cloud_keeps_its_world_orientation_as_the_camera_rolls() {
    let layer = Layer {
        sprites: vec![Sprite {
            position: vec3(5.0, 1.0, -20.0),
            half_size: 3.0,
            colour: [1.0; 4],
            cell: 6,
            phase: 0.7,
            rate: 0.0,
        }],
    };
    let mut level = Vec::new();
    layer.extend_vertices(&mut level, &rolled(0.0));
    for angle in [0.3, -0.3] {
        let mut out = Vec::new();
        layer.extend_vertices(&mut out, &rolled(angle));
        for (a, b) in level.iter().zip(&out) {
            for axis in 0..3 {
                let d = (a.position[axis] - b.position[axis]).abs();
                assert!(
                    d < 1e-4,
                    "roll {angle}: {:?} vs {:?}",
                    a.position,
                    b.position
                );
            }
        }
    }
}

/// `FUN_089322f8`'s atlas: a 4x2 grid of quarter-width, half-height cells, and
/// `kind == 2` draws one of the bottom row's right two.
#[test]
fn atlas_cells_are_a_four_by_two_grid() {
    assert_eq!(field::cell_uv(0), (0.0, 0.0, 0.25, 0.5));
    assert_eq!(field::cell_uv(6), (0.5, 0.5, 0.75, 1.0));
    assert_eq!(field::cell_uv(7), (0.75, 0.5, 1.0, 1.0));
}

/// A sprite's quad samples only its own cell, bottom-left texel on the
/// `(-h, -h)` corner.
#[test]
fn a_sprite_samples_only_its_cell() {
    let mut s = sprite(Vec3::ZERO, 0.0, 0.0);
    s.cell = 7;
    let verts = s.vertices(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), 0.0);
    assert_eq!(verts[0].texcoord, [0.75, 1.0]);
    assert_eq!(verts[4].texcoord, [1.0, 0.5]);
}

/// A cube authors one record per ten units of volume, truncated: the live
/// `05_Track` cube, uniform scale 7.76697, authored 46.
#[test]
fn a_cube_authors_a_record_per_ten_units_of_volume() {
    let s = 7.766_97;
    let mut world = [0.0; 16];
    (world[0], world[5], world[10], world[15]) = (s, s, s, 1.0);
    let cube = oag_vex::cloud::GroupCube {
        kind: 2,
        scale: 1.0,
        world,
        sprite_radius: 4.0,
        sprite_radius_var: 0.2,
    };
    assert_eq!(field::record_count(&cube), 46);
}

/// The cull removes a later record closer than `(ha + hb) * (1 - Overlap)` to
/// an earlier live one, and keeps it at that distance or further.
#[test]
fn the_overlap_cull_removes_only_the_later_of_a_close_pair() {
    let record = |x: f32| field::Record {
        position: [x, 0.0, 0.0],
        half_size: 20.0,
        cube: Some(0),
    };
    // (20 + 20) * (1 - 0.65) = 14.
    let mut records = [record(0.0), record(13.0), record(30.0)];
    field::cull(&mut records, 0.65);
    assert_eq!(records.map(|r| r.cube.is_some()), [true, false, true]);
}
