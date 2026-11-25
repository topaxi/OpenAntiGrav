//! A score Eliminator on a layout headed by the score over its target: `SCORE`, `18/50` and
//! `Name (18)` rows. 2048's native `Elimination_HUD.xml`, which Omega ships for 2048's circuits;
//! told apart from HD's by its `ScoreTxt` and no `PositionTxt2`. The bind and update that write
//! it: `docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`, "Other titles".

use super::{KillTag, Label, Layout, Readout};

/// The person's row colour, `0xff30ff30`.
const PERSON: [f32; 4] = [48.0 / 255.0, 1.0, 48.0 / 255.0, 1.0];
const OTHER: [f32; 4] = [1.0; 4];
/// The widget scale the update writes over a row's authored one.
const PERSON_SCALE: f32 = 0.6;
const OTHER_SCALE: f32 = 0.5;

/// Whether this frame's HUD is the score-over-target one.
fn applies(layout: &Layout, readout: &Readout) -> bool {
    readout.score_rows
        && layout.label("ScoreTxt").is_some()
        && layout.label("PositionTxt2").is_none()
}

/// The craft `PosTag<n>` names this frame.
fn row<'a>(layout: &Layout, readout: &'a Readout, name: &str) -> Option<&'a KillTag> {
    let row: usize = name.strip_prefix("PosTag")?.parse().ok()?;
    readout
        .kill_tags
        .get(row)
        .filter(|_| applies(layout, readout))
}

/// What `name` reads where this module decides it: `Some(None)` for a widget the original hides.
pub(super) fn text(
    layout: &Layout,
    readout: &Readout,
    name: &str,
    strings: &oag_ui::language::StringTable,
) -> Option<Option<String>> {
    if !applies(layout, readout) {
        return None;
    }
    if let Some(tag) = row(layout, readout, name) {
        return Some(Some(format!(
            "{} ({})",
            strings.get_or_id(&tag.team),
            tag.count
        )));
    }
    match name {
        "Score" => Some(
            readout
                .kill_tags
                .iter()
                .find(|tag| tag.player)
                .map(|tag| format!("{}/{}", tag.count, readout.kill_target)),
        ),
        // `PositionTxt` is hidden by the bind, and the mode's HUD flags (`0x10a93`) leave out
        // the position bit. The bind hides `TimeLimitParent` for a race with no time limit.
        "PositionTxt" | "Position" | "TimeLimitTxt" | "TimeLimit" => Some(None),
        _ => None,
    }
}

/// A row's own colour.
pub(super) fn colour(layout: &Layout, readout: &Readout, name: &str) -> Option<[f32; 4]> {
    row(layout, readout, name).map(|tag| if tag.player { PERSON } else { OTHER })
}

/// The factor that takes a row's authored scale to the one the update writes.
pub(super) fn scale(layout: &Layout, readout: &Readout, label: &Label) -> Option<f32> {
    let tag = row(layout, readout, &label.name)?;
    let written = if tag.player {
        PERSON_SCALE
    } else {
        OTHER_SCALE
    };
    (label.scale > 0.0).then(|| written / label.scale)
}

#[cfg(test)]
mod tests;
