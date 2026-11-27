//! The emissive of a lightmapped slot, for the families whose own program adds it.
//!
//! The generic glow reading (`super::emissive`) binds the added texture second, and on a
//! lightmapped slot the atlas holds that binding, so these families' lights went dark. Each
//! family here is bound third (or fourth, where the third holds the normal map) and tagged with
//! a glow-table entry `shade.wesl` adds on a plain program only.
//!
//! **A family is listed only when its own `HalfBrightIleLightmapSunSpot0SVC0` program was read**
//! (`docs/rendering/hd-zone-recolour.md`, "Window lights on a plain chunk" and "The other
//! lightmapped emissive families"). Three laws:
//!
//! - [`PROGRAM_LIGHTMAPPED_ADD`]: `light * D + E * tint`, `E` at the diffuse coordinate,
//!   ungated by the diffuse alpha ([`Law::Add`]);
//! - [`PROGRAM_LIGHTMAPPED_SCROLL`]: `light * D + D.a * E(u, (v + a) * b + time) * tint`
//!   ([`Law::Scroll`]);
//! - [`PROGRAM_LIGHTMAPPED_KEYED`]: the same with the material's own curve `k` in place of
//!   `time` ([`Law::Keyed`]).
//!
//! The Zone variants of every listed family declare no such sampler, except
//! `mr_shopwin_reflecttranspemis`, whose Zone law is unread.
//!
//! Left out, named: `tunnel_fx_noalpha`'s `X` term (facing ramp times specular map), so its
//! screen blend reduces to the add.

use std::sync::Arc;

use oag_rcs::rcsmaterial;
use oag_rcs::rcsmodel;

use crate::mesh::{
    ANIM_TRACK_LIMIT, AnimTrack, Emissive, PROGRAM_LIGHTMAPPED_ADD, PROGRAM_LIGHTMAPPED_KEYED,
    PROGRAM_LIGHTMAPPED_SCROLL, TextureSlots, slots,
};

use super::curve_track::ParamCurveTrack;
use super::emissive::{EMISSIVE_LIMIT, family};
use super::skin::decode_texture;
use super::{Report, Textures};

/// `EmissiveTexture`.
const EMISSIVE_TEXTURE: u32 = 0xb1f2_a176;
/// The circuit lightmap's sampler, which takes the second binding.
const LIGHTMAP: u32 = 0x37b5_db58;
/// The scrolling families' tint, `a` and `b`, and the engine's `time`.
const SCROLL_TINT: u32 = 0xe8bc_d7f5;
const SCROLL_OFFSET: u32 = 0x7825_6a45;
const SCROLL_SCALE: u32 = 0x7878_7596;
const TIME: u32 = 0x906b_67ba;
/// The keyed family's `k`, which takes `time`'s place in its program and which the material's
/// curve drives.
const KEY: u32 = 0x1c5d_e6ec;
/// Modesto Heights' compile of `uvanim_diffuse_emissive` also declares a scale over both
/// coordinates and a speed on `time`: `(u, (v + a) * b + speed * time) * k`.
const SCROLL_UV_SCALE: u32 = 0xa24b_c055;
const SCROLL_SPEED: u32 = 0xf0d9_0109;

/// Glow-table entries left free for the readers that run after material setup: the circuit's pad
/// masks (`pads::bind_scene_pad_masks`, up to three) and the advert cards' Zone exemption
/// (`oag_render::gantry`, one). Without them Modesto Heights' light bars go dark.
const LATER_READERS: usize = 4;

/// Which of the two laws a family's program runs.
#[derive(Clone, Copy)]
enum Law {
    /// `tint` names the parameter that multiplies the sample, `None` where nothing does.
    Add {
        tint: Option<u32>,
    },
    Scroll,
    Keyed,
}

/// Where the picture is bound.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Binding {
    Third,
    /// The third holds the family's normal map (`track_surface`, `track_wall`).
    Fourth,
}

struct Family {
    name: &'static str,
    sampler: u32,
    law: Law,
    binding: Binding,
}

