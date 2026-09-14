//! The three screens a race ends on: **`EndRace Results`**, **`EndRace
//! Rewards`** and **`EndRace Menu`**.
//!
//! Wipeout Pulse authors all three in
//! `Data\Plugins\PI001\GUI\EndRace_Definition.xml` (`Data.wad`) - see
//! [`docs/formats/endrace-screens.md`](../../../docs/formats/endrace-screens.md)
//! for the widget-by-widget read and
//! [`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`](../../../docs/ghidra/functions/psp-pulse-usa/endrace-screens.md)
//! for the decompiled law behind each one, and
//! [`docs/ui/endrace-screens.md`](../../../docs/ui/endrace-screens.md) for
//! what this build actually draws and why. This build's own results table
//! (`oag_game::scoreboard`) stays as the fallback for a title with no such
//! screen - see that crate's own module doc.
//!
//! **The tree is ours, the presentation is the disc's** - the same rule
//! [`crate::campaign`] follows, whose [`crate::campaign::Layout`] this module
//! reuses outright rather than duplicating: neither screen needs anything
//! campaign-specific from it.
//!
//! # What draws, and what does not
//!
//! - **`EndRace Results`**: the headline (`Line1`) and a per-lap table
//!   (`Lap`/`Time`), off whatever this build's own [`oag_race::Standing::lap_splits`]
//!   kept. The table's own **third column is never filled** - its meaning is
//!   unread (`docs/formats/endrace-screens.md`), and drawing a number into it
//!   would be inventing what the column means. Neither are the
//!   `perfectlap{n}` icons: the flag's own direction (does nonzero mean
//!   "perfect" or the reverse) is not settled either.
//! - **`EndRace Rewards`**: the medal-award phrase and, for a no-medal
//!   campaign race, the disc's own hex-dash glyph - the one case this
//!   project's own capture (`results-02.png`) actually shows. A medal that
//!   *is* earned resolves to a paused 3D trophy the composition root
//!   unpauses (`oag_game::preview`, the same mesh path a picker's own ship
//!   preview uses) rather than anything this crate draws itself - see the
//!   module's own "not runtime-verified" note in the ghidra page for why
//!   `MedalImg`'s own state under an earned trophy is left undrawn rather
//!   than guessed. **The loyalty row draws when the model carries a
//!   [`Loyalty`]** - `Race_ComputeLoyaltyAward`/`Loyalty_AccumulateTotal`
//!   (confidence 95/90, `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`)
//!   landed after this crate's own first pass, which is why the module's
//!   earlier reading (nothing drawn at all) no longer holds; `None` still
//!   draws nothing, for a caller with no team to compute one for.
//! - **`EndRace Menu`**: the option list, built per mode the same way
//!   `EndRaceMenu_PopulateOptions` builds it, and the just-driven run's own
//!   best lap (`GhostTime2`/`ER_NEW_GHOST`). **No existing-ghost comparison
//!   and no `SAVE GHOST`/`DELETE DATA` rows** - this project keeps no on-disk
//!   ghost yet, the same absence `docs/ui/fe-menu-definitions.md`'s own
//!   `RECORDS` section already chose for a column it could not back.
//!
//! [`oag_race::Standing::lap_splits`]: ../../../crates/race/src/standing.rs

use oag_tables::race_campaign::Medal;

use oag_gameplay::input::{Button, Input};

pub use crate::campaign::Layout;

pub mod draw;
pub mod pointer;

pub use draw::{endrace_menu_draw_list, results_draw_list, rewards_draw_list};

#[cfg(test)]
mod tests;

/// One selectable thing an [`Event`] can act on - the same three-event
/// vocabulary [`crate::campaign::Event`] uses, minus `Help`: none of these
/// three screens authors a `Cell Help`-shaped overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The menu's own selection moved.
    Moved,
    /// The player confirmed - `Results`/`Rewards` advance to the next
    /// screen, `Menu` acts on the selected row.
    Confirmed,
    /// The player backed out - `Menu`'s own secondary-button case, per
    /// `docs/formats/endrace-screens.md`; `Results`/`Rewards` do not answer
    /// it (the disc authors no `Back` redirect on either).
    Back,
}

/// One completed lap, as `EndRace Results`' own `lap{n}.0`/`lap{n}.1` show
/// it. `lap` is 1-based, matching [`oag_race::Standing::lap`]'s own meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LapSplit {
    pub lap: u32,
    /// This project's own fixed 60 Hz tick - see
    /// [`docs::determinism`](../../../docs/architecture/determinism.md).
    pub ticks: u32,
}

/// `Line1`'s own two-step narrowing - see
/// `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`'s
/// `EndRaceResults_OnEnter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Headline {
    /// `ER_TT_COM` - `"TIME TRIAL COMPLETE!"`.
    TimeTrial,
    /// `ER_SL_COM` - `"SPEED LAP COMPLETE!"`.
    SpeedLap,
    /// A finishing position exists: `ER_1STP`..`ER_8THP`, 1-based.
    Position(u8),
    /// No finishing position (a field of one) - `ER_SHIP_DES`.
    NoPosition,
    /// `Zone`/`Eliminator` go through a populate helper
    /// (`FUN_088db574`/`FUN_088db1ec`) this project has not decompiled -
    /// drawing nothing rather than guessing its own headline. See
    /// `docs/formats/endrace-screens.md`.
    Unresolved,
}

/// `EndRace Results`: the headline and the per-lap table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Results {
    pub headline: Headline,
    /// Up to [`oag_race::MAX_RECORDED_LAPS`] splits - the fewer of "how many
    /// laps this race ran" and "how many this project's own
    /// `Standing::lap_splits` kept". A race longer than that keeps running
    /// splits for its own last four laps' worth of storage and no more -
    /// see that constant's own doc for why widening it is not this pass's
    /// call to make.
    pub laps: Vec<LapSplit>,
    pub total_ticks: u64,
}

