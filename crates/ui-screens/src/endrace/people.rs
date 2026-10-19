//! `EndRace Results` for a race several people flew: the split-screen table.
//!
//! **What is authored.** Pulse's `EndRaceResults_PopulateMultiplayerTable`
//! (`0x088d9588`) fills the same authored table every other populate fills with a
//! `PRO_POS` / `PRO_NAME` / `PRO_TOT_TIME` header and one row per craft: its place,
//! its name, and its finishing time, with `ER_RACING` where the craft had not
//! finished, and the player's row highlighted
//! (`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`). The original gates it
//! on the network-play modes, not on split screen (the PS2 pressing authors an
//! `EndRace Results Split` of its own instead: two stacked per-player lap tables, see
//! `docs/ui/split-end-race.md`), so **that this table answers a split-screen race is
//! chosen, not measured**, as is naming a person `PLAYER 1`, `PLAYER 2` by the seat
//! they flew. Everything inside the table, its header, its columns and its row
//! geometry, is the disc's.
//!
//! The one highlight widget the disc authors is drawn once per person's row
//! ([`super::table::Shown::also_highlight`]).

use oag_ui::frontend::Placed;
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Layers, Picture, Skin};

use super::Layout;
use super::draw::format_ticks;
use super::table::{LapCell, Shown, lap_cell, table_layers};

/// Whose row it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersonName {
    /// A person, by the seat they flew, 1-based: `PLAYER 1`.
    Player(u8),
    /// A person by the name they raced under (a network race): what the original's
    /// network-play table prints in `PRO_NAME`.
    Named(String),
    /// An opponent's team, by string-table id (its folder name), which the draw
    /// resolves to the display name as the Eliminator and Tournament tables do.
    /// `None` draws the cell absent: no team is known for the slot.
    Team(Option<String>),
}

/// One craft's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonRow {
    /// Race position, 1-based.
    pub place: u8,
    pub name: PersonName,
    /// The tick it crossed the line for the last time, or `None` for a craft still
    /// racing when the race ended.
    pub time_ticks: Option<u64>,
    /// Whether a person flew this craft: the row the highlight is drawn on.
    pub person: bool,
}

/// The table, in place order. At most eight rows are authored, the grid's size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeopleResults {
    pub rows: Vec<PersonRow>,
}

impl PeopleResults {
    fn name(&self, row: &PersonRow, strings: &StringTable) -> Option<String> {
        match &row.name {
            PersonName::Player(seat) => {
                Some(format!("{} {seat}", strings.get_or_id("OAG_JOIN_PLAYER")))
            }
            PersonName::Named(name) => Some(name.clone()),
            PersonName::Team(id) => id.as_deref().map(|id| strings.get_or_id(id).to_string()),
        }
    }

    /// A cell's text: the header row, a craft's place, name or time.
    fn cell(&self, name: &str, strings: &StringTable) -> Option<String> {
        let LapCell { row, column, .. } = lap_cell(name)?;
        if row == 0 {
            let id = match column {
                0 => "PRO_POS",
                1 => "PRO_NAME",
                2 => "PRO_TOT_TIME",
                _ => return None,
            };
            return Some(strings.get_or_id(id).to_string());
        }
        let craft = self.rows.get(row - 1)?;
        match column {
            0 => Some(craft.place.to_string()),
            1 => self.name(craft, strings),
            2 => Some(
                craft
                    .time_ticks
                    .map_or_else(|| strings.get_or_id("ER_RACING").to_string(), format_ticks),
            ),
            _ => None,
        }
    }
}

/// `EndRace Results` for a race several people flew. See the module doc.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes"
)]
pub fn people_results_draw_list(
    model: &PeopleResults,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let mut people = model
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.person)
        .map(|(index, _)| index + 1);
    let shown = Shown {
        table: true,
        rows: model.rows.len(),
        header_bar: true,
        boost_icon: false,
        highlight: people.next(),
        also_highlight: people.fold(0, |mask, row| mask | (1 << row)),
    };
    table_layers(
        layout,
        skin,
        frame,
        backdrop,
        race_behind,
        sprites,
        &[(None, shown)],
        &|text| {
            let name = text.name.as_deref().unwrap_or("");
            if name.starts_with("lap") {
                model.cell(name, strings)
            } else if name == "Line1" {
                // `BigTopText` already reads `ER_RES`; a headline under it would say
                // it twice, and no one place is the race's.
                None
            } else {
                text.string.clone()
            }
        },
    )
}
