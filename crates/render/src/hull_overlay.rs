//! The weapon-absorb hull overlay: `absorb_surface.mip` projected over a
//! craft's own hull for the second after it absorbs a pickup.
//!
//! Recovered from Pulse's PSP executable - `HullOverlay_DrawAbsorb`
//! (`0x0890d744`) and the shared `HullOverlay_Submit` (`0x0890e304`), gated by
//! `HullOverlay_AbsorbFade` (`0x0883e950`) on a timestamp the absorb handler
//! writes. Addresses and evidence are in
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "the hull
//! overlay pair"; this module implements that reading and cites it.
//!
//! # What the original draws
//!
//! The craft's own hull batches a second time, textured with
//! `Data\Tex\Weapons\absorb_surface.mip` through a **texture-matrix
//! projection of the vertices' own model-space positions** (`GU_TEXTURE_MATRIX`
//! with the projection source `GU_POSITION`, the only one the executable ever
//! sets), additively blended, tinted a flat `(a, a, a, a)`:
//!
//! - `u = 10 x`, `v = -10 z - 1.5 p`, where `x`/`z` are the vertex's GE input
//!   coordinates (the `s16 / 32768` a `.vex` stores, before its batch scale)
//!   and `p` is [`pulse`]. The `10` is `DAT_08abf4a4`; the `-1.5` an
//!   immediate. So the pattern is a top-down planar projection that slides
//!   along the craft's length as the pulse runs - not a rotation: the matrix's
//!   rotation half is built from `vsin`/`vcos` of `vcst 2/PI * pi/2`, a
//!   constant quarter turn.
//! - `a` is [`alpha`]: the pulse folded into a triangle, `0 -> 1 -> 0` over
//!   the second.
//!
//! # What is ours
//!
//! - **The depth test**: the original draws a hull batch that is not in the
//!   second batch list with `EQUAL` and depth writes on - exactly over the
//!   surface the hull already drew. This draws every batch through the
//!   renderer's blended pipeline, which tests `LessEqual` and never writes
//!   depth. Chosen, not measured.
//! - **Which meshes**: the original also gates each mesh on a per-mesh byte
//!   (`mesh+0x79`) whose writer is unread. This overlays every batch of the
//!   hull.
//! - **The airbrake flaps' projection source**: their vertices are the hull's
//!   baked positions taken back through the hinge ([`crate::mesh::Flap::hinge`]),
//!   which is exact for a stowed flap. A deflected flap's overlay stays
//!   stowed. Chosen, not measured.

use std::sync::Arc;

use oag_core::math::Vec3;
use oag_vex::vex;

use crate::mesh::{Model, ModelTexture};

/// `DAT_08abf4a4`: texture repeats per GE input unit, on both axes.
pub const REPEAT: f32 = 10.0;

/// The `v` offset per unit of [`pulse`], an immediate in `HullOverlay_Submit`.
pub const SCROLL: f32 = -1.5;

/// How long the overlay runs, in seconds: `HullOverlay_AbsorbWindowActive`'s
/// upper bound, `_DAT_08a7b6ac`.
pub const WINDOW: f32 = 1.0;

/// `HullOverlay_AbsorbFade`: `elapsed * 2` inside the one-second window, or
/// `None` outside it or at its very start, where the draw gate
/// (`fade > 0.0`) draws nothing.
#[must_use]
pub fn pulse(elapsed: f32) -> Option<f32> {
    if !(0.0..=WINDOW).contains(&elapsed) {
        return None;
    }
    Some(elapsed * 2.0).filter(|p| *p > 0.0)
}

/// `HullOverlay_Submit`'s colour: the pulse folded at `1.0`, so the overlay
/// fades up for half a second and back down for the other half. Every channel
/// of `Gu_Color` takes this same value.
#[must_use]
pub fn alpha(pulse: f32) -> f32 {
    if pulse >= 1.0 { 2.0 - pulse } else { pulse }
}

