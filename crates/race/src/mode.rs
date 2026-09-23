//! The race modes, and what each one does differently.
//!
//! Three single-ship modes came first, because none of them needs opponents,
//! weapons or a grid: they are the part of the race layer that can be finished
//! rather than stubbed. [`Mode::SingleRace`] is the fourth and is the first that
//! races with weapons - see its own docs for what it still does without.
//!
//! # Where the rules come from
//!
//! Each rule below is either observed on the running game, read out of the
//! executable, or ours. They are not interchangeable and the doc comments say
//! which is which - see `docs/gameplay/race-modes.md` for the evidence and the
//! confidence score behind every one.

pub use oag_tables::handling::SpeedClass;

/// One of the race modes this crate implements.
///
/// The variants are ordered as the menu offers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// A fixed number of laps against the clock. The default, and the mode the
    /// menu opens on.
    #[default]
    TimeTrial,
    /// Unlimited laps, chasing one fast lap. Never ends on its own.
    SpeedLap,
    /// Escalating auto-speed. Never ends on its own either, for now - see
    /// [`crate::state`].
    Zone,
    /// A lap race with weapons.
    ///
    /// **The mode that makes pickups reachable at all**, and the reason it was
    /// added: the original clears `g_weapons_enabled` for all three modes above,
    /// and a weapons-off race in the original does not merely skip the pickup
    /// logic - it hides every `Weapon Pad` and empties the trigger list. The
    /// disc corroborates that from the presentation side: `TimeTrial_HUD.xml`
    /// and `Zone_HUD.xml` carry no pickup widgets at all, while
    /// `Arcade_HUD.xml` - this mode's layout - carries `PickupBackground` and
    /// one icon per weapon. See `docs/gameplay/pickups.md`.
    ///
    /// **What this build does without, and the original does not.** The disc's
    /// own description is *"Single Race: take on a full grid of opponents,
    /// weapons optional"* (`MSC_EVENT_SR`), so the original races this with AI.
    /// This build races it with AI too: [`Self::has_opponents`] answers `true`,
    /// and a full grid of seven driven opponents takes the start.
    SingleRace,
    /// Race for kills, not position: a full grid, weapons on, no lap target.
    ///
    /// **Recovered from the disc's own event text**, `MSC_EVENT_ELIM`:
    /// *"Eliminator: race for kills, not position, against a full grid of
    /// trigger happy contenders in a weapons-heavy environment. Weapons do
    /// more damage, and you cannot absorb pickups, but you regain health
    /// after each lap. The lap count is not fixed, and the race will end
    /// when the kill count is reached."* Three mechanics follow directly
    /// from that sentence and none of them was known before the
    /// 2026-09-08 campaign scoping pass: no pickup absorption, scaled
    /// weapon damage, and a kill-count ending. See
    /// `docs/gameplay/race-modes.md#eliminator`.
    Eliminator,
    /// One leg of a series of single races, scored as one event - see
    /// [`crate::tournament`] for the points law and
    /// `docs/gameplay/race-modes.md#tournament` for the full picture.
    ///
    /// **Every per-leg racing rule mirrors [`Self::SingleRace`], and that is
    /// measured rather than assumed.** `docs/formats/race-setup.md`'s own
    /// reading of `docs/ui/hud.md`'s five-layout census finds no
    /// `Tournament_HUD.xml` at all - `Arcade_HUD.xml`, single race's own
    /// layout, is read there as "the single-race and tournament layout" - and
    /// `MSC_EVENT_TOURN` itself calls a tournament "a series of single
    /// races". So [`Self::has_opponents`], [`Self::weapons_enabled`],
    /// [`Self::pickups_absorb`] and [`Self::laps_target`] all answer exactly
    /// as [`Self::SingleRace`] does for this variant - a leg is not a new set
    /// of in-race rules, only the campaign bookkeeping around it is new.
    ///
    /// **Deliberately absent from [`Self::ALL`].** Every other variant there
    /// is something the RACE page's own `values_from = "race_modes"` row
    /// (`oag_ui::menu::mode_choices`) can launch by picking one track and
    /// pressing go; a tournament cannot, because the original's own
    /// `Tournament C` (`Racebox`'s redirect target for `Mode == Tournament`,
    /// `docs/formats/race-setup.md`) is a twelve-slot leg picker this engine
    /// does not read yet. Exposing this variant there would offer a
    /// one-track "tournament" the disc never shows a player - a stand-in this
    /// project's own rule against inventing presentation forbids - so a
    /// tournament is reachable only through a campaign cell's own
    /// `tournament_tracks`, which already names every leg
    /// (`Session::launch_campaign_cell`).
    Tournament,
}

