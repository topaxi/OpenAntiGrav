//! What a race leaves behind after the window closes: the best lap, the best
//! total time and the last result, per circuit, mode and speed class.
//!
//! Read from and written to `<config dir>/oag/records.toml` - a sibling of
//! [`crate::settings`]'s `settings.toml` and [`crate::pilots`]'s
//! `pilots/*.toml`, in the same directory, and following the shape those two
//! already established: `#[serde(default)]` on every field, so an older file
//! keeps loading as this grows, and no version key - the file migrates by
//! what shape it is, not by a number it carries.
//!
//! # Where this is captured, and where it is not
//!
//! **Never from inside [`crate::race::Race::tick`].** The simulation does not
//! know persistence exists - see `CLAUDE.md`'s core principle - so nothing
//! here is called from the gameplay crates at all. The composition root reads
//! [`crate::race::Race`]'s already-public state (`world`, `finished`,
//! `places`) from outside the tick, at two moments: the tick a race's own
//! finish condition is first true, and whenever a player leaves a race that
//! has not - `escape`, or an unfinished `--race` run quitting outright. That
//! second path is why this cannot be built as "capture the finish and nothing
//! else": [`oag_race::Mode::laps_target`] is `None` for `SpeedLap` and
//! `Zone`, so [`crate::race::Race::finished`] never turns `true` for either
//! of them - and Speed Lap is the one mode whose entire point is a fast lap
//! time. Leaving a race is the only exit every mode has in common, so a best
//! lap set there is captured on the way out rather than never.
//!
//! **What a hard quit (window close, a killed process) still loses**: a lap
//! set in Speed Lap or Zone between the last `escape`/finish and the crash.
//! Nothing here runs on a timer or a background thread - see the module doc's
//! opening rule again - so there is no cheaper fix than writing on every
//! personal-best lap, which was deliberately not built: it would turn a
//! several-times-a-race lap crossing into a filesystem write, for a case
//! (`kill -9` mid-lap) this project has no other autosave story for either.
//! Recorded as a known gap rather than silently accepted.
//!
//! # The schema is chosen, not measured, and carries no confidence score
//!
//! The original stores race results in its own PSP save-data format, on its
//! own trigger (`"Race End Save"` - see `crate::scoreboard`'s module doc for
//! where that state name was found and why its own screen is not built). This
//! project has no obligation to match that layout: nothing here loads an
//! original save, so there is no original shape to be unfaithful to. Every
//! field below, and the file format itself, is this project's own choice.
//!
//! # Deliberately not [`crate::settings::Settings`]'s own rule on a bad file
//!
//! `settings.rs` treats a malformed value as an error that reaches the
//! player rather than a silently discarded default - "a typo should be
//! visible, not swallowed". A settings file with one bad key is small enough
//! to read and fix, and the game does not start until it is.
//!
//! A records file is a different kind of loss: it is the only copy of
//! however many races' worth of times a player has driven, nobody edits it by
//! hand, and refusing to boot over one bad row would hold an unrelated race
//! hostage to a write this module made itself. So the rule here is the
//! opposite one, on purpose - the same kind of deliberate divergence
//! [`crate::pilots`]'s own module doc records for why *that* format does not
//! rewrite a hand-authored file:
//!
//! - **One malformed row does not lose the rest.** [`parse`] decodes the
//!   `records` array one entry at a time and keeps every row that decodes,
//!   dropping only the ones that do not - see its own doc.
//! - **A file that will not parse as TOML at all is moved aside, never
//!   overwritten in place.** [`load`] renames it to `records.toml.invalid`
//!   next to itself and starts empty, so whatever was in it is still on disk
//!   for a player or a future version to recover, and the next [`save`]
//!   cannot collide with the backup's name.
//! - **Nothing here ever panics.** [`load`] has no `Result` for exactly this
//!   reason: every failure it can hit - no config directory, a permissions
//!   error, a corrupt file - degrades to an empty [`Store`] plus a logged
//!   line, not a boot failure and not an `unwrap`.
//!
//! # Where a career system attaches
//!
//! [`Record`] carries a best-ever campaign [`Medal`] and its points, and the
//! most recent race's own medal - the law behind them
//! (`Cell_EvaluateMedal`/`Cell_MedalPoints`) was a sibling member's own
//! finding, recovered and reimplemented as
//! `oag_tables::race_campaign::Cell::evaluate_medal`, not guessed at here;
//! see `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`.
//! [`Observation::campaign_medal`] is what feeds both: [`RaceStage::observation`]
//! (`crates/game/src/main/race_stage.rs`) evaluates it once a race carries a
//! [`RaceStage::campaign_cell`], and [`Store::record`] folds it into the
//! ordinary `[[records]]` row the way every other field is.
//!
//! **A second, cell-keyed table carries the same medal for a different
//! question.** `[[records]]` answers "what is the best result on this
//! track/mode/class", which a campaign race also contributes to; `Cell
//! Selection`/`Grid Selection` ask "what did *this cell* earn", which is not
//! the same key - two different campaign cells can share a track, a mode
//! and a class, and a `Zone` cell's own `class="Zone"` does not even round-
//! trip through [`Key`] (see [`CampaignRecord`]'s own doc for why). So
//! [`Store::campaign_medal`]/[`Store::record_campaign`] keep a sibling
//! `[[campaign]]` table, keyed on `(title, cell name)` alone - the same key
//! the original's own record store uses, per
//! `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "how a campaign
//! event launches" section - and neither [`Key`], [`parse`]'s `records`
//! handling nor [`Store::record`] had to change to add it.
//!
//! What is **still** not here: no unlock and no tournament standing - see
//! `docs/architecture/persistence.md`'s "where a career system attaches" for
//! the two ways left to grow this file further, both still additive.
//!
//! # No circuit list is hardcoded here, and none should ever be added
//!
//! [`Key::track`] is whatever entry name the race that just ran actually
//! loaded - the disc's own `.vex` path, not a name this module invented or
//! enumerated. A title with no circuits played yet simply has no rows; the
//! store is populated by racing, the same way [`crate::pilots::Roster`] is
//! populated by a player's own files rather than by a list this project
//! ships.

