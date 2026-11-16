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
//! a craft or prop.
//!
//! **The target is drawn** ([`split_refraction_target`]): the left-out meshes and the
//! `wohdmeshreflect` ones (flags `0x30`) go to the model the refraction target draws, which the
//! tunnel glass samples (`psp2::refraction`). Its size is half the main surface
//! ([`target_size`]); its camera (HD's 0.75 clip narrowing), single-sided drawing and the
//! glass's sample coordinate are **inherited from HD**, as the Vita's vertex program and target
//! camera are unread.

use oag_rcs::rcsmodel::psp2;

use super::super::scene::{keep_drawn_textures, single_sided};
use crate::mesh::Model;
use crate::mesh::rcs::BehindGlass;

/// The Maya export tag that sends a mesh to the refraction target alone.
pub const TAG: &str = "wohdmeshrefract";

/// Whether a mesh object of this name is left out of the main view.
#[must_use]
pub fn leaves_main_view(name: &str) -> bool {
    name.to_ascii_lowercase().contains(TAG)
}

/// The Maya export tag that sends a mesh into the refraction target while it stays in the main view.
pub const REFLECT_TAG: &str = "wohdmeshreflect";

/// Whether a mesh object of this name is drawn into the refraction target: flags `0x30`, the
/// refract and reflect tags (`TrackPass_FilterSubmesh`, pass kind 1).
#[must_use]
pub fn is_in_target(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.contains(TAG) || name.contains(REFLECT_TAG)
}

/// The refraction target's size in pixels: half the main surface (`FUN_81035b10` creates it
/// at `(width >> 1, height >> 1)`), kept at that size whatever the window's, as HD's is.
///
/// The Vita's main surface is 960x544 (Vita3K capture, `docs/ghidra/functions/vita-2048-eu-v104/exposure.md`),
/// so 480x272, measured. **Omega's target is not traced**: it takes the same halving of its
/// 1920x1080 surface, 960x540, inherited.
#[must_use]
pub const fn target_size(ps4: bool) -> (u32, u32) {
    if ps4 { (960, 540) } else { (480, 272) }
}

/// What the split of one track model came to.
#[derive(Debug, Default)]
pub struct Split {
    /// Mesh objects left out of the main view.
    pub out: usize,
    /// Mesh objects drawn into the refraction target (flags `0x30`).
    pub in_target: usize,
    /// The refraction target's draws, in two fog groups, each `None` when empty.
    pub target: BehindGlass,
}

/// Removes from `model` every draw whose mesh object carries [`TAG`] and returns them, with
/// the `0x30` meshes that stay in the main view, as the model the refraction target draws.
/// `model` must be what [`super::build`] made of `model_blob`.
///
/// **One fog group, the primary pair.** HD fogs the target's chunks with an alternate pair where
/// chunk flag `0x20` says so. 2048's and Omega's `.envsettings` author no alternate pair at all
/// (only `Lighting.Fog colour`), so there is no second pair to draw: the mesh tag the loader
/// turns into `0x1000` (`wohdsecondaryfog`, on 236 of Vineta K's 238 refract meshes, 200 of 212
/// on Omega) is recorded, and what the Vita's pass does with it is unread.
#[must_use]
pub fn split_refraction_target(model: &mut Model, model_blob: &[u8]) -> Split {
    let Ok(container) = psp2::container::read(model_blob) else {
        return Split::default();
    };
    let Some(cpu) = container.section(model_blob, 0) else {
        return Split::default();
    };
    let layout = psp2::nodes::Layout::for_ps4(psp2::is_ps4(model_blob));
    let Some(scene) = psp2::nodes::read(cpu, layout) else {
        return Split::default();
    };
    let names: Vec<String> = scene
        .meshes
        .iter()
        .map(|m| m.name.to_ascii_lowercase())
        .collect();
    let out: Vec<bool> = names.iter().map(|n| n.contains(TAG)).collect();
    let target: Vec<bool> = names.iter().map(|n| is_in_target(n)).collect();
    let flag = |list: &[bool], d: &crate::mesh::DrawCall| {
        d.chunk
            .and_then(|c| list.get(c as usize))
            .copied()
            .unwrap_or(false)
    };
    let mut part = model.clone();
    keep_draws(&mut part, |d| flag(&target, d));
    let drawn = !part.draws.is_empty()
        || !part.alpha_tested_draws.is_empty()
        || !part.transparent_draws.is_empty();
    let groups = BehindGlass {
        alternate_fog: None,
        primary_fog: drawn.then(|| {
            compact(&mut part);
            keep_drawn_textures(&mut part);
            single_sided(&mut part);
            part
        }),
    };
    let leave = |d: &crate::mesh::DrawCall| !flag(&out, d);
    model.draws.retain(leave);
    model.alpha_tested_draws.retain(leave);
    model.transparent_draws.retain(leave);
    model.shine_draws.retain(leave);
    Split {
        out: out.iter().filter(|o| **o).count(),
        in_target: target.iter().filter(|o| **o).count(),
        target: groups,
    }
}

/// Keeps the draws `keep` accepts, in every list.
fn keep_draws(model: &mut Model, keep: impl Fn(&crate::mesh::DrawCall) -> bool) {
    model.draws.retain(&keep);
    model.alpha_tested_draws.retain(&keep);
    model.transparent_draws.retain(&keep);
    model.shine_draws.retain(&keep);
}

/// Rewrites `model`'s vertices and indices to what its draws name, so a target group does not
/// carry the whole circuit's vertex buffer.
fn compact(model: &mut Model) {
    let mut remap: Vec<u32> = vec![u32::MAX; model.vertices.len()];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let lists = [
        &mut model.draws,
        &mut model.alpha_tested_draws,
        &mut model.transparent_draws,
        &mut model.shine_draws,
    ];
    for draw in lists.into_iter().flatten() {
        let first = u32::try_from(indices.len()).unwrap_or(u32::MAX);
        for &index in &model.indices[draw.range.start as usize..draw.range.end as usize] {
            let slot = &mut remap[index as usize];
            if *slot == u32::MAX {
                *slot = u32::try_from(vertices.len()).unwrap_or(u32::MAX);
                vertices.push(model.vertices[index as usize]);
            }
            indices.push(*slot);
        }
        draw.range = first..u32::try_from(indices.len()).unwrap_or(u32::MAX);
    }
    model.vertices = vertices;
    model.indices = indices;
    model.node_vertex_ranges.clear();
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

    #[test]
    fn the_target_takes_refract_and_reflect_meshes() {
        assert!(is_in_target("WOHDMeshRefract_Tower"));
        assert!(is_in_target("lamp_wohdmeshreflectShape"));
        assert!(!is_in_target("wohdtrack_road"));
    }

    #[test]
    fn the_target_is_half_the_main_surface() {
        assert_eq!(target_size(false), (960 / 2, 544 / 2));
        assert_eq!(target_size(true), (1920 / 2, 1080 / 2));
    }
}
