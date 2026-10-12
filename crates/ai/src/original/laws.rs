//! Where one title's original driver differs from Pulse's, as data.
//!
//! Wipeout HD's opponent is Pulse's driver function for function
//! (`docs/ghidra/functions/ps3-hdfury-eu/ai-controls.md`); what differs is a
//! handful of gated branches, each read at instruction level on HD's
//! executable and carried here as a value, so no code asks which title it is
//! (ADR-0058). [`Laws::PULSE`] is Pulse's own behaviour, the values
//! `docs/ghidra/functions/psp-pulse-usa/ai-controls.md` records.

/// One race's variant of the original driver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Laws {
    /// The formation spacing a finished racer's thrust law steers by
    /// (`AI_ComputeOpponentThrust`'s `racer+0x82` branch): Pulse `50`, HD `75`.
    pub finished_spread: f32,
    /// A finished racer's wander sets its slot, start and end to
    /// `8 - place` and returns (HD's `AI_UpdateSpreadWander`, `0x000fdd40`);
    /// Pulse's wander has no finished branch.
    pub finished_wander_to_place: bool,
    /// In an Eliminator race the `WhenBehind` term is left out
    /// (HD `0x000fe384`, modes `8` and `0x14`); Pulse subtracts it in every mode.
    pub eliminator_drops_when_behind: bool,
    /// The round robin flags its next racer before the first tick rather than
    /// after (HD's `AIManager_Update` increments, then flags): the first lane
    /// tick goes to racer `1` instead of `0`. Same period, one tick of phase.
    pub round_robin_pre_increment: bool,
    /// This race's halved-lookahead window, or `None`: outside the lap
    /// distance `[lo, hi]` the steering's lookahead time is halved (HD's
    /// `driver+0x8b4` gate, `0x001030b4`, read by `AI_ComputeControls` at
    /// `0x00100930`). Resolved per race from the circuit and the class.
    pub halved_lookahead: Option<(f32, f32)>,
}

impl Laws {
    /// Pulse's driver: every branch as `0x08855904` and its neighbours read.
    pub const PULSE: Self = Self {
        finished_spread: 50.0,
        finished_wander_to_place: false,
        eliminator_drops_when_behind: false,
        round_robin_pre_increment: false,
        halved_lookahead: None,
    };

    /// Whether the lookahead is halved at lap distance `lap_distance`.
    #[must_use]
    pub fn halves_lookahead(&self, lap_distance: f32) -> bool {
        self.halved_lookahead
            .is_some_and(|(lo, hi)| lap_distance > hi || lap_distance < lo)
    }
}

impl Default for Laws {
    fn default() -> Self {
        Self::PULSE
    }
}