const fn add(name: &'static str, sampler: u32, tint: Option<u32>) -> Family {
    Family {
        name,
        sampler,
        law: Law::Add { tint },
        binding: Binding::Third,
    }
}

const fn scroll(name: &'static str) -> Family {
    Family {
        name,
        sampler: EMISSIVE_TEXTURE,
        law: Law::Scroll,
        binding: Binding::Third,
    }
}

const fn keyed(name: &'static str) -> Family {
    Family {
        name,
        sampler: EMISSIVE_TEXTURE,
        law: Law::Keyed,
        binding: Binding::Third,
    }
}

const fn track(name: &'static str) -> Family {
    Family {
        name,
        sampler: EMISSIVE_TEXTURE,
        law: Law::Add { tint: None },
        binding: Binding::Fourth,
    }
}

const FAMILIES: &[Family] = &[
    add("tunnel_fx_noalpha", EMISSIVE_TEXTURE, None),
    add("tunnel_fx_noalpha_mult", EMISSIVE_TEXTURE, None),
    add(
        "diffuse_emissive_with_specular_from_alpha",
        0x3bdc_0403,
        None,
    ),
    add(
        "diffuse_with_specular_from_alpha_scalar",
        0x3bdc_0403,
        Some(0x6a30_e9cd),
    ),
    add(
        "diffuseemissivealphaspecular",
        EMISSIVE_TEXTURE,
        Some(0x4d60_d566),
    ),
    add(
        "diffuse_spec_fresnel_emissive",
        0xa2d5_55b9,
        Some(0x03e5_5ee0),
    ),
    add("and_diffuse_emissive", 0xfa79_b1cd, Some(0xb6a7_c284)),
    add("and_diffuse_emissive_spec", 0xfa79_b1cd, Some(0xb6a7_c284)),
    add(
        "mr_shopwin_reflecttranspemis",
        0xd274_75a1,
        Some(0x9ce1_ba7e),
    ),
    scroll("uvanim_diffuse_emissive"),
    scroll("mt_uvanim_diffuse_emissive"),
    scroll("mt_uvanim_diffuse_emissive2"),
    keyed("mt_uvanim_diffuse_emissive3tokey"),
    track("track_surface"),
    track("track_wall"),
];

/// The pictures bound, per binding, indexed by material slot.
pub(super) struct Bound {
    pub(super) third: TextureSlots,
    pub(super) fourth: TextureSlots,
}

/// The animation rows the keyed materials' curves play on, one per material slot: slots that
/// share a `.rcsmaterial` name author different curves.
struct Keys<'a> {
    model_blob: &'a [u8],
    tracks: &'a mut Vec<AnimTrack>,
    blob: Option<Arc<[u8]>>,
}

impl Keys<'_> {
    /// A new row playing `material`'s curve; `None` where the table is full.
    fn row(&mut self, material: &rcsmodel::Material) -> Option<usize> {
        if self.tracks.len() + 1 >= ANIM_TRACK_LIMIT {
            return None;
        }
        let blob = self
            .blob
            .get_or_insert_with(|| Arc::from(self.model_blob))
            .clone();
        self.tracks
            .push(AnimTrack::Param(ParamCurveTrack::of(material, blob, KEY)));
        Some(self.tracks.len())
    }
}

