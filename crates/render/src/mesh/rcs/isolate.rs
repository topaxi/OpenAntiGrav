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
//! - `OAG_OPAQUE_ONLY=1` drops every chunk the material asks to be blended,
//!   which is the one-run answer to "is there anything solid behind all this
//!   see-through geometry at this camera pose".
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

use oag_formats::rcsmodel;
use std::sync::OnceLock;

/// The parsed environment, computed once.
struct Filter {
    skip: Vec<String>,
    only: Vec<String>,
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
    if filter.skip.is_empty() && filter.only.is_empty() && !filter.opaque_only {
        return false;
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
