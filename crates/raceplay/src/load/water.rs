//! `enhanced` water on a title's `.vex` circuit: the loader's half of `oag_mesh::mesh::vex_water`.

use oag_display::display::Water;
use oag_mesh::mesh::{self, Model};

/// Tags `track_model`'s sea under `enhanced`, where `title` lists this circuit's water
/// (`oag_title::water`); anything else leaves the model as built. Says what it did in the
/// loader report.
pub(super) fn enhanced(
    track_model: &mut Model,
    law: Water,
    title: &oag_title::Title,
    track: &str,
    report: &mut Vec<String>,
) {
    if law != Water::Enhanced {
        return;
    }
    let Some(names) = title.water.textures_of(track) else {
        return;
    };
    let tagged = mesh::vex_water::enhanced(track_model, names);
    report.push(format!(
        "{track}: enhanced water on {} texture(s), {} draw(s), {} vertex(es){}",
        tagged.slots,
        tagged.draws,
        tagged.vertices,
        if tagged.missing.is_empty() {
            String::new()
        } else {
            format!(" ({} not bound here)", tagged.missing.join(", "))
        }
    ));
}
