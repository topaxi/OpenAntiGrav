//! The samplers a material's shader reads for lighting alone, for the title
//! whose programs are not read.
//!
//! 2048's Vita materials carry their fragment programs, and
//! [`crate::gxp::sampler_use`] reads which samplers those programs declare only
//! in their lit permutations. **Omega's programs are PS4 GCN, in a container
//! nothing here parses**, so the same question cannot be asked of them. What
//! Omega shares with 2048 is the material *instance*: the same `.rcsmaterial`
//! name, the same sampler name hashes and the same parameter hashes in the same
//! table. [`INHERITED_FROM_2048`] is the 2048 answer for the families whose
//! Omega instance is byte-for-byte that record, and nothing else: a family
//! whose Omega instance differs (a different sampler set, an extra parameter)
//! is not matched and keeps drawing what it drew.
//!
//! **Inherited from 2048, not measured on Omega.** The shader sources are
//! recompiled for the PS4 (CLAUDE.md, "A 2048 finding is checked against
//! Omega"), so this is `checked, applies`: three titles agree where one is
//! readable (HD's `constantdiffuse_specular_normal` program samples `Texture1`
//! only through a `2c - 1` unpack into the light dot products, traced in
//! `docs/formats/rcsmaterial.md`; 2048's programs declare it in lit
//! permutations only; Omega's record is 2048's). The table is re-derived from
//! 2048's own programs and the Omega instances by
//! `crates/rcs/tests/psp2_sampler_use_ground_truth.rs`.

use super::material::Material;

/// One family's answer: the instance record that must match and what follows.
#[derive(Debug, PartialEq, Eq)]
pub struct Inherited {
    /// The material's file stem, lower case.
    pub family: &'static str,
    /// Its non-lightmap sampler name hashes, sorted.
    pub samplers: &'static [u32],
    /// Its parameter name hashes, in table order.
    pub params: &'static [u32],
    /// Of those samplers, the ones 2048's programs read for lighting alone.
    pub lighting_only: &'static [u32],
    /// The parameter 2048's ambient-only program names as the colour.
    pub colour: u32,
}

/// `Texture1`, the unnumbered sampler of the constant-diffuse families.
const TEXTURE1: u32 = 0x3bdc_0403;
/// `Constant1`, the diffuse constant.
const CONSTANT1: u32 = 0x7611_a2d8;

/// Sorted by family.
pub const INHERITED_FROM_2048: [Inherited; 3] = [
    Inherited {
        family: "constantdiffuse_specular_normal",
        samplers: &[TEXTURE1],
        params: &[CONSTANT1],
        lighting_only: &[TEXTURE1],
        colour: CONSTANT1,
    },
    Inherited {
        family: "ds_booth_glass",
        samplers: &[TEXTURE1],
        params: &[CONSTANT1],
        lighting_only: &[TEXTURE1],
        colour: CONSTANT1,
    },
    Inherited {
        family: "reflectplane_dc_seawater",
        samplers: &[0x06a7_7e50, 0x11cb_4f74],
        params: &[
            0x4d60_d566,
            0x5e5b_1937,
            0x370a_63cb,
            0x81e0_e773,
            0x906b_67ba,
        ],
        lighting_only: &[0x06a7_7e50, 0x11cb_4f74],
        colour: 0x4d60_d566,
    },
];

/// The family's file stem, lower case.
#[must_use]
pub fn family(material: &Material) -> String {
    material
        .name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&material.name)
        .trim_end_matches(".rcsmaterial")
        .trim_end_matches(".RCSMATERIAL")
        .to_ascii_lowercase()
}

/// The table row `material` matches exactly, if any.
#[must_use]
pub fn inherited(material: &Material) -> Option<&'static Inherited> {
    let family = family(material);
    let row = INHERITED_FROM_2048.iter().find(|r| r.family == family)?;
    let mut samplers: Vec<u32> = material
        .samplers
        .iter()
        .map(|(h, _)| *h)
        .filter(|h| *h != crate::rcsmaterial::LIGHTMAP_SAMPLER)
        .collect();
    samplers.sort_unstable();
    let params: Vec<u32> = material.params.iter().map(|p| p.hash).collect();
    (samplers == row.samplers && params == row.params).then_some(row)
}
