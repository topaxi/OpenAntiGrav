//! Which scenery glass reads the circuit's sky reflection map.
//!
//! `dc_windows_a`'s lit block (`@0x43d0`, traced `hd-one-pair-moa`) is
//! `rgb = fog(reflection * 0.8 + light * Texture1 + specular)`, the reflection being
//! `paraboloidReflectionTex` read at `reflect(-V, N)` through the hull's own dual-paraboloid
//! address, tinted by the engine's `iblScalePower` (`0x7480de6d`) and scaled by the material's
//! own weight, parameter `0x5e5b1937` (`0.8` on every instance of the family in Moa Therma).
//! The map is the circuit's sky (`mesh::sky_paraboloid`); this module tags the slots whose
//! program is that block, the way `probe_lit` tags a hull's, and `shade.wesl` adds the term.
//!
//! **`mt_windows_a` is the same window** (Anulpha Pass, lightmapped block #15 `@0xc6e0`, traced
//! `hd-glass-families`): its picture at unit 1, the normal map at unit 0, the sky at unit 3 times
//! the same weight parameter (`0.336` there), added to the lit picture and the specular, alpha the
//! picture's.
//!
//! **`glass_texture_customr` is a pane, not a window** (block #5 `@0x2db0`, the variant
//! Modesto Heights' and Talon's Junction's slots resolve to, traced `hd-glass-families`):
//! `rgb = fog(a * light * T1.rgb + (1 - a) * (specular + k * reflection)) / 2`, alpha `a`, with
//! `T1` the texture at `Texture1` (`0x3bdc0403`, unit 0) and `a` its alpha, `k` the material's
//! `Refbrightness` (`0xab31c2b1`), drawn `One`/`OneMinusSrcAlpha`. So the pane's picture and
//! coverage are `T1` and nothing else: the `j_sky.gtf` its entry 0 names (`0x8365b1f3`) is
//! declared by none of the material's twelve fragment blocks, which [`picture`] answers
//! for `skin::picks`. [`Mode::flags`] ride `tint.y` and `tint.z` of the entry.
//!
//! **`glass_2nduv_reflect_glow` is an opaque reflective window** (blocks #5 `@0x4380`, lightmapped,
//! and #21 `@0xc900`, traced on Chenghou Project): `rgb = fog(light * T.rgb + glow * (1 - G) +
//! (1 - T.a) * reflection) / 2`, alpha 1, no specular, with `T` the window picture at unit 1
//! (`0xc29b64e4`), `G` the alpha of the mask at unit 0 (`0x290a5adb`, the second coordinate set)
//! and `glow` the colour `0xfc21467a`. The reflection carries no weight parameter. The mask sits
//! at entry 0, so the generic reading bound it as the picture.
//!
//! **`ds_booth_glass` is an opaque mirror over a constant** (block #7, `@0x5f80` on Tech de Ra and
//! `@0x5ca0` on Sebenco Climb, the lightmapped variant both circuits' slots resolve to):
//! `rgb = fog(light * C + Texture1.a * sky) / 2`, alpha 0, drawn opaque, with `C` the constant
//! `0x7611a2d8` (black on every booth on the disc's race circuits) and `Texture1` the normal map
//! at unit 0. The reflection carries no weight parameter and its tint is `iblScalePower`'s, as
//! the window's. The light term and the gate both ride the picture `skin` builds for the family
//! (the constant, with the map's alpha), so the reflection is weighted by `a`.
//!
//! **Left out, named**: the program perturbs `N` with a tangent-space normal map (`Texture` at
//! unit 0) before the reflection and the specular, which a track vertex carries no frame for in
//! this renderer, so the reflection reads the vertex normal. A slot that already holds a glow
//! index keeps it and is counted, not overwritten.

use std::sync::Arc;

use oag_rcs::rcsmaterial;
use oag_rcs::rcsmodel;