use std::path::PathBuf;

use anyhow::{Context, Result};
use log::{error, warn};
use serde::{Deserialize, Serialize};

/// Which circuit/mode/class a [`Record`] is about.
///
/// Every part is lower-cased on construction - see [`Key::new`] - so a class
/// spelled `"Venom"` in one place and `"venom"` in another lands on the same
/// row rather than opening a second one silently. This is a lookup key, never
/// shown to a player, so the case that survives here is not the case a menu
/// would want to display.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    /// [`oag_title::Title::name`], lower-cased.
    pub title: String,
    /// The archive entry name of the circuit's `.vex`, exactly as
    /// [`crate::catalogue::Track::entry_name`] spells it, lower-cased.
    ///
    /// **The disc's own path, not [`crate::catalogue::Track::id`] alone.**
    /// The entry name already carries the reversed-direction distinction
    /// (`track.vex` against `track_reversed.vex`), so a forward and a
    /// reversed run of the same circuit are two rows rather than one
    /// overwriting the other - which is correct, they are different lines to
    /// drive.
    ///
    /// **Never left as the caller's own "nothing was named" value.**
    /// `race::Options::track` is `None` on the ordinary `--race` invocation
    /// with no `--track` flag, because `race::load` resolves its own
    /// default track *internally* and never hands the resolved name back -
    /// confirmed live, the first version of this fallback wrote every such
    /// race under one shared `"(no circuit)"` bucket regardless of which
    /// circuit was actually driven. `Stage::build_race_stage` now falls back
    /// to `title.race.track` (`oag_title::race::RaceDefaults::track`) first,
    /// itself the disc's own stated default and the same value `race::load`
    /// falls back to - so `"(no circuit)"` is reached only when a source
    /// offers no circuit at all. See [`Key::new`].
    pub track: String,
    /// [`oag_race::Mode::name`] - already lower-cased with underscores, so
    /// this is carried through unchanged.
    pub mode: String,
    /// The speed class, spelled the way [`crate::race::Options::class`]
    /// carries it, lower-cased.
    pub class: String,
}

/// Named for [`Key::track`]'s last-resort fallback: a race whose source
/// offered no circuit at all still has to record *something* rather than
/// fail the capture. See `Session::handle_menu`'s own "this source offers no
/// circuit at all" warning, which is the only place this can be reached
/// from now that `Stage::build_race_stage` falls back to `title.race.track`
/// first - see [`Key::track`]'s own doc.
const NO_CIRCUIT: &str = "(no circuit)";

impl Key {
    /// Builds a key, lower-casing every part - see the struct's own doc for
    /// why. `track` is `None` only on the path [`NO_CIRCUIT`] documents.
    #[must_use]
    pub fn new(title: &str, track: Option<&str>, mode: &str, class: &str) -> Self {
        Self {
            title: title.trim().to_ascii_lowercase(),
            track: track.unwrap_or(NO_CIRCUIT).trim().to_ascii_lowercase(),
            mode: mode.trim().to_ascii_lowercase(),
            class: class.trim().to_ascii_lowercase(),
        }
    }
}

/// A campaign medal tier, [`oag_tables::race_campaign::Cell::evaluate_medal`]'s
/// own three-value law restated here rather than imported - see that
/// function's doc for `Cell_EvaluateMedal` (`0x088bf620`) and
/// [`oag_tables::race_campaign::Medal::points`] for `Cell_MedalPoints`
/// (`0x088bf530`), gold 3 / silver 2 / bronze 1.
///
/// **Duplicated, not imported, on purpose.** [`Observation`]'s own doc
/// already keeps this module free of `oag-race`/`oag-gameplay`/`crate::race`
/// so [`Store::record`] and [`laps_completed`] stay testable with no disc, no
/// GPU and no simulation step; pulling in `oag-formats` here for one enum
/// would trade that guarantee for a single shared type. [`laps_completed`]
/// already makes the identical trade for `crate::scoreboard::build`'s
/// arithmetic - see its own doc.
///
/// Serialized as a lowercase word (`medal = "gold"`), never the bare
/// ordinal: the in-race HUD carries a *different* medal-tier ordinal running
/// the opposite direction (`0 = BRONZE`, per
/// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "what is not
/// determined" section), so a bare integer in this file would be a standing
/// invitation to misread which convention it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Medal {
    /// The best tier. Declared first so `Ord`'s derived order makes `Gold`
    /// the smallest value - [`Medal::better`] relies on this to pick the
    /// better of two medals with a plain `min`.
    Gold,
    /// The middle tier.
    Silver,
    /// The worst tier a race can still be awarded.
    Bronze,
}

impl Medal {
    /// `Cell_MedalPoints`'s own table: gold 3, silver 2, bronze 1.
    #[must_use]
    pub fn points(self) -> u32 {
        match self {
            Self::Gold => 3,
            Self::Silver => 2,
            Self::Bronze => 1,
        }
    }

    /// The better of two medals. Named rather than inlined as `self.min(other)`
    /// at the call site, so a reader of [`Store::record`] does not have to
    /// work out for themselves why `min` means "best" here - see this type's
    /// own doc for the `Ord` direction that makes it true.
    #[must_use]
    fn better(self, other: Self) -> Self {
        self.min(other)
    }
}

