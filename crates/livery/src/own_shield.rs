//! Our own HD/Fury-style shield's shell: the craft's own hull, drawn procedurally.
//!
//! Built when the player's `graphics.own_shield` is on, on a title whose `Looks::own_shield` rule
//! offers it (Pulse, Pure), in place of the shell the disc authors. The geometry is the hull's
//! first level of detail, so the shell is hull-shaped as HD's is; nothing is read from any other
//! disc and no texture is used. See `docs/rendering/own-shield.md`.

use oag_mesh::mesh::Model;

/// How far each vertex of the shell stands off the hull, in world units: about three percent of a
/// craft's length.
const INFLATE: f32 = 0.4;

/// The shell off `hull`, or `None`, reported, when the hull's meshes do not map back to its file:
/// that craft then draws no shield, never the disc's shell under our palette.
pub(super) fn shell(hull: &Model, blob: &[u8], report: &mut Vec<String>) -> Option<Model> {
    let Some(ranges) = oag_fx::hull_overlay::overlaid_meshes(hull, blob) else {
        report.push(format!(
            "{}: meshes that do not map back to the file - no shield shell on this craft",
            hull.label
        ));
        return None;
    };
    let mut model = hull.clone();
    model.label = format!("{} (own shield)", hull.label);
    for (index, vertex) in model.vertices.iter_mut().enumerate() {
        let lit = ranges.iter().any(|range| range.contains(&(index as u32)));
        vertex.colour = if lit { [1.0; 4] } else { [0.0; 4] };
        vertex.lit = 0.0;
        vertex.anim = 0;
    }
    for draw in model
        .draws
        .iter_mut()
        .chain(&mut model.alpha_tested_draws)
        .chain(&mut model.transparent_draws)
    {
        draw.texture = None;
        draw.blend = None;
    }
    // Every batch draws through the blended list, so the shell adds to the hull it wraps rather
    // than replacing it.
    let all: Vec<_> = model
        .draws
        .drain(..)
        .chain(model.alpha_tested_draws.drain(..))
        .chain(model.transparent_draws.drain(..))
        .collect();
    model.transparent_draws = all;
    model.textures.clear();
    model.lightmaps.clear();
    model.material_slots.clear();
    model.material_specular_exponent.clear();
    model.material_glow.clear();
    model.glow_from_program = false;
    model.material_variants.clear();
    model.material_anim.clear();
    model.anim_tracks.clear();
    model.emissive.clear();
    model.vertex_colour_is_light = false;
    model.stamps_glow = false;
    model.flame = None;
    model.own_shield = Some(INFLATE);
    report.push(format!(
        "{}: {} mesh(es) of the hull as our own shield shell",
        hull.label,
        ranges.len()
    ));
    Some(model)
}

/// The sphere the player sees from inside the cockpit, themed to match: the disc's own sphere
/// geometry under the same shader, at its authored size, with every vertex white so the shield's
/// colour is the whole tint.
pub fn cockpit(model: &mut Model) {
    for vertex in &mut model.vertices {
        vertex.colour = [1.0; 4];
    }
    model.own_shield = Some(0.0);
}