impl Mode {
    /// Every mode a plain "pick a track, press go" launch can run, as a
    /// fixed-size array.
    ///
    /// Fixed-size so that adding one of *these* modes is a compile error at
    /// every caller that enumerates them, rather than a silently short list.
    /// The same reason `menu::Action::all` and `perf::FrameLimit::OFFERED`
    /// are arrays in the composition root.
    ///
    /// **[`Self::Tournament`] is not here** - see its own doc comment for
    /// why a tournament needs a leg list rather than a single track and
    /// cannot be reached from this array's own consumer, the RACE page's
    /// mode row.
    pub const ALL: [Self; 5] = [
        Self::TimeTrial,
        Self::SpeedLap,
        Self::Zone,
        Self::SingleRace,
        Self::Eliminator,
    ];

    /// Laps in a time trial, per speed class, slowest rung first.
    ///
    /// **Observed live at all four rungs, 2026-09-09, and it replaced this
    /// module's second-weakest number.** This used to be a flat
    /// `TIME_TRIAL_LAPS = 3`, carried over unresolved from the same census that
    /// fixed [`Self::SINGLE_RACE_LAPS_BY_CLASS`]: the campaign's 47 `Time Trial`
    /// cells read the identical 3/4/4/5 table, but the flat constant had its own
    /// independent live evidence (`Lap 1 of 3`) and nobody had driven a time
    /// trial in a class above Venom to check whether the census applied outside
    /// the campaign too. It does: a Custom Race - no campaign cell in play at
    /// all - launched once per rung under PPSSPP (`pulse-psp-usa.chd`, Talon's
    /// Junction White) reads `Lap 1 of 3` on Venom, `Lap 1 of 4` on Flash,
    /// `Lap 1 of 4` on Rapier and `Lap 1 of 5` on Phantom - one screenshot per
    /// rung, all four matching the census exactly. Confidence **90**, level with
    /// [`Self::SINGLE_RACE_LAPS_BY_CLASS`]: that table rests on a larger census
    /// (236 cells against 47) plus one live point; this one rests on a smaller
    /// census plus a live point at every rung it has, which is the stronger
    /// half of the same trade.
    ///
    /// A run that passes the last lap starts a fresh attempt with the best time
    /// cleared rather than ending outright - see
    /// `docs/reverse-engineering/ppsspp-debugger.md:707-709` - which this crate
    /// still approximates as an ending; see [`Self::laps_target`]'s own docs.
    /// Note the count is *configuration* in the original rather than a constant:
    /// the race-setup format string carries `laps="%d"`.
    pub const TIME_TRIAL_LAPS_BY_CLASS: [u32; SpeedClass::ALL.len()] = [3, 4, 4, 5];

