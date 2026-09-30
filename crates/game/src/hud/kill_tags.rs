//! The Eliminator's kill column: `PosTag0` to `PosTag7`.
//!
//! `Elimination_HUD.xml` authors eight text rows at a fixed column
//! (`x=460`, `y=25..165`, `font="Default"`, right-aligned), with no `idstring`
//! and no `string`; the text is written every time a kill lands by
//! `FUN_0881af38` (`0x0881af38`, called from `Hud_Update` when the HUD's
//! `0x200` flag is set, which `hud+0x40` reads `0x229f` for on a live
//! Eliminator race). Each row is one craft:
//!
//! - **Ordered by kills, most first.** A craft's row is `count - 1 - (crafts
//!   with strictly fewer kills)`; when that row is taken it moves up one until
//!   it finds a free one, so **among equal kills the craft later in the grid
//!   array sits higher**. See [`ranked`].
//! - **`"<name> <kills>"`** for an opponent, in the `Default` face at alpha
//!   `0.8`, the name being the team's display name (`AG Systems`, `Goteki 45`
//!   on the live frame); **`"<name>  <kills>"`** (two spaces) for the player,
//!   at alpha `1.0`, the name being the profile's tag (`AAA` on the live
//!   frame).
//! - Rows past the field's size are never written, so they stay empty.
//!
//! The mode-`>= 14` half of the function (multiplayer names) and the
//! `hud+0x40 & 0x800` place list in `FUN_0881d458` are the same widgets in a
//! mode this build does not run. In every solo race the eight rows of
//! `Arcade_HUD.xml` are bound to empty text and never written, which is why
//! nothing draws for them there.
//!
//! **What is chosen, not measured.** This build has no profile tag, so the
//! player's row carries the player's *team's* display name. The grid order the
//! tie-break reads is this build's own (the original's is its random grid), so
//! which of two tied craft is on top can differ. The `0x200` flag the original
//! sets on the player's row (`Head2Head`'s player row throbs on the same flag)
//! is unread and not drawn.

use super::Readout;

/// One row of the kill column, before its name is looked up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillTag {
    /// The team id the craft flies (`Feisar`), which the string table turns
    /// into the name a player reads.
    pub team: String,
    /// The craft's kills.
    pub kills: u32,
    /// Whether this is the player's own craft.
    pub player: bool,
}

/// Opponents' rows are drawn at this alpha, the player's at `1.0`
/// (`0x3f4ccccd` and `0x3f800000` written to the widget's `+0xa4`/`+0xa8`).
const OPPONENT_ALPHA: f32 = 0.8;

/// The row each craft takes, top to bottom: `result[row]` is an index into
/// `kills`.
///
/// `FUN_0881af38`'s rule, read off its decompile: a craft's row is
/// `count - 1 - (number of crafts with strictly fewer kills)`, and a taken
/// row moves the craft up one at a time until a free one turns up, crafts
/// taken in array order.
#[must_use]
pub fn ranked(kills: &[u32]) -> Vec<usize> {
    let count = kills.len();
    let mut rows = vec![usize::MAX; count];
    for (craft, &mine) in kills.iter().enumerate() {
        let fewer = kills.iter().filter(|&&other| other < mine).count();
        let mut row = count - 1 - fewer;
        while rows[row] != usize::MAX {
            row -= 1;
        }
        rows[row] = craft;
    }
    rows
}

/// The row index `PosTag<n>` names.
fn row_of(name: &str) -> Option<usize> {
    name.strip_prefix("PosTag")?.parse().ok()
}

/// `name`'s text this frame, or `None` for a row nothing was written to.
pub(super) fn text(
    readout: &Readout,
    name: &str,
    strings: &oag_ui::language::StringTable,
) -> Option<String> {
    let tag = readout.kill_tags.get(row_of(name)?)?;
    let who = strings.get_or_id(&tag.team);
    Some(if tag.player {
        format!("{who}  {}", tag.kills)
    } else {
        format!("{who} {}", tag.kills)
    })
}

/// `KillsText`: the layout's caption and the kill target, `KILLS (5)`.
///
/// `Hud_BindWidgets` (`0x0881fbec`) formats it once at bind time as
/// `"%s (%d)"` (`0x08a7a0c8`) of the `IG_HUD_KILLS` string and the target - a
/// campaign cell's gold, else the race box's kill row. Read on the live frame
/// as `KILLS (5)`. Without a target (`Readout::kill_target` zero) it is the
/// caption alone.
pub(super) fn header(
    readout: &Readout,
    label: &super::Label,
    strings: &oag_ui::language::StringTable,
) -> Option<String> {
    let caption = super::draw::caption(label, strings)?;
    Some(match readout.kill_target {
        0 => caption,
        target => format!("{caption} ({target})"),
    })
}

/// `name`'s colour: the layout's own, with the alpha the row is written at.
pub(super) fn colour(readout: &Readout, name: &str, authored: [f32; 4]) -> Option<[f32; 4]> {
    let tag = readout.kill_tags.get(row_of(name)?)?;
    let alpha = if tag.player { 1.0 } else { OPPONENT_ALPHA };
    Some([authored[0], authored[1], authored[2], authored[3] * alpha])
}

#[cfg(test)]
mod tests;
