//! HD's emissive-in-alpha families: the picture's own alpha is a glow mask.
//!
//! Six families share one sampler and one parameter: the picture binds to
//! `alpha_emissive` (`0x6f9e6faf`) and the material authors a colour for
//! `glowtinrt` (`0x8f3d0540`)
//! (`diffuse_emissive_alpha`, `diffuse_emissive_alpha_glow` and its `_v01`,
//! `dn_diffuse_specular_emiss`, `mr_diffuse_emissive_alpha_glow_specular`,
//! `vertclr_diff_emissive_alpha_glow`). Every lit variant of every one of them
//! multiplies the picture by the light and then runs
//!
//! ```text
//! MAD H.xyz, T.wwww, {glowtinrt}, H      ; T the picture's own fetch
//! ```
//!
//! before the fog and the closing `/2`: `picture.rgb * light + picture.a *
//! colour`. It is how Metropia's neon rings, Vineta K's and Ubermall's window
//! bands and Anulpha Pass's light strips glow whatever the light on them is
//! (`docs/formats/rcsmaterial.md`, "The picture's alpha times a colour").
//! The `ZoneMode` and `ZoneTrans` variants do not declare the parameter, so
//! `shade.wesl` drops the term in a Zone race.
//!
//! The term is read off the resolved program
//! ([`Program::alpha_gated_parameter`] on the picture's unit) and admitted
//! only for that sampler and parameter: other programs gate other constants
//! by a unit-0 alpha (`diffusetextureemissivealphascalar`, `animhexlights`)
//! and are not read here.
//!
//! **The generic additive glow steps aside on these programs.**
//! `dn_diffuse_specular_emiss` also ends `MAD H1.xyz, H5, H4, H1` with `H4`
//! its `SpecularTexture` (unit 1) and `H5` the specular term, which
//! [`Program::accumulates`] reads as an added second texture. It is the
//! specular's own tint, so a slot [`super::emissive::emissive`] gave that
//! reading loses it here. The specular map itself stays unbound.

use oag_rcs::rcsmaterial::{self, fragment::Program};
use oag_rcs::rcsmodel;

use crate::mesh::{Emissive, PROGRAM_PICTURE_ALPHA_EMISSIVE, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::Pick;
use super::{Report, Textures};

/// `alpha_emissive`: the sampler the six families bind their picture to.
pub(super) const PICTURE: u32 = 0x6f9e_6faf;
/// `glowtinrt` (the disc's spelling): the emissive colour the picture's alpha
/// is multiplied by. 2048's GXP copies of the same programs declare both names
/// in the clear.
pub(super) const COLOUR: u32 = 0x8f3d_0540;

/// Writes each admitted material's colour into the glow table as an entry of
/// [`PROGRAM_PICTURE_ALPHA_EMISSIVE`] and its index into `packed`.
pub(super) fn alpha_emissive(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    picks: &[Pick],
    textures: Textures<'_>,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    report: &mut Report,
) {
    if super::isolate::alpha_emissive_off() {
        return;
    }
    let mut blobs: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        // The bound picture has to be the sampler the term reads.
        let bound = picks
            .get(slot)
            .and_then(|pick| pick.albedo)
            .and_then(|entry| material.samplers.get(entry))
            .map(|&(hash, _)| hash);
        if bound != Some(PICTURE) {
            continue;
        }
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = blobs
            .entry(material.name.clone())
            .or_insert_with(|| textures(&format!("/{}", material.name)))
            .as_ref()
        else {
            continue;
        };
        let Some(program) = Program::parse(blob, variant.fragment.offset) else {
            continue;
        };
        let Some(colour) = gated_colour(&program) else {
            continue;
        };
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        // A slot another family's program reading holds keeps it.
        let held = slots::material_index(*word) as usize;
        let generic = held == 0
            || (*word & slots::ADD_SECOND != 0
                && table.get(held - 1).is_some_and(|e| e.program == [0.0; 2]));
        if !generic {
            report.alpha_emissive_unread += 1;
            continue;
        }
        let Some(value) = material.parameters.iter().find(|p| p.hash == colour) else {
            report.alpha_emissive_unread += 1;
            continue;
        };
        let layer = Emissive {
            tint: [value.value[0], value.value[1], value.value[2]],
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
            program: [PROGRAM_PICTURE_ALPHA_EMISSIVE, 0.0],
        };
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index.and_then(|i| u32::try_from(i + 1).ok()) else {
            report.alpha_emissive_unread += 1;
            continue;
        };
        *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
        *word |= index << slots::MATERIAL_SHIFT;
        report.alpha_emissive_bound += 1;
    }
}

/// [`COLOUR`] when `program` multiplies it by the alpha of its [`PICTURE`]
/// fetch and accumulates.
fn gated_colour(program: &Program) -> Option<u32> {
    let unit = program
        .declared
        .samplers
        .iter()
        .find(|&&(hash, _)| hash == PICTURE)
        .and_then(|&(_, unit)| u8::try_from(unit).ok())?;
    program
        .alpha_gated_parameter(unit)
        .filter(|&hash| hash == COLOUR)
}