    /// Laps in a single race, per speed class, slowest rung first.
    ///
    /// **Measured, confidence 90, and it replaced this module's weakest
    /// number.** This used to be a flat `SINGLE_RACE_LAPS = 3` that said of
    /// itself that it was a guess carried over from time trial's own flat
    /// constant (now [`Self::TIME_TRIAL_LAPS_BY_CLASS`], not flat either). The
    /// campaign's own authored content settled it: `Data\Plugins\grids\grid_00.xml`
    /// .. `grid_15.xml` inside
    /// `Data.wad` hold **236 `PI_Cell` records**, each carrying a `laps`
    /// attribute, and across all 236 that attribute is *exactly* **3 for
    /// Venom, 4 for Flash, 4 for Rapier and 5 for Phantom**, with no
    /// exception. A flat census over 236 authored records - see
    /// [`race-campaign.md`](../../../docs/ghidra/functions/psp-pulse-usa/race-campaign.md).
    ///
    /// **A table rather than a transcription.** The rule `CLAUDE.md` states -
    /// never hand-transcribe what the assets author - is about one record read
    /// by hand standing in for a population. Here the census *is* the
    /// measurement and this array is its shape: four entries because the
    /// attribute varies along exactly one axis, the speed class, and along no
    /// other. The guess it replaced was right for Venom and wrong for the
    /// other three, which is why three of four classes raced short.
    ///
    /// **Indexed by [`SpeedClass`], and the order is that enum's own.**
    /// `SpeedClass::ALL` is slowest-first and its position is the
    /// discriminant, which is what the four-wide handling tables already rely
    /// on; a fifth rung is deliberately not representable, for the reason
    /// `oag_title::SpeedClasses::VECTOR` gives at length. Resolving a title's
    /// class *name* onto this enum is the caller's job and happens in
    /// `oag_game::race::Race::start`, where the name already lives.
    ///
    /// **This table is the fallback, not the authority.** A race launched from
    /// the campaign should take the lap count from *its own* cell -
    /// `oag_tables::race_campaign::Cell` already parses `laps: Option<u32>` -
    /// and the cell's value wins over this array the moment a campaign launch
    /// can be wired, the same retirement clause
    /// [`Self::ELIMINATOR_KILL_TARGET_DEFAULT`] carries. What keeps the array
    /// necessary is that nothing selects a cell yet, and a Custom Race started
    /// outside the campaign has no cell to read at all.
    pub const SINGLE_RACE_LAPS_BY_CLASS: [u32; SpeedClass::ALL.len()] = [3, 4, 4, 5];

    /// Kills that end an Eliminator event, **when nothing more specific is
    /// available**.
    ///
    /// **Not the flat, measured number this constant used to claim.** It read
    /// `FEData.wad`'s 24-entry per-track family as authoring an identical
    /// `<Targets Elimination="10">` on every one and called that a flat
    /// census; `progress`'s Ghidra pass (2026-09-08, after this constant was
    /// first written) found the number was never flat at all -
    /// `Data\Plugins\grids\grid_00..15.xml` authors 236 `PI_Cell` records,
    /// each with its **own** gold-medal kill target, and the values actually
    /// used are **10, 7 and 5**, not one figure. `FEData.wad`'s `10` was one
    /// sample of three read as the whole population. See
    /// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`.
    ///
    /// **So this is a chosen default, not a recovery, kept only because the
    /// campaign grid this number actually lives on is not wired into this
    /// engine at all** - there is no `PI_Cell` for a `--mode eliminator` race
    /// started outside the campaign to read a target from. `10` is one of the
    /// three real values rather than an invented one, which is the one thing
    /// that keeps this from being the kind of number `CLAUDE.md` forbids
    /// authoring - but it carries no confidence score, because a default is
    /// not a measurement. `oag_game::race::Options::eliminator_kill_target`
    /// is where a caller overrides it; wiring the campaign grid to supply the
    /// cell's own value is the fix that retires this constant outright.
    ///
    /// **Whose kill count ends the race is measured, and it is not only the
    /// player's.** `progress`'s same pass reads the ending off `entity+0x8d8`
    /// on *any* craft - the race ends the moment one ship's own tally reaches
    /// the target, whichever ship that is. [`RaceState::eliminator_finished`]
    /// takes the target and the highest count across the whole field for
    /// exactly this reason, rather than the player's own alone.
    pub const ELIMINATOR_KILL_TARGET_DEFAULT: u32 = 10;