/// **Wipeout HD/Fury only.** The rung a [`CampaignRecord::best_medal`] was
/// earned at - `oag_tables::race_campaign::Cell::targets_for_difficulty`'s
/// own three rungs, restated here for the same reason [`Medal`] is: this
/// module stays free of `oag-tables`, and a caller converts at the boundary
/// (`crate::main::campaign_stage::to_campaign_difficulty` and its own
/// inverse), the same shape `to_campaign_medal` already gives [`Medal`].
///
/// **Serialized as a lowercase word for the identical reason [`Medal`]'s
/// own doc gives, not merely by analogy**: `Cell::targets_for_difficulty`'s
/// own index convention (`0` easy .. `2` hard) is a plain `u8` everywhere
/// else in this codebase (`CellSelection::difficulty`,
/// `Cell::skill_for_difficulty`, `oag_ui::campaign::hd::hd_medal_frame`) -
/// deliberately left that way there, since changing an established,
/// widely-used index convention to chase type safety in code that already
/// bounds-checks it (`difficulty.min(2)`) would be exactly the abstraction
/// this project's own house rules say not to add. A saved file is a
/// different case: nothing bounds-checks a hand-edited `records.toml`
/// against a comment, and this project already has one HUD ordinal that
/// runs a different direction from the one this rung's own name suggests
/// (see [`Medal`]'s own doc) - a bare `0`/`1`/`2` here would be the same
/// standing invitation to misread it, for a file meant to outlive any one
/// session's memory of which convention was live when it was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    /// `0` - Novice on `Elimination`/`NitroBattle` cells.
    Easy,
    /// `1` - Skilled. Also what [`oag_tables::race_campaign::Cell::gold`]/
    /// `silver`/`bronze` themselves mean on a cell with no per-difficulty
    /// targets at all - see that field's own doc.
    Medium,
    /// `2` - Elite. What
    /// `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s
    /// `SaveData_MigrateCellMedalsToHardElite` credits a pre-existing,
    /// difficulty-less medal at.
    Hard,
}

impl Difficulty {
    /// `Cell::targets_for_difficulty`'s own index, `0` easy .. `2` hard -
    /// the inverse of [`Self::from_index`]. Declared in ascending
    /// easy-to-hard order, so `Ord`'s derived direction already makes
    /// `Hard` the greatest [`Difficulty`] with no inversion trick - unlike
    /// [`Medal::better`], which needs one.
    #[must_use]
    pub fn index(self) -> u8 {
        match self {
            Self::Easy => 0,
            Self::Medium => 1,
            Self::Hard => 2,
        }
    }

    /// [`Self::index`]'s own inverse, clamping `2` and up to [`Self::Hard`]
    /// the same way `Cell::targets_for_difficulty` itself clamps any index
    /// past its own three rungs.
    #[must_use]
    pub fn from_index(index: u8) -> Self {
        match index {
            0 => Self::Easy,
            1 => Self::Medium,
            _ => Self::Hard,
        }
    }
}

/// One race's outcome, read off [`crate::race::Race`]'s already-public state
/// from *outside* the tick - see the module doc's "where this is captured"
/// section for why there are exactly two call sites and no others.
///
/// Deliberately primitives rather than a `&Race` or a `&Board`: this keeps
/// `records.rs` free of any dependency on `oag-race`, `oag-gameplay` or this
/// crate's own `race` module beyond what a caller already read off public
/// fields, which is what makes [`Store::record`] and [`laps_completed`]
/// testable with no disc, no GPU and no simulation step at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    /// Whether the race had reached its own finish condition - never
    /// re-derived here, always [`crate::race::Race::finished`]'s own answer.
    pub finished: bool,
    /// The player's own race position at the moment of capture, from
    /// [`crate::race::Race::places`]. Always `Some` for a race that had a
    /// [`crate::race::Race`] to read at all.
    pub place: Option<u8>,
    /// Laps completed - see [`laps_completed`], which is what every caller
    /// builds this field with.
    pub laps_completed: u32,
    /// The player's own finishing tick if they had one, or however far the
    /// clock had got when the race was left. Ticks, never a formatted
    /// string - `crate::hud::format_lap_time` takes ticks too, so nothing
    /// here forces a caller to format before it can persist.
    pub tick: u64,
    /// The player's own quickest completed lap, in ticks - `None` if they
    /// never finished one.
    pub best_lap_ticks: Option<u32>,
    /// This race's own campaign medal, already evaluated by
    /// `oag_tables::race_campaign::Cell::evaluate_medal` and converted to
    /// this module's own [`Medal`] by the caller - this module knows
    /// nothing about a campaign cell's targets or mode, and never computes
    /// this itself. `None` both when the race was not run against a
    /// campaign cell at all, and when the cell's own law says the value
    /// scored met no medal.
    ///
    /// **Fed by a real race since 2026-09-14.** `RaceStage::observation`
    /// (`crates/game/src/main/race_stage.rs`) calls `RaceStage::campaign_medal`,
    /// which evaluates a real `oag_tables::race_campaign::Cell` once one is
    /// selected - see that method's own doc for the per-mode law. Still
    /// `None` for a race launched any other way (the ordinary RACE page,
    /// RACE REMIX, `--race`), since none of those set `RaceStage::campaign_cell`
    /// at all. See the module doc's "where a career system attaches" section.
    pub campaign_medal: Option<Medal>,
    /// **Wipeout HD/Fury only.** The rung [`Self::campaign_medal`] was
    /// evaluated against - `RaceStage::campaign_difficulty`'s own doc names
    /// when this is `None` even with a campaign cell in play. Independent
    /// of whether [`Self::campaign_medal`] itself is `Some`: a run that
    /// scored no tier still ran at a real difficulty, and
    /// [`CampaignRecord::last_difficulty`] wants that regardless.
    pub campaign_difficulty: Option<Difficulty>,
}

/// Laps completed, for the `laps_completed` field of an [`Observation`].
///
/// [`crate::scoreboard::build`]'s own rule, replicated here rather than
/// shared: that function takes a whole `Board` this module has no business
/// depending on - see [`Observation`]'s own doc - and the arithmetic is three
/// lines. A finished craft's own lap counter reads one past the target by the
/// time its finish tick is written, which is what the cap is for.
#[must_use]
pub fn laps_completed(lap: u32, finished: bool, laps_target: Option<u32>) -> u32 {
    match (finished, laps_target) {
        (true, Some(target)) => target,
        _ => lap.saturating_sub(1),
    }
}

