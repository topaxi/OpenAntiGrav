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
//! # The glow mask
//!
//! `Gfx_BuildBatchStateList(0x282)` opens the alpha channel and sets the
//! stencil to `ALWAYS`, ref `0xff`, `REPLACE` (GE commands `0xDC`/`0xDD`,
//! read at instruction level). So every overlay fragment that survives the
//! alpha test (`GREATER 0`) and the colour test (not black) writes a **full**
//! glow into the bloom's mask, whatever the fade. That is why the original's
//! hull blooms into a white blob at the peak. A blend state cannot write a
//! constant alpha, so this moves the `SRC_ALPHA` factor into the texture
//! instead: [`glow_texels`] premultiplies each texel and sets its alpha to 1,
//! or clears a texel the two tests would discard. [`tint`] carries the fade.
//! [`BLEND`] then adds the colour and replaces the mask. The sum is the
//! original's own, `tex.rgb * tex.a * a^2 + dst`.
//!
//! # What is ours
//!
//! - **The depth test**: the original draws a hull batch that is not in the
//!   second batch list with `EQUAL` and depth writes on - exactly over the
//!   surface the hull already drew. This draws every batch through the
//!   renderer's blended pipeline, which tests `LessEqual` and never writes
//!   depth. Chosen, not measured: a frame cannot tell the two apart on an
//!   unmoving coplanar redraw.
//! - **The colour test runs on the texel, not the fragment.** The GE compares
//!   the modulated colour, so in the first and last frames a dark texel times
//!   a tiny `a` rounds to black and writes no glow. Here only a black texel
//!   is dropped. Chosen, not measured.
//! - **`self_illuminatedShape` and `glowingShape` take the tint.** Their
//!   batches carry vertex colour, and with lighting off the GE uses that in
//!   place of `Gu_Color`, so the original's overlay on them does not fade.
//!   Chosen, not measured. So is overlaying both of `glowingShape`'s list-0
//!   batches, where the original was seen to submit one of the two.
//! - **No fog.** `HullOverlay_Submit` calls `Fog_Disable` unless the mesh's
//!   material has flag `& 8` (the `0x0891eac0` branch, not followed). The
//!   race never writes the overlay's scene uniform, so it keeps `Fog::off`.
//!   That matters because [`BLEND`] adds the fragment at full weight, so a
//!   fog lerp would add fog colour on every overlaid pixel.
//! - **The airbrakes project from model space.** The GE projects each
//!   batch's own input coordinates, and the airbrake meshes sit under
//!   locator transforms, so their pattern in the original is offset from
//!   this one's by those transforms. Not measured.
//!
//! # Which meshes
//!
//! Every hull mesh with a list-0 batch, except those under a `LodGroup`'s
//! later children - see [`overlaid_meshes`]. That was measured on a live
//! absorb: the race draws the overlay through `Mesh_DrawBatchSet`, which
//! submits every entry of the hull's batch set. On Assegai that was
//! `shipShape`, both airbrakes, `self_illuminatedShape` and `glowingShape`,
//! but not `canopyShape` (list 1) or `lodShape`. The per-mesh name gate
//! (`mesh+0x79`, set for `...ship...`) belongs to the other, per-entity path,
//! which the race was not seen to take. Meshes left out stay in the model,
//! coloured transparent black, which adds nothing and writes no glow.

use std::sync::Arc;

use oag_core::math::Vec3;
use oag_vex::vex;

use crate::mesh::{Model, ModelTexture};

/// Whether a race draws the overlay at all: **on**, because the original
/// draws it in play.
///
/// Measured on PPSSPP with a real absorb on Talon's Junction, Assegai
/// (2026-09-23, second probe). Logged, non-halting watchpoints caught the
/// absorb handler's stamp store (`PC 0x088455b4`). They then caught
/// `HullOverlay_AbsorbFade` reading the player's stamp (`0x0883e954`) and
/// `HullOverlay_Submit` reading the `10` (`0x0890e670`, 11 times a frame).
/// Both ran for exactly the one-second window and stopped. The matrix handed
/// to `Gu_SetMatrix(3)` was read at `0x0890e754` and gives the projection
/// below: `u = 10x`, `v = -10z - 1.5p`. The earlier probe that switched this
/// off had two execution breakpoints armed at once, and PPSSPP v1.20.4 only
/// fires the last one added. See
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
/// "2026-09-23 (later)".
pub const DRAWN: bool = true;

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

