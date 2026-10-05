//! What the blob tier's placement arithmetic is asserted to do.
//!
//! Its own file rather than a `#[cfg(test)]` block in `shadow.rs`, per the
//! 200-line inline rule in `scripts/check-file-size.py`. Nothing here needs a
//! GPU: the pipeline is one call into `wgpu`, and everything that can be
//! wrong in a way a screenshot does not show is in [`super::quad`] and
//! [`super::fade`].

use super::*;

/// A craft two units above flat ground, pointing along `+z`.
fn placement() -> Placement {
    Placement {
        contact: Vec3::new(10.0, 0.0, -5.0),
        normal: Vec3::Y,
        forward: Vec3::Z,
        half_length: 3.0,
        half_width: 1.5,
        strength: 1.0,
        silhouette: 0,
    }
}

#[test]
fn the_quad_lies_in_the_surface_plane_one_lift_above_it() {
    let quad = quad(&placement());
    for vertex in quad {
        assert!(
            (vertex.position[1] - LIFT).abs() < 1e-6,
            "every corner sits exactly one lift above the plane: {vertex:?}"
        );
    }
}

#[test]
fn the_quad_is_the_hulls_own_footprint() {
    let quad = quad(&placement());
    let x: Vec<f32> = quad.iter().map(|v| v.position[0]).collect();
    let z: Vec<f32> = quad.iter().map(|v| v.position[2]).collect();
    let span = |values: &[f32]| {
        values.iter().fold(f32::MIN, |a, b| a.max(*b))
            - values.iter().fold(f32::MAX, |a, b| a.min(*b))
    };
    // Width across, length along - not the texture's aspect. See `quad`.
    assert!((span(&x) - 3.0).abs() < 1e-5, "width is 2 * half_width");
    assert!((span(&z) - 6.0).abs() < 1e-5, "length is 2 * half_length");
}

#[test]
fn a_banked_surface_tilts_the_quad_with_it() {
    // A 45-degree bank: every corner has to stay in that plane, or the shadow
    // stands up out of the road on a corner.
    let normal = Vec3::new(1.0, 1.0, 0.0).normalize();
    let quad = quad(&Placement {
        normal,
        ..placement()
    });
    let contact = placement().contact;
    for vertex in quad {
        let offset = Vec3::from_array(vertex.position) - contact;
        assert!(
            (offset.dot(normal) - LIFT).abs() < 1e-5,
            "corner is off the banked plane: {vertex:?}"
        );
    }
}

#[test]
fn the_long_axis_follows_the_craft_and_not_the_camera() {
    // Yawed 90 degrees: the six-unit span moves from z to x, which is the
    // whole point of projecting `forward` rather than using a camera basis.
    let quad = quad(&Placement {
        forward: Vec3::X,
        ..placement()
    });
    let span = |axis: usize| {
        let values: Vec<f32> = quad.iter().map(|v| v.position[axis]).collect();
        values.iter().fold(f32::MIN, |a, b| a.max(*b))
            - values.iter().fold(f32::MAX, |a, b| a.min(*b))
    };
    assert!((span(0) - 6.0).abs() < 1e-5, "length now runs along x");
    assert!((span(2) - 3.0).abs() < 1e-5, "width now runs along z");
}

#[test]
fn a_craft_pointing_straight_down_still_gets_a_finite_quad() {
    // `forward` parallel to `normal` leaves nothing to project. The fallback
    // basis is arbitrary - what is asserted is that it is a basis at all.
    let quad = quad(&Placement {
        forward: Vec3::Y,
        ..placement()
    });
    for vertex in quad {
        assert!(
            vertex.position.iter().all(|c| c.is_finite()),
            "degenerate basis produced {vertex:?}"
        );
    }
    let corners: Vec<Vec3> = quad.iter().map(|v| Vec3::from_array(v.position)).collect();
    assert!(
        corners.iter().any(|c| *c != corners[0]),
        "the quad collapsed to a point"
    );
}

#[test]
fn a_hovering_craft_casts_a_full_strength_shadow() {
    // The case a player sees almost always, and the one the first cut of this
    // got wrong: a craft resting at its own ride height read 0.030.
    assert_eq!(fade(0.0, 4.125), 1.0, "on the floor");
    assert_eq!(fade(4.0, 4.125), 1.0, "resting, just under the target");
    assert_eq!(fade(4.125, 4.125), 1.0, "exactly at the target");
}

