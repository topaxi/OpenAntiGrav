//! A diagnostic chunk filter, read once from the environment.
//!
//! **Off unless asked for, and never a rendering decision.** This exists for
//! one job: answering "what is actually painting these pixels" on a real race
//! frame, at a real camera pose, by taking geometry *out* of the build and
//! looking at what is left. It is the technique
//! `docs/formats/rcsmaterial.md`'s "The cloud plate paints a solid sheet"
//! section used to find that bug, kept rather than rewritten each time.
//!
//! - `OAG_SKIP_MATERIAL=clouds,glass` drops every chunk whose material path
//!   contains any of the comma-separated needles.
//! - `OAG_ONLY_MATERIAL=road` keeps only those and drops everything else.
//! - `OAG_ONLY_SLOT=331,341` keeps only those material **ordinals**, which is
//!   the exact form of the same question: a circuit names one material path
//!   in dozens of slots, and a tinted frame identifies a surface by ordinal,
//!   so the needle that follows up on it has to be an ordinal too. Ordinals
//!   are `oag_rcs::rcsmodel::Mesh::material`, the same numbering
//!   `crates/render/examples/hd_slot_list.rs` prints.
//! - `OAG_OPAQUE_ONLY=1` drops every chunk the material asks to be blended,
//!   which is the one-run answer to "is there anything solid behind all this
//!   see-through geometry at this camera pose".
//! - `OAG_TINT_MATERIALS=1` replaces every material's picture with a flat
//!   colour keyed by its slot ordinal, which answers "**what** is this pixel"
//!   - see [`tint`].
//! - `OAG_ALBEDO_ONLY=1` keeps every real picture but switches the light rig
//!   off, which answers the question that follows: "is this surface dark
//!   because its *art* is dark, or because the light reaching it is". Both
//!   this and `OAG_TINT_MATERIALS` work by clearing `GpuVertex::lit`, the
//!   flag `mesh.wesl` already reads to pick its unlit path, so neither needs a
//!   pipeline or a shader of its own.
//!
//! Both match the material's own archive path
//! (`data/environments/talons_junction/materials/clouds.rcsmaterial`) **or
//! either of the two `.gtf` paths it names**, case-insensitively, so a needle
//! can be a directory, a file stem or a fragment of either. Matching the
//! texture as well as the material matters: a circuit declares one material
//! path many times over with a different texture each time - Talon's Junction
//! names `track_surface.rcsmaterial` in fifteen slots - and the texture is
//! then the only thing that tells those slots apart. With both variables set,
//! `SKIP` is applied after `ONLY`.

use oag_rcs::rcsmodel;
use std::sync::OnceLock;

/// The parsed environment, computed once.
struct Filter {
    skip: Vec<String>,
    only: Vec<String>,
    only_slots: Vec<u32>,
    opaque_only: bool,
}

fn needles(var: &str) -> Vec<String> {
    std::env::var(var)
        .ok()
        .into_iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn filter() -> &'static Filter {
    static FILTER: OnceLock<Filter> = OnceLock::new();
    FILTER.get_or_init(|| Filter {
        skip: needles("OAG_SKIP_MATERIAL"),
        only: needles("OAG_ONLY_MATERIAL"),
        only_slots: needles("OAG_ONLY_SLOT")
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect(),
        opaque_only: std::env::var_os("OAG_OPAQUE_ONLY").is_some(),
    })
}

/// Whether a chunk is dropped from the build by the diagnostic filter.
///
/// Takes the model and the chunk rather than the material row so that both
/// call sites in [`super`] stay one line - that file sits against the
/// 1,000-line ceiling `scripts/check-file-size.py` enforces.
pub(super) fn excludes(model: &rcsmodel::Model, mesh: &rcsmodel::Mesh) -> bool {
    let filter = filter();
    if filter.skip.is_empty()
        && filter.only.is_empty()
        && filter.only_slots.is_empty()
        && !filter.opaque_only
    {
        return false;
    }
    if !filter.only_slots.is_empty() && !filter.only_slots.contains(&mesh.material) {
        return true;
    }
    let material = model.materials.get(mesh.material as usize);
    if filter.opaque_only && material.is_some_and(|m| m.blend() != rcsmodel::Blend::Opaque) {
        return true;
    }
    if filter.skip.is_empty() && filter.only.is_empty() {
        return false;
    }
    let paths: Vec<String> = material
        .into_iter()
        .flat_map(|m| {
            [
                Some(m.name.as_str()),
                Some(m.texture.as_str()),
                m.second_texture.as_deref(),
            ]
        })
        .flatten()
        .map(str::to_ascii_lowercase)
        .collect();
    let matches = |needles: &[String]| {
        needles
            .iter()
            .any(|n| paths.iter().any(|p| p.contains(n.as_str())))
    };
    if !filter.only.is_empty() && !matches(&filter.only) {
        return true;
    }
    matches(&filter.skip)
}

/// Whether every material's picture is replaced by a flat colour keyed by its
/// slot ordinal (`OAG_TINT_MATERIALS`).
pub(super) fn tinting() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("OAG_TINT_MATERIALS").is_some())
}

/// Whether to draw every surface's own picture with no light on it at all.
///
/// The complement of [`tinting`]: that one keeps the lighting question out of
/// the way to answer *which* material a pixel is, this one keeps the material
/// and takes the lighting out to answer *what its art looks like* before the
/// rig touches it. Reading the two frames against the lit one attributes a
/// black surface to its texture or to its light in two runs.
pub(super) fn unlit() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("OAG_ALBEDO_ONLY").is_some())
}

/// The flat colour material `slot` paints under [`tinting`].
///
/// **The cheap half of an ID render.** The technique
/// `docs/formats/rcsmaterial.md` used to find the cloud plate tints every
/// draw and reads the frame back; this tints every *material*, which needs no
/// pipeline of its own - the texture is simply replaced - and answers the
/// question that keeps coming up: a large surface that renders as a flat
/// black band, or a structure that looks absent, is identified by sampling
/// one pixel and matching its hue.
///
/// **It is a hue, not an exact value.** The lit path still multiplies by the
/// light rig, so the frame's colours are the palette *shaded*; the hue
/// survives that and the magnitude does not. A golden-ratio walk round the
/// wheel keeps neighbouring ordinals far apart, which matters because
/// adjacent slots are usually adjacent surfaces.
pub(super) fn tint(slot: usize) -> [u8; 4] {
    // 0.618034 is 1/phi: the standard low-discrepancy hue step.
    let hue = (slot as f32 * 0.618_034).fract() * 6.0;
    let sector = hue as u32;
    let f = hue - sector as f32;
    let (q, t) = (1.0 - f, f);
    let [r, g, b] = match sector {
        0 => [1.0, t, 0.0],
        1 => [q, 1.0, 0.0],
        2 => [0.0, 1.0, t],
        3 => [0.0, q, 1.0],
        4 => [t, 0.0, 1.0],
        _ => [1.0, 0.0, q],
    };
    [
        (r * 255.0) as u8,
        (g * 255.0) as u8,
        (b * 255.0) as u8,
        0xff,
    ]
}
