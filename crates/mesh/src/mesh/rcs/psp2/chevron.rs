//! Anulpha Pass's chevron and centre-lane glows on 2048 and Omega: an emissive mask and the
//! wave, added unlit.
//!
//! Both titles' chevron programs (`cf_chevron_pulse`, `cf_chevron_pulse_time`,
//! `cf_chevron_pulse_displacement2out`) end, after the albedo-alpha light (`super::lit_alpha`):
//!
//! ```text
//! rgb = light * DS.a + specular * DS.rgb + max(E.rgb, Colour * mix(E.rgb, W.rgb, E.a))
//! ```
//!
//! `E` the `Emiss_Mask` sampler, `W` the `Wave` sampler at `uv * UVScale + offset`
//! (`docs/rendering/hd-track-tone.md`, "2048 and Omega"). That is not HD's combine
//! (`super::super::chevron_pulse`), so it rides HD's glow-table mode [`PROGRAM_CHEVRON`] with
//! its second lane set to [`LINEAGE_COMBINE`], the emask on the fifth binding and the wave on the
//! sixth. Both colour samples go through the title's `decode`: the square on 2048, which is the
//! program's own `E * E` and `W * W`, and the 2.2 curve on Omega, whose sampler decodes them.
//!
//! **The wave offset is held**, at the material's authored `offset` (Omega) or 0 where none is
//! authored (2048): the curve that animates it is unread on both titles.
//!
//! **`Colour` is held at zero, chosen**, so the term draws its lower bound `E.rgb`. The live
//! value is unread on both titles (HD's was read off a hooked draw), and the authored one (Omega
//! `(0, 1.49, 0)`, 2048 `(0, 1, 0)`) paints every chevron plate and wall panel green at Omega's
//! grid, where the native-port original shows no green in any of its seven frames.
//!
//! **The centre lane, `cf_centre_plasma_pulse`**, lights its colour as every other surface does
//! and adds `E.a * W.rgb` (`E` the `Normal_EmissMask` sampler, a normal map whose alpha is the
//! mask; Omega `data01` `@0xb404`, `@0x320` to `@0x374`; 2048 #16 words 21 and 30, `W * W`). That
//! is HD's chevron glow with a white tint, so its entry's second lane is [`PLASMA_COMBINE`] and
//! carries no `LIT_ALPHA`; the normal map rides the fifth binding for its alpha and the wave the
//! sixth, at the same held offset.

use std::sync::Arc;

use oag_rcs::rcsmodel::psp2::{self, material::Material};

use super::Report;
use crate::mesh::rcs::Textures;
use crate::mesh::rcs::chevron_pulse::{COLOUR, OFFSET, SCALE};
use crate::mesh::rcs::mag_wave::{EMISSIVE_SAMPLER, WAVE_SAMPLER};
use crate::mesh::{ANIM_TRACK_LIMIT, AnimTrack, Emissive, ModelTexture, PROGRAM_CHEVRON, slots};

/// [`Emissive::program`]'s second lane on a [`PROGRAM_CHEVRON`] entry: the 2048/Omega combine,
/// `max(E, Colour * mix(E, W, E.a))` after the light, with no emask inside the light.
pub const LINEAGE_COMBINE: f32 = 1.0;

/// `Colour` as drawn: held at zero (see the module's doc).
const HELD_COLOUR: [f32; 3] = [0.0; 3];

/// [`Emissive::program`]'s second lane on a [`PROGRAM_CHEVRON`] entry: the centre lane,
/// `E.a * tint * W` after the light, with the surface lit by its colour.
pub const PLASMA_COMBINE: f32 = 2.0;

/// `~crc32("Normal_EmissMask")`: the centre lane's normal map, whose alpha is its mask.
const NORMAL_EMISS_MASK: u32 = 0xe4ee_aa66;

/// One admitted material: its glow-table slot (entry index plus one) and its two pictures.
pub(super) struct Pulse {
    pub slot: u32,
    pub emask: Arc<ModelTexture>,
    pub wave: Arc<ModelTexture>,
}

/// Per material, the pulse it binds.
#[derive(Default)]
pub(super) struct Plan {
    pub pulses: Vec<Option<Pulse>>,
}

impl Plan {
    pub fn of(&self, material: Option<usize>) -> Option<&Pulse> {
        self.pulses.get(material?)?.as_ref()
    }

    pub fn any(&self) -> bool {
        self.pulses.iter().any(Option::is_some)
    }

