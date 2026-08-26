//! Which surface a circuit draws: its own art, or the ribbon this build derives.
//!
//! Its own module because `super` is at this project's 1,000-line ceiling, and
//! because the choice is one decision made in one place - a `.rcsmodel` that is
//! absent, one that will not decode, and one in a container this build reads
//! are three outcomes with one fallback between them.

use oag_render::mesh::{self, Model};

/// The circuit's authored geometry, or `None` when the caller should draw the
/// derived ribbon instead.
///
/// **Two containers wear the `.rcsmodel` extension.** Wipeout HD's needs the
/// `.vex` beside it - a chunk is addressed by hash and its vertex stride comes
/// from the node's authored box - and Wipeout 2048's needs neither, so it takes
/// a builder of its own rather than a branch inside HD's. See
/// `oag_render::mesh::rcs::psp2`.
///
/// A sibling that will not decode falls back exactly as a missing one does, and
/// the report says which happened: they need different work to fix.
pub(super) fn track_model(
    archives: &mut oag_assets::Archives,
    track: &str,
    track_blob: &[u8],
    geometry: &Option<Vec<u8>>,
    report: &mut Vec<String>,
) -> Option<Model> {
    match geometry {
        None => None,
        // **Two containers wear this extension.** Wipeout HD's needs the `.vex`
        // beside it - a chunk is addressed by hash and its stride comes from the
        // node's box - and Wipeout 2048's needs neither, so it takes a builder
        // of its own rather than a branch inside HD's. See
        // `oag_render::mesh::rcs::psp2`.
        Some(geometry) if mesh::rcs::psp2::is_psp2(geometry) => {
            match mesh::rcs::psp2::build(track, geometry) {
                Ok((model, built)) => {
                    report.push(format!("{track}: {}", built.describe()));
                    Some(model)
                }
                Err(error) => {
                    report.push(format!(
                        "{track}: the .rcsmodel beside it will not decode ({error:#}) - \
                         drawing the derived ribbon instead, as --ribbon does"
                    ));
                    None
                }
            }
        }
        Some(geometry) => match mesh::rcs::build_scene(track, track_blob, geometry, &mut |path| {
            archives.read_name(path).ok()
        }) {
            Ok((model, built)) => {
                report.push(format!("{track}: {}", built.describe()));
                Some(model)
            }
            Err(error) => {
                report.push(format!(
                    "{track}: the .rcsmodel beside it will not decode ({error:#}) - \
                     drawing the derived ribbon instead, as --ribbon does"
                ));
                None
            }
        },
    }
}