/// The `v` offset the texture matrix adds at `pulse`.
#[must_use]
pub fn scroll(pulse: f32) -> f32 {
    SCROLL * pulse
}

/// The one batch scale every mesh batch of a hull shares, which is what turns
/// a built model's positions back into the GE's own input coordinates.
///
/// `None` when the batches disagree, or when any is a PS2 VIF batch: the PS2
/// port's overlay is unread, and a VIF batch's positions are not the `s16`
/// GE input the projection is defined on.
#[must_use]
pub fn projection_scale(blob: &[u8]) -> Option<f32> {
    let mesh = vex::classes_of(blob).ok()?.mesh?;
    let nodes = vex::nodes(blob).ok()?;
    let mut scale = None;
    for node in nodes.iter().filter(|node| node.class_id == mesh) {
        let start = node.offset + node.header_size;
        let payload = blob.get(start..start + node.data_size)?;
        for list in [0u8, 1u8] {
            for batch in vex::mesh_batches(payload, list).ok()? {
                if vex::is_vif_batch(batch.vertex_type) || batch.scale <= 0.0 {
                    return None;
                }
                match scale {
                    None => scale = Some(batch.scale),
                    Some(seen) if seen == batch.scale => {}
                    Some(_) => return None,
                }
            }
        }
    }
    scale
}

/// The overlay model for `hull`: the same vertices and triangles, every one
/// textured with `texture` through the projection, unlit and white so the
/// per-frame tint is the whole colour.
///
/// `scale` is [`projection_scale`] of the file `hull` was built from. The
/// texture coordinates carry no [`scroll`]; the caller adds it per frame.
#[must_use]
pub fn build(hull: &Model, scale: f32, texture: Arc<ModelTexture>) -> Model {
    let mut model = hull.clone();
    model.label = format!("{} (absorb overlay)", hull.label);
    let hinges: Vec<_> = hull
        .airbrakes
        .iter()
        .flatten()
        .map(|flap| (flap.vertices.clone(), flap.hinge.inverse()))
        .collect();
    for (index, vertex) in model.vertices.iter_mut().enumerate() {
        let baked = Vec3::from_array(vertex.position);
        let local = hinges
            .iter()
            .find(|(range, _)| range.contains(&(index as u32)))
            .map_or(baked, |(_, unhinge)| unhinge.transform_point3(baked));
        let input = local / scale;
        vertex.texcoord = [REPEAT * input.x, -REPEAT * input.z];
        vertex.colour = [1.0; 4];
        vertex.lit = 0.0;
        vertex.anim = 0;
    }
    for draw in model
        .draws
        .iter_mut()
        .chain(&mut model.alpha_tested_draws)
        .chain(&mut model.transparent_draws)
    {
        draw.texture = Some(0);
        draw.blend = None;
    }
    // One texture, and nothing else the hull's own materials carried.
    model.textures = vec![Some(texture)];
    model.lightmaps.clear();
    model.material_slots.clear();
    model.material_specular_exponent.clear();
    model.material_variants.clear();
    model.material_anim.clear();
    model.anim_tracks.clear();
    model.emissive.clear();
    model.vertex_colour_is_light = false;
    model.flame = None;
    model
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pulse_runs_twice_as_fast_as_the_clock_and_only_inside_the_window() {
        assert_eq!(pulse(0.0), None, "the absorb frame itself draws nothing");
        assert_eq!(pulse(0.25), Some(0.5));
        assert_eq!(pulse(1.0), Some(2.0));
        assert_eq!(pulse(1.01), None);
        assert_eq!(pulse(-0.1), None);
    }

    #[test]
    fn the_tint_is_a_triangle_peaking_half_a_second_in() {
        assert_eq!(alpha(0.5), 0.5);
        assert_eq!(alpha(1.0), 1.0);
        assert_eq!(alpha(1.5), 0.5);
        assert_eq!(alpha(2.0), 0.0);
    }

    #[test]
    fn the_scroll_slides_three_repeats_over_the_window() {
        assert_eq!(scroll(2.0), -3.0);
    }
}
