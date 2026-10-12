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
    /// Every opponent runs its thrust law, speed floor and lane choice on
    /// every tick, not on its round-robin turn: 2048's `Ai_Construct` stores
    /// `GameMode_IsHdLineage()` at `driver+0x98`, and `AI_UpdateCraft`
    /// (`0x8119dda4`) sets the racer's turn flag whenever that byte is clear,
    /// which it is in every 2048 event (measured on Vita3K, 308 of 308
    /// opponent rows flagged).
    pub every_tick: bool,
    /// The ring's lateral rate follows the event's two AI skills
    /// ([`event_ring_rate`]) instead of Pulse's fixed `0.001`: 2048's
    /// `FUN_8119853c`, in a mode of its own lineage.
    pub event_ring_rate: bool,
    /// The lane choice searches its two directions in the order a per-driver
    /// coin says, and the coin is redrawn now and then: 2048's
    /// `Ai_PlanLane` (`0x8119d53a`), see [`super::lane`].
    pub coin_ordered_search: bool,
    /// A well-scored choice moves the preferred lane to `1 - chosen^2` on
    /// its side: 2048's `Ai_PlanLane` (`0x8119dcc4`), see [`super::lane`].
    pub preferred_follows_choice: bool,
}

impl Laws {
    /// Pulse's driver: every branch as `0x08855904` and its neighbours read.
    pub const PULSE: Self = Self {
        finished_spread: 50.0,
        finished_wander_to_place: false,
        eliminator_drops_when_behind: false,
        round_robin_pre_increment: false,
        halved_lookahead: None,
        every_tick: false,
        event_ring_rate: false,
        coin_ordered_search: false,
        preferred_follows_choice: false,
    };

    /// Whether the lookahead is halved at lap distance `lap_distance`.
    #[must_use]
    pub fn halves_lookahead(&self, lap_distance: f32) -> bool {
        self.halved_lookahead
            .is_some_and(|(lo, hi)| lap_distance > hi || lap_distance < lo)
    }
}

/// Pulse's ring rate, `0.001` lane units a slot per slot (`0x08855758`).
pub const RING_RATE: f32 = 0.001;

/// The ring rate a 2048 event's two AI skills give (`FUN_8119853c`,
/// `0x811985ae..0x811985ee`): `c = clamp(max(pad, avoid), 0, 1)`, then
/// `0.001 + 0.00065 * (1 - (1 - c)^2)`, each product rounded as the NEON
/// `vmls`/`vmla` round it. Both skills read `1.0` live on an event that
/// authors neither (the `GameModeBase` constructor's default), giving
/// `0.00165`.
#[must_use]
pub fn event_ring_rate(pad_tracking: f32, weapon_avoidance: f32) -> f32 {
    let c = if weapon_avoidance > pad_tracking {
        weapon_avoidance
    } else {
        pad_tracking
    };
    let c = if c < 0.0 { 0.0 } else { c.min(1.0) };
    let free = 1.0 - c;
    let curve = 1.0 - free * free;
    RING_RATE + curve * 0.000_65
}

impl Default for Laws {
    fn default() -> Self {
        Self::PULSE
    }
}
