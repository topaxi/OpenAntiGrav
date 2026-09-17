//! The countdown's own `<Mode3D><Model>` mesh.
//!
//! Split out of `load.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py` - `load.rs` was already at the ceiling and
//! the LeachBeam ribbon's own texture load needed the room. A move, with no
//! behaviour change.

use super::*;

/// See `crate::hud::countdown` for why this one is drawn through the mesh
/// pipeline rather than `crate::hud::draw::model_draw`'s baked-quad shortcut
/// every other `<Mode3D>` widget uses. Placed by this mode's own layout,
/// found by name rather than assumed present: Pure's HUD authors
/// `Ready_GO.vex` and no `Cockpit321Go` widget at all, so a title or mode
/// with no such widget has nothing to place this model by and gets none, the
/// same "absence is reported, not fatal" rule `weapon_models::load` follows.
pub(super) fn model(
    hud: &crate::hud::Assets,
    archives: &mut oag_assets::Archives,
    report: &mut Vec<String>,
) -> Option<(Model, crate::hud::Model)> {
    hud.layout
        .as_ref()
        .and_then(|layout| {
            layout
                .models
                .iter()
                .find(|model| model.name == "Cockpit321Go")
        })
        .filter(|widget| !widget.src.is_empty())
        .and_then(|widget| match archives.read_name(&widget.src) {
            Ok(blob) => match mesh::build(&widget.src, &blob) {
                Ok(model) => {
                    report.push(format!(
                        "{}: {} triangle(s), radius {:.2} - the countdown's own Mode3D \
                         model, placed at its widget's authored {:?}",
                        widget.src,
                        model.indices.len() / 3,
                        model.radius,
                        widget.position
                    ));
                    Some((model, widget.clone()))
                }
                Err(error) => {
                    report.push(format!("{}: did not decode ({error})", widget.src));
                    None
                }
            },
            Err(_) => {
                report.push(format!(
                    "{}: absent - the countdown draws nothing this race",
                    widget.src
                ));
                None
            }
        })
}
