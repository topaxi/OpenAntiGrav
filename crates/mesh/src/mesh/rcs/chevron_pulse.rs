//! HD's chevron pulse: `cf_chevron_pulse`'s emask and its swept outline.
//!
//! The lit block (`15_anulpha_pass/materials/cf_chevron_pulse.rcsmaterial`, fragment block #7
//! `@0xf6d0`, `scripts/ps3-microcode.py fp-file`) samples five units, all but the atlas at the
//! diffuse coordinate `f[TC4].xy`: the albedo (unit 0), its normal map (1), the emask (2,
//! sampler `0x1202d8df`), the lightmap atlas (3, `f[TC4].zw`) and the wave (4, `0x85c9fd48`):
//!
//! ```text
//! @0x15  TEX H6, f[TC4] unit2                          e = emask
//! @0x16  MUL H7.xyz, H6.wwww, [Colour 0x02ab9f07]
//! @0x1e  MOV R2.y, [offset 0xa6f5352f]
//! @0x27  MAD R0.xy, f[TC4].xy, [k 0x220cf0e6], R2.yy   uv * k + offset
//! @0x2f  TEX H0.xyz, R0 unit4                          w = wave
//! @0x37  MUL H7.xyz, H7, H0                            e.a * Colour * w
//! @0x43  ADD H6.xyz, H4.wwww, H6                       albedo.a + e.rgb
//! @0x44  MAD H5.xyz, H2, H6, H7                        light * that + outline
//! @0x49  MAD H2.xyz, H2, H4, H5                        + specular * albedo.rgb
//! @0x4a  MAD /2 H0.xyz, fog, H2, fogColour
//! ```
//!
//! `offset` is not a constant: the material's own Edge Animation curve drives it (one channel,
//! component 0, `offset = fract(t / 3)` on Anulpha Pass), so it rides an
//! [`AnimTrack::Param`] whose row the glow-table entry names, and `mesh.wesl` lays the wave's
//! coordinate in the vertex stage, where the animation table is bound. The emask rides the
//! fifth binding and the wave the sixth, so the normal map keeps the third
//! (`super::hull_normal`). The clock is the scenery's animation clock: **chosen, not
//! measured** (which clock a scenery material's curve runs on is unread).

use std::sync::Arc;

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{ANIM_TRACK_LIMIT, AnimTrack, Emissive, PROGRAM_CHEVRON, TextureSlots, slots};

use super::curve_track::ParamCurveTrack;
use super::emissive::EMISSIVE_LIMIT;
use super::mag_wave::{EMISSIVE_SAMPLER, WAVE_SAMPLER};
use super::skin::decode_texture;
use super::{Report, Textures};

/// `Colour`: what the emask's alpha and the wave are multiplied by.
const COLOUR: u32 = 0x02ab_9f07;
/// `k`: what the wave's coordinate is scaled by.
const SCALE: u32 = 0x220c_f0e6;
/// `~crc32("offset")`: what is added to it, driven by the material's curve.
const OFFSET: u32 = 0xa6f5_352f;
/// `~crc32("time")`: the magstrip family declares it and this one does not.
const TIME: u32 = 0x906b_67ba;

/// Whether a resolved program is the chevron's, by what it declares: the emask and the wave
/// with `offset` in place of the magstrips' clock.
fn takes_pulse(declared: &rcsmaterial::Declared) -> bool {
    let sampler = |h: u32| declared.samplers.iter().any(|&(s, _)| s == h);
    sampler(EMISSIVE_SAMPLER)
        && sampler(WAVE_SAMPLER)
        && [COLOUR, SCALE, OFFSET]
            .iter()
            .all(|h| declared.parameters.contains(h))
        && !declared.parameters.contains(&TIME)
}

/// Binds each chevron material's emask and wave and gives it a glow-table entry of mode
/// [`PROGRAM_CHEVRON`] carrying `Colour`, `k` and its offset track's row. Returns the fifth
/// and sixth binding lists (emask, wave).
#[allow(clippy::too_many_arguments)]
pub(super) fn chevron_pulse(
    model: &rcsmodel::Model,
    model_blob: &[u8],
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    tracks: &mut Vec<AnimTrack>,
    report: &mut Report,
) -> (TextureSlots, TextureSlots) {
    let mut emasks: TextureSlots = vec![None; model.materials.len()];
    let mut waves: TextureSlots = vec![None; model.materials.len()];
    let mut decoded: std::collections::HashMap<String, Option<Arc<crate::mesh::ModelTexture>>> =
        Default::default();
    let mut rows: std::collections::HashMap<String, usize> = Default::default();
    let mut blob: Option<Arc<[u8]>> = None;
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(material_blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let Some(declared) = rcsmaterial::Declared::parse(&material_blob, variant.fragment.offset)
        else {
            continue;
        };
        if !takes_pulse(&declared) {
            continue;
        }
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        // The program lights the albedo's alpha, which `skin` says with `LIT_ALPHA`; a slot
        // read otherwise, or one whose glow index is taken, keeps its reading.
        if *word & slots::LIT_ALPHA == 0 || slots::material_index(*word) != 0 {
            report.chevron_pulse_unread += 1;
            continue;
        }
        let path_of = |hash: u32| {
            material
                .samplers
                .iter()
                .find(|(h, p)| *h == hash && p.is_some())
                .and_then(|(_, p)| p.clone())
        };
        let mut load = |path: String| {
            decoded
                .entry(path.clone())
                .or_insert_with(|| {
                    textures(&format!("/{path}"))
                        .and_then(|bytes| decode_texture(&path, &bytes))
                        .map(Arc::new)
                })
                .clone()
        };
        let value_of = |hash: u32| material.parameters.iter().find(|p| p.hash == hash);
        let (Some(emask), Some(wave), Some(colour), Some(scale)) = (
            path_of(EMISSIVE_SAMPLER).and_then(&mut load),
            path_of(WAVE_SAMPLER).and_then(&mut load),
            value_of(COLOUR),
            value_of(SCALE),
        ) else {
            report.chevron_pulse_unread += 1;
            continue;
        };
        let row = match rows.get(&material.name) {
            Some(&row) => row,
            None if tracks.len() + 1 < ANIM_TRACK_LIMIT => {
                let blob = blob.get_or_insert_with(|| Arc::from(model_blob)).clone();
                tracks.push(AnimTrack::Param(ParamCurveTrack::of(
                    material, blob, OFFSET,
                )));
                rows.insert(material.name.clone(), tracks.len());
                tracks.len()
            }
            None => {
                report.chevron_pulse_unread += 1;
                continue;
            }
        };
        let layer = Emissive {
            tint: [colour.value[0], colour.value[1], colour.value[2]],
            #[allow(clippy::cast_precision_loss)]
            offset: row as f32,
            scale: scale.value[0],
            rate: 0.0,
            program: [PROGRAM_CHEVRON, 0.0],
        };
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            report.chevron_pulse_unread += 1;
            continue;
        };
        emasks[slot] = Some(emask);
        waves[slot] = Some(wave);
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        report.chevron_pulse_bound += 1;
    }
    (emasks, waves)
}