#[test]
fn the_shadow_fades_out_once_the_craft_is_airborne() {
    let ride = 4.0;
    assert!((fade(ride * 2.0, ride) - 0.5).abs() < 1e-6, "half way up");
    assert_eq!(fade(ride * FADE_REACH, ride), 0.0, "at the end of the fade");
    assert_eq!(fade(99.0, ride), 0.0, "and past it, still gone");
    // A degenerate ride height must not divide by zero into a NaN that
    // reaches the vertex buffer.
    assert_eq!(fade(1.0, 0.0), 0.0);
}

#[test]
fn the_quads_alpha_is_the_fade_and_its_colour_is_black() {
    let quad = quad(&Placement {
        strength: 0.25,
        ..placement()
    });
    for vertex in quad {
        assert_eq!(vertex.colour, [0.0, 0.0, 0.0, 0.25]);
        assert_eq!(vertex.lit, 0.0, "a shadow is not lit geometry");
    }
}

#[test]
fn the_generated_falloff_is_coverage_in_red_with_opaque_alpha() {
    // The convention HD's own `ambient_shadow.gtf` decodes to, so
    // `shadow.wgsl` reads one channel whichever path supplied the pixels.
    let image = Silhouette::falloff(16);
    assert_eq!(image.rgba.len(), 16 * 16 * 4);
    for texel in image.rgba.as_chunks::<4>().0 {
        assert_eq!(texel[0], texel[1]);
        assert_eq!(texel[1], texel[2]);
        assert_eq!(texel[3], 0xff);
    }
    let at = |x: usize, y: usize| image.rgba[(y * 16 + x) * 4];
    assert!(at(8, 8) > at(2, 2), "the middle is darker than the edge");
    assert_eq!(at(0, 0), 0, "and the corner is nothing at all");
}

#[test]
fn a_one_texel_falloff_does_not_divide_by_zero() {
    // `size = 1` puts the centre at 0 and would divide by it.
    let image = Silhouette::falloff(1);
    assert_eq!(image.rgba.len(), 4);
    assert_eq!(image.rgba[3], 0xff);
}

/// A tetrahedron built as a decoded hull rather than as bytes: this crate
/// draws an `Occluder`, and how one is parsed is `oag_formats`' own business.
fn hull() -> Occluder {
    use oag_vex::shadow_occluder::{Face, NO_NEIGHBOUR};
    let faces = [
        ([0.0, -1.0, 0.0], [0, 1, 2], [2, 3, 1]),
        ([-1.0, 0.0, 0.0], [0, 2, 3], [0, 3, 2]),
        ([0.0, 0.0, -1.0], [0, 3, 1], [1, 3, 0]),
        ([0.577_35, 0.577_35, 0.577_35], [1, 3, 2], [2, 1, 0]),
    ];
    Occluder {
        bounds: ([0.0; 3], [1.0; 3]),
        faces: faces
            .iter()
            .map(|(normal, index, neighbour)| Face {
                normal: *normal,
                count: 3,
                vertices: [index[0], index[1], index[2], index[0]],
                neighbours: [neighbour[0], neighbour[1], neighbour[2], NO_NEIGHBOUR],
            })
            .collect(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
        ],
    }
}

/// A cast of that hull straight down onto `y = 0`, from `height` above it.
fn cast_from(hull: &Occluder, height: f32, strength: f32) -> Cast<'_> {
    Cast {
        hull,
        model: Mat4::from_translation(Vec3::new(0.0, height, 0.0)),
        axis: Vec3::new(0.0, -1.0, 0.0),
        contact: Vec3::ZERO,
        normal: Vec3::Y,
        strength,
    }
}

#[test]
fn the_projected_hull_lies_flat_on_the_surface() {
    let hull = hull();
    let mut out = Vec::new();
    let rings = hull_triangles(&cast_from(&hull, 5.0, 1.0), &mut out);
    assert_eq!(rings, 1);
    // A ring of n vertices fills as n - 2 triangles, which for a triangle is
    // one: ear clipping fills the ring's interior once, where the fan this
    // replaced drew one triangle per edge and overlapped them.
    assert_eq!(out.len(), 3);
    for vertex in &out {
        assert!(
            (vertex.position[1] - LIFT).abs() < 1e-5,
            "{vertex:?} is not on the plane"
        );
    }
}