/// The vertex ranges of `hull` the overlay covers: every mesh node with a
/// batch in list 0, outside the subtree of any `LodGroup` child but the
/// first. `None` when `hull`'s per-node ranges cannot be matched back to the
/// file's mesh nodes.
///
/// Measured on Assegai (see the module docs): the race path submits every
/// entry of the hull's batch set, and that set held the list-0 meshes of the
/// first level of detail and nothing else.
///
/// The ranges skip nodes the build left out (a `LodGroup`'s other child), so
/// they are matched to the file's mesh nodes in order by vertex count.
#[must_use]
pub fn overlaid_meshes(hull: &Model, blob: &[u8]) -> Option<Vec<std::ops::Range<u32>>> {
    let classes = vex::classes_of(blob).ok()?;
    let mesh = classes.mesh?;
    let nodes = vex::nodes(blob).ok()?;
    let later_lod = |mut index: usize| {
        while let Some(parent) = nodes[index].parent {
            if Some(nodes[parent].class_id) == classes.lod_group {
                let first = nodes.iter().position(|node| node.parent == Some(parent));
                return first != Some(index);
            }
            index = parent;
        }
        false
    };
    let mut meshes = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.class_id == mesh)
        .map(|(index, node)| {
            let start = node.offset + node.header_size;
            let payload = blob.get(start..start + node.data_size)?;
            let lists = [0u8, 1u8].map(|list| {
                vex::mesh_batches(payload, list)
                    .ok()
                    .unwrap_or_default()
                    .iter()
                    .map(|batch| batch.vertices.len())
                    .collect::<Vec<_>>()
            });
            let count: usize = lists.iter().flatten().sum();
            let overlaid = !lists[0].is_empty() && !later_lod(index);
            Some((overlaid, count))
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter();
    let mut overlaid = Vec::new();
    for range in &hull.node_vertex_ranges {
        let len = (range.end - range.start) as usize;
        let (lit, _) = meshes.find(|(_, count)| *count == len)?;
        if lit {
            overlaid.push(range.clone());
        }
    }
    Some(overlaid)
}

/// `Gfx_BuildBatchStateList(0x282)`'s blend with the glow mask written the
/// way its stencil writes it: colour `src + dst` (the `SRC_ALPHA` factor is
/// already in the texel, see [`glow_texels`]), and alpha `REPLACE` where the
/// fragment's alpha is 1, `KEEP` where it is 0.
pub const BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

/// Rewrites `absorb_surface.mip`'s RGBA8 texels for [`BLEND`]: a texel the
/// GE's alpha test (`GREATER 0`) or colour test (not black) would discard
/// becomes transparent black, and every other one is premultiplied by its
/// own alpha and made opaque, so the fragment's alpha is free to carry the
/// stencil's `0xff`.
pub fn glow_texels(rgba: &mut [u8]) {
    for texel in rgba.as_chunks_mut::<4>().0 {
        let alpha = u16::from(texel[3]);
        if alpha == 0 || texel[..3] == [0, 0, 0] {
            *texel = [0; 4];
            continue;
        }
        for channel in &mut texel[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
        texel[3] = 255;
    }
}

/// The vertex colour the overlay draws with at `alpha` ([`alpha`]): the
/// original's `Gu_Color(a, a, a, a)` modulates both the texel and its alpha,
/// and the blend multiplies by that alpha again, so the colour weight is
/// `a^2`. The alpha stays 1 for [`BLEND`]'s mask write.
#[must_use]
pub fn tint(alpha: f32) -> [f32; 4] {
    let weight = alpha * alpha;
    [weight, weight, weight, 1.0]
}

/// The overlay model for `hull`: the same vertices and triangles, textured
/// with `texture` through the projection, unlit and white so the per-frame
/// tint is the whole colour - on `ships` (see [`overlaid_meshes`]), and
/// transparent black everywhere else.
///
/// `scale` is [`projection_scale`] of the file `hull` was built from. The
/// texture coordinates carry no [`scroll`]; the caller adds it per frame.
#[must_use]
pub fn build(
    hull: &Model,
    scale: f32,
    ships: &[std::ops::Range<u32>],
    texture: Arc<ModelTexture>,
) -> Model {
    let mut model = hull.clone();
    model.label = format!("{} (absorb overlay)", hull.label);
    for (index, vertex) in model.vertices.iter_mut().enumerate() {
        let input = Vec3::from_array(vertex.position) / scale;
        vertex.texcoord = [REPEAT * input.x, -REPEAT * input.z];
        let lit = ships.iter().any(|range| range.contains(&(index as u32)));
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
    fn a_texel_the_ge_would_discard_writes_nothing_and_the_rest_are_premultiplied() {
        let mut texels = [
            0, 0, 0, 255, 200, 100, 50, 0, 200, 100, 50, 128, 10, 20, 30, 255,
        ];
        glow_texels(&mut texels);
        assert_eq!(texels[0..4], [0; 4], "black fails the colour test");
        assert_eq!(texels[4..8], [0; 4], "alpha 0 fails the alpha test");
        assert_eq!(texels[8..12], [100, 50, 25, 255]);
        assert_eq!(texels[12..16], [10, 20, 30, 255]);
    }

    #[test]
    fn the_tint_weighs_the_colour_by_the_square_and_keeps_the_mask_whole() {
        assert_eq!(tint(0.5), [0.25, 0.25, 0.25, 1.0]);
        assert_eq!(tint(1.0), [1.0; 4]);
    }

    #[test]
    fn the_scroll_slides_three_repeats_over_the_window() {
        assert_eq!(scroll(2.0), -3.0);
    }
}
