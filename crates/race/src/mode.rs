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
}

impl Mode {
    /// Every mode, as a fixed-size array.
    ///
    /// Fixed-size so that adding a mode is a compile error at every caller that
    /// enumerates them, rather than a silently short list. The same reason
    /// `menu::Action::all` and `perf::FrameLimit::OFFERED` are arrays in the
    /// composition root.
    pub const ALL: [Self; 5] = [
        Self::TimeTrial,
        Self::SpeedLap,
        Self::Zone,
        Self::SingleRace,
        Self::Eliminator,
    ];

    /// Laps in a time trial.
    ///
    /// **Observed, not invented.** The original's own counter reads `Lap 1 of 3`,
    /// and a run that passes the third lap starts a fresh attempt with the best
    /// time cleared - see `docs/reverse-engineering/ppsspp-debugger.md:707-709`.
    /// Note the count is *configuration* in the original rather than a constant:
    /// the race-setup format string carries `laps="%d"`. Three is what a time
    /// trial was seen configured with, not a limit of the format.
    pub const TIME_TRIAL_LAPS: u32 = 3;

    /// Laps in a single race.
    ///
    /// **Ours, and the weakest number in this module.** The setup format's
    /// `laps="%d"` says the original configures this per event rather than
    /// fixing it, and no single race has been watched long enough to read what
    /// a *Custom Race* is configured with. Three is [`Self::TIME_TRIAL_LAPS`]
    /// carried over because a race has to end somewhere; it is not a
    /// measurement, and one screenshot of the original's own lap counter on a
    /// SINGLE RACE would replace it.
    pub const SINGLE_RACE_LAPS: u32 = 3;

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
        }
    }

    /// The mode a token names, or `None` if nothing does.
    ///
    /// `None` rather than a default, so a settings file carrying a mode this
    /// build does not have is visible to the caller instead of silently becoming
    /// a time trial.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }

    /// Laps the mode finishes after, or `None` when it never ends on its own.
    ///
    /// **`None` for Eliminator, measured rather than assumed from "no lap
    /// target here either".** `MSC_EVENT_ELIM` says outright: *"The lap count
    /// is not fixed, and the race will end when the kill count is reached"* -
    /// a kill count ends it instead, and that ending is not this method's to
    /// report; see [`crate::RaceState::eliminator_finished`].
    #[must_use]
    pub const fn laps_target(self) -> Option<u32> {
        match self {
            Self::TimeTrial => Some(Self::TIME_TRIAL_LAPS),
            Self::SingleRace => Some(Self::SINGLE_RACE_LAPS),
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
    #[must_use]
    pub const fn has_opponents(self) -> bool {
        matches!(self, Self::SingleRace | Self::Eliminator)
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
    #[must_use]
    pub const fn weapons_enabled(self) -> bool {
        matches!(self, Self::SingleRace | Self::Eliminator)
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
    /// [`ShipState::shield`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/physics/src/lib.rs
    #[must_use]
    pub const fn pickups_absorb(self) -> bool {
        !matches!(self, Self::Eliminator)
    }
}

#[cfg(test)]
mod tests {
    use super::Mode;

    #[test]
    fn every_mode_round_trips_through_its_token() {
        for mode in Mode::ALL {
            assert_eq!(Mode::from_name(mode.name()), Some(mode));
        }
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
        assert_eq!(Mode::TimeTrial.laps_target(), Some(3));
        assert_eq!(Mode::SingleRace.laps_target(), Some(3));
        assert_eq!(Mode::SpeedLap.laps_target(), None);
        assert_eq!(Mode::Zone.laps_target(), None);
        // Eliminator has no lap target either, and for a different reason
        // from Speed Lap's or Zone's: it has one, a kill count, that this
        // method does not report. See `Mode::ELIMINATOR_KILL_TARGET_DEFAULT`.
        assert_eq!(Mode::Eliminator.laps_target(), None);
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