    /// The token this mode is stored and configured as.
    ///
    /// Lowercase with underscores, matching the speed-class and team rows the
    /// menu already has. This is a settings key, not a label: what the player
    /// reads is the menu row's own `label`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::TimeTrial => "time_trial",
            Self::SpeedLap => "speed_lap",
            Self::Zone => "zone",
            Self::SingleRace => "single_race",
            Self::Eliminator => "eliminator",
            Self::Tournament => "tournament",
        }
    }

    /// The mode a token names, or `None` if nothing does.
    ///
    /// `None` rather than a default, so a settings file carrying a mode this
    /// build does not have is visible to the caller instead of silently becoming
    /// a time trial.
    ///
    /// **Never resolves `"tournament"`**, because it only searches
    /// [`Self::ALL`] - see [`Self::Tournament`]'s own doc for why it is
    /// excluded there. A settings file or a `--mode` flag naming it falls
    /// back the same way an unknown token always has; only
    /// `Session::launch_campaign_cell` ever constructs this variant.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }

    /// Laps the mode finishes after in `class`, or `None` when it never ends
    /// on its own.
    ///
    /// **The class arrives as a parameter rather than as a crate dependency.**
    /// The lap count is per speed class - see
    /// [`Self::SINGLE_RACE_LAPS_BY_CLASS`] - so this method needs the class,
    /// and [`SpeedClass`] comes from `oag-formats`, which this crate already
    /// depends on for [`crate::Course`]'s own track types. `oag-title`, where a
    /// title's *ladder* is measured, is deliberately not added: that crate
    /// carries class **names** because a ladder's length is per-title data, and
    /// a race-rules crate has no business resolving a name. The caller
    /// resolves; this method matches.
    ///
    /// **[`Mode::TimeTrial`] and [`Mode::SingleRace`] both vary with the
    /// class, and both are now checked live rather than resting on the census
    /// alone.** The 236-record census reads `Time Trial`'s 47 cells as
    /// following the identical 3/4/4/5 table `Single Race`'s do, and a Custom
    /// Race launched in each of the four classes - no campaign cell in play at
    /// all - reads `Lap 1 of 3`/`4`/`4`/`5` in exactly that order under
    /// PPSSPP. So the table is not a campaign peculiarity: it is what a plain
    /// Custom Race is configured with too. See [`Self::TIME_TRIAL_LAPS_BY_CLASS`].
    ///
    /// **`None` for Eliminator, measured rather than assumed from "no lap
    /// target here either".** `MSC_EVENT_ELIM` says outright: *"The lap count
    /// is not fixed, and the race will end when the kill count is reached"* -
    /// a kill count ends it instead, and that ending is not this method's to
    /// report; see [`crate::RaceState::eliminator_finished`]. The census agrees
    /// from the data side: an `Elimination` cell carries no `laps` attribute at
    /// all.
    ///
    /// **`None` for Speed Lap, and the reason is now settled rather than
    /// merely asserted, confidence 75.** The census reads `Speed Lap` as `7`
    /// on all 42 of its cells, and a Custom Race shows exactly that: `Lap 1
    /// of 7`, live, on Venom, under PPSSPP - so the `7` is real and is not
    /// campaign-only either. It still does not end the race. Speed Lap's own
    /// in-race pause menu carries a seventh row, `END SESSION`, that neither
    /// Time Trial's nor [`Mode::SingleRace`]'s pause menu has - both are
    /// otherwise identical, six rows each, and both are modes that *do* end
    /// on their own lap count, checked as the falsifier this claim needs
    /// rather than assumed safe to skip. Three modes, screenshotted,
    /// 2026-09-09: the one row present exactly where the mode's own text says
    /// it never ends. A mode whose own pause menu offers a dedicated way to
    /// deliberately conclude an open-ended run is a mode that does not
    /// conclude one on its own - independent of, and agreeing with,
    /// `MSC_EVENT_SL`'s text: *"never ends, escape leaves"*. 75 rather than
    /// higher because this is still a correlated UI signal standing in for a
    /// lap-8 crossing nobody watched directly - an open-loop scripted replay
    /// could not complete even one lap of Talon's Junction in nine real-time
    /// minutes to check it the direct way. The `7` itself is display
    /// convention shared with Time Trial's own `Lap X of Y` widget, not a
    /// target this method should report.
    ///
    /// **`None` for Zone too**, on `MSC_EVENT_ZONE`'s identical wording; the
    /// census's `0` there reads as `laps` not being the applicable field for a
    /// mode counting zones instead - see `docs/gameplay/race-modes.md`.
    #[must_use]
    pub const fn laps_target(self, class: SpeedClass) -> Option<u32> {
        match self {
            Self::TimeTrial => Some(Self::TIME_TRIAL_LAPS_BY_CLASS[class as usize]),
            // A tournament leg races the same table `Single Race` does - see
            // `Self::Tournament`'s own doc. The census `race-setup.md` reads
            // (3/4/4/5, no exception across all 236 `PI_Cell` records) covers
            // every mode's cells, `Tournament`'s 27 included.
            Self::SingleRace | Self::Tournament => {
                Some(Self::SINGLE_RACE_LAPS_BY_CLASS[class as usize])
            }
            Self::SpeedLap | Self::Zone | Self::Eliminator => None,
        }
    }

    /// The string-table id whose text names this mode.
    ///
    /// **The disc names the modes; this repository does not.** These entries are
    /// the front end's event descriptions and each one opens with the mode's own
    /// name followed by a colon - `"Zone: your ship accelerates automatically
    /// and the top speed increases after every ten second period..."` - so the
    /// name is the part before that colon. See
    /// `oag_game::menu::mode_label`, which does the splitting, and
    /// `the_string_table_names_the_race_modes` in
    /// `crates/game/tests/boot_ground_truth.rs`, which is where the ids were
    /// found.
    ///
    /// Localised for free: a French disc's table answers the same ids in French.
    #[must_use]
    pub const fn string_id(self) -> &'static str {
        match self {
            Self::TimeTrial => "MSC_EVENT_TT",
            Self::SpeedLap => "MSC_EVENT_SL",
            Self::Zone => "MSC_EVENT_ZONE",
            Self::SingleRace => "MSC_EVENT_SR",
            Self::Eliminator => "MSC_EVENT_ELIM",
            Self::Tournament => "MSC_EVENT_TOURN",
        }
    }

    /// What to show when the disc has nothing to say.
    ///
    /// Only reached on a source whose string table is missing or does not carry
    /// [`Self::string_id`]. Not a translation and not authored content - it is
    /// the settings token, spaced out, so a row is never blank.
    #[must_use]
    pub const fn fallback_label(self) -> &'static str {
        match self {
            Self::TimeTrial => "TIME TRIAL",
            Self::SpeedLap => "SPEED LAP",
            Self::Zone => "ZONE",
            Self::SingleRace => "SINGLE RACE",
            Self::Eliminator => "ELIMINATOR",
            Self::Tournament => "TOURNAMENT",
        }
    }

    /// Whether the mode drives the throttle itself.
    ///
    /// Zone does: the original replaces the engine's thrust with an auto-speed
    /// law and disables the brakes entirely. See
    /// `docs/ghidra/functions/psp-pulse-usa/engine.md`.
    #[must_use]
    pub const fn is_auto_throttle(self) -> bool {
        matches!(self, Self::Zone)
    }

    /// Whether *this build* races this mode with other craft on the grid.
    ///
    /// **`false` for every mode, and for two different reasons.**
    ///
    /// For the three single-ship modes it is the original's own answer, measured
    /// on the running game rather than assumed from "single-ship": selecting
    /// TIME TRIAL, SPEED LAP or ZONE on the Custom Race screen greys
    /// `AI DIFFICULTY` to `N/A`, the same tell `race-modes.md` already uses for
    /// weapons - see [`Self::weapons_enabled`].
    ///
    /// **For [`Mode::SingleRace`] it is `true`, and agrees with the original.**
    /// `AI DIFFICULTY` is selectable there and `MSC_EVENT_SR` promises "a full
    /// grid of opponents". This method returned `false` until 2026-08-11 for a
    /// reason that has now gone: nothing drove an opponent, and seven parked
    /// hulls would have read as a regression rather than as progress. `oag-ai`
    /// drives them, so the field is real. What it is *not* yet is a field of
    /// distinguishable craft - they still wear the player's livery, which is the
    /// separate roadmap item.
    ///
    /// `oag_game::race::Options::opponents` remains the escape hatch the grid's
    /// own ground-truth test uses to place all eight from a mode that does not
    /// ask for them.
    ///
    /// **`true` for Eliminator too, and this one needs no escape hatch to be
    /// honest about it**: `MSC_EVENT_ELIM` promises *"a full grid of trigger
    /// happy contenders"*, stronger wording than a single race's "optional"
    /// opponents.
    ///
    /// **`true` for Tournament, mirroring [`Self::SingleRace`]** - see
    /// [`Self::Tournament`]'s own doc for the evidence that a leg is raced
    /// exactly like an ordinary single race.
    #[must_use]
    pub const fn has_opponents(self) -> bool {
        matches!(self, Self::SingleRace | Self::Eliminator | Self::Tournament)
    }

    /// Whether the original arms `Weapon Pad`s for this mode.
    ///
    /// **`false` for the three single-ship modes, measured on the running
    /// original; `true` for [`Mode::SingleRace`].** Selecting
    /// each of TIME TRIAL, SPEED LAP and ZONE on the Custom Race screen greys
    /// the `WEAPONS` row to `OFF` and the setting cannot be changed - confirmed
    /// live, 2026-08-10, PPSSPP v1.20.4 under Xvfb, one screenshot per race
    /// type. `Race_ReadSetupOptions` (`0x08896b84`) corroborates it in code:
    /// modes `5`/`10` (time trial/speed lap) hard-code
    /// `g_weapons_enabled = 0` and route around the `<Weapons>` setup
    /// attribute entirely, so no menu path can turn it back on. Zone (mode
    /// `6`) defaults the global to `1` but the front end always supplies an
    /// explicit `<Weapons>Off</Weapons>` for it, which is what the greyed row
    /// is showing.
    ///
    /// The original does more than skip the pickup logic when this is `false`:
    /// `World_CollectNodeLists` (`0x088879d4`) clears every `Weapon Pad`
    /// node's visibility bit (`node+0x2c &= ~4`, the same bit
    /// `exhaust.md` names for the boost plume) and zeroes the trigger list's
    /// own count, so a weapons-off race neither draws them nor can trigger
    /// them - not merely "nothing happens if you cross one". See
    /// [`docs/ghidra/functions/psp-pulse-usa/pads.md`](../../../docs/ghidra/functions/psp-pulse-usa/pads.md).
    ///
    /// Single race is on the other side of that switch: it is not in
    /// `Race_ReadSetupOptions`' weapons-off set, its `AI DIFFICULTY` and
    /// `WEAPONS` rows are both selectable, and `MSC_EVENT_SR` calls weapons
    /// "optional". **This build takes the default rather than offering the
    /// choice** - there is no `WEAPONS` row on the race menu, so a single race
    /// always has them on. That is a missing setting, not a different rule.
    ///
    /// **`true` for Eliminator too** - it is the mode `MSC_EVENT_ELIM` calls
    /// *"a weapons-heavy environment"*, and `Race_ReadSetupOptions`' own
    /// weapons-off set (modes `5`/`10`, time trial and speed lap) does not
    /// include it.
    ///
    /// **`true` for Tournament, mirroring [`Self::SingleRace`]** - see
    /// [`Self::Tournament`]'s own doc.
    #[must_use]
    pub const fn weapons_enabled(self) -> bool {
        matches!(self, Self::SingleRace | Self::Eliminator | Self::Tournament)
    }

    /// Whether the mode lets a hit pickup pad refill the shield pool.
    ///
    /// **`true` everywhere except Eliminator, and that one is measured
    /// rather than assumed from "an Eliminator craft has other things to
    /// worry about".** `MSC_EVENT_ELIM` says outright: *"you cannot absorb
    /// pickups, but you regain health after each lap"* - two mechanics in one
    /// sentence, and this method is the first of them. The second - a lap
    /// completion refilling the pool instead - touches [`ShipState::shield`],
    /// which this crate cannot see (`oag-race`'s `Cargo.toml` says why); it is
    /// `oag_game::race::Race::tick`'s to apply, on the same `lap_completed`
    /// edge the free Time Trial/Speed Lap turbo already reads.
    ///
    /// **`true` for Tournament too, mirroring [`Self::SingleRace`]** - see
    /// [`Self::Tournament`]'s own doc; nothing in `MSC_EVENT_TOURN`'s text
    /// says otherwise.
    ///
    /// [`ShipState::shield`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/physics/src/lib.rs
    #[must_use]
    pub const fn pickups_absorb(self) -> bool {
        !matches!(self, Self::Eliminator)
    }
}

