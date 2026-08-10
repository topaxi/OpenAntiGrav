//! The race modes, and what each one does differently.
//!
//! Three single-ship modes. None of them needs opponents, weapons or a grid,
//! which is why they come first: they are the part of the race layer that can be
//! finished rather than stubbed.
//!
//! # Where the rules come from
//!
//! Each rule below is either observed on the running game, read out of the
//! executable, or ours. They are not interchangeable and the doc comments say
//! which is which - see `docs/gameplay/race-modes.md` for the evidence and the
//! confidence score behind every one.

/// One of the three single-ship race modes.
///
/// The variants are ordered as the menu offers them, and
/// [`Mode::TIME_TRIAL_LAPS`] is the only lap count any of them has.
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
}

impl Mode {
    /// Every mode, as a fixed-size array.
    ///
    /// Fixed-size so that adding a fourth mode is a compile error at every
    /// caller that enumerates them, rather than a silently short list. The same
    /// reason `menu::Action::all` and `perf::FrameLimit::OFFERED` are arrays in
    /// the composition root.
    pub const ALL: [Self; 3] = [Self::TimeTrial, Self::SpeedLap, Self::Zone];

    /// Laps in a time trial.
    ///
    /// **Observed, not invented.** The original's own counter reads `Lap 1 of 3`,
    /// and a run that passes the third lap starts a fresh attempt with the best
    /// time cleared - see `docs/reverse-engineering/ppsspp-debugger.md:707-709`.
    /// Note the count is *configuration* in the original rather than a constant:
    /// the race-setup format string carries `laps="%d"`. Three is what a time
    /// trial was seen configured with, not a limit of the format.
    pub const TIME_TRIAL_LAPS: u32 = 3;

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
    #[must_use]
    pub const fn laps_target(self) -> Option<u32> {
        match self {
            Self::TimeTrial => Some(Self::TIME_TRIAL_LAPS),
            Self::SpeedLap | Self::Zone => None,
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

    /// Whether the original races this mode with other craft on the grid.
    ///
    /// **`false` for all three, measured on the running original rather than
    /// assumed from "single-ship" in this module's own name.** Selecting
    /// TIME TRIAL, SPEED LAP or ZONE on the Custom Race screen greys
    /// `AI DIFFICULTY` to `N/A`, the same tell `race-modes.md` already uses for
    /// weapons - see [`Self::weapons_enabled`]. `SINGLE RACE`, `HEAD TO HEAD`,
    /// `TOURNAMENT` and `ELIMINATOR` all leave it selectable; none of those four
    /// is a mode this crate implements yet.
    #[must_use]
    pub const fn has_opponents(self) -> bool {
        false
    }

    /// Whether the original arms `Weapon Pad`s for this mode.
    ///
    /// **`false` for all three, measured on the running original.** Selecting
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
    #[must_use]
    pub const fn weapons_enabled(self) -> bool {
        false
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
        assert_eq!(Mode::from_name("eliminator"), None);
        assert_eq!(Mode::from_name(""), None);
        assert_eq!(Mode::from_name("TIME_TRIAL"), None);
    }

    #[test]
    fn the_default_is_the_time_trial() {
        assert_eq!(Mode::default(), Mode::TimeTrial);
        assert_eq!(Mode::ALL[0], Mode::TimeTrial);
    }

    #[test]
    fn only_the_time_trial_ends_on_laps() {
        assert_eq!(Mode::TimeTrial.laps_target(), Some(3));
        assert_eq!(Mode::SpeedLap.laps_target(), None);
        assert_eq!(Mode::Zone.laps_target(), None);
    }

    #[test]
    fn only_zone_drives_its_own_throttle() {
        assert!(Mode::Zone.is_auto_throttle());
        assert!(!Mode::TimeTrial.is_auto_throttle());
        assert!(!Mode::SpeedLap.is_auto_throttle());
    }

    #[test]
    fn none_of_the_three_single_ship_modes_have_opponents() {
        for mode in Mode::ALL {
            assert!(!mode.has_opponents(), "{mode:?} should have no opponents");
        }
    }

    #[test]
    fn none_of_the_three_single_ship_modes_arm_weapon_pads() {
        for mode in Mode::ALL {
            assert!(
                !mode.weapons_enabled(),
                "{mode:?} should race with weapons off"
            );
        }
    }
}
