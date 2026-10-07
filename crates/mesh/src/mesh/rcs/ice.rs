//! HD's Sebenco ice pool: `sebenco_ice.rcsmaterial`, a surface whose colour is
//! three authored constants chosen by a facing term and a pond mask, not a
//! picture.
//!
//! # What the program does
//!
//! Read off the resolved fogged variants (`scripts/ps3-microcode.py fp-file`
//! on `sebenco_ice.rcsmaterial`, forward lightmapped block `@0x111b0`, no
//! lightmap `@0x4480`), with `V` the unit vector to the eye, `N'` the normal
//! perturbed by two normal-map taps, and `F = V . N'`:
//!
//! ```text
//! mask  = tex(unit 1, TC4.zw).x           the pond transition map, second UV set
//! snow  = tex(unit 2, TC4.xy).rgb         and_snow3alpha
//! water = deep + F (cyan - deep)          0x1f3b345d, 0xef8869cd
//! ice   = pale + F (snow - pale)          0x4042b6e6
//! A     = water + mask (ice - water)
//! out   = fog(A * L + specular + reflection * w)
//! ```
//!
//! with `L` the lightmap lighting every baked surface uses. The reflection
//! weight `w` is `0xe7a83aef` and `0xa1b54b80`, authored **0** on this
//! material, so the engine's paraboloid probe (`0x9edd3243`) drops out and this
//! surface closes entirely from data.
//!
//! The three colours are authored `(0.0475, 0.106, 0.147)` deep,
//! `(0.131, 0.755, 0.872)` cyan and `(0.663, 0.763, 1.0)` pale. The pond mask
//! is dark in the pond and white around it, so the pond is the water layer and
//! the surround is ice.
//!
//! # What is approximated
//!
//! **Chosen, not measured:** `N'` is the vertex normal (the program bumps it
//! with `and_snow_norm` and `256norm4`), so `F` is smooth where the original's
//! streaks; the three constants are taken as display values and decoded to the
//! linear domain the albedo textures are sampled in; the program's own specular
//! term is left to the generic one.

use oag_rcs::rcsmaterial::fragment::Program;
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::water::PARABOLOID;
use super::{Report, Textures};

/// The pond mask's sampler, read one lane wide through the second UV set.
pub(super) const MASK: u32 = 0xf1d8_75a1;
/// The snow picture's sampler.
pub(super) const SNOW: u32 = 0x2e7d_71db;
/// The three authored colours, in table order: deep, cyan, pale.
const COLOURS: [u32; 3] = [0x1f3b_345d, 0xef88_69cd, 0x4042_b6e6];

/// Whether a resolved program is the ice combine: it declares the paraboloid
/// probe, the pond mask and the snow, and patches all three colours.
pub(super) fn is_ice(declared: &rcsmaterial::Declared, program: &Program) -> bool {
    let sampler = |h: u32| declared.samplers.iter().find(|&&(s, _)| s == h);
    let (Some(&(_, mask_unit)), Some(&(_, snow_unit))) = (sampler(MASK), sampler(SNOW)) else {
        return false;
    };
    sampler(PARABOLOID).is_some()
        && COLOURS.iter().all(|h| declared.parameters.contains(h))
        && !program.samples_colour(u8::try_from(mask_unit).unwrap_or(u8::MAX))
        && program.samples_colour(u8::try_from(snow_unit).unwrap_or(u8::MAX))
}

/// The second coordinate set of a submesh whose material is an ice material
/// (`roles` carries [`slots::ICE`]), and `None` for every other surface, so
/// nothing but the pool reads it.
pub(super) fn second_uv(
    roles: u32,
    mesh: &rcsmodel::Mesh,
    model_blob: &[u8],
    submesh: &rcsmodel::SubMesh,
    stride: usize,
) -> Option<Vec<[f32; 2]>> {
    (roles & slots::ICE != 0)
        .then(|| mesh.second_texcoords(model_blob, submesh, stride).ok())
        .flatten()
}

/// Binds each ice material's snow and pond mask, writes [`slots::ICE`] and the
/// three colours into consecutive glow-table entries, and returns the mask
/// slots (the third binding).
pub(super) fn ice(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    skins: &mut TextureSlots,
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> TextureSlots {
    if super::isolate::water_off() {
        return vec![None; model.materials.len()];
    }
    let mut masks: TextureSlots = vec![None; model.materials.len()];
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let (Some(declared), Some(program)) = (
            rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
            Program::parse(&blob, variant.fragment.offset),
        ) else {
            continue;
        };
        if !is_ice(&declared, &program) {
            continue;
        }
        let path_of = |hash: u32| {
            material
                .samplers
                .iter()
                .find(|(h, p)| *h == hash && p.is_some())
                .and_then(|(_, p)| p.clone())
        };
        let mut decode = |path: &str| {
            textures(&format!("/{path}"))
                .and_then(|bytes| decode_texture(path, &bytes))
                .map(std::sync::Arc::new)
        };
        let colours: Option<Vec<Emissive>> = COLOURS
            .iter()
            .map(|h| {
                material
                    .parameters
                    .iter()
                    .find(|p| p.hash == *h)
                    .map(|p| Emissive {
                        tint: [p.value[0], p.value[1], p.value[2]],
                        offset: 0.0,
                        scale: 0.0,
                        rate: 0.0,
                    })
            })
            .collect();
        let (Some(colours), Some(snow), Some(mask)) = (
            colours,
            path_of(SNOW).and_then(|p| decode(&p)),
            path_of(MASK).and_then(|p| decode(&p)),
        ) else {
            report.ice_unread += 1;
            continue;
        };
        let first = table
            .windows(3)
            .position(|w| w == colours.as_slice())
            .or_else(|| {
                (table.len() + 3 < EMISSIVE_LIMIT).then(|| {
                    table.extend(colours.iter().copied());
                    table.len() - 3
                })
            });
        let Some(first) = first else {
            report.ice_unread += 1;
            continue;
        };
        skins[slot] = Some(snow);
        masks[slot] = Some(mask);
        if let Some(word) = packed.get_mut(slot) {
            *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
            *word |= slots::ICE;
            *word |= u32::try_from(first + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        }
        report.ice_bound += 1;
    }
    masks
}
