//! Pulse's Zone and Eliminator `EndRace Results`: the two tables that are not a
//! per-lap grid.
//!
//! `EndRaceResults_OnEnter` (`0x088d98cc`) hands Zone (mode 6) to
//! `EndRaceResults_PopulateZoneTable` (`0x088db574`) and Eliminator (mode 8) to
//! `EndRaceResults_PopulateEliminationTable` (`0x088db1ec`), which fill the same
//! authored table the lap grid uses - see [`super::table`] - with different
//! content. The Eliminator table is **confirmed on a live frame** (PPSSPP, 2026-09-30:
//! eight craft, the sort law, the three columns, the highlight); the Zone table is
//! read off the decompile alone, since a Zone race cannot be reached on a fresh
//! profile. See `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`'s "The
//! variant populates" for every address and confidence.

use oag_ui::frontend::Placed;
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Layers, Picture, Skin};

use super::Layout;
use super::draw::ordinal_idstring;
use super::table::{Shown, lap_slot, table_layers};

/// One craft's row on the Eliminator table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EliminationRow {
    /// The team's string-table id - its folder name, `AG_Systems` - which the draw
    /// resolves to the display name (`AG Systems`) the way the original's own
    /// `localise(craft+0x798)` does. `None` when no team is known for the slot
    /// (a `--race` launch that named none): the cell draws absent, never a
    /// placeholder.
    pub team_name: Option<String>,
    /// `craft+0x8d8`.
    pub kills: u32,
    /// `craft+0x8d4`.
    pub deaths: u32,
    /// Whether this is the player's craft - `tablehighlight`'s own condition.
    pub player: bool,
}

/// Pulse's Eliminator `EndRace Results`: the whole field ranked by kills.
///
/// [`Self::new`] applies the order `Race_BuildEndRaceResult` (`0x0882a498`) sorts the
/// records into before the screen reads them: **kills descending, then deaths
/// ascending, and on a full tie the player's row first**; any other tie keeps the
/// order the field came in. The player's place is their index after the sort, and it
/// is what `Line1` reads (`ER_ELIM_COM` and the ordinal). Confirmed on a live frame:
/// `EG-X` 5/5, `Piranha` 4/1, `Goteki 45` 3/1, `Qirex` 3/3, `AG Systems` 2/4, `Feisar`
/// 2/4, `Triakis` 1/2, the player `Assegai` 0/1 - eighth.
///
/// **This ranks the screen, not the race.** The simulation's own finishing place for
/// an Eliminator race (`oag_race`'s board) is not consulted and not changed: the rank
/// the disc shows is a fact about this table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EliminationResults {
    rows: Vec<EliminationRow>,
}

impl EliminationResults {
    /// Ranks `field` the way the original does. `field` is in slot order.
    #[must_use]
    pub fn new(mut field: Vec<EliminationRow>) -> Self {
        // A stable sort: the original's bubble sort swaps only on a strict
        // inequality (or the player tie-break), so equal rows keep their order.
        field.sort_by_key(|row| (std::cmp::Reverse(row.kills), row.deaths, !row.player));
        Self { rows: field }
    }

    /// The rows in the order the table shows them.
    #[must_use]
    pub fn rows(&self) -> &[EliminationRow] {
        &self.rows
    }

    /// The player's 1-based place, or `None` when no row is the player's.
    #[must_use]
    pub fn place(&self) -> Option<u8> {
        self.rows
            .iter()
            .position(|row| row.player)
            .and_then(|index| u8::try_from(index + 1).ok())
    }
}

/// Pulse's Zone `EndRace Results`: six label/value rows off the Zone mode object's
/// stats block (`+0x1a10`..`+0x1a1c`, copied through its `vtable+0x74`).
///
/// Every value but the first and last is an `Option`: **`None` draws the label and
/// leaves the value blank**, which is this build's answer for a statistic it does not
/// keep or has not recovered rather than a `0` that would read as a measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneResults {
    /// `ER_ZONE_CLEAR`: `+0x1a10`, the zone number - one per ten seconds survived.
    pub zones_cleared: u32,
    /// `ER_PERF_ZONE`: `+0x1a12`, zones that ended without a wall contact.
    pub perfect_zones: Option<u32>,
    /// `ER_LAPSC`: `+0x1a14`. What steps it (`craft+0x911` bit 0 with `craft+0xacc >
    /// 2`) is not recovered, so this build leaves it `None`.
    pub laps_cleared: Option<u32>,
    /// `MSC_DATA_PLAP`: `+0x1a16`, stepped with a 2000-point bonus by `craft+0x860 &
    /// 0x200000`, which is not recovered either.
    pub perfect_laps: Option<u32>,
    /// `ER_TOP_SPEED`: `+0x1a1a`, the fastest speed seen, already in km/h here.
    pub top_speed_kmh: Option<u32>,
    /// `ER_ZONE_SCORE`: `+0x1a1c`.
    pub score: i32,
}