/// One circuit/mode/class's whole history: the two records worth keeping
/// forever, and the last race run under this key.
///
/// `#[serde(default)]` on every field but the four key ones, which
/// [`parse`] would already refuse a row missing - so a field this build adds
/// later is absent rather than refused on a file an older version wrote, the
/// same promise `settings.rs`'s own `Settings` makes. See the module doc's
/// "chosen, not measured" and "where a career system attaches" sections.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// See [`Key::title`].
    pub title: String,
    /// See [`Key::track`].
    pub track: String,
    /// See [`Key::mode`].
    pub mode: String,
    /// See [`Key::class`].
    pub class: String,
    /// The quickest completed lap ever recorded on this row, in ticks.
    /// `None` until one race here completes one.
    #[serde(default)]
    pub best_lap_ticks: Option<u32>,
    /// The quickest *finished* race on this row, in ticks. `None` until a
    /// race here actually finishes - an abandoned Speed Lap contributes to
    /// [`Self::best_lap_ticks`] alone, never to this.
    #[serde(default)]
    pub best_total_ticks: Option<u64>,
    /// Whether the most recent race on this row finished.
    #[serde(default)]
    pub last_finished: bool,
    /// The most recent race's position, from [`Observation::place`].
    #[serde(default)]
    pub last_place: Option<u8>,
    /// The most recent race's completed laps.
    #[serde(default)]
    pub last_laps_completed: u32,
    /// The most recent race's own tick - the finish tick if it had one, the
    /// tick it was left on otherwise.
    #[serde(default)]
    pub last_tick: u64,
    /// The most recent race's own best lap, which may differ from
    /// [`Self::best_lap_ticks`] whenever this race was not the fastest one
    /// ever driven on this row.
    #[serde(default)]
    pub last_best_lap_ticks: Option<u32>,
    /// The best campaign medal ever earned on this row - never downgraded,
    /// the same "best-of" rule [`Self::best_lap_ticks`] follows, using
    /// [`Medal::better`]. `None` until a race on this row carries an
    /// [`Observation::campaign_medal`] - which, today, is every race: see
    /// that field's own doc for why nothing feeds it yet.
    #[serde(default)]
    pub best_medal: Option<Medal>,
    /// The points [`Self::best_medal`] is worth - always
    /// `Self::best_medal.map(Medal::points)`, kept as its own field so a
    /// reader of the file sees the number a career total would sum without
    /// also having to know the medal-to-points law. [`Store::record`] is
    /// the only writer, and keeps the two in sync.
    #[serde(default)]
    pub best_points: Option<u32>,
    /// The most recent race's own campaign medal - may differ from
    /// [`Self::best_medal`] whenever this race was not the best one ever
    /// driven on this row. `None` for a race with no campaign cell in
    /// play, the same "last" semantics every other `last_*` field already
    /// carries.
    #[serde(default)]
    pub last_medal: Option<Medal>,
}

impl Record {
    fn key(&self) -> Key {
        Key {
            title: self.title.clone(),
            track: self.track.clone(),
            mode: self.mode.clone(),
            class: self.class.clone(),
        }
    }
}

/// What a finish transition's own results-table comparison looks like: the
/// row's standing bests, and whether the race that just ran is the one that
/// set them - read once, at the same capture site that persists an
/// [`Observation`] into a [`Store`], and drawn by `crate::scoreboard`.
///
/// **A pure function of a `Record` and an `Observation`, on purpose.** It
/// mirrors [`Store::record`]'s own best-of rule rather than calling it, so a
/// caller can compare *before* merging without a second, mutated `Store` to
/// read back - `Session::frame`'s finish arm needs exactly that: the row as
/// it stood before this race, to say whether this race is the one that moved
/// it. Testable with no disc, no GPU and no `Store` at all, the same
/// guarantee [`laps_completed`] and [`Observation`] itself already keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PersonalBest {
    /// The best lap on this row once this race is folded in - this race's own
    /// lap when it improved on what stood before, the standing one otherwise.
    /// `None` only when neither this race nor any before it ever completed
    /// one.
    pub best_lap_ticks: Option<u32>,
    /// Whether *this* race is the one that set [`Self::best_lap_ticks`] -
    /// `true` for the first race ever run on a row, the same "seeded from
    /// itself" rule [`Store::record`]'s own tests carry.
    pub lap_improved: bool,
    /// The best finished time on this row once this race is folded in. `None`
    /// when no race on this row, including this one, has ever finished.
    pub best_total_ticks: Option<u64>,
    /// Whether this race is the one that set [`Self::best_total_ticks`].
    /// Always `false` when `obs.finished` was `false`: an abandoned race has
    /// no total to compare, so it cannot be the one that set it.
    pub total_improved: bool,
    /// The best campaign medal ever earned on this row once this race is
    /// folded in. `None` until a race here - this one or an earlier one -
    /// carries a medal at all, which today is never: see
    /// [`Observation::campaign_medal`]'s own doc.
    pub best_medal: Option<Medal>,
    /// Whether this race is the one that set [`Self::best_medal`].
    pub medal_improved: bool,
}

