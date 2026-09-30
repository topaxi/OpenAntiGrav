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
    /// A two-craft race: the player against exactly one AI opponent.
    ///
    /// **Field size is measured, not designed.** Every one of the 23
    /// authored `Head2Head` campaign cells carries `AICount="1"`, and the
    /// per-mode grid-size table read off `FUN_0880502c` gives mode `9`
    /// (`Head2Head`) a field of `2` against the eight-craft default every
    /// other opponent-fielding mode uses - see
    /// `docs/ghidra/functions/psp-pulse-usa/head2head.md`. [`Self::opponent_count`]
    /// is `1` for exactly this reason, not [`Self::SingleRace`]'s `7`.
    ///
    /// **Weapons are off and locked, measured two ways.** `shield.md`'s
    /// `g_weapons_enabled` switch puts mode `9` in both its default-off set
    /// and its no-override set, and all 23 authored cells carry
    /// `Weapons="off"` with no exception - so [`Self::weapons_enabled`] is
    /// `false` here, unlike [`Self::SingleRace`]/[`Self::Tournament`].
    ///
    /// **Laps follow the same per-class census as [`Self::SingleRace`]** -
    /// every authored cell's own `laps` attribute is `4` (Flash, Rapier) or
    /// `5` (Phantom); no Venom-class cell exists, so [`Self::laps_target`]
    /// falls back to [`Self::SINGLE_RACE_LAPS_BY_CLASS`]'s `3` for that rung
    /// on the same "chosen, not measured" footing every other unauthored
    /// combination in this crate already carries.
    ///
    /// **The medal law is win-or-nothing, flat across all 23 cells**: gold
    /// target `1`, silver and bronze both `0` - `oag_tables::race_campaign
    /// ::Cell::evaluate_medal` already handles this correctly, since a
    /// finishing position is always `>= 1` and can never satisfy a `0`
    /// target by accident.
    ///
    /// **Which team the opponent flies is not measured**, and deliberately
    /// not chased further - it is the same open question
    /// `crate::livery::teams_for_slots` already carries for
    /// [`Self::SingleRace`]'s own seven opponents (`docs/ghidra/functions
    /// /psp-pulse-usa/grid.md`'s "which team flies which slot"). This build
    /// answers it the same way: `teams_for_slots`'s own cyclic assignment,
    /// labelled chosen rather than measured at its own call site.
    ///
    /// **Deliberately absent from [`Self::ALL`]**, like [`Self::Tournament`],
    /// reachable only through a campaign cell
    /// (`oag_game::campaign::race_mode_for_cell`). Unlike Tournament this is
    /// not a leg-list problem; it is kept out to keep this mode's field-size
    /// axis off the RACE page's one-track launch, which has no way to ask
    /// for "one opponent" today.
    Head2Head,
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

    /// Kills that end an Eliminator event when no campaign cell supplies one:
    /// `5`, **the first entry of the race box's `KILLS` list**.
    ///
    /// **Measured 2026-09-30, confidence 90**: `RaceBox_ApplySetupGlobals`
    /// (`0x088e5ff4`) `atoi`s the selected entry of the `Eliminations` list
    /// (`5`, `10`, `15`, `20`, `25`) into `DAT_08b30fb0`, the figure
    /// `Eliminator_UpdateKillTarget` ends the race on when there is no cell,
    /// and a fresh profile's Racebox shows `KILLS 5` with the row untouched. A
    /// Racebox Eliminator on the original ended at exactly five kills. See
    /// `docs/ghidra/functions/psp-pulse-usa/eliminator-kill-target.md`. It was
    /// `10`, read off `FEData.wad`'s `<Targets Elimination="10">`, which is
    /// what the list's own `Default="10"` attribute also says and which the
    /// running screen does not honour.
    ///
    /// **It is the list's first entry, not a fixed rule**: this build has no
    /// `KILLS` row on its race page yet, so the other four (`10` to `25`) are
    /// unreachable from a Custom Race, which is what retires this constant.
    /// A campaign cell's own gold figure (`10`, `7` or `5`) still wins, wired
    /// in `Session::launch_campaign_cell`.
    ///
    /// **Whose kill count ends the race is measured, and it is not only the
    /// player's.** `Eliminator_UpdateKillTarget` reads `entity+0x8d8` on *any*
    /// craft - the race ends the moment one ship's own tally reaches the
    /// target, whichever ship that is. [`RaceState::eliminator_finished`]
    /// takes the target and the highest count across the whole field for
    /// exactly this reason, rather than the player's own alone.
    pub const ELIMINATOR_KILL_TARGET_DEFAULT: u32 = 5;

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
            Self::Head2Head => "head_to_head",
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
            // every mode's cells, `Tournament`'s 27 and `Head2Head`'s 23
            // included - no Venom-class `Head2Head` cell is authored, so
            // that rung falls back to the same table's `3` on this crate's
            // usual "chosen, not measured" footing for an unauthored
            // combination.
            Self::SingleRace | Self::Tournament | Self::Head2Head => {
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
            Self::Head2Head => "MSC_EVENT_HTH",
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
            Self::Head2Head => "HEAD TO HEAD",
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
    ///
    /// **`true` for Head2Head too, with a field of one rather than seven** -
    /// see [`Self::opponent_count`], which this delegates to.
    #[must_use]
    pub const fn has_opponents(self) -> bool {
        self.opponent_count() > 0
    }

    /// How many AI opponents this mode fields, `0` for every single-ship
    /// mode.
    ///
    /// **Not the same axis as [`Self::has_opponents`] used to be** - every
    /// mode that fields opponents at all fielded a full eight-craft grid
    /// until [`Self::Head2Head`] existed. `AICount="1"` on all 23 authored
    /// `Head2Head` cells, and mode `9`'s own row in the grid-size table read
    /// off `FUN_0880502c`, agree: a field of the player plus exactly one AI
    /// opponent, not seven. See
    /// `docs/ghidra/functions/psp-pulse-usa/head2head.md`.
    #[must_use]
    pub const fn opponent_count(self) -> u8 {
        match self {
            Self::SingleRace | Self::Eliminator | Self::Tournament => 7,
            Self::Head2Head => 1,
            Self::TimeTrial | Self::SpeedLap | Self::Zone => 0,
        }
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
    /// "optional". **This is the mode's own answer; the RACE page's `WEAPONS` row
    /// overrides it for a single race only** (`race::Options::weapons_override`),
    /// so a race asks that, not this, whether weapons are on.
    ///
    /// **`true` for Eliminator too** - it is the mode `MSC_EVENT_ELIM` calls
    /// *"a weapons-heavy environment"*, and `Race_ReadSetupOptions`' own
    /// weapons-off set (modes `5`/`10`, time trial and speed lap) does not
    /// include it.
    ///
    /// **`true` for Tournament, mirroring [`Self::SingleRace`]** - see
    /// [`Self::Tournament`]'s own doc.
    ///
    /// **`false` for Head2Head, and it is locked off the same way the three
    /// single-ship modes are - not merely defaulted off.** `shield.md`'s own
    /// `g_weapons_enabled` switch (`FUN_08896b84`) puts mode `9` in *both*
    /// the default-off set (`5`/`9`/`10`/`15`/`17`) and the no-override set
    /// (`5`/`8`/`9`/`10`/`15`/`17`), and all 23 authored cells carry
    /// `Weapons="off"` with no exception - checked live on the Custom Race
    /// screen too, 2026-08-10: "Head to Head" shows `WEAPONS: Off`, greyed,
    /// unlike Tournament's editable row. See
    /// `docs/ghidra/functions/psp-pulse-usa/head2head.md`.
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
mod tests;
