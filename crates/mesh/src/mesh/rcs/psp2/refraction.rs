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
use crate::mesh::{Emissive, slots};

/// The factor the Vita program multiplies a picture-weighted grab by.
pub const PICTURE_SCALE: f32 = 2.003_906_3;

/// Per material, the vertex bits that make its submeshes the glass.
#[derive(Default)]
pub(super) struct Plan {
    roles: Vec<u32>,
}

impl Plan {
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
    let stem = material
        .name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&material.name);
    let stem = stem.strip_suffix(".rcsmaterial").unwrap_or(stem);
    stem.eq_ignore_ascii_case("mt_tunnelrefraction")
        || stem.eq_ignore_ascii_case("cl_tunnelrefraction")
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
        if !is_glass(material) {
            continue;
        }
        let at = out.roles.len() - 1;
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