/// The idstrings of Zone's six rows, in order.
const ZONE_LABELS: [&str; 6] = [
    "ER_ZONE_CLEAR",
    "ER_PERF_ZONE",
    "ER_LAPSC",
    "MSC_DATA_PLAP",
    "ER_TOP_SPEED",
    "ER_ZONE_SCORE",
];

impl ZoneResults {
    /// Row `row` (1-based)'s value cell, or `None` for one this build cannot fill.
    fn value(&self, row: usize, strings: &StringTable) -> Option<String> {
        match row {
            1 => Some(self.zones_cleared.to_string()),
            2 => self.perfect_zones.map(|count| count.to_string()),
            3 => self.laps_cleared.map(|count| count.to_string()),
            4 => self.perfect_laps.map(|count| count.to_string()),
            // `"%d %s"` of the speed and `RC_KMH`.
            5 => self
                .top_speed_kmh
                .map(|kmh| format!("{kmh} {}", strings.get_or_id("RC_KMH"))),
            6 => Some(self.score.to_string()),
            _ => None,
        }
    }
}

/// The Eliminator table's `Line1`: `"%s %s"` of `ER_ELIM_COM` and the place's
/// ordinal. `ER_ELIM_COM` ends in a space, so the two spaces on screen are the
/// original's. Blank outside places 1-8, as the original's switch leaves it.
fn elimination_line1(model: &EliminationResults, strings: &StringTable) -> Option<String> {
    let ordinal = ordinal_idstring(model.place()?)?;
    Some(format!(
        "{} {}",
        strings.get_or_id("ER_ELIM_COM"),
        strings.get_or_id(ordinal)
    ))
}

/// `EndRace Results` for an Eliminator race: `Line1`, the `Deaths`/`Team`/`Kills`
/// header, and a row per craft with the player's highlighted. See
/// [`EliminationResults`].
///
/// **No `ER_DNF`**: the original prints it in place of both numbers when a record's
/// `+0x140` word is `-1`, and nothing writes that word on the Eliminator path - it
/// read `0` on all eight records of the live race.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes"
)]
pub fn elimination_results_draw_list(
    model: &EliminationResults,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let rows = model.rows();
    let shown = Shown {
        table: true,
        rows: rows.len(),
        header_bar: true,
        boost_icon: false,
        highlight: model.place().map(usize::from),
    };
    table_layers(
        layout,
        skin,
        frame,
        backdrop,
        race_behind,
        sprites,
        shown,
        &|text| {
            let name = text.name.as_deref().unwrap_or("");
            let Some((row, column)) = lap_slot(name) else {
                return match name {
                    "Line1" => elimination_line1(model, strings),
                    _ => text.string.clone(),
                };
            };
            if row == 0 {
                let id = match column {
                    0 => "ER_DEATHS",
                    1 => "ER_TEAM",
                    2 => "IG_HUD_KILLS",
                    _ => return None,
                };
                return Some(strings.get_or_id(id).to_string());
            }
            let craft = rows.get(row - 1)?;
            match column {
                0 => Some(craft.deaths.to_string()),
                1 => craft
                    .team_name
                    .as_deref()
                    .map(|id| strings.get_or_id(id).to_string()),
                2 => Some(craft.kills.to_string()),
                _ => None,
            }
        },
    )
}

/// `EndRace Results` for a Zone race: `Line1` (`ER_ZONE_COM`) and six rows, the label
/// in the first column and the value in the third. There is no header row and no
/// highlight, and the header bar is hidden. See [`ZoneResults`].
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes"
)]
pub fn zone_results_draw_list(
    model: &ZoneResults,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let shown = Shown {
        table: true,
        rows: ZONE_LABELS.len(),
        header_bar: false,
        boost_icon: false,
        highlight: None,
    };
    table_layers(
        layout,
        skin,
        frame,
        backdrop,
        race_behind,
        sprites,
        shown,
        &|text| {
            let name = text.name.as_deref().unwrap_or("");
            let Some((row, column)) = lap_slot(name) else {
                return match name {
                    "Line1" => Some(strings.get_or_id("ER_ZONE_COM").to_string()),
                    _ => text.string.clone(),
                };
            };
            let label = ZONE_LABELS.get(row.checked_sub(1)?)?;
            match column {
                0 => Some(strings.get_or_id(label).to_string()),
                2 => model.value(row, strings),
                _ => None,
            }
        },
    )
}

#[cfg(test)]
mod tests;
