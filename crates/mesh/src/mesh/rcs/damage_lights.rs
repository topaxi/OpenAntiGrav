//! HD's Zone-mode side light cells: `dg_zonelights1.rcsmaterial`, the hull's
//! damage lights.
//!
//! The program (`scripts/ps3-microcode.py fp-file`, block 17 `@0xbb50`, the
//! `HalfBrightIBLSunShadowMapSpot0SVC1` permutation the original draws it
//! with) adds a scrolling flash to the picture before the light multiplies it:
//!
//! ```text
//! uv2   = TC6.xy                          the second diffuse set
//! mask  = tex(unit 4, dg_flasha, (uv2.x + 0.4 * time, uv2.y - 0.09 * K))
//! blue  = tex(unit 5, dg_flash3, (uv2.x + 0.09 * K, uv2.y))
//! flash = mask * (blue - C) + C            C = 0xef18f362
//! P     = tex(unit 0, cf_damage_lights, uv) + flash
//! out   = P * (light + 1)
//! ```
//!
//! `time` is `0x906b67ba`, `C` is `0xef18f362` and `K` is `0xffede334`. The file
//! authors `C` and `K` as `0`, and so does the original for `C`: read live on
//! RPCS3 (Zone mode, zones 3 and 4, two frames) `C` is `(0, 0, 0)` and `K` is `6.0`
//! at zone 3 and `7.0` at zone 4. What `K` follows is not read.
//!
//! **Chosen, not measured:** `K` is the constant [`DAMAGE_STATE`] (the value read at
//! zone 3). `dg_flash3` is blue except one red corner, and `K` moves which part of
//! it a cell reads, so this draws the undamaged blue the original shows.
//!
//! The textures ride in the bindings the magstrips use (`blue` third, the mask
//! fourth); the role is [`slots::MAG_WAVE`] with [`DAMAGE_LIGHTS_RATE`] in the
//! glow entry's rate, which no magstrip entry carries.

use std::sync::Arc;

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, ModelTexture, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// `dg_flasha`'s sampler (unit 4): the scrolling mask.
const MASK_SAMPLER: u32 = 0x3bdc_0403;
/// `dg_flash3`'s sampler (unit 5): the blue.
const BLUE_SAMPLER: u32 = 0xa2d5_55b9;
/// `K`, the state the cells are drawn in.
const STATE: u32 = 0xffed_e334;
/// `C`, the colour the flash is lerped from.
const COLOUR: u32 = 0xef18_f362;
/// `time`.
const TIME: u32 = 0x906b_67ba;

/// The glow entry's `rate` that tells the shader this is a damage-lights material.
/// Between water's `2` and [`crate::mesh::ZONE_EXEMPT_RATE`], so no other test reads it.
pub const DAMAGE_LIGHTS_RATE: f32 = 3.0;
/// `K`, chosen (see the module doc).
pub(super) const DAMAGE_STATE: f32 = 6.0;
/// The coordinate shift per unit of `K`, a literal of the microcode.
const STATE_STEP: f32 = 0.09;
/// The mask's scroll rate in `u`, per second, a literal of the microcode.
const SCROLL_RATE: f32 = 0.4;

/// Whether a resolved program is the damage-lights combine, by what it declares.
fn takes_flash(declared: &rcsmaterial::Declared) -> bool {
    let sampler = |h: u32| declared.samplers.iter().any(|&(s, _)| s == h);
    sampler(MASK_SAMPLER)
        && sampler(BLUE_SAMPLER)
        && [STATE, COLOUR, TIME]
            .iter()
            .all(|h| declared.parameters.contains(h))
}

/// Binds each damage-lights material's blue and mask, writes its glow entry and
/// [`slots::MAG_WAVE`], and returns the two slot lists (blue, mask).
pub(super) fn damage_lights(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> (TextureSlots, TextureSlots) {
    let mut blues: TextureSlots = vec![None; model.materials.len()];
    let mut masks: TextureSlots = vec![None; model.materials.len()];
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
        if !takes_flash(&declared) {
            continue;
        }
        let path_of = |hash: u32| {
            material
                .samplers
                .iter()
                .find(|(h, p)| *h == hash && p.is_some())
                .and_then(|(_, p)| p.clone())
        };
        let mut decode = |path: &str| -> Option<Arc<ModelTexture>> {
            textures(&format!("/{path}"))
                .and_then(|bytes| decode_texture(path, &bytes))
                .map(Arc::new)
        };
        let colour = material.parameters.iter().find(|p| p.hash == COLOUR);
        let (Some(blue), Some(mask), Some(colour)) = (
            path_of(BLUE_SAMPLER).and_then(|p| decode(&p)),
            path_of(MASK_SAMPLER).and_then(|p| decode(&p)),
            colour,
        ) else {
            report.damage_lights_unread += 1;
            continue;
        };
        let entry = Emissive {
            tint: [colour.value[0], colour.value[1], colour.value[2]],
            offset: SCROLL_RATE,
            scale: STATE_STEP * DAMAGE_STATE,
            rate: DAMAGE_LIGHTS_RATE,
        };
        let index = table.iter().position(|seen| *seen == entry).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(entry);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            report.damage_lights_unread += 1;
            continue;
        };
        blues[slot] = Some(blue);
        masks[slot] = Some(mask);
        if let Some(word) = packed.get_mut(slot) {
            *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
            *word |= slots::MAG_WAVE;
            *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        }
        report.damage_lights_bound += 1;
    }
    (blues, masks)
}