use super::skin::decode_texture;
use super::{Report, Textures, slots};
use crate::mesh::{Emissive, ModelTexture, TextureSlots};

/// `~crc32("paraboloidReflectionTex")`.
const REFLECTION_SAMPLER: u32 = 0x9edd_3243;
/// `Texture1`, the pane's picture and coverage.
const TEXTURE1: u32 = 0x3bdc_0403;
/// `glass_2nduv_reflect_glow`'s window picture, unit 1 of both its race blocks.
const GLOW_WINDOW_PICTURE: u32 = 0xc29b_64e4;
/// `glass_2nduv_reflect_glow`'s glow mask, unit 0, its alpha read at the second coordinate set.
const GLOW_MASK: u32 = 0x290a_5adb;
/// `glass_2nduv_reflect_glow`'s glow colour, added times `1 - mask.a`.
const GLOW_COLOUR: u32 = 0xfc21_467a;

/// How a family's program combines the reflection with the rest of its sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// `dc_windows_a`: the reflection times the weight, added.
    Window,
    /// `glass_texture_customr`: the lit picture times `a`, the specular and the reflection
    /// times `1 - a`.
    Pane,
    /// `glass_2nduv_reflect_glow`: opaque, the reflection times `1 - a` added to the lit picture.
    Reflective,
    /// `ds_booth_glass`: opaque, the reflection times `a` added to the lit picture.
    Gated,
}

impl Mode {
    /// `tint.y` and `tint.z` of the entry: whether the lit picture is weighted by the coverage
    /// (and the specular by its complement), and what weights the reflection: `0` nothing, `1`
    /// `1 - a`, `2` `a`.
    fn flags(self) -> [f32; 2] {
        match self {
            Self::Window => [0.0, 0.0],
            Self::Pane => [1.0, 1.0],
            Self::Reflective => [0.0, 1.0],
            Self::Gated => [0.0, 2.0],
        }
    }
}

/// One family whose lit block reads the sky map: the parameter its weight is (`None` where the
/// program takes the reflection unscaled), the sampler its picture is when the generic picture
/// reading cannot tell, and how it combines.
struct Row {
    family: &'static str,
    weight: Option<u32>,
    picture: Option<u32>,
    mode: Mode,
    /// The mask sampler and colour parameter of a glow the program adds as
    /// `colour * (1 - mask.a)`, the mask at the second coordinate set.
    glow: Option<(u32, u32)>,
}

/// The families whose lit blocks are traced above.
const FAMILIES: &[Row] = &[
    Row {
        family: "dc_windows_a",
        weight: Some(0x5e5b_1937),
        picture: None,
        mode: Mode::Window,
        glow: None,
    },
    Row {
        family: "mt_windows_a",
        weight: Some(0x5e5b_1937),
        picture: None,
        mode: Mode::Window,
        glow: None,
    },
    Row {
        family: "glass_texture_customr",
        weight: Some(0xab31_c2b1),
        picture: Some(TEXTURE1),
        mode: Mode::Pane,
        glow: None,
    },
    Row {
        family: "glass_2nduv_reflect_glow",
        weight: None,
        picture: Some(GLOW_WINDOW_PICTURE),
        mode: Mode::Reflective,
        glow: Some((GLOW_MASK, GLOW_COLOUR)),
    },
    Row {
        family: "ds_booth_glass",
        weight: None,
        picture: None,
        mode: Mode::Gated,
        glow: None,
    },
];

fn row(material: &rcsmodel::Material) -> Option<&'static Row> {
    let family = super::emissive::family(material);
    FAMILIES.iter().find(|row| row.family == family)
}

/// Whether `material` is one of the families this module tags, whose texture sums the generic
/// emissive reading must not take for an additive layer: the sky map and the picture are all
/// they add.
pub(super) fn reads_sky(material: &rcsmodel::Material) -> bool {
    row(material).is_some()
}

