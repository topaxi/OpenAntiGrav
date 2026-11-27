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
    (geometry, store): (&[u8], oag_gpu::formats::SceneStore),
    report: &mut Vec<String>,
) -> Result<(Option<mesh::Model>, Option<mesh::Model>)> {
    let (pads, built) = mesh::rcs::build_pads(
        track,
        track_blob,
        geometry,
        &mut crate::stages::reader(archives),
        store,
    )?;
    report.push(format!("speedup pads: {}", built.describe()));
    let pad_model = (!pads.indices.is_empty()).then_some(pads);

    let (pads, built) = mesh::rcs::build_weapon_pads(
        track,
        track_blob,
        geometry,
        &mut crate::stages::reader(archives),
        store,
    )?;
    report.push(format!("weapon pads: {}", built.describe()));
    let weapon_pad_model = (!pads.indices.is_empty()).then_some(pads);

    Ok((pad_model, weapon_pad_model))
}

/// A Detonator race's pads on a title that ships no Detonator pad model
/// (`oag_title::DetonatorPads::circuit_weapon_pad_key`): the race circuit's own first
/// `Weapon Pad`, drawn once at each of `places` and held at key `key` of the weapon pad's
/// ready cycle (`oag_render::weapon_pad::READY_KEYFRAMES`). The race circuit is `track`'s
/// race twin where the title derives one (a Zone circuit authors speed pads alone), else
/// `track` itself. `None`, with a report line, where nothing is placed or nothing builds.
pub(super) fn circuit_weapon_pads(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    track: &str,
    places: &[[f32; 16]],
    key: usize,
    report: &mut Vec<String>,
) -> Option<mesh::Model> {
    let donor = title
        .race
        .zone
        .race_of(track)
        .unwrap_or_else(|| track.to_string());
    let Some(colour) = oag_render::weapon_pad::READY_KEYFRAMES.get(key) else {
        report.push(format!(
            "Detonator pads: the weapon pad's ready cycle has no key {key}; no pad is drawn"
        ));
        return None;
    };
    let blob = match archives.read_name(&donor) {
        Ok(blob) => blob,
        Err(e) => {
            report.push(format!("Detonator pads: {donor}: {e}; no pad is drawn"));
            return None;
        }
    };
    let mut build = |external| match mesh::build_weapon_pads_placed(&donor, &blob, external, places)
    {
        Ok(model) => Some(model),
        Err(e) => {
            report.push(format!("Detonator pads: {donor}: {e}; no pad is drawn"));
            None
        }
    };
    let mut model = build(None)?;
    if !model.textures.is_empty()
        && model.textures.iter().all(Option::is_none)
        && let Some(external) = ps2_texture_set(archives, &donor)
    {
        model = build(Some(&external))?;
    }
    if model.indices.is_empty() {
        report.push(format!(
            "Detonator pads: {donor} authors no Weapon Pad; no pad is drawn"
        ));
        return None;
    }
    for vertex in &mut model.vertices {
        vertex.colour = [
            colour[0] / 255.0,
            colour[1] / 255.0,
            colour[2] / 255.0,
            vertex.colour[3],
        ];
    }
    report.push(format!(
        "Detonator pads: {} pad(s) carry {donor}'s weapon pad, held at its ready cycle's key {key}",
        model.node_vertex_ranges.len()
    ));
    Some(model)
}
