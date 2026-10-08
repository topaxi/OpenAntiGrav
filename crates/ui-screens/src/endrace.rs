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
//!   kept, under the `boostimg` header icon the disc puts over the third column.
//!   That column's **values are never filled** - what the number counts is
//!   unread (`docs/formats/endrace-screens.md`), and drawing one would be inventing
//!   what the column means - and the `perfectlap{n}` icons never show: their flag's
//!   direction is settled (nonzero shows the icon) but this build keeps no per-lap
//!   perfect flag. Eliminator and Zone races have tables of their own,
//!   [`EliminationResults`] and [`ZoneResults`], and a Tournament leg's is
//!   [`TournamentResults`]; see the `table` module for how a mode's populate shows only the
//!   rows it fills.
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
pub mod hd;
mod modes;
pub mod photo;
pub mod pointer;
mod table;
pub mod touch;

pub use draw::{
    endrace_menu_draw_list, results_draw_list, rewards_draw_list, tournament_results_draw_list,
};
pub use modes::{
    EliminationResults, EliminationRow, ZoneResults, elimination_results_draw_list,
    zone_results_draw_list,
};

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

impl Event {
    /// The navigation sound for it. See `crate::picker::Event::nav`.
    #[must_use]
    pub fn nav(self) -> oag_ui::menu::nav::Nav {
        use oag_ui::menu::nav::Nav;
        match self {
            Self::Moved => Nav::UpDown,
            Self::Confirmed => Nav::Accept,
            Self::Back => Nav::Decline,
        }
    }
}

/// One completed lap, as `EndRace Results`' own `lap{n}.0`/`lap{n}.1` show
/// it. `lap` is 1-based, matching [`oag_race::Standing::lap`]'s own meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LapSplit {
    pub lap: u32,
    /// This project's own fixed 60 Hz tick - see
    /// [`docs::determinism`](../../../docs/architecture/determinism.md).
    pub ticks: u32,
    /// Speedup pads entered on this lap - the third column, headed by the `boostimg`
    /// icon: `Ship_ApplySpeedupPad`'s per-lap counter at `craft + 0x900 + lap * 0x10 +
    /// 0x94`, copied by `Race_BuildEndRaceResult` (`docs/ghidra/functions/psp-pulse-usa/
    /// endrace-screens.md`). `None` leaves the cell blank, for a source that keeps no
    /// such tally.
    pub boosts: Option<u32>,
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
    /// `Zone`/`Eliminator` on Wipeout HD/Fury, whose own populate for them is
    /// unread - drawing nothing rather than guessing a headline. Pulse's two
    /// have tables of their own ([`EliminationResults`], [`ZoneResults`]) with
    /// their own `Line1`, so they never reach this.
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

/// One row of Pulse's own Tournament standings table - see
/// [`TournamentResults`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentRow {
    /// `ER_TEAM`'s own value for this row - `None` when this project cannot
    /// resolve a team name for the slot (a `--race` launch that named no
    /// team at all; a real tournament, reached through `Team Selection`,
    /// always names one). `None` draws the cell absent rather than a
    /// placeholder like `"SLOT 3"`, the same rule every other unresolved
    /// label on this screen follows.
    pub team_name: Option<String>,
    /// This row's own points: this leg's
    /// (`oag_race::tournament::points_for_finish`, [`TournamentResults::leg`])
    /// or the running total across every leg so far
    /// (`crate::race::tournament::Progress::points`,
    /// [`TournamentResults::standings`]) - which one depends on which table
    /// the row is on, not on the row itself.
    pub points: u32,
    /// Whether this is the player's own row - `tablehighlight`'s own
    /// condition, `g_endrace_result`'s per-craft player flag
    /// (`+0x34`/`+0x8b4`).
    pub player: bool,
}