    /// The vertex bits that select this material's glow-table entry; `0` for any other.
    pub fn role(&self, material: Option<usize>) -> u32 {
        self.of(material)
            .map_or(0, |p| p.slot << slots::MATERIAL_SHIFT)
    }
}

/// What one admitted material reads: its mask's and wave's paths, `UVScale`, and its entry's
/// tint and second lane.
struct Reads<'m> {
    mask: &'m str,
    wave: &'m str,
    scale: f32,
    tint: [f32; 3],
    combine: f32,
}

/// What `material` reads, when it is an alpha-lit chevron that authors `Colour`, or the centre
/// lane.
fn reads(material: &Material) -> Option<Reads<'_>> {
    let stem = super::refraction::file_stem(material).to_ascii_lowercase();
    let path = |hash| material.sampler(hash).filter(|p| !p.is_empty());
    let scale = *material.param(SCALE)?.first()?;
    let (mask, tint, combine) = if stem == "cf_centre_plasma_pulse" {
        (path(NORMAL_EMISS_MASK)?, [1.0; 3], PLASMA_COMBINE)
    } else if super::lit_alpha::admits(material) && stem.contains("chevron_pulse") {
        material.param(COLOUR).filter(|c| c.len() >= 3)?;
        (path(EMISSIVE_SAMPLER)?, HELD_COLOUR, LINEAGE_COMBINE)
    } else {
        return None;
    };
    Some(Reads {
        mask,
        wave: path(WAVE_SAMPLER)?,
        scale,
        tint,
        combine,
    })
}

/// Admits each chevron material whose two pictures decode, writing its glow-table entry into
/// `emissive` and its held offset into `tracks`.
pub(super) fn plan(
    decoded: &psp2::Model,
    emissive: &mut Vec<Emissive>,
    tracks: &mut Vec<AnimTrack>,
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    let mut out = Plan::default();
    let mut cache: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for material in &decoded.materials {
        out.pulses.push(None);
        let Some(Reads {
            mask: emask_path,
            wave: wave_path,
            scale,
            tint,
            combine,
        }) = reads(material)
        else {
            continue;
        };
        let at = out.pulses.len() - 1;
        let mut picture = |path: &str| {
            cache
                .entry(path.to_string())
                .or_insert_with(|| super::picture_memo::picture(path, textures, report))
                .clone()
        };
        let (Some(emask), Some(wave)) = (picture(emask_path), picture(wave_path)) else {
            crate::mesh::rcs::census::claim(
                at,
                "chevron",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.chevron_unread += 1;
            continue;
        };
        let offset = material
            .param(OFFSET)
            .and_then(|v| v.first().copied())
            .unwrap_or(0.0);
        let row = match tracks
            .iter()
            .position(|t| matches!(t, AnimTrack::Held(v) if v.to_bits() == offset.to_bits()))
        {
            Some(i) => i + 1,
            None if tracks.len() + 1 < ANIM_TRACK_LIMIT => {
                tracks.push(AnimTrack::Held(offset));
                tracks.len()
            }
            None => {
                crate::mesh::rcs::census::claim(
                    at,
                    "chevron",
                    crate::mesh::rcs::census::Outcome::NotDrawn,
                );
                report.chevron_unread += 1;
                continue;
            }
        };
        #[allow(clippy::cast_precision_loss)]
        let entry = Emissive {
            tint,
            offset: row as f32,
            scale,
            rate: 0.0,
            program: [PROGRAM_CHEVRON, combine],
        };
        let index = emissive.iter().position(|e| *e == entry).or_else(|| {
            (emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.push(entry);
                emissive.len() - 1
            })
        });
        let Some(slot) = index.and_then(|i| u32::try_from(i + 1).ok()) else {
            crate::mesh::rcs::census::claim(
                at,
                "chevron",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.chevron_unread += 1;
            continue;
        };
        out.pulses[at] = Some(Pulse { slot, emask, wave });
        crate::mesh::rcs::census::claim(at, "chevron", crate::mesh::rcs::census::Outcome::Drawn);
        report.chevron_materials += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_rcs::rcsmaterial::name_hash;

    #[test]
    fn the_hashes_are_the_names_the_programs_declare() {
        assert_eq!(EMISSIVE_SAMPLER, name_hash("Emiss_Mask"));
        assert_eq!(WAVE_SAMPLER, name_hash("Wave"));
        assert_eq!(COLOUR, name_hash("Colour"));
        assert_eq!(SCALE, name_hash("UVScale"));
        assert_eq!(OFFSET, name_hash("offset"));
        assert_eq!(NORMAL_EMISS_MASK, name_hash("Normal_EmissMask"));
    }
}
