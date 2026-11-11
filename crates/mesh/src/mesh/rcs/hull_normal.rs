//! HD's ship hulls sample their `_n` normal map.
//!
//! A hull's lit fragment program (`diffuse_with_specular_from_alpha_n`,
//! `carbonfibre`, `nitro_body_new` and kin) reads texture unit 1 as a
//! tangent-space normal and builds the lighting normal from it:
//!
//! ```text
//! N = (2 t.x - 1) T + (2 t.y - 1) B + (2 t.z - 1) N0         then normalised
//! B = cross(N0, T) * T.w   or   cross(N0, T) * (2 T.w - 1)   the vertex program, per compile
//! ```
//!
//! with `T` the chunk's authored `tangent` attribute (`2b - 1` on its first
//! three bytes, the fourth byte `w` the factor on `B`, raw or as `2w - 1` by the compile: the
//! material's vertex program decides, [`TangentFrame`]) and `N0` the vertex normal. The normalised `N` then feeds every term the program lights with:
//! the sun's `N.L`, the specular's `N.H`, the probe lookups. Plain RGB, not
//! DXT5nm. `docs/rendering/hd-ship-materials.md`, "Hull normal maps".
//!
//! **The two-lane packing.** Some compiles fetch the map's green and alpha only
//! and rebuild the third component (`Program::samples_as_packed_normal`; Moa
//! Therma's `diffuse_normal_specular`, block #7 `@0x9960`):
//!
//! ```text
//! x = 2 t.a - 1    y = 2 t.g - 1    z = sqrt(1 - x^2 - y^2)
//! ```
//!
//! into the same frame, built by the same vertex formula. [`unpacked`] bakes
//! such a map into the three-lane form once, so the shader reads one layout.
//! The program rebuilds `z` after the filter and this before it: the two agree
//! at a texel centre. `docs/formats/rcsmaterial.md`, "The two-lane normal map".
//!
//! This binds the picture in the third slot ([`Model::pad_masks`]) and marks
//! the material, so `mesh::rcs::emit` fills [`GpuVertex::tangent`] for its
//! chunks; `shade.wesl` reads both. Every vertex of every other material
//! carries a zero tangent, which is the shader's off switch.
//!
//! **Scope: opaque materials whose program reads unit 1 as a normal and whose third
//! binding no other family claimed.** A material whose normal sits on another unit
//! (`detonator_shield`'s unit 2, `detonator_ship_rich_iridescent`'s unit 0) or that
//! blends (the canopy glass, `nitro_perspex_new`) is a different program and is left
//! as it drew. Since 2026-10-21 (`hd-track-surface`) this includes a circuit's own
//! normal-mapped surfaces - `track_surface`'s road and the walls - whose vertex
//! program derives the same frame from the same `tangent` attribute.

use std::sync::Arc;

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{ModelTexture, TangentFrame, TextureSlots};

use super::skin::decode_texture;
use super::{Report, Textures};

/// The fragment unit the hull programs read their normal map on.
const NORMAL_UNIT: u32 = 1;

