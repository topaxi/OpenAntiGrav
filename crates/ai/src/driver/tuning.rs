//! The constants the whole field of drivers shares.
//!
//! Split out of [`super`] for `scripts/check-file-size.py`'s thousand-line
//! rule; [`Tuning`] is re-exported from there.

/// The controller's constants.
///
/// **These numbers are this project's own.** The disc authors a per-class
/// controller (`Data\XML\AIControlStats.xml`, read in full at
/// `docs/ghidra/functions/psp-pulse-usa/ai-stats.md`) and none of its values
/// appear anywhere in the tree, per
/// [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md)
/// and `docs/formats/handling-stats.md`: a field name describes the format, a
/// tuning table is the content itself.
///
/// The defaults were tuned against this engine's own physics through
/// `tests/closed_loop.rs` until a craft got round without weaving. Sweep
/// evidence is in `docs/gameplay/ai.md`, "Tuning sweep tables".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Lookahead distance at a standstill.
    pub look_min: f32,
    /// Extra lookahead per unit of forward speed, roughly "seconds ahead".
    ///
    /// Longer is calmer and cuts corners; shorter tracks harder and, past a
    /// point, makes the loop ring. **Ours, chosen, not measured**: the disc's
    /// `LookAheadSecs` has undetermined units and no identified consumer
    /// (`docs/ghidra/functions/psp-pulse-usa/ai-stats.md`,
    /// `AiStats_ParseController`).
    ///
    /// Lowered from `0.35`: `13_Track`'s driver carried lateral momentum across
    /// a wide S-bend, and pure pursuit's `curvature = 2 * offset / distance^2`
    /// answers a given error more gently as the lookahead grows. `rate_gain`
    /// was ruled out (saturated at `10.0`, field floors fail).
    ///
    /// `0.30` stays shipped although the 2026-09-07 re-sweep removed its field
    /// justification and `0.22` scores better solo: nothing replaces the guard
    /// against a value that wrecks a field. Sweep tables, the 16-seed
    /// re-measurement and `07_Track`'s wall-grind note:
    /// `docs/gameplay/ai.md`, "Tuning sweep tables".
    pub look_speed: f32,
    /// Lookahead ceiling. Past this a craft stops seeing the corner it is in.
    pub look_max: f32,
    /// Gain from turn-rate error onto the steering input.
    pub rate_gain: f32,
    /// Ceiling on the turn rate the geometry may ask for, in radians per second.
    ///
    /// Without it a craft thrown far off its line asks for a rate no hull can
    /// produce and holds full lock through the recovery, which is its own kind
    /// of weave. Swept on `16_Track`: 1.2 bound a real corner (Talon's
    /// Junction), 1.8 is where it stops binding.
    ///
    /// **This is the permission, not the corner limit.** The kinematic limit in
    /// [`pace::corner_target`] reads
    /// [`pace::hull_yaw_ceiling`](super::pace::hull_yaw_ceiling) (1.204 to
    /// 1.667 rad/s per team, [`oag_physics::forces::YAW_INVERSE_INERTIA`]) and
    /// takes the smaller of the two. It keeps [`crate::Difficulty::tune`]'s
    /// ladder meaningful. Table and board: `docs/gameplay/ai.md`, "Tuning
    /// sweep tables" and "The clean-Ace board".
    pub max_turn_rate: f32,
    /// The lateral acceleration a craft is assumed to hold through a corner.
    ///
    /// The speed target on curvature `k` is `sqrt(lateral_accel / k)`. Raising
    /// it commits harder and, past what the hull holds, into the wall.
    ///
    /// Measured against the hull: `55.0` left opponents off the throttle 45 per
    /// cent of a race. 260 is the knee of the 2026-08-12 sweep (`sweep_grip` in
    /// `race_ground_truth.rs`, `#[ignore]`d, gated on `OAG_SWEEP`); 340 buys
    /// 0.1 s and quadruples recoveries. Tables: `docs/gameplay/ai.md`, "Tuning
    /// sweep tables".
    pub lateral_accel: f32,
    /// How far ahead the speed target looks for the sharpest bend, as a multiple
    /// of the lookahead. A corner has to be seen before it is entered.
    pub brake_lookahead: f32,
    /// Ceiling on the chord the curvature estimator measures over, in units, or
    /// `None` for no ceiling.
    ///
    /// Callers pass half their lookahead as the span, so without a ceiling the
    /// estimator's resolution is tied to speed and a short corner reads
    /// 2.5-3x too gentle (`07_Track`'s tightest arc). Eleven was swept on a
    /// solo and a field board; ten passes solo and fails two field tests.
    /// `None` is kept so the uncapped row stays re-runnable.
    ///
    /// The sweep predates `pace::corner_target`'s hull ceiling (2026-09-12), so
    /// the choice wants re-establishing; both field ground-truth tests are
    /// green at eleven. Tables and criterion: `docs/gameplay/ai.md`, "Tuning
    /// sweep tables".
    pub curvature_span: Option<f32>,
    /// The chord each curvature reading averages over, where it should differ
    /// from [`Self::curvature_span`] (otherwise both the chord and the walk
    /// step). `None` keeps them equal, the shipped behaviour.
    ///
    /// Measured 2026-09-12 and no help: every shortening, coupled or decoupled,
    /// scores below `11`, so a short chord's cost is resolution, not sampling
    /// density. What is left is a different estimator. See
    /// `Line::max_curvature_stepped` and the table in `docs/gameplay/ai.md`,
    /// "Tuning sweep tables".
    pub curvature_chord: Option<f32>,
    /// Fraction over target at which the airbrakes come on, rather than merely
    /// lifting off.
    pub brake_margin: f32,
    /// How much of the corridor either side of the line a driver may spend on
    /// [`Personality`]'s bias and drift, as a fraction of the room on that side.
    ///
    /// **Well under one on purpose.** The corridor edge clamp only knows the
    /// point being aimed at, not where the craft ends up, so it is a backstop
    /// and this decides how wide the field runs.
    pub corridor_use: f32,
    /// The lowest both-sides airbrake command that counts as braking.
    ///
    /// **`oag_physics::controls::update` gates the brake on both inputs being
    /// strictly positive and never reads their level**, so an epsilon command
    /// buys the whole deceleration at almost no grip cost (`max(L, R)` is what
    /// the grip coefficient reads). Faithful for digital buttons, degenerate
    /// for the analog axis `oag_gameplay`'s input snapshot admits; this floor
    /// keeps the driver out of it.
    pub brake_floor: f32,
    /// Extra both-sides brake per unit of overspeed, as a fraction of target.
    ///
    /// Buys no extra deceleration (see [`Tuning::brake_floor`]); it spends grip,
    /// so it is how fast a driver gives up cornering to shed speed.
    pub brake_gain: f32,
    /// Turn-rate error, in radians per second, below which no differential
    /// airbrake is applied.
    ///
    /// Keeps the differential out of the small-signal regime, so the loop
    /// linearised about the line is the one `tests/closed_loop.rs` measured.
    /// Lowered `0.15` to `0.05` on 2026-09-11 with [`Tuning::trail_gain`] and
    /// [`Tuning::trail_saturation`]: at the old value almost no
    /// `rate_error - trail_deadband` was left once the gate opened, and the
    /// differential peaked at 7.67 of the `0..=100` range. See `docs/gameplay/ai.md`,
    /// "Tuning sweep tables".
    pub trail_deadband: f32,
    /// Differential airbrake per radian per second of turn-rate error past
    /// [`Tuning::trail_deadband`]. Raised `1.2` to `3.0` on 2026-09-11 with it.
    pub trail_gain: f32,
    /// Ceiling on the differential.
    pub trail_max: f32,
    /// How saturated the steering command must be, as a fraction of full lock,
    /// before the differential engages.
    ///
    /// **The gate that makes the differential safe.** Below it the steering loop
    /// still has authority, and a parallel path is the oscillation this crate
    /// was rewritten to remove. Above it the loop has run out of lock.
    ///
    /// Lowered `0.85` to `0.7` on 2026-09-11: `0.85` left 19 engaged ticks on
    /// Talon's Junction's U-turn, `0.6` costs the field test's energy floor
    /// (3 craft depleted), `0.7` doubles engagement with the margin intact
    /// (`opponent_weapons_ground_truth::a_field_racing_with_real_pads_does_not_mine_itself_to_death`).
    /// Table: `docs/gameplay/ai.md`, "Tuning sweep tables".
    pub trail_saturation: f32,
    /// Curvature below which [`pace::trail`] treats the line as straight,
    /// whatever [`pace::corner_target`] says about it.
    ///
    /// **Fitted, not derived**, to a two-lap Talon's Junction capture
    /// (`docs/gameplay/ai.md`, "Airbrakes, and what a differential one actually
    /// does"): straights read 0.0003-0.0005 (chord noise), saturated bends
    /// 0.0056 and up; `0.001` sits between. No disc track's `target` is ever
    /// literally infinite, so `!target.is_finite()` could not do this.
    ///
    /// [`pace::trail`]: super::pace::trail
    /// [`pace::corner_target`]: super::pace::corner_target
    pub trail_curvature_floor: f32,
    /// Fraction of [`Driver::peak_curvature`](crate::Driver::peak_curvature)
    /// curvature may fall to before [`pace::trail`] reads it as corner exit.
    ///
    /// **Fitted against the same capture**: past Talon's Junction's tightest
    /// apex curvature ran 0.0274 down to 0.0117 over 63 ticks, against ~5e-5
    /// tick-to-tick mid-corner noise. `0.7` sits inside the decline and outside
    /// the noise.
    ///
    /// [`pace::trail`]: super::pace::trail
    pub trail_exit_decay: f32,
    /// Per-tick leak on [`Driver::peak_curvature`](crate::Driver::peak_curvature),
    /// the high-water mark [`trail_exit_decay`](Self::trail_exit_decay) is read
    /// against.
    ///
    /// `1.0` latches the mark for the whole race. That was a bug:
    /// `pace::track_peak_curvature`'s reset never fires on this disc's geometry
    /// (`07_Track` bottoms at `0.00127` against a floor of `0.00100`), so the
    /// exit gate rejected 74.8 % of ticks. A leak fixes the mechanism but does
    /// not pay: `sweep_trail_peak_decay` shows every value below `1.0` costing
    /// contact ticks and end shield for 0.16 s of lap, because the differential
    /// is spent reactively without `pace::corner_target` banking the speed it
    /// permits.
    ///
    /// **So `1.0` is not an endorsement of the latch**; it changes no behaviour
    /// until the differential becomes a plan. Table: `docs/gameplay/ai.md`,
    /// "Tuning sweep tables"; see also `pace::track_peak_curvature`.
    pub trail_peak_decay: f32,
    /// How often a driver misses a braking point, per tick, while it is at one.
    ///
    /// **Zero for a driver that never errs**, the hardest difficulty. Scaled by
    /// [`Difficulty::mistakes`] so the driver never learns difficulties exist:
    /// a level transforms the tunables, the controller does not branch on it.
    ///
    /// [`Difficulty::mistakes`]: crate::Difficulty::mistakes
    pub mistake_rate: f32,
    /// How long a driver takes to notice a craft arriving beside, in front of
    /// or behind it, in ticks.
    ///
    /// **Zero for a driver that reacts on the frame**, the hardest difficulty;
    /// see [`Difficulty::reaction_ticks`] and [`super::reflex`]. An integer
    /// because it counts ticks and rides in the world snapshot.
    ///
    /// [`Difficulty::reaction_ticks`]: crate::Difficulty::reaction_ticks
    pub reaction_ticks: u16,
    /// The share of the speed plan's verified pace a driver of this level holds
    /// anywhere on the lap. One at the top two levels; see
    /// [`Difficulty::pace_share`].
    ///
    /// [`Difficulty::pace_share`]: crate::Difficulty::pace_share
    pub pace_share: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            look_min: 20.0,
            look_speed: 0.30,
            look_max: 90.0,
            rate_gain: 5.0,
            max_turn_rate: 1.8,
            lateral_accel: 260.0,
            brake_lookahead: 2.5,
            curvature_span: Some(11.0),
            curvature_chord: None,
            brake_margin: 0.05,
            corridor_use: 0.6,
            brake_floor: 0.35,
            brake_gain: 2.0,
            trail_deadband: 0.05,
            trail_gain: 3.0,
            trail_max: 0.6,
            trail_saturation: 0.7,
            trail_curvature_floor: 0.001,
            trail_exit_decay: 0.7,
            trail_peak_decay: 1.0,
            // Zero: the competent driver. The rate arrives by `Difficulty::tune`;
            // a default that errs would make every caller that chose no
            // difficulty (closed-loop harness, replay) non-deterministic.
            mistake_rate: 0.0,
            // Zero for the same reason, and load-bearing: a nonzero default
            // changes every closed-loop assertion. See `super::reflex`.
            reaction_ticks: 0,
            pace_share: 1.0,
        }
    }
}
