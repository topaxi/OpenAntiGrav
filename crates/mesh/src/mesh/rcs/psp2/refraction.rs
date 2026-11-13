//! 2048's and Omega's tunnel glass: a surface that reads the refraction target.
//!
//! `mt_tunnelrefraction` declares the engine-bound sampler `screenSpaceRefractionTex`
//! (`0x88a0df95`) and the record names no texture for it, exactly as HD's does
//! (`mesh::rcs::refraction`). Its Vita fragment program (both cached permutations, SHA-256 of
//! the GXP container under Vita3K's SPIR-V cache, walked symbolically; the formulas are
//! written by hand) ends on
//!
//! ```text
//! out = mix(fogColour, W * 2.0039 * D.a * G^2 + spec + lit * D^2, fog visibility)
//! ```
//!
//! where `W` is the record's `0x78575769` (`[0.2, 0.6, 0.6]`, HD's value), `D` the diffuse
//! picture, `G` the target texel and `lit` the lighting every material of the family shares.
//! HD's program is `fog(C + grab * W)` with `W = D.a * weight`: **the same law**, plus the
//! doubled factor, which the Vita's `2.0039` constant (half-float `0x4002`) carries here and
//! HD's `1.0` does not. This renderer's lit path already applies the `D^2` decode and the
//! target read is decoded as HD's is (`refraction.wesl`), so the entry is HD's with
//! [`PICTURE_SCALE`]. Confidence 80 (a symbolic read of the translated program; the
//! sample coordinate comes from the vertex program, which no cached permutation covers, so
//! the **coordinate law is HD's, inherited**: the surface pushed one unit along its normal,
//! projected through the target's matrix).
//!
//! **Omega: inherited from 2048.** Its `mt_tunnelrefraction` record is 2048's exactly (the
//! same two samplers and the same two parameters, 2048's values), its programs are GCN and
//! unread. Omega's `cl_tunnelrefraction` has no 2048 counterpart and takes HD's picture-less
//! law (`cl_tunnelrefraction`: lit term tinted by `DiffuseColour`, the grab doubled).

use oag_rcs::rcsmodel::psp2::material::Material;

use super::Report;
#[cfg(test)]
use crate::mesh::rcs::refraction::SCREEN_GRAB;
use crate::mesh::rcs::refraction::{DIFFUSE, DIFFUSE_COLOUR, WEIGHT, layer, table_index};
use crate::mesh::{Emissive, PROGRAM_ICE_GLASS, slots};

/// `Constant1`, the colour `rt_icetunnel`'s program multiplies its grab by.
const CONSTANT1: u32 = 0x7611_a2d8;
/// `Refraction`, how far along the normal `rt_icetunnel`'s program pushes its sample point.
const REFRACTION: u32 = 0x2c1d_4018;

/// The factor the Vita program multiplies a picture-weighted grab by.
pub const PICTURE_SCALE: f32 = 2.003_906_3;

/// Per material, the vertex bits that make its submeshes the glass.
#[derive(Default)]
pub(super) struct Plan {
    roles: Vec<u32>,
    /// Per material, whether it is `rt_icetunnel`, whose weight `a` rides each vertex's
    /// `colour.a` (the baked colour's red).
    ice: Vec<bool>,
}

impl Plan {
    /// Whether the material is `rt_icetunnel` and bound: its vertices carry `a` in `colour.a`.
    pub fn is_ice(&self, material: Option<usize>) -> bool {
        material
            .and_then(|m| self.ice.get(m))
            .copied()
            .unwrap_or(false)
    }

    /// The vertex bits for one material, `0` for any other.
    pub fn role(&self, material: Option<usize>) -> u32 {
        material
            .and_then(|m| self.roles.get(m))
            .copied()
            .unwrap_or(0)
    }
}

/// Whether the material is one of the family: a record that carries the grab's weight and
/// whose file stem is `mt_` or `cl_tunnelrefraction`.
fn is_glass(material: &Material) -> bool {
    let stem = file_stem(material);
    stem.eq_ignore_ascii_case("mt_tunnelrefraction")
        || stem.eq_ignore_ascii_case("cl_tunnelrefraction")
}