/// Pulse's own Tournament `EndRace Results`: cycles every 3 seconds between
/// this leg's own placings (`ER_RACE_STAN`, this leg's own finish order and
/// points) and the running standings (`ER_TOUR_STAN`, ranked by cumulative
/// points) - `EndRaceResults_PopulateTournamentTable` (`0x088dad90`,
/// confidence 82) and `EndRaceResults_Update`'s (`0x088da530`, confidence
/// 85) own 3-second toggle. See
/// `docs/ghidra/functions/psp-pulse-usa/tournament.md`'s
/// `EndRaceResults_PopulateTournamentTable` section for the full decompile
/// this reimplements, and `docs/ui/endrace-screens.md` for what draws.
///
/// **Every leg shows this**, not only the last - `EndRaceResults_OnEnter`'s
/// own `case 4: case 0x10:` runs on every Tournament leg's own results
/// screen; only `BigTopText` (`last_leg`/`leg_number`/`leg_count`) differs
/// between a mid-tournament leg and the last one.
#[derive(Debug, Clone, PartialEq)]
pub struct TournamentResults {
    /// `BigTopText` reads `ER_END_TOUR` when `true`, otherwise `"%s %d/%d"`
    /// of `ER_RES`, `leg_number` and `leg_count`.
    pub last_leg: bool,
    /// This leg's own 1-based index.
    pub leg_number: u32,
    /// The tournament's own leg count.
    pub leg_count: u32,
    /// `ER_RACE_STAN`'s own rows: this leg's own finish order (unsorted by
    /// `EndRaceResults_PopulateTournamentTable` itself - the row order is
    /// simply the order the field already finished in).
    pub leg: Vec<TournamentRow>,
    /// `ER_TOUR_STAN`'s own rows: ranked by cumulative points, descending -
    /// `crate::race::tournament::Progress::rank`'s own order.
    pub standings: Vec<TournamentRow>,
    /// Seconds since this screen was entered or last toggled - advanced by
    /// [`Self::tick`]. Starts at `0.0`, matching `EndRaceResults_OnEnter`'s
    /// own `*(param_1+0xdc) = 0`.
    elapsed: f32,
    /// Which page is current - starts `false` (the leg table draws first,
    /// the same table `EndRaceResults_OnEnter`'s own initial
    /// `EndRaceResults_PopulateTournamentTable(param_1, 0)` call populates).
    showing_standings: bool,
}

impl TournamentResults {
    #[must_use]
    pub fn new(
        last_leg: bool,
        leg_number: u32,
        leg_count: u32,
        leg: Vec<TournamentRow>,
        standings: Vec<TournamentRow>,
    ) -> Self {
        Self {
            last_leg,
            leg_number,
            leg_count,
            leg,
            standings,
            elapsed: 0.0,
            showing_standings: false,
        }
    }

    /// One tick of the 3-second leg/standings toggle -
    /// `EndRaceResults_Update`'s own law: accumulate, and past `3.0` seconds
    /// reset to `0.0` and flip the page. `seconds` is this project's own
    /// fixed 60 Hz tick (`1.0 / 60.0`) - see
    /// [`docs::determinism`](../../../docs/architecture/determinism.md).
    pub fn tick(&mut self, seconds: f32) {
        self.elapsed += seconds;
        if self.elapsed > 3.0 {
            self.elapsed = 0.0;
            self.showing_standings = !self.showing_standings;
        }
    }

    /// Whether the standings page (`ER_TOUR_STAN`) is current, rather than
    /// the leg page (`ER_RACE_STAN`).
    #[must_use]
    pub fn showing_standings(&self) -> bool {
        self.showing_standings
    }

    /// The current page's own rows.
    #[must_use]
    pub fn current(&self) -> &[TournamentRow] {
        if self.showing_standings {
            &self.standings
        } else {
            &self.leg
        }
    }

    /// `Line1`'s own idstring for the current page.
    #[must_use]
    pub fn line1_id(&self) -> &'static str {
        if self.showing_standings {
            "ER_TOUR_STAN"
        } else {
            "ER_RACE_STAN"
        }
    }
}

/// One craft's own row on Wipeout HD/Fury's `EndRace Results` - the whole
/// field's finishing order, not Pulse's own per-lap table. See [`hd`] for
/// what draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldRow {
    /// Race position, 1-based.
    pub place: u8,
    /// The tick this craft crossed for the last time, or `None` for one
    /// still racing when the field's own clock stopped - the same "no time"
    /// case `oag_game::scoreboard::Row::finish_tick` carries, off the same
    /// feed.
    pub time_ticks: Option<u64>,
    /// Whether this is the player's own row - what
    /// [`hd::hd_results_draw_list`] repositions `GridHighlight` onto.
    pub player: bool,
}

/// Wipeout HD/Fury's own `EndRace Results`: the headline (the same
/// [`Headline`] Pulse's screen resolves, reused onto HD's own placeholder
/// `Line1` - see [`hd`]'s module doc for why that is a chosen substitution
/// rather than a measured one) and the whole field, ordered by place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldResults {
    pub headline: Headline,
    pub rows: Vec<FieldRow>,
    /// The loyalty block `Results` itself carries on HD (`loyalty1.1`,
    /// `loyalty2`). `None` draws neither - a race with no team behind it has
    /// no total to show, and the disc's own placeholder text never draws.
    pub loyalty: Option<HdLoyalty>,
}

