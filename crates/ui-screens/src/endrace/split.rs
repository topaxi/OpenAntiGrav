//! `EndRace Results Split` for a race two people flew: a lap table each.
//!
//! **What is authored.** Pulse's PS2 pressing (`EndRace_Definition.xml` in `WADS2.WAD`)
//! authors the screen: the ordinary `EndRace Results` table twice, one per person,
//! stacked in the 640x448 space (`<Item name="Player_0" OffsetY="-96">`, `Player_1` at
//! `110`), each with its own `Line1_N` headline, `table_N`, eight `tablebg{r}_N` rows
//! at a 25 px pitch, `lap{r}.{c}_N` cells, `boostimg_N` and `tablehighlight_N`; it also
//! keeps the one-person table (`TableSingle`) and a `BigTopText` hidden off screen. A
//! person's table is what a one-person race's is, so [`super::Results`] is the model
//! and [`super::draw::lap_cell_text`] the cell text, per person.
//!
//! **Chosen, not measured:** that the screen answers a split race (the original's entry
//! law is in an executable that is not in a Ghidra project here), the person's
//! highlight row (the totals row, as the one-person table), and the one-person table
//! staying hidden. The PSP has no such screen: it borrows this design, see
//! [`super::borrow`].
//!
//! `EndRace Rewards Split` is the Rewards panel twice (`PlayerRW_0` at `-52`,
//! `PlayerRW_1` at `176`), drawn by [`super::split_rewards_draw_list`].

use oag_ui::frontend::Placed;
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Layers, Picture, Skin};

use super::Layout;
use super::Results;
use super::draw::{headline_text, lap_cell_text};
use super::table::{Shown, lap_cell, split_suffix, table_layers};

/// The screen's name in `EndRace_Definition.xml`.
pub const RESULTS_SPLIT: &str = "EndRace Results Split";

/// `EndRace Rewards Split`'s name in the same file.
pub const REWARDS_SPLIT: &str = "EndRace Rewards Split";

/// `EndRace Results Split`'s draw list: `people[n]` in the table suffixed `_n`.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes"
)]
pub fn split_results_draw_list(
    people: &[Results],
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let shows: Vec<_> = people
        .iter()
        .enumerate()
        .map(|(person, results)| {
            let laps = results.laps.len();
            let shown = Shown {
                table: laps > 0,
                rows: laps + 1,
                header_bar: true,
                // The column is headed only for a person whose pads were counted.
                boost_icon: results.laps.iter().any(|lap| lap.boosts.is_some()),
                highlight: Some(laps + 1),
                also_highlight: 0,
            };
            (u8::try_from(person).ok(), shown)
        })
        .collect();
    table_layers(
        layout,
        skin,
        frame,
        backdrop,
        race_behind,
        sprites,
        &shows,
        &|text| {
            let name = text.name.as_deref().unwrap_or("");
            if let Some(cell) = lap_cell(name) {
                let person = people.get(usize::from(cell.table?))?;
                return lap_cell_text(name, person, strings, true);
            }
            match split_suffix(name) {
                ("Line1", Some(person)) => {
                    headline_text(people.get(usize::from(person))?.headline, strings)
                }
                // The one-person screen's own headline and title: hidden or off screen
                // on the original, and nothing here says them twice.
                ("Line1" | "BigTopText", _) => None,
                _ => text.string.clone(),
            }
        },
    )
}

#[cfg(test)]
mod tests;
