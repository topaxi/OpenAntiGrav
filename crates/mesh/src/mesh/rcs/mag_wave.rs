//! HD's magstrip wave: an emissive picture dodged by a scrolling texture.
//!
//! Every `mageffect08`, `mageffectloop` and `mag_effect_loop_opaque` fragment
//! program on the disc declares the same five inputs this module keys on - the
//! emissive picture (sampler `0x1202d8df`), the wave texture (`0x85c9fd48`,
//! `ds_mag_wave_c.gtf`), the engine `time`, `Colour` (`0x02ab9f07`) and a scale
//! `k` (`0x220cf0e6`) - and read them as, in the lit blocks
//! (`scripts/ps3-microcode.py fp-file`, block `@0x6ae0` of
//! `mag_effect_loop_opaque`, `@0x6b20` of `mageffectloop`, `@0x61d0` of
//! `mageffect08_floor`):
//!
//! ```text
//! MAD  R.xy, uv, {k}, time        ; the same time added to both axes
//! TEX  wave, R.xy, unit 3|4
//! TEX  e, uv, unit 0|2            ; the emissive picture
//! m = e.rgb + e.a * (wave - e.rgb)
//! d = e.rgb / (1 - m * Colour)    ; MAD 1 - x*c, then RCP, per channel
//! ```
//!
//! `time` is the engine's seconds clock (`renderer.md`, "the engine's own
//! parameter table"), added with no multiplier, so the wave repeats once a
//! second on both axes. `mesh.wgsl` reads `d` through [`slots::MAG_WAVE`]; the
//! emissive picture rides in the third binding and the wave in the fourth,
//! `k`, `Colour` and the rate (1, read) in the material's glow-table entry.
//! The material's `ADD_SECOND` is cleared: its second texture is not a glow.

use std::sync::Arc;

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, ModelTexture, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// The emissive picture's sampler hash.
pub(super) const EMISSIVE_SAMPLER: u32 = 0x1202_d8df;
/// The wave texture's sampler hash.
pub(super) const WAVE_SAMPLER: u32 = 0x85c9_fd48;
/// `~crc32("time")`.
const TIME: u32 = 0x906b_67ba;
/// `Colour`: what the wave is multiplied by inside the dodge.
const COLOUR: u32 = 0x02ab_9f07;
/// `k`: what the coordinate is scaled by before the clock is added.
const SCALE: u32 = 0x220c_f0e6;
/// The strip floor's grid sampler (`glass_etched_tech.gtf`) and its facing
/// ramp (`dc_iridescent_gradient.gtf`): present together only on the two
/// floor-loop materials, which pick [`slots::MAG_LOOP`].
const GRID_SAMPLER: u32 = 0xa2d5_55b9;
const RAMP_SAMPLER: u32 = 0xcc98_c527;
/// `c`, the constant added to the facing ramp before the light multiplies it.
const RAMP_BIAS: u32 = 0x6c57_ba63;

/// Whether a resolved program takes the wave, by what it declares.
fn takes_wave(declared: &rcsmaterial::Declared) -> bool {
    let sampler = |h: u32| declared.samplers.iter().any(|&(s, _)| s == h);
    sampler(EMISSIVE_SAMPLER)
        && sampler(WAVE_SAMPLER)
        && [TIME, COLOUR, SCALE]
            .iter()
            .all(|h| declared.parameters.contains(h))
}

/// Binds each magstrip material's emissive picture and wave, and writes
/// [`slots::MAG_WAVE`] with its glow-table index into `packed`. Returns the
/// two slot lists (emissive, wave).
pub(super) fn mag_wave(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    (skins, seconds): (&mut TextureSlots, &mut TextureSlots),
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> (TextureSlots, TextureSlots) {
    let mut emissive: TextureSlots = vec![None; model.materials.len()];
    let mut wave: TextureSlots = vec![None; model.materials.len()];
    let mut decoded: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        Default::default();
    let mut load = |path: &str, textures: Textures<'_>| {
        decoded
            .entry(path.to_string())
            .or_insert_with(|| {
                textures(&format!("/{path}"))
                    .and_then(|bytes| decode_texture(path, &bytes))
                    .map(Arc::new)
            })
            .clone()
    };
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.fragment.offset) else {
            continue;
        };
        if !takes_wave(&declared) {
            continue;
        }
        let path_of = |hash: u32| {
            material
                .samplers
                .iter()
                .find(|(h, p)| *h == hash && p.is_some())
                .and_then(|(_, p)| p.clone())
        };
        let value_of = |hash: u32| material.parameters.iter().find(|p| p.hash == hash);
        let (Some(e_path), Some(w_path), Some(colour), Some(scale)) = (
            path_of(EMISSIVE_SAMPLER),
            path_of(WAVE_SAMPLER),
            value_of(COLOUR),
            value_of(SCALE),
        ) else {
            report.mag_wave_unread += 1;
            continue;
        };
        let (Some(e_tex), Some(w_tex)) = (load(&e_path, textures), load(&w_path, textures)) else {
            report.mag_wave_unread += 1;
            continue;
        };
        // The floor loop binds its grid and ramp where the generic reading put
        // the diffuse and the lightmap, and carries `c` in `offset`.
        let is_loop = declared.samplers.iter().any(|&(s, _)| s == GRID_SAMPLER)
            && declared.samplers.iter().any(|&(s, _)| s == RAMP_SAMPLER);
        let floor = if is_loop {
            match (
                path_of(GRID_SAMPLER).and_then(|p| load(&p, textures)),
                path_of(RAMP_SAMPLER).and_then(|p| load(&p, textures)),
                value_of(RAMP_BIAS),
            ) {
                (Some(grid), Some(ramp), Some(bias)) => Some((grid, ramp, bias.value[0])),
                _ => {
                    report.mag_wave_unread += 1;
                    continue;
                }
            }
        } else {
            None
        };
        let layer = Emissive {
            tint: [colour.value[0], colour.value[1], colour.value[2]],
            offset: floor.as_ref().map_or(0.0, |f| f.2),
            scale: scale.value[0],
            rate: 1.0,
        };
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            report.mag_wave_unread += 1;
            continue;
        };
        if let Some((grid, ramp, _)) = floor {
            skins[slot] = Some(grid);
            seconds[slot] = Some(ramp);
        }
        emissive[slot] = Some(e_tex);
        wave[slot] = Some(w_tex);
        if let Some(word) = packed.get_mut(slot) {
            *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
            *word |= slots::MAG_WAVE;
            if is_loop {
                *word |= slots::MAG_LOOP;
            }
            *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        }
        report.mag_wave_bound += 1;
    }
    (emissive, wave)
}

/// Lays `from`'s bound slots over `into`, which may be shorter or empty.
pub(super) fn merge(into: &mut TextureSlots, from: TextureSlots) {
    if into.len() < from.len() {
        into.resize(from.len(), None);
    }
    for (slot, texture) in from.into_iter().enumerate() {
        if texture.is_some() {
            into[slot] = texture;
        }
    }
}