/// HD's loyalty block, in its final state - the one the ticker
/// (`EndRaceResults_UpdateLoyaltyTicker`, `0x00225030`) ends on and the one a
/// zero award jumps straight to
/// (`docs/ghidra/functions/ps3-hdfury-eu/endrace-loyalty.md`). The ticker's
/// own animation, a line per reason, is not reproduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdLoyalty {
    /// This race's award - `loyalty1.1`'s `"%d %s"` of this and `ER_POINTS`.
    pub award: u32,
    /// The team's running total after the award - `loyalty2`'s `"%d"`. No bar:
    /// HD's `Results` code looks up no slider.
    pub total: u32,
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
    /// own fill width in pixels (`total * 0.00124`, not a fraction).
    pub total: u32,
}

/// Wipeout HD/Fury's own `EndRace Rewards` - see [`hd::hd_rewards_draw_list`]
/// for what draws off it and why. **No loyalty field**: HD shows its loyalty on
/// `Results` ([`HdLoyalty`], on [`FieldResults`]), and its own law is not
/// [`Loyalty`]'s (the PSP's), so this screen's four loyalty widgets draw
/// nothing rather than Pulse's numbers under HD's layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdRewards {
    /// The player's own finishing place, 1-based - `BigPos`'s figure.
    /// `None` for a race with no place (a field of one that never finished),
    /// which draws `BigPos` and the tile it sits on as absent.
    pub place: Option<u8>,
    /// This race's own campaign medal, the same value [`Rewards::medal`]
    /// carries on Pulse.
    pub medal: Option<Medal>,
    /// Whether a campaign cell was in play - `RewardLine1` draws only when
    /// it was, since a race with no cell has no medal law to report on.
    pub campaign: bool,
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
    /// Mid-Tournament only, offered in place of `RaceAgain` on every leg but
    /// a Tournament cell's own last - see `crate::race_stage::endrace::menu_options`'s
    /// own `tournament_next_leg` doc.
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

    /// The `<Block name="...">` this option is on Wipeout HD/Fury's own
    /// `EndRace Menu` - see [`hd`]'s module doc. Every one of these five
    /// names is quoted directly off `Data\Plugins\Frontend\Gui\EndRace_Definition.xml`
    /// (`docs/formats/hd-endrace-screens.md`); the disc's own `idstring` on
    /// each Block is identical to [`Self::idstring`] already returns
    /// (`ER_NEXT_RACE`/`ER_RACE_AGAIN`/`ER_RETURN_GRID`/`ER_RETURN_MENU`/`ER_VIEW_AGAIN`),
    /// which is the corroboration that this is the same option rather than a
    /// same-named coincidence.
    #[must_use]
    pub fn hd_block_name(self) -> &'static str {
        match self {
            Self::NextRace => "next_race",
            Self::ReturnToGrid => "return_to_grid",
            Self::ReturnToMenu => "return_to_menu",
            Self::RaceAgain => "race_again",
            Self::ViewResultsAgain => "view_again",
        }
    }
}

/// `EndRace Menu`: the option list and the just-driven run's own best lap.
#[derive(Debug, Clone, PartialEq)]
pub struct EndRaceMenu {
    options: Vec<MenuOption>,
    index: usize,
    /// Each option's own `Block_Update` state, index for index with
    /// `options` - drawn by Wipeout HD/Fury, whose options are free-standing
    /// `<Block>`s ([`hd`]); Pulse's `<Menu>` list ignores it.
    focus: Vec<oag_ui::menu::block::Focus>,
    /// `GhostLine2`/`GhostTime2`'s own `ER_NEW_GHOST` row - this run's own
    /// best lap, in ticks. `None` when no lap was ever completed, which
    /// draws the row absent rather than a formatted zero.
    pub new_best_lap_ticks: Option<u32>,
}

impl EndRaceMenu {
    #[must_use]
    pub fn new(options: Vec<MenuOption>, new_best_lap_ticks: Option<u32>) -> Self {
        let focus = vec![oag_ui::menu::block::Focus::default(); options.len()];
        Self {
            options,
            index: 0,
            focus,
            new_best_lap_ticks,
        }
    }

    /// One tick of every option block's own focus state - see
    /// [`oag_ui::menu::block::Focus`]. Called once a tick while the screen is
    /// up, the way `Block_Update` runs once a frame.
    pub fn tick(&mut self) {
        let index = self.index;
        for (row, focus) in self.focus.iter_mut().enumerate() {
            focus.tick(row == index);
        }
    }

    /// Option `row`'s own focus state; the default (unfocused, never
    /// ticked) for a row past the end.
    #[must_use]
    pub fn focus(&self, row: usize) -> oag_ui::menu::block::Focus {
        self.focus.get(row).copied().unwrap_or_default()
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