impl PersonalBest {
    /// Compares `obs` against `previous` - the row [`Store::get`] returned
    /// for this key *before* `obs` was folded into the store, or `None` for a
    /// key with no row yet.
    ///
    /// **Every "improved" flag means "this race", not "ever".** A previous
    /// [`Medal::Gold`] and this race's own [`Medal::Silver`] leaves
    /// `best_medal` at gold and `medal_improved` at `false` - the standing
    /// medal did not move, whatever this particular race scored.
    #[must_use]
    pub fn compare(previous: Option<&Record>, obs: &Observation) -> Self {
        let previous_lap = previous.and_then(|record| record.best_lap_ticks);
        let (best_lap_ticks, lap_improved) = match (previous_lap, obs.best_lap_ticks) {
            (None, new) => (new, new.is_some()),
            (Some(prev), None) => (Some(prev), false),
            (Some(prev), Some(new)) => (Some(prev.min(new)), new < prev),
        };

        let previous_total = previous.and_then(|record| record.best_total_ticks);
        let (best_total_ticks, total_improved) = if obs.finished {
            match previous_total {
                None => (Some(obs.tick), true),
                Some(prev) => (Some(prev.min(obs.tick)), obs.tick < prev),
            }
        } else {
            (previous_total, false)
        };

        let previous_medal = previous.and_then(|record| record.best_medal);
        let (best_medal, medal_improved) = match (previous_medal, obs.campaign_medal) {
            (None, new) => (new, new.is_some()),
            (Some(prev), None) => (Some(prev), false),
            (Some(prev), Some(new)) => (Some(prev.better(new)), new < prev),
        };

        Self {
            best_lap_ticks,
            lap_improved,
            best_total_ticks,
            total_improved,
            best_medal,
            medal_improved,
        }
    }
}

/// One campaign cell's whole history, keyed on the cell's own `name` rather
/// than on [`Key`].
///
/// **Why a sibling table rather than a fourth field on [`Key`].** The
/// original hashes the cell's own `name` string for its record-store key
/// (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "how a campaign
/// event launches" section), not a track/mode/class tuple, and a cell's own
/// `class` is not always a usable lookup key even if it were: a `Zone` cell
/// authors the literal `class="Zone"`, `Cell::speed_class` answers `None`
/// for it, and `Race::start` falls back to whatever class was last selected.
/// A `Key` built from `cell.class` at read time would therefore not match
/// the `Key` `setup.class` built at load time, and a medal earned there
/// would write once and never read back. Keying on the cell's own name
/// alone has no such mismatch, matches the measured mechanism, and needs no
/// change to [`Key`], [`parse`] or [`Store::record`] - see
/// `docs/architecture/persistence.md`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignRecord {
    /// [`oag_title::Title::name`], lower-cased - see [`Key::title`].
    pub title: String,
    /// The cell's own authored `name`, e.g. `"grid0_2_1"`, lower-cased.
    /// Never a grid/cell index pair - see this struct's own doc.
    pub cell: String,
    /// The best medal ever earned on this cell - never downgraded, the same
    /// best-of rule [`Record::best_lap_ticks`] follows, via [`Medal::better`].
    #[serde(default)]
    pub best_medal: Option<Medal>,
    /// [`Self::best_medal`]'s points, kept alongside it for the same reason
    /// [`Record::best_points`] is: a reader of the file sees the number a
    /// career total would sum without also knowing the medal-to-points law.
    #[serde(default)]
    pub best_points: Option<u32>,
    /// **Wipeout HD/Fury only.** The rung [`Self::best_medal`] was earned
    /// at - `#[serde(default)]`, so a file [`Self::best_medal`] already
    /// existed in before this field did (every pre-2026-09-28 row, and
    /// every Pulse row forever) loads as `None` rather than refusing to
    /// parse. `oag_ui::campaign::hd::hd_medal_frame`'s own doc names the
    /// default it falls back to when a saved medal carries no difficulty at
    /// all. See [`Store::record_campaign`] for how the two are kept
    /// together.
    #[serde(default)]
    pub best_difficulty: Option<Difficulty>,
    /// The most recent race run against this cell's own medal - `None` both
    /// for a cell never raced and for a run that scored no tier at all,
    /// which is what "the most recent race scored nothing" means here.
    #[serde(default)]
    pub last_medal: Option<Medal>,
    /// [`Self::last_medal`]'s own difficulty - `None` under the identical
    /// two conditions [`Self::best_difficulty`]'s own doc gives, plus
    /// whenever [`Self::last_medal`] itself is `None`.
    #[serde(default)]
    pub last_difficulty: Option<Difficulty>,
}

impl CampaignRecord {
    fn matches(&self, title: &str, cell: &str) -> bool {
        self.title == title && self.cell == cell
    }
}

/// One team's persistent loyalty total - `EndRace Rewards`' own `Total
/// loyalty: <n>` line and fill bar, and the original's `DAT_08b31774` store
/// mirrored the same way [`CampaignRecord`] mirrors the campaign's own
/// per-cell record: a sibling table, not a fourth part of [`Key`], because
/// the key ([`oag_title::Title::name`] + team) has nothing to do with a
/// circuit/mode/class row.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoyaltyRecord {
    /// [`oag_title::Title::name`], lower-cased - see [`Key::title`].
    pub title: String,
    /// The team's own id (`Livery::team`, e.g. `"Assegai"`), lower-cased.
    pub team: String,
    /// The running total - `Loyalty_AccumulateTotal`'s own `+= award`,
    /// capped the same place the original caps it: `100000`. See
    /// [`Store::record_loyalty`].
    #[serde(default)]
    pub total: u32,
}

impl LoyaltyRecord {
    fn matches(&self, title: &str, team: &str) -> bool {
        self.title == title && self.team == team
    }
}

/// Every row [`load`] found or [`Store::record`] has added since, in one
/// file.
///
/// `records` is private so every write goes through [`Store::record`], which
/// is what keeps the array sorted - `save` writes it back in a stable order
/// on every run, the same reason [`crate::settings::Settings::render_profiles`]
/// is a `BTreeMap` rather than a `HashMap`: an unordered rewrite is a diff
/// with nothing changed in it every single launch. [`Self::campaign`] follows
/// the identical rule through [`Store::record_campaign`].
#[derive(Debug, Clone, Default, Serialize)]
pub struct Store {
    records: Vec<Record>,
    /// One row per campaign cell ever raced - see [`CampaignRecord`]'s own
    /// doc for why this is a sibling table rather than a fourth part of
    /// [`Key`]. A file with no `[[campaign]]` table at all - every row
    /// written before this existed - parses to an empty one; see [`parse`].
    campaign: Vec<CampaignRecord>,
    /// One row per team ever raced - see [`LoyaltyRecord`]'s own doc. A file
    /// with no `[[loyalty]]` table at all parses to an empty one, the same
    /// way `campaign`'s own absence does.
    loyalty: Vec<LoyaltyRecord>,
}

