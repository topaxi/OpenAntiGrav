//! The track meshes 2048 draws only into its refraction target, never in the main view.
//!
//! `Environment_Load` (`0x8102f6d0`) matches each mesh object's name against
//! `wohdmeshrefract`. The match is a case-insensitive substring, and a hit sets
//! instance flag `0x20`. `TrackPass_FilterSubmesh` (`0x813b9670`) then rejects
//! that flag in the main view's pass kind 0, and draws only flags `0x30` into
//! the half-size target the glass samples as `screenSpaceRefractionTex`.
//! Confidence 82, evidence in
//! `docs/ghidra/functions/vita-2048-eu-v104/refraction-pass.md`. HD's
//! counterpart is chunk render flag `0x10` (`rcsmodel::RENDER_BEHIND_GLASS`).
//! Omega's loader (`0x015fb4c0`) sets the same flag into record `+0xa0` and its
//! filter `0x01605760` makes the same two tests (confidence 80,
//! `docs/ghidra/functions/ps4-omega-eu/refraction-pass.md`).
//!
//! **Outside Zone mode only.** The Zone passes (kinds 2 to 4 on the Vita at
//! `0x813b96e6`, 3 to 5 on Omega) never reach the `0x20` test, so a Zone race
//! keeps these meshes in its main view; the caller skips this in Zone.
//!
//! The rule is the environment track model's alone: the original applies it
//! in `Environment_Load` only, so a caller applies it to the circuit, never to
//! a craft or prop. This project draws no 2048 or Omega refraction target, so
//! these meshes are drawn nowhere.

use oag_rcs::rcsmodel::psp2;

use crate::mesh::Model;

/// The Maya export tag that sends a mesh to the refraction target alone.
pub const TAG: &str = "wohdmeshrefract";

/// Whether a mesh object of this name is left out of the main view.
#[must_use]
pub fn leaves_main_view(name: &str) -> bool {
    name.to_ascii_lowercase().contains(TAG)
}

/// Removes from `model` every draw whose mesh object carries [`TAG`].
/// `model` must be what [`super::build`] made of `model_blob`. Returns how
/// many mesh objects were left out.
#[must_use]
pub fn leave_out_of_main_view(model: &mut Model, model_blob: &[u8]) -> usize {
    let Ok(container) = psp2::container::read(model_blob) else {
        return 0;
    };
    let Some(cpu) = container.section(model_blob, 0) else {
        return 0;
    };
    let layout = psp2::nodes::Layout::for_ps4(psp2::is_ps4(model_blob));
    let Some(scene) = psp2::nodes::read(cpu, layout) else {
        return 0;
    };
    let out: Vec<bool> = scene
        .meshes
        .iter()
        .map(|m| leaves_main_view(&m.name))
        .collect();
    let keep = |d: &crate::mesh::DrawCall| {
        !d.chunk
            .and_then(|c| out.get(c as usize))
            .copied()
            .unwrap_or(false)
    };
    model.draws.retain(keep);
    model.alpha_tested_draws.retain(keep);
    model.transparent_draws.retain(keep);
    model.shine_draws.retain(keep);
    out.iter().filter(|o| **o).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tag_matches_anywhere_in_the_name_in_any_case() {
        assert!(leaves_main_view(
            "track_part_animation:jay_pumping_piston3_4_4_3__wohdmeshrefractShape"
        ));
        assert!(leaves_main_view("WOHDMeshRefract_Tower"));
        assert!(!leaves_main_view("wohdmeshreflect_Tower"));
        assert!(!leaves_main_view("tunnel_understruts5Shape"));
    }
}