#[test]
fn the_projection_is_parallel_so_height_does_not_change_the_shape() {
    // The direction is a *direction*, not a point light: a craft twice as far
    // above the road casts the same outline, only fainter through `fade`.
    let hull = hull();
    let mut near = Vec::new();
    let mut far = Vec::new();
    hull_triangles(&cast_from(&hull, 2.0, 1.0), &mut near);
    hull_triangles(&cast_from(&hull, 40.0, 1.0), &mut far);
    assert_eq!(near.len(), far.len());
    for (a, b) in near.iter().zip(&far) {
        for axis in 0..3 {
            assert!(
                (a.position[axis] - b.position[axis]).abs() < 1e-4,
                "{:?} against {:?}",
                a.position,
                b.position
            );
        }
    }
}

#[test]
fn a_direction_pointing_away_from_the_surface_draws_nothing() {
    // Up, at a floor: the volume never reaches the plane, and forcing an
    // intersection would put the shadow behind the craft.
    let hull = hull();
    let mut out = Vec::new();
    let rings = hull_triangles(
        &Cast {
            axis: Vec3::Y,
            ..cast_from(&hull, 5.0, 1.0)
        },
        &mut out,
    );
    assert_eq!(rings, 0);
    assert!(out.is_empty());
}

#[test]
fn a_hull_carries_the_fade_and_the_tiers_own_darkness() {
    let hull = hull();
    let mut out = Vec::new();
    hull_triangles(&cast_from(&hull, 5.0, 0.5), &mut out);
    for vertex in &out {
        assert_eq!(vertex.colour[3], 0.5 * HULL_DARKNESS);
        assert_eq!(&vertex.colour[..3], &[0.0, 0.0, 0.0]);
    }
}

#[test]
fn a_banked_surface_takes_the_projection_with_it() {
    let hull = hull();
    let normal = Vec3::new(1.0, 1.0, 0.0).normalize();
    let mut out = Vec::new();
    let rings = hull_triangles(
        &Cast {
            normal,
            ..cast_from(&hull, 5.0, 1.0)
        },
        &mut out,
    );
    assert_eq!(rings, 1);
    for vertex in &out {
        let offset = Vec3::from_array(vertex.position) - Vec3::ZERO;
        assert!(
            (offset.dot(normal) - LIFT).abs() < 1e-4,
            "{vertex:?} is off the banked plane"
        );
    }
}

/// A road that is not the plane the cast found: it rises one unit for every
/// ten along x, and the hull was projected onto `y = 0`.
fn ramp(at: Vec3, normal: Vec3) -> Option<f32> {
    let road = Vec3::new(at.x, at.x * 0.1, at.z);
    Some((road - at).dot(normal))
}

#[test]
fn a_conformed_hull_sits_the_lift_above_the_road_under_every_vertex() {
    let hull = hull();
    let mut out = Vec::new();
    hull_triangles(&cast_from(&hull, 5.0, 1.0), &mut out);
    let before = out.len();
    conform_to_floor(&mut out, 0, ramp);
    assert_eq!(
        out.len(),
        before * 4,
        "each triangle is split once, into four"
    );
    for vertex in &out {
        let at = Vec3::from_array(vertex.position);
        let gap = at.y - at.x * 0.1;
        assert!((gap - LIFT).abs() < 1e-5, "{at:?} is {gap} above the road");
    }
}

#[test]
fn a_hull_is_only_conformed_from_the_first_vertex_it_is_told_to() {
    let hull = hull();
    let mut out = Vec::new();
    hull_triangles(&cast_from(&hull, 5.0, 1.0), &mut out);
    hull_triangles(&cast_from(&hull, 5.0, 1.0), &mut out);
    let first = out.len() / 2;
    let kept: Vec<_> = out[..first].to_vec();
    conform_to_floor(&mut out, first, ramp);
    assert_eq!(
        &out[..first].iter().map(|v| v.position).collect::<Vec<_>>(),
        &kept.iter().map(|v| v.position).collect::<Vec<_>>()
    );
    assert_eq!(out.len(), first * 5);
}

#[test]
fn a_point_with_no_road_under_it_stays_where_the_plane_put_it() {
    let hull = hull();
    let mut out = Vec::new();
    hull_triangles(&cast_from(&hull, 5.0, 1.0), &mut out);
    let planar: Vec<f32> = out.iter().map(|v| v.position[1]).collect();
    conform_to_floor(&mut out, 0, |_, _| None);
    assert!(out.iter().all(|v| (v.position[1] - planar[0]).abs() < 1e-6));
}