impl Store {
    /// Every row, in the file's own stable order.
    #[must_use]
    pub fn rows(&self) -> &[Record] {
        &self.records
    }

    /// The row `key` names, if one exists yet.
    #[must_use]
    pub fn get(&self, key: &Key) -> Option<&Record> {
        self.records.iter().find(|record| record.key() == *key)
    }

    /// Merges one race's [`Observation`] into the row `key` names, creating
    /// it if this is the first result ever recorded there.
    ///
    /// **Best-of, never overwritten downward.** [`Record::best_lap_ticks`]
    /// and [`Record::best_total_ticks`] only ever shrink, and the second
    /// stays untouched entirely when `obs.finished` is `false` - an
    /// abandoned race has no total time to compare. [`Record::best_medal`]
    /// only ever improves too, via [`Medal::better`], and stays untouched
    /// when `obs.campaign_medal` is `None` - no campaign cell in play is not
    /// evidence the row's standing medal should be forgotten. The `last_*`
    /// fields always take `obs`'s own values, whatever they say: that is
    /// what "the last race" means, better or worse than the one before it.
    pub fn record(&mut self, key: Key, obs: Observation) {
        let row = match self.records.iter_mut().find(|record| record.key() == key) {
            Some(row) => row,
            None => {
                self.records.push(Record {
                    title: key.title,
                    track: key.track,
                    mode: key.mode,
                    class: key.class,
                    ..Record::default()
                });
                self.records
                    .last_mut()
                    .expect("just pushed onto this exact vec")
            }
        };
        if let Some(lap) = obs.best_lap_ticks {
            row.best_lap_ticks = Some(row.best_lap_ticks.map_or(lap, |best| best.min(lap)));
        }
        if obs.finished {
            row.best_total_ticks = Some(
                row.best_total_ticks
                    .map_or(obs.tick, |best| best.min(obs.tick)),
            );
        }
        row.last_finished = obs.finished;
        row.last_place = obs.place;
        row.last_laps_completed = obs.laps_completed;
        row.last_tick = obs.tick;
        row.last_best_lap_ticks = obs.best_lap_ticks;
        if let Some(medal) = obs.campaign_medal {
            row.best_medal = Some(row.best_medal.map_or(medal, |best| best.better(medal)));
            row.best_points = row.best_medal.map(Medal::points);
        }
        row.last_medal = obs.campaign_medal;

        // Stable, so the file `save` writes is the same shape every run bar
        // the numbers that actually changed - see the struct's own doc.
        self.records.sort_by_key(Record::key);
    }

    /// Every campaign row, in the file's own stable order.
    #[must_use]
    pub fn campaign_rows(&self) -> &[CampaignRecord] {
        &self.campaign
    }

    /// The row `(title, cell)` names, if this cell has ever been raced.
    /// Lower-cased the same way [`Key::new`] lower-cases every part, so a
    /// caller passing a cell's own mixed-case `name` still finds its row.
    #[must_use]
    pub fn campaign_medal(&self, title: &str, cell: &str) -> Option<&CampaignRecord> {
        let title = title.trim().to_ascii_lowercase();
        let cell = cell.trim().to_ascii_lowercase();
        self.campaign.iter().find(|row| row.matches(&title, &cell))
    }

    /// Merges one campaign race's own medal into the row `(title, cell)`
    /// names, creating it if this is the first result ever recorded there.
    ///
    /// **The same best-of/last shape as [`Self::record`], on the one field
    /// this table carries a medal for.** `medal` is `None` both for a race
    /// with no campaign cell in play - which is never this function's
    /// caller's business to call it for at all - and for a race that *was*
    /// run against `cell` but scored no tier: [`CampaignRecord::last_medal`]
    /// still takes that `None`, the same "last means last, better or worse"
    /// rule [`Record::last_medal`] follows, while
    /// [`CampaignRecord::best_medal`] is left untouched, the same
    /// never-downgraded rule [`Self::record`] applies to
    /// [`Record::best_medal`].
    ///
    /// **`difficulty` breaks a tie against [`Medal::better`], chosen rather
    /// than measured.** `None` on every Pulse call (that title never has
    /// one), which makes this identical to the pre-2026-09-28 behaviour:
    /// falls straight through to comparing `medal` alone. On Wipeout HD,
    /// when both the stored and the new result know their own rung and the
    /// rungs differ, the **harder** rung wins outright, whatever either
    /// medal is - a bronze just earned at `Hard` replaces a stored `Gold`
    /// earned at `Easy`. This is not read off a found comparison function
    /// (`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s medal-law
    /// section explicitly did not find one); it generalises
    /// `SaveData_MigrateCellMedalsToHardElite`'s own one-time "credit
    /// existing medals at the hardest rung" grandfather clause into an
    /// ongoing rule, on the reasoning that a game which upgrades old medals
    /// to look as good as possible at the hardest difficulty is unlikely to
    /// then let an easier medal outrank a harder one during ordinary play -
    /// but that reasoning is this project's, not the executable's. When the
    /// stored row predates this field (`best_difficulty: None`) or a rung
    /// genuinely is not known for one side, this falls back to comparing
    /// `medal` alone, the same as before this field existed.
    pub fn record_campaign(
        &mut self,
        title: &str,
        cell: &str,
        medal: Option<Medal>,
        difficulty: Option<Difficulty>,
    ) {
        let title = title.trim().to_ascii_lowercase();
        let cell = cell.trim().to_ascii_lowercase();
        let row = match self
            .campaign
            .iter_mut()
            .find(|row| row.matches(&title, &cell))
        {
            Some(row) => row,
            None => {
                self.campaign.push(CampaignRecord {
                    title,
                    cell,
                    ..CampaignRecord::default()
                });
                self.campaign
                    .last_mut()
                    .expect("just pushed onto this exact vec")
            }
        };
        if let Some(medal) = medal {
            let improves = match (row.best_medal, row.best_difficulty, difficulty) {
                (None, ..) => true,
                (Some(_), Some(best), Some(new)) if new != best => new > best,
                (Some(best_medal), _, _) => medal < best_medal,
            };
            if improves {
                row.best_medal = Some(medal);
                row.best_difficulty = difficulty;
                row.best_points = row.best_medal.map(Medal::points);
            }
        }
        row.last_medal = medal;
        row.last_difficulty = difficulty;

        // Stable, for the same reason `Self::record` sorts `self.records`.
        self.campaign
            .sort_by(|a, b| (&a.title, &a.cell).cmp(&(&b.title, &b.cell)));
    }