/// `EndRace Rewards`: the medal award, whether a campaign cell was in play
/// at all - `EndRaceRewards_OnEnter`'s own `DAT_08b30ffc != 0` branch, which
/// decides whether a trophy is even attempted - and this race's own loyalty
/// award, if one was computed. `campaign` is `false` on a Racebox/custom
/// race, which never resolves a trophy and hides `MedalImg` outright - see
/// the module doc.
#[derive(Debug, Clone, PartialEq)]
pub struct Rewards {
    pub medal: Option<Medal>,
    pub campaign: bool,
    /// `RewardLine2`/`RewardLoyaltyActive`/`loyaltynum`/`loyaltybar`'s own
    /// row - `None` draws none of it, the same absence this project's own
    /// rule prefers over a guessed number. See [`Loyalty`]'s own doc for the
    /// law behind the two numbers it carries.
    pub loyalty: Option<Loyalty>,
}

/// This race's own loyalty award and the team's running total -
/// `Race_ComputeLoyaltyAward`/`Loyalty_AccumulateTotal`
/// (`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`), confidence 95/90,
/// confirmed on two live Time-Trial/Speed-Lap races; the Race/Zone/
/// Eliminator branches are decompiled under the same law but not
/// independently live-verified - see that page's own "Still open".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loyalty {
    /// The team's own display name (e.g. `"Assegai"`) - `RewardLine2`'s own
    /// `"%s %s"` of this and `ER_LOY` is built at draw time, not stored
    /// pre-formatted, so a source with no team name resolves this screen
    /// exactly as honestly as every other label on it.
    pub team_name: String,
    /// This race's own award - `RewardLoyaltyActive`'s `"%d %s"` of this and
    /// `ER_POINTS`.
    pub award: u32,
    /// The team's running total *after* this race's award is folded in -
    /// `loyaltynum`'s `"%s %d"` of `ER_TOT_LOY` and this, and `loyaltybar`'s
    /// own fill fraction (`total * 0.00124`).
    pub total: u32,
}

impl Rewards {
    /// Whether `MedalImg`'s own hex-dash glyph draws - the one measured
    /// case (`results-02.png`): a campaign race with no medal. A medal that
    /// *is* earned resolves to a 3D trophy instead (drawn by the
    /// composition root, not this crate - see the module doc); a
    /// non-campaign race hides `MedalImg` outright per the decompile.
    #[must_use]
    pub fn shows_no_medal_glyph(&self) -> bool {
        self.campaign && self.medal.is_none()
    }
}

/// One row `EndRace Menu`'s own `Endrace Options` can hold - see
/// `docs/formats/endrace-screens.md`'s numbered populate list.
/// `SaveGhost`/`DeleteData` are not modelled - see the module doc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuOption {
    /// Mid-Tournament only - not reachable by this engine, which implements
    /// no Tournament mode, but named for completeness against the disc's own
    /// list.
    NextRace,
    ReturnToGrid,
    ReturnToMenu,
    RaceAgain,
    ViewResultsAgain,
}

impl MenuOption {
    /// The idstring `EndRaceMenu_PopulateOptions` binds this row to.
    #[must_use]
    pub fn idstring(self) -> &'static str {
        match self {
            Self::NextRace => "ER_NEXT_RACE",
            Self::ReturnToGrid => "ER_RETURN_GRID",
            Self::ReturnToMenu => "ER_RETURN_MENU",
            Self::RaceAgain => "ER_RACE_AGAIN",
            Self::ViewResultsAgain => "ER_VIEW_AGAIN",
        }
    }
}

/// `EndRace Menu`: the option list and the just-driven run's own best lap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndRaceMenu {
    options: Vec<MenuOption>,
    index: usize,
    /// `GhostLine2`/`GhostTime2`'s own `ER_NEW_GHOST` row - this run's own
    /// best lap, in ticks. `None` when no lap was ever completed, which
    /// draws the row absent rather than a formatted zero.
    pub new_best_lap_ticks: Option<u32>,
}

impl EndRaceMenu {
    #[must_use]
    pub fn new(options: Vec<MenuOption>, new_best_lap_ticks: Option<u32>) -> Self {
        Self {
            options,
            index: 0,
            new_best_lap_ticks,
        }
    }

    #[must_use]
    pub fn options(&self) -> &[MenuOption] {
        &self.options
    }

    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    #[must_use]
    pub fn selected(&self) -> Option<MenuOption> {
        self.options.get(self.index).copied()
    }

    /// Up/down wrap over the option list, matching
    /// [`crate::campaign::GridSelection::update`]'s own wrapping-list idiom.
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        if self.options.is_empty() {
            return out;
        }
        if input.take(Button::Down) {
            self.step(1);
            out.push(Event::Moved);
        }
        if input.take(Button::Up) {
            self.step(-1);
            out.push(Event::Moved);
        }
        if input.take(Button::Cross) || input.take(Button::Start) {
            out.push(Event::Confirmed);
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        out
    }

    pub(super) fn step(&mut self, step: i32) {
        let count = self.options.len() as i64;
        if count == 0 {
            return;
        }
        self.index = (self.index as i64 + i64::from(step)).rem_euclid(count) as usize;
    }

    /// Puts the selection on `index`, if it names a different row - the
    /// pointer path's own move, [`crate::campaign::GridSelection::select`]'s
    /// shape.
    pub(super) fn select(&mut self, index: usize) -> Option<Event> {
        if index >= self.options.len() || index == self.index {
            return None;
        }
        self.index = index;
        Some(Event::Moved)
    }
}