/// The entry `spec`'s program draws this material with, or `None` where a value it reads is not
/// in the material record.
fn entry(
    spec: &Family,
    material: &rcsmodel::Material,
    declared: &rcsmaterial::Declared,
    keys: &mut Keys<'_>,
) -> Option<Emissive> {
    let value = |hash: u32| {
        material
            .parameters
            .iter()
            .find(|p| p.hash == hash)
            .map(|p| p.value)
    };
    let rgb = |hash: u32| value(hash).map(|v| [v[0], v[1], v[2]]);
    Some(match spec.law {
        Law::Add { tint } => Emissive {
            tint: match tint {
                Some(hash) => rgb(hash)?,
                None => [1.0, 1.0, 1.0],
            },
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
            program: [
                PROGRAM_LIGHTMAPPED_ADD,
                if spec.binding == Binding::Fourth {
                    1.0
                } else {
                    0.0
                },
            ],
        },
        Law::Scroll => {
            let declares = |hash: u32| declared.parameters.contains(&hash);
            if !declares(TIME) {
                return None;
            }
            let (uv_scale, speed) = if declares(SCROLL_UV_SCALE) || declares(SCROLL_SPEED) {
                (value(SCROLL_UV_SCALE)?[0], value(SCROLL_SPEED)?[0])
            } else {
                (1.0, 1.0)
            };
            Emissive {
                tint: rgb(SCROLL_TINT)?,
                offset: value(SCROLL_OFFSET)?[0],
                scale: value(SCROLL_SCALE)?[0] * uv_scale,
                rate: speed * uv_scale,
                program: [PROGRAM_LIGHTMAPPED_SCROLL, uv_scale],
            }
        }
        Law::Keyed => {
            if !declared.parameters.contains(&KEY) {
                return None;
            }
            #[allow(clippy::cast_precision_loss)]
            let row = keys.row(material)? as f32;
            Emissive {
                tint: rgb(SCROLL_TINT)?,
                offset: value(SCROLL_OFFSET)?[0],
                scale: value(SCROLL_SCALE)?[0],
                rate: 0.0,
                program: [PROGRAM_LIGHTMAPPED_KEYED, row],
            }
        }
    })
}

/// Binds each listed family's emissive on its lightmapped slots and points the slot at its
/// entry. `thirds` and `fourths` are what earlier families bound there: a slot holding one is
/// left alone and counted unread.
#[allow(clippy::too_many_arguments)]
pub(super) fn lightmapped_emissive(
    model: &rcsmodel::Model,
    model_blob: &[u8],
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    thirds: &TextureSlots,
    fourths: &TextureSlots,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    tracks: &mut Vec<AnimTrack>,
    report: &mut Report,
) -> Bound {
    let mut keys = Keys {
        model_blob,
        tracks,
        blob: None,
    };
    let mut bound = Bound {
        third: vec![None; model.materials.len()],
        fourth: vec![None; model.materials.len()],
    };
    for (slot, material) in model.materials.iter().enumerate() {
        let name = family(material);
        let Some(spec) = FAMILIES.iter().find(|f| f.name == name) else {
            continue;
        };
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        if *word & slots::SECOND_IS_LIGHTMAP == 0 {
            continue;
        }
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(declared) = textures(&format!("/{}", material.name))
            .and_then(|blob| rcsmaterial::Declared::parse(&blob, variant.fragment.offset))
        else {
            report.lightmapped_emissive_unread += 1;
            continue;
        };
        let declares = |hash: u32| declared.samplers.iter().any(|(h, _)| *h == hash);
        if !(declares(spec.sampler) && declares(LIGHTMAP)) {
            continue;
        }
        let (occupied, target) = match spec.binding {
            Binding::Third => (thirds, &mut bound.third),
            Binding::Fourth => (fourths, &mut bound.fourth),
        };
        if slots::material_index(*word) != 0 || occupied.get(slot).is_some_and(Option::is_some) {
            report.lightmapped_emissive_unread += 1;
            continue;
        }
        let picture = material
            .samplers
            .iter()
            .find(|(h, p)| *h == spec.sampler && p.is_some())
            .and_then(|(_, p)| p.clone())
            .and_then(|path| {
                textures(&format!("/{path}")).and_then(|bytes| decode_texture(&path, &bytes))
            });
        let index = entry(spec, material, &declared, &mut keys).and_then(|layer| {
            table.iter().position(|seen| *seen == layer).or_else(|| {
                (table.len() + 1 + LATER_READERS < EMISSIVE_LIMIT).then(|| {
                    table.push(layer);
                    table.len() - 1
                })
            })
        });
        let (Some(picture), Some(index)) = (picture, index) else {
            report.lightmapped_emissive_unread += 1;
            continue;
        };
        target[slot] = Some(Arc::new(picture));
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        report.lightmapped_emissive_bound += 1;
    }
    bound
}