    /// The team `(title, team)` names' own running total - `0` for a team
    /// never raced, matching the original's own fresh-profile reading
    /// (`EndRace Rewards`' `Total loyalty: 0`).
    #[must_use]
    pub fn loyalty_total(&self, title: &str, team: &str) -> u32 {
        let title = title.trim().to_ascii_lowercase();
        let team = team.trim().to_ascii_lowercase();
        self.loyalty
            .iter()
            .find(|row| row.matches(&title, &team))
            .map_or(0, |row| row.total)
    }

    /// Adds `award` to `(title, team)`'s own running total, creating the row
    /// if this is the first race ever recorded for it, and returns the new
    /// total - `Loyalty_AccumulateTotal`'s own `record+8 += award`, capped
    /// the same place the original caps it: `100000`.
    pub fn record_loyalty(&mut self, title: &str, team: &str, award: u32) -> u32 {
        const CAP: u32 = 100_000;
        let title = title.trim().to_ascii_lowercase();
        let team = team.trim().to_ascii_lowercase();
        let row = match self
            .loyalty
            .iter_mut()
            .find(|row| row.matches(&title, &team))
        {
            Some(row) => row,
            None => {
                self.loyalty.push(LoyaltyRecord {
                    title,
                    team,
                    total: 0,
                });
                self.loyalty
                    .last_mut()
                    .expect("just pushed onto this exact vec")
            }
        };
        row.total = row.total.saturating_add(award).min(CAP);
        let total = row.total;

        // Stable, for the same reason `Self::record`/`Self::record_campaign`
        // sort their own tables.
        self.loyalty
            .sort_by(|a, b| (&a.title, &a.team).cmp(&(&b.title, &b.team)));
        total
    }
}

/// The footer ticker's own honest tip rotation - the `TKR_NO*` family, the
/// only strings on disc that carry no `%d`/`%s`/`%.2f` template this build has
/// a real counter for. See `oag_ui::campaign::footer`'s own module doc for why
/// nothing here invents a play-time or song-count statistic instead.
///
/// **One function, three callers that must agree.** The Race Campaign's own
/// screens (`CampaignStage::ticker_tips` in the `oag-game` binary), the
/// ordinary menu pages (`Session::draw`'s call into `MenuStage::render`, same
/// binary) and `--menu-page`'s still (`capture::menu_page`, this crate) all
/// rotate through the same list off the same save file - a live session and a
/// screenshot of it must not show two different tips for one
/// `<config dir>/oag/records.toml`. Living here rather than beside either
/// `CampaignStage` or `capture::menu_page` is what lets both reach it: neither
/// binary module is visible to this crate's own `capture`, and this crate
/// cannot depend on the binary the other way (`workspace-layout.md`'s rule 2).
///
/// `Tournament`/`Head2Head` never launch in this engine
/// (`crate::campaign::race_mode_for_cell`), so `TKR_NOTOURN`/`TKR_NOHH` are
/// unconditionally included; the other five are gated on whether `store`
/// carries any row for that mode - `oag_race::Mode::name`'s own spelling, the
/// same string [`Key::new`] normalises every record's `mode` to - which reads
/// real save data rather than a guess, at the cost of never re-showing a tip
/// once its mode has been raced even once.
#[must_use]
pub fn ticker_tips(strings: &oag_ui::language::StringTable, store: &Store) -> Vec<String> {
    let never_raced = |mode: &str| !store.rows().iter().any(|row| row.mode == mode);
    let mut ids = vec!["TKR_NOTOURN", "TKR_NOHH"];
    if never_raced(oag_race::Mode::SingleRace.name()) {
        ids.push("TKR_NOSR");
    }
    if never_raced(oag_race::Mode::TimeTrial.name()) {
        ids.push("TKR_NOTT");
    }
    if never_raced(oag_race::Mode::SpeedLap.name()) {
        ids.push("TKR_NOSL");
    }
    if never_raced(oag_race::Mode::Zone.name()) {
        ids.push("TKR_NOZONE");
    }
    if never_raced(oag_race::Mode::Eliminator.name()) {
        ids.push("TKR_NOELIM");
    }
    ids.into_iter()
        .filter_map(|id| strings.get(id))
        .map(str::to_string)
        .collect()
}