/// Binds each ship hull material's normal map and returns, per material slot,
/// whether its chunks carry the tangent and how the vertex program scales the bitangent.
pub(super) fn hull_normal(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    third: &TextureSlots,
    report: &mut Report,
) -> (TextureSlots, Vec<TangentFrame>) {
    let mut maps: TextureSlots = vec![None; model.materials.len()];
    let mut marked = vec![TangentFrame::Absent; model.materials.len()];
    let mut decoded: std::collections::HashMap<(String, bool), Option<Arc<ModelTexture>>> =
        Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        // A material another family already gave the third binding (a magstrip's emissive picture,
        // a pad's mask, the ice's pond) reads that, not a normal map.
        if material.blend() != rcsmodel::Blend::Opaque
            || third.get(slot).is_some_and(Option::is_some)
        {
            continue;
        }
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let (Some(declared), Some(program)) = (
            rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
            rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset),
        ) else {
            continue;
        };
        let Some(&(hash, _)) = declared
            .samplers
            .iter()
            .find(|&&(_, unit)| unit == NORMAL_UNIT)
        else {
            continue;
        };
        // One map, one frame. A program that decodes a second unit as a normal (the sea
        // planes, two wave maps) combines them its own way and is not this reading.
        let another_normal = declared.samplers.iter().any(|&(_, unit)| {
            unit != NORMAL_UNIT
                && u8::try_from(unit).is_ok_and(|unit| {
                    program.samples_as_normal(unit) || program.samples_as_packed_normal(unit)
                })
        });
        let packed = !program.samples_as_normal(NORMAL_UNIT as u8)
            && program.samples_as_packed_normal(NORMAL_UNIT as u8)
            && !another_normal;
        if !packed && !program.samples_as_normal(NORMAL_UNIT as u8) {
            continue;
        }
        let Some(path) = material
            .samplers
            .iter()
            .find(|(h, p)| *h == hash && p.is_some())
            .and_then(|(_, p)| p.clone())
        else {
            report.hull_normal_unread += 1;
            continue;
        };
        let texture = decoded
            .entry((path.clone(), packed))
            .or_insert_with(|| {
                let bytes = textures(&format!("/{path}"))?;
                if packed {
                    // Decoded here, not through the sink: the unpacking reads the texels.
                    unpacked(&ModelTexture::from_gtf(&path, &bytes)?).map(Arc::new)
                } else {
                    decode_texture(&path, &bytes).map(Arc::new)
                }
            })
            .clone();
        let Some(texture) = texture else {
            report.hull_normal_unread += 1;
            continue;
        };
        maps[slot] = Some(texture);
        if packed {
            report.hull_normal_packed += 1;
        }
        // The vertex program says how it scales the bitangent: `2w - 1` or the raw `w`.
        marked[slot] = if rcsmaterial::vertex::Program::of(&blob, variant.vertex)
            .is_some_and(|p| p.bitangent_is_signed(rcsmodel::vertex_decl::TANGENT_HASH))
        {
            TangentFrame::Signed
        } else {
            TangentFrame::Raw
        };
        report.hull_normal_bound += 1;
    }
    (maps, marked)
}

/// A two-lane normal map (`x` in the alpha, `y` in the green) as the three-lane
/// picture `shade.wesl` reads: `(x, y, sqrt(1 - x^2 - y^2))`, each stored as
/// `(v + 1) / 2`.
fn unpacked(map: &ModelTexture) -> Option<ModelTexture> {
    let texels = map.to_rgba()?;
    let mut out = Vec::with_capacity(texels.len());
    for texel in texels.as_chunks::<4>().0 {
        let x = f32::from(texel[3]) / 255.0 * 2.0 - 1.0;
        let y = f32::from(texel[1]) / 255.0 * 2.0 - 1.0;
        let z = (1.0 - x * x - y * y).max(0.0).sqrt();
        out.extend([
            texel[3],
            texel[1],
            ((z * 0.5 + 0.5) * 255.0).round() as u8,
            0xff,
        ]);
    }
    Some(crate::mesh_render::offer_to_texture_sink(
        ModelTexture::rgba8(
            format!("unpacked normal {}", map.label),
            map.width,
            map.height,
            out,
            None,
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::Texels;

    fn map(texels: &[[u8; 4]]) -> ModelTexture {
        ModelTexture {
            label: "n".into(),
            width: texels.len() as u32,
            height: 1,
            texels: Texels::Rgba8(texels.concat()),
            mip_count: None,
        }
    }

    /// `x` comes from the alpha, `y` from the green, and `z` is what a unit
    /// vector leaves: a flat texel (both lanes 128) points straight up, and a
    /// texel tilted fully along `x` has no `z` left.
    #[test]
    fn a_two_lane_normal_unpacks_to_x_y_and_the_rebuilt_z() {
        let out = unpacked(&map(&[
            [9, 128, 77, 128],
            [0, 128, 0, 255],
            [0, 230, 0, 26],
        ]))
        .expect("an RGBA8 map unpacks");
        let texels = out.to_rgba().expect("the unpacked map is RGBA8");
        let texels = texels.as_chunks::<4>().0;
        assert_eq!(
            texels[0],
            [128, 128, 255, 255],
            "flat: red and blue are not read"
        );
        assert_eq!(texels[1], [255, 128, 128, 255], "all of it along x");
        let [x, y, z, _] = texels[2].map(|b| f32::from(b) / 255.0 * 2.0 - 1.0);
        assert!(
            (x + 0.796).abs() < 0.01 && (y - 0.804).abs() < 0.01,
            "{x} {y}"
        );
        // 1 - x^2 - y^2 is below zero here: the root is clamped, not NaN.
        assert!(z.abs() < 0.01, "{z}");
    }
}