/// The entry a family's program samples as its picture (and, for a pane, its coverage), when
/// its row names the sampler, the resolved variant declares it and the material names a `.gtf`
/// there.
pub(super) fn picture(
    material: &rcsmodel::Material,
    declared: &rcsmaterial::Declared,
) -> Option<usize> {
    let hash = row(material)?.picture?;
    if !declared.samplers.iter().any(|(h, _)| *h == hash) {
        return None;
    }
    material
        .samplers
        .iter()
        .position(|(h, path)| *h == hash && path.is_some())
}

pub use crate::mesh::WINDOW_REFLECTION_RATE;

/// Tags every slot of the families above, and returns the glow masks to bind third (`None`
/// where a slot has none). A family with a glow takes two consecutive entries: the window's,
/// with `offset` 1, and the glow colour's in the `tint` of the next.
pub(super) fn window_glass(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> TextureSlots {
    let mut masks: TextureSlots = vec![None; model.materials.len()];
    let mut decoded: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        Default::default();
    let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(row) = row(material) else {
            continue;
        };
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        let blob = cache
            .entry(material.name.clone())
            .or_insert_with(|| textures(&format!("/{}", material.name)))
            .clone();
        let Some(blob) = blob else { continue };
        let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.fragment.offset) else {
            continue;
        };
        if !declared
            .samplers
            .iter()
            .any(|(hash, _)| *hash == REFLECTION_SAMPLER)
        {
            continue;
        }
        let Some(weight) = row.weight.map_or(Some(1.0), |hash| {
            material
                .parameters
                .iter()
                .find(|p| p.hash == hash)
                .map(|p| p.value[0])
        }) else {
            report.window_glass_unread += 1;
            continue;
        };
        if slots::material_index(*word) != 0 {
            report.window_glass_blocked += 1;
            continue;
        }
        let glow = row.glow.and_then(|(mask, colour)| {
            let path = material
                .samplers
                .iter()
                .find(|(h, p)| *h == mask && p.is_some())
                .and_then(|(_, p)| p.clone())?;
            let picture = decoded
                .entry(path.clone())
                .or_insert_with(|| {
                    textures(&format!("/{path}"))
                        .and_then(|bytes| decode_texture(&path, &bytes))
                        .map(Arc::new)
                })
                .clone()?;
            let value = material.parameters.iter().find(|p| p.hash == colour)?.value;
            Some((picture, [value[0], value[1], value[2]]))
        });
        if row.glow.is_some() && glow.is_none() {
            report.window_glass_unread += 1;
            continue;
        }
        let layer = Emissive {
            tint: [weight, row.mode.flags()[0], row.mode.flags()[1]],
            offset: f32::from(u8::from(glow.is_some())),
            scale: 1.0,
            rate: WINDOW_REFLECTION_RATE,
            program: [0.0; 2],
        };
        let index = match &glow {
            // Two entries, consecutive; a pair another slot already laid is shared.
            Some((_, colour)) => {
                let second = Emissive {
                    tint: *colour,
                    offset: 0.0,
                    scale: 0.0,
                    rate: 0.0,
                    program: [0.0; 2],
                };
                table
                    .windows(2)
                    .position(|pair| pair[0] == layer && pair[1] == second)
                    .or_else(|| {
                        (table.len() + 2 < super::EMISSIVE_LIMIT).then(|| {
                            table.push(layer);
                            table.push(second);
                            table.len() - 2
                        })
                    })
            }
            None => table.iter().position(|seen| *seen == layer).or_else(|| {
                (table.len() + 1 < super::EMISSIVE_LIMIT).then(|| {
                    table.push(layer);
                    table.len() - 1
                })
            }),
        };
        let Some(index) = index else {
            report.window_glass_blocked += 1;
            continue;
        };
        if let Some((picture, _)) = glow {
            masks[slot] = Some(picture);
        }
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        report.window_glass_bound += 1;
    }
    masks
}
