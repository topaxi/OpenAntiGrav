//! A PS3 circuit's `Speedup Pad`/`Weapon Pad` geometry.
//!
//! Its own module for the reason `geometry.rs`'s own doc comment gives: a
//! circuit's pad geometry needs a node-ordered pass entirely separate from
//! `mesh::rcs::build_scene`'s world-space one, and building both models here
//! keeps that decision in one place rather than repeated at each call site.

use super::*;

/// Both pad models for a PS3 source, through `mesh::rcs::build_pads`/
/// `build_weapon_pads` - see that pair's own doc comment for why a PS3
/// circuit needs a dedicated pass rather than `vex_geometry`'s ordinary one.
///
/// `geometry` is the `.rcsmodel` blob beside `track_blob`, already read by
/// the caller (it is what decided `ps3_geometry` was `Some` in the first
/// place).
pub(super) fn ps3_pad_models(
    archives: &mut oag_assets::Archives,
    track: &str,
    track_blob: &[u8],
    geometry: &[u8],
    report: &mut Vec<String>,
) -> Result<(Option<mesh::Model>, Option<mesh::Model>)> {
    let (pads, built) = mesh::rcs::build_pads(track, track_blob, geometry, &mut |path| {
        archives.read_name(path).ok()
    })?;
    report.push(format!("speedup pads: {}", built.describe()));
    let pad_model = (!pads.indices.is_empty()).then_some(pads);

    let (pads, built) = mesh::rcs::build_weapon_pads(track, track_blob, geometry, &mut |path| {
        archives.read_name(path).ok()
    })?;
    report.push(format!("weapon pads: {}", built.describe()));
    let weapon_pad_model = (!pads.indices.is_empty()).then_some(pads);

    Ok((pad_model, weapon_pad_model))
}
