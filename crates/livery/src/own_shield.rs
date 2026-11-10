//! Our own HD/Fury-style shield's shell: a closed surface standing off the craft's own hull.
//!
//! Built when the player's `graphics.own_shield` is on, on a title whose `Looks::own_shield` rule
//! offers it (Pulse, Pure), in place of the shell the disc authors. The geometry is computed from
//! the hull's first level of detail, so the shell is hull-shaped as HD's is, with a gap between
//! the two; nothing is read from any other disc and no texture is used. See
//! `docs/rendering/own-shield.md`.

mod offset_surface;

use oag_core::math::Vec3;
use oag_mesh::mesh::{self, Bounds, DrawCall, GpuVertex, Model};
use oag_vex::vex;

/// How far the shell stands off the hull, in model units: the gap a player sees between the two
/// in the chase view.
pub const STAND_OFF: f32 = 0.26;

/// The size of the cubes the shell is cut from, which is the size of its facets.
const FACET: f32 = 0.11;

/// The shell off `hull`, or `None`, reported, when the hull's meshes do not map back to its file
/// or give no shell: that craft then draws no shield, never the disc's shell under our palette.
pub(super) fn shell(hull: &Model, blob: &[u8], report: &mut Vec<String>) -> Option<Model> {
    let Some(ranges) = oag_fx::hull_overlay::overlaid_meshes(hull, blob) else {
        report.push(format!(
            "{}: meshes that do not map back to the file - no shield shell on this craft",
            hull.label
        ));
        return None;
    };
    // The first level of detail's own triangles, less any an `Anim Transform` moves: those are
    // authored in their node's space, not the hull's.
    let fixed = |index: u32| {
        ranges.iter().any(|range| range.contains(&index))
            && hull.vertices[index as usize].xform == 0
    };
    let triangles: Vec<[Vec3; 3]> = hull
        .indices
        .chunks_exact(3)
        .filter(|triangle| triangle.iter().all(|index| fixed(*index)))
        .map(|triangle| [0, 1, 2].map(|n| Vec3::from(hull.vertices[triangle[n] as usize].position)))
        .collect();
    let Some(surface) = offset_surface::around(&triangles, STAND_OFF, FACET) else {
        report.push(format!(
            "{}: {} fixed triangle(s) give no surface - no shield shell on this craft",
            hull.label,
            triangles.len()
        ));
        return None;
    };
    let mut model = Model::none(&format!("{} (own shield)", hull.label));
    model.vertices = surface
        .positions
        .iter()
        .zip(&surface.normals)
        .map(|(position, normal)| vertex(*position, *normal))
        .collect();
    model.indices = surface.indices;
    // One batch through the blended list, so the shell adds to the hull it wraps; two-sided, as
    // the far wall of the shell shows through the near one.
    model.transparent_draws = vec![DrawCall {
        writes_depth: false,
        range: 0..model.indices.len() as u32,
        texture: None,
        bounds: Bounds {
            centre: hull.centre,
            radius: hull.radius + STAND_OFF + FACET,
        },
        moving: false,
        culled: false,
        blend: None,
        blend_state: None,
        depth_point: None,
        layer: vex::LAYER_DEFAULT,
        node: None,
        chunk: None,
        alpha_test_ref: None,
    }];
    model.centre = hull.centre;
    model.radius = hull.radius + STAND_OFF + FACET;
    model.mesh_count = 1;
    model.own_shield = true;
    report.push(format!(
        "{}: our own shield shell, {} triangle(s) standing {STAND_OFF} off {} of the hull's",
        hull.label,
        model.indices.len() / 3,
        triangles.len()
    ));
    Some(model)
}

/// A shell vertex: white, so the shield's colour is the whole tint, and unlit.
fn vertex(position: Vec3, normal: Vec3) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position: position.into(),
        normal: normal.into(),
        colour: [1.0; 4],
        texcoord: [0.0; 2],
        lit: 0.0,
        anim: 0,
        lightmap_texcoord: [0.0; 2],
        xform: 0,
        sun_mask: 1.0,
        slots: mesh::slots::DEFAULT,
        specular_exponent: mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0; 2],
        tangent: 0,
    }
}

/// The sphere the player sees from inside the cockpit, themed to match: the disc's own sphere
/// geometry under the same shader, at its authored size, with every vertex white so the shield's
/// colour is the whole tint.
pub fn cockpit(model: &mut Model) {
    for vertex in &mut model.vertices {
        vertex.colour = [1.0; 4];
    }
    model.own_shield = true;
}