#[cfg(test)]
mod tests {
    use super::{Mode, SpeedClass};

    #[test]
    fn every_mode_round_trips_through_its_token() {
        for mode in Mode::ALL {
            assert_eq!(Mode::from_name(mode.name()), Some(mode));
        }
    }

    /// See [`Mode::ALL`]'s own doc for why: a one-track launch cannot run
    /// what the disc's own `Tournament C` needs, a leg list.
    #[test]
    fn tournament_is_not_in_all() {
        assert!(!Mode::ALL.contains(&Mode::Tournament));
    }

    /// A leg races exactly like a single race - see [`Mode::Tournament`]'s
    /// own doc for the evidence.
    #[test]
    fn tournament_mirrors_single_race() {
        for class in SpeedClass::ALL {
            assert_eq!(
                Mode::Tournament.laps_target(class),
                Mode::SingleRace.laps_target(class),
                "{class} disagrees with single race"
            );
        }
        assert!(Mode::Tournament.has_opponents());
        assert!(Mode::Tournament.weapons_enabled());
        assert!(Mode::Tournament.pickups_absorb());
        assert_eq!(Mode::Tournament.string_id(), "MSC_EVENT_TOURN");
    }

    #[test]
    fn tokens_are_distinct() {
        let mut names: Vec<&str> = Mode::ALL.iter().map(|mode| mode.name()).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "two modes share a token");
    }

    #[test]
    fn an_unknown_token_is_none_rather_than_a_default() {
        // `eliminator` is a real mode from 2026-09-08 and belongs in
        // `every_mode_round_trips_through_its_token` instead - kept here as
        // `elimination` (Eliminator's own descriptive token in the disc's
        // string table is `MSC_EVENT_ELIM`, not this crate's `name()`) so
        // the "unknown token" shape this test checks is not lost.
        assert_eq!(Mode::from_name("elimination"), None);
        assert_eq!(Mode::from_name(""), None);
        assert_eq!(Mode::from_name("TIME_TRIAL"), None);
    }

    #[test]
    fn the_default_is_the_time_trial() {
        assert_eq!(Mode::default(), Mode::TimeTrial);
        assert_eq!(Mode::ALL[0], Mode::TimeTrial);
    }

    #[test]
    fn only_the_two_unlimited_modes_never_end_on_laps() {
        assert_eq!(Mode::TimeTrial.laps_target(SpeedClass::Venom), Some(3));
        assert_eq!(Mode::SingleRace.laps_target(SpeedClass::Venom), Some(3));
        // Speed Lap's HUD does show a lap count (`7`, live-confirmed on
        // Venom) - it just never turns into an ending. See `laps_target`'s
        // own docs for the pause-menu evidence.
        assert_eq!(Mode::SpeedLap.laps_target(SpeedClass::Venom), None);
        assert_eq!(Mode::Zone.laps_target(SpeedClass::Venom), None);
        // Eliminator has no lap target either, and for a different reason
        // from Speed Lap's or Zone's: it has one, a kill count, that this
        // method does not report. See `Mode::ELIMINATOR_KILL_TARGET_DEFAULT`.
        assert_eq!(Mode::Eliminator.laps_target(SpeedClass::Venom), None);
    }

    /// The census, reproduced as an assertion: 3 Venom, 4 Flash, 4 Rapier, 5
    /// Phantom across all 236 authored `PI_Cell` records. The whole point of
    /// the change that introduced it is that three of these four are *not* 3.
    #[test]
    fn a_single_race_runs_the_campaigns_own_per_class_lap_count() {
        assert_eq!(Mode::SingleRace.laps_target(SpeedClass::Venom), Some(3));
        assert_eq!(Mode::SingleRace.laps_target(SpeedClass::Flash), Some(4));
        assert_eq!(Mode::SingleRace.laps_target(SpeedClass::Rapier), Some(4));
        assert_eq!(Mode::SingleRace.laps_target(SpeedClass::Phantom), Some(5));
    }

    /// The same table, live-confirmed rather than census-only: one Custom
    /// Race Time Trial per rung under PPSSPP read `Lap 1 of 3`/`4`/`4`/`5`,
    /// 2026-09-09, `pulse-psp-usa.chd`, Talon's Junction White.
    #[test]
    fn a_time_trial_runs_the_same_per_class_lap_count() {
        assert_eq!(Mode::TimeTrial.laps_target(SpeedClass::Venom), Some(3));
        assert_eq!(Mode::TimeTrial.laps_target(SpeedClass::Flash), Some(4));
        assert_eq!(Mode::TimeTrial.laps_target(SpeedClass::Rapier), Some(4));
        assert_eq!(Mode::TimeTrial.laps_target(SpeedClass::Phantom), Some(5));
    }

    /// Every rung the enum has must have a row, in both tables, or a class
    /// would index past one. Cheap here, and the alternative is a panic
    /// mid-race.
    #[test]
    fn every_speed_class_has_a_lap_count() {
        for (index, class) in SpeedClass::ALL.into_iter().enumerate() {
            assert_eq!(index, class as usize, "{class} is not at its own index");
            assert_eq!(
                Mode::SingleRace.laps_target(class),
                Some(Mode::SINGLE_RACE_LAPS_BY_CLASS[index]),
            );
            assert_eq!(
                Mode::TimeTrial.laps_target(class),
                Some(Mode::TIME_TRIAL_LAPS_BY_CLASS[index]),
            );
        }
    }

    /// The class parameter is taken and ignored by every mode except the two
    /// that field a real per-class table - see `laps_target`'s own docs for
    /// Speed Lap's `7`, which is real but does not end the race.
    #[test]
    fn only_time_trial_and_single_race_vary_with_the_speed_class() {
        for class in SpeedClass::ALL {
            assert_eq!(Mode::SpeedLap.laps_target(class), None);
            assert_eq!(Mode::Zone.laps_target(class), None);
            assert_eq!(Mode::Eliminator.laps_target(class), None);
        }
    }

    #[test]
    fn only_zone_drives_its_own_throttle() {
        assert!(Mode::Zone.is_auto_throttle());
        assert!(!Mode::TimeTrial.is_auto_throttle());
        assert!(!Mode::SpeedLap.is_auto_throttle());
        assert!(!Mode::SingleRace.is_auto_throttle());
    }

    /// The three single-ship modes are the original's own answer, measured on
    /// the running game; single race and Eliminator are the two that field a
    /// grid.
    #[test]
    fn single_race_and_eliminator_field_a_grid() {
        assert!(Mode::SingleRace.has_opponents());
        assert!(Mode::Eliminator.has_opponents());
        assert!(!Mode::TimeTrial.has_opponents());
        assert!(!Mode::SpeedLap.has_opponents());
        assert!(!Mode::Zone.has_opponents());
    }

    /// The switch the pickup system hangs off. It is the *only* mode-dependent
    /// thing about a weapon pad in the original: a weapons-off race hides the
    /// pads and empties the trigger list rather than ignoring a crossing.
    #[test]
    fn single_race_and_eliminator_are_the_only_modes_that_arm_weapon_pads() {
        assert!(Mode::SingleRace.weapons_enabled());
        assert!(Mode::Eliminator.weapons_enabled());
        for mode in [Mode::TimeTrial, Mode::SpeedLap, Mode::Zone] {
            assert!(
                !mode.weapons_enabled(),
                "{mode:?} should race with weapons off"
            );
        }
    }

    /// The mechanic `MSC_EVENT_ELIM` states outright: Eliminator is the one
    /// mode where a crossed pickup pad cannot be absorbed for health.
    #[test]
    fn only_eliminator_refuses_pickup_absorption() {
        assert!(!Mode::Eliminator.pickups_absorb());
        for mode in Mode::ALL {
            if mode != Mode::Eliminator {
                assert!(mode.pickups_absorb(), "{mode:?} should absorb pickups");
            }
        }
    }
}
