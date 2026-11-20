//! The Vita materials whose vertex programs write `1 - v` into the diffuse coordinate.
//!
//! Four of 2048's material files do, in every permutation: Amphiseum's `track_surface` and
//! `track_surface_no_emissive`, Modesto Heights' `track_wall` and Tech De Ra's `track_surface`
//! (`oag_rcs::gxp::texcoord_flip`, `docs/formats/2048-material-params.md`, "The track wall that
//! writes `1 - v`"). Drawn with the file's own `v`, Modesto's wall showed the picture's
//! chevron band where the original shows its panel and hex rows. HD's circuits ship the same
//! materials and flip the same way (`skin::flips`, read off the RSX microcode).
//!
//! **Read per material file, off its own programs, never by name.** PS4 programs are GCN and
//! unread here, so a PS4 model flips nothing.

use oag_rcs::gxp::texcoord_flip::flips_v;
use oag_rcs::rcsmodel::psp2;

use super::Report;
use crate::mesh::rcs::Textures;

/// Which material slots flip `v`.
#[derive(Default)]
pub(super) struct Plan {
    flipped: Vec<bool>,
}

/// Reads each material file once for the flip.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    if ps4 {
        return Plan::default();
    }
    let mut by_file: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    let flipped: Vec<bool> = decoded
        .materials
        .iter()
        .map(|material| {
            *by_file
                .entry(material.name.as_str())
                .or_insert_with(|| textures(&material.name).is_some_and(|blob| flips_v(&blob)))
        })
        .collect();
    report.flipped_v_materials += flipped.iter().filter(|f| **f).count();
    Plan { flipped }
}

impl Plan {
    /// The diffuse coordinate of a vertex of `material`.
    pub(super) fn apply(&self, material: Option<usize>, [u, v]: [f32; 2]) -> [f32; 2] {
        if material.is_some_and(|m| self.flipped.get(m).copied().unwrap_or(false)) {
            [u, 1.0 - v]
        } else {
            [u, v]
        }
    }
}