/// The material's file stem: its `.rcsmaterial` name without directory or extension.
fn file_stem(material: &Material) -> &str {
    let stem = material
        .name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&material.name);
    stem.trim_suffix(".rcsmaterial")
}

/// Whether the material is Omega's `rt_icetunnel` (Sebenco Climb's ice tunnel). Its GCN pixel
/// program reads `screenSpaceRefractionTex`; see [`ice_glass`].
fn is_ice(material: &Material) -> bool {
    file_stem(material).eq_ignore_ascii_case("rt_icetunnel")
}

/// The glass's own glow-table entry: `2 * Constant1 * a * grab` (see [`ice_glass`]).
fn ice_layer(constant1: [f32; 3], refraction: f32) -> Emissive {
    Emissive {
        tint: constant1,
        offset: 1.0,
        scale: 2.0,
        rate: 0.0,
        program: [PROGRAM_ICE_GLASS, refraction],
    }
}

/// Whether the material is glass whose program samples no diffuse picture
/// (`cl_tunnelrefraction`: its one sampler is a specular map). HD's picture-less program takes
/// the lit term untextured, tinted by `DiffuseColour`, so the builder binds no picture for it.
pub(super) fn is_pictureless(material: &Material) -> bool {
    is_glass(material) && material.sampler(DIFFUSE).is_none_or(str::is_empty)
}

/// Takes the glass back to the generic reading: clears the refraction bit and the table index
/// from every vertex. A circuit without a target (Zone mode, where nothing is left out of the
/// main view) has nothing for the glass to sample.
pub fn clear(model: &mut crate::mesh::Model) {
    let bits = slots::REFRACTION | (0x7f << slots::MATERIAL_SHIFT);
    for vertex in &mut model.vertices {
        if vertex.slots & slots::REFRACTION != 0 {
            vertex.slots &= !bits;
        }
    }
}

/// Admits each glass material, writing its glow-table entry into `emissive`.
pub(super) fn plan(
    materials: &[Material],
    emissive: &mut Vec<Emissive>,
    report: &mut Report,
) -> Plan {
    let mut out = Plan::default();
    for material in materials {
        out.roles.push(0);
        out.ice.push(false);
        let at = out.roles.len() - 1;
        if is_ice(material) {
            let constant1 = material.param(CONSTANT1).filter(|c| c.len() >= 3);
            let refraction = material.param(REFRACTION).and_then(|v| v.first().copied());
            let (Some(c), Some(refraction)) = (constant1, refraction) else {
                report.refraction_unread += 1;
                continue;
            };
            let entry = ice_layer([c[0], c[1], c[2]], refraction);
            let index = table_index(emissive, entry).and_then(|i| u32::try_from(i + 1).ok());
            let Some(index) = index else {
                report.refraction_unread += 1;
                continue;
            };
            out.roles[at] = slots::REFRACTION | (index << slots::MATERIAL_SHIFT);
            out.ice[at] = true;
            report.refraction_bound += 1;
            continue;
        }
        if !is_glass(material) {
            continue;
        }
        let weight = material.param(WEIGHT).filter(|w| w.len() >= 3);
        let Some(weight) = weight else {
            report.refraction_unread += 1;
            continue;
        };
        let picture = material.sampler(DIFFUSE).is_some_and(|p| !p.is_empty());
        let diffuse_colour = material
            .param(DIFFUSE_COLOUR)
            .and_then(|v| v.first().copied());
        let entry = layer(
            [weight[0], weight[1], weight[2]],
            picture,
            diffuse_colour,
            PICTURE_SCALE,
        );
        let index = table_index(emissive, entry).and_then(|i| u32::try_from(i + 1).ok());
        let Some(index) = index else {
            report.refraction_unread += 1;
            continue;
        };
        out.roles[at] = slots::REFRACTION | (index << slots::MATERIAL_SHIFT);
        report.refraction_bound += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_rcs::rcsmaterial::name_hash;

    #[test]
    fn the_hash_is_the_grab_samplers_name() {
        assert_eq!(name_hash("screenSpaceRefractionTex"), SCREEN_GRAB);
    }
}