/// Parses `text` as a records file, keeping every row that decodes and
/// noting - never discarding the whole file over - one that does not.
///
/// **A malformed row is not the same failure as a malformed document.** This
/// walks the `records` array one entry at a time; a row missing one of the
/// four key fields, or carrying a value of the wrong shape, is dropped and
/// named in the returned notes, and every other row is kept. See the module
/// doc's "deliberately not `Settings`'s own rule" section for why that is the
/// opposite of what `settings.rs` does on purpose.
///
/// # Errors
///
/// Only when `text` is not valid TOML *at all* - there is no `records` array
/// to salvage rows from at that point. [`load`] is what decides what happens
/// to the file when that happens; this function only reports it.
pub fn parse(text: &str) -> Result<(Store, Vec<String>)> {
    let table: toml::Table = text.parse().context("parsing records")?;
    let mut notes = Vec::new();
    let mut records = Vec::new();
    match table.get("records") {
        Some(toml::Value::Array(rows)) => {
            for (index, row) in rows.iter().enumerate() {
                match row.clone().try_into::<Record>() {
                    Ok(record) => records.push(record),
                    Err(e) => notes.push(format!("records[{index}] dropped: {e}")),
                }
            }
        }
        // Absent is the ordinary case of a fresh file; present-but-wrong-shape
        // is a hand edit or a future format this build cannot read, and
        // starts empty the same way rather than refusing to boot over it.
        Some(_) => notes.push("`records` is not an array of tables; ignoring it".to_string()),
        None => {}
    }
    records.sort_by_key(Record::key);

    // `campaign` is read the same tolerant way, and its own absence - every
    // file this project wrote before this table existed - is exactly the
    // ordinary "fresh file" case above, not a degraded read.
    let mut campaign = Vec::new();
    match table.get("campaign") {
        Some(toml::Value::Array(rows)) => {
            for (index, row) in rows.iter().enumerate() {
                match row.clone().try_into::<CampaignRecord>() {
                    Ok(record) => campaign.push(record),
                    Err(e) => notes.push(format!("campaign[{index}] dropped: {e}")),
                }
            }
        }
        Some(_) => notes.push("`campaign` is not an array of tables; ignoring it".to_string()),
        None => {}
    }
    campaign.sort_by(|a, b| (&a.title, &a.cell).cmp(&(&b.title, &b.cell)));

    // `loyalty` is read the same tolerant way, for the same "every file
    // written before this table existed" reason `campaign` is.
    let mut loyalty = Vec::new();
    match table.get("loyalty") {
        Some(toml::Value::Array(rows)) => {
            for (index, row) in rows.iter().enumerate() {
                match row.clone().try_into::<LoyaltyRecord>() {
                    Ok(record) => loyalty.push(record),
                    Err(e) => notes.push(format!("loyalty[{index}] dropped: {e}")),
                }
            }
        }
        Some(_) => notes.push("`loyalty` is not an array of tables; ignoring it".to_string()),
        None => {}
    }
    loyalty.sort_by(|a, b| (&a.title, &a.team).cmp(&(&b.title, &b.team)));

    Ok((
        Store {
            records,
            campaign,
            loyalty,
        },
        notes,
    ))
}

/// Where the records file lives: `<config dir>/oag/records.toml`, beside
/// `settings.toml` and the `pilots/` directory.
#[must_use]
pub fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("oag").join("records.toml"))
}

const HEADER: &str = "\
# OpenAntiGrav race records - best lap, best total time and the last result,
# per circuit, mode and speed class.
#
# This project's own schema, not the original's save format - see
# `crates/game/src/records.rs`'s module doc. Rewritten on every change with
# every row present; a row missing a key this build does not recognise is
# dropped rather than failing the file, and hand edits are not preserved
# across a rewrite the way a pilot file's comments are.

";

/// Loads the records file, or starts empty when there is none yet, it cannot
/// be read, or it will not parse.
///
/// **Never fails, and never panics** - see the module doc's own section on
/// why this diverges from [`crate::settings::load`]. Every failure this can
/// hit degrades to [`Store::default`] plus a logged line:
///
/// - No config directory on this platform: silent, the same case
///   [`crate::settings::load`] and [`crate::pilots::load`] both treat as
///   "nothing to persist to" rather than an error.
/// - No file yet: silent, the ordinary case of a player who has not finished
///   a race since installing this.
/// - The file exists and will not read (permissions, mid-write on another
///   process): logged, starts empty.
/// - The file reads but is not valid TOML: logged, moved aside to
///   `records.toml.invalid` next to itself rather than overwritten in place,
///   starts empty. A second bad file in a row simply replaces the first
///   `.invalid` backup - this keeps one, not a growing pile.
/// - The file is valid TOML but one or more rows do not decode: logged per
///   row, every other row is kept. This is the ordinary partial-corruption
///   case the whole design exists for.
#[must_use]
pub fn load() -> Store {
    let Some(path) = path() else {
        return Store::default();
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Store::default(),
        Err(e) => {
            error!(
                "could not read {}: {e:#}; starting with no race records",
                path.display()
            );
            return Store::default();
        }
    };
    match parse(&text) {
        Ok((store, notes)) => {
            for note in &notes {
                warn!("{}: {note}", path.display());
            }
            store
        }
        Err(e) => {
            let backup = path.with_file_name(format!(
                "{}.invalid",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
            match std::fs::rename(&path, &backup) {
                Ok(()) => error!(
                    "{} would not parse ({e:#}); moved aside to {} and starting with no race \
                     records",
                    path.display(),
                    backup.display()
                ),
                Err(rename_err) => error!(
                    "{} would not parse ({e:#}) and could not be moved aside ({rename_err:#}); \
                     starting with no race records, but the broken file is untouched",
                    path.display()
                ),
            }
            Store::default()
        }
    }
}

/// Writes `store` back to [`path`], creating the directory if this is the
/// first record saved on this machine.
///
/// # Errors
///
/// Propagates a directory that cannot be created and a file that cannot be
/// written - the same contract [`crate::settings::save`] makes, and the same
/// thing the caller does with the failure: log it and carry on, never panic
/// or abort a race over a write that did not land.
pub fn save(store: &Store) -> Result<()> {
    let Some(path) = path() else {
        return Ok(());
    };
    let text = format!(
        "{HEADER}{}",
        toml::to_string_pretty(store).context("serialising race records")?
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests;
