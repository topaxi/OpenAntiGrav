//! The constants the whole field of drivers shares.
//!
//! Its own file rather than another block in [`super`], which is over
//! `scripts/check-file-size.py`'s thousand-line rule; a move only, and
//! [`Tuning`] is re-exported from there so every path that named it still does.

/// The controller's constants.
///
/// **These numbers are this project's own.** Both games author a per-class
/// controller on the disc - `Data\XML\AIControlStats.xml`, five attributes, read
/// in full at `docs/ghidra/functions/psp-pulse-usa/ai-stats.md` - and none of
/// its values appear here or anywhere else in the tree, per
/// [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md) and
/// the rule `docs/formats/handling-stats.md` states: a field name describes the
/// format, a tuning table is the content itself. The shipped values are read off
/// the player's own disc if anything ever wants them.
///
/// The defaults below were tuned against this engine's own physics, through
/// `tests/closed_loop.rs`, until a craft got round without weaving.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Lookahead distance at a standstill.
    pub look_min: f32,
    /// Extra lookahead per unit of forward speed.
    ///
    /// Roughly "how many seconds ahead the craft looks". Longer is calmer and
    /// cuts corners; shorter tracks the line harder and, past a point, is what
    /// makes the loop ring.
    pub look_speed: f32,
    /// Lookahead ceiling. Past this a craft stops seeing the corner it is in.
    pub look_max: f32,
    /// Gain from turn-rate error onto the steering input.
    pub rate_gain: f32,
    /// Ceiling on the turn rate the geometry may ask for, in radians per second.
    ///
    /// A craft thrown far off its line computes an enormous required curvature;
    /// without this it asks for a rate no hull can produce and holds full lock
    /// all the way through the recovery, which is its own kind of weave.
    ///
    /// **It was `1.2`, and that turned out to be the binding constraint on a
    /// real corner** - reported from play as the field making a hard turn on
    /// Talon's Junction badly. What the trace showed was *not* an overspeed:
    /// through the whole corner the craft sat below its own speed target and
    /// never touched the airbrakes, while drifting from 18 units inside the
    /// line to 30 outside it. Thirty units off with a 57-unit lookahead is a
    /// pure-pursuit request of about 2 rad/s, and this clamped it to 1.2 while
    /// the craft was already achieving 1.08 - so the controller was asking for
    /// everything it was allowed and was still not permitted to turn hard
    /// enough to get back.
    ///
    /// Swept on `16_Track`, a minute a run, seven craft:
    ///
    /// | value | worst excursion | mean off line | mean speed |
    /// | --- | --- | --- | --- |
    /// | 1.2 | 36 | 7.5 | 114 |
    /// | 1.6 | 27 | 6.0 | 122 |
    /// | **1.8** | **24** | **6.0** | **122** |
    /// | 2.2 | 29 | 6.1 | 122 |
    ///
    /// It flattens either side of 1.8 rather than continuing to improve, which
    /// is the shape of a constraint that has stopped binding: past it the limit
    /// is the hull, not the permission.
    pub max_turn_rate: f32,
    /// The lateral acceleration a craft is assumed to hold through a corner.
    ///
    /// Sets the speed target: on a corner of curvature `k` the target is
    /// `sqrt(lateral_accel / k)`, the standard cornering limit. Raising it makes
    /// a driver commit harder and, past what the hull can hold, into the wall.
    ///
    /// **Measured against the hull rather than guessed at**, and it was
    /// guessed at once: this was `55.0` until 2026-08-11, which was reported
    /// from play as "my craft is faster than the AI, first place within a few
    /// seconds". It was. At 55 an opponent spent **45 per cent of a real race
    /// off the throttle entirely**, braking for corners it could hold flat.
    ///
    /// Swept on `16_Track`, a minute a run, seven craft:
    ///
    /// | value | mean speed | off throttle | furthest off the line |
    /// | --- | --- | --- | --- |
    /// | 55 | 90 | 45% | 28 |
    /// | 130 | 113 | 16% | 32 |
    /// | **180** | **116** | **8%** | 36 |
    /// | 220 | 99 | 4% | 46 |
    ///
    /// **The curve turns over**, which is what made 180 a measurement and not
    /// a preference: past it a craft was not cornering faster, it was sliding
    /// wide, and the mean speed and the lap count both fell while the distance
    /// off the line climbed. Nothing wrecked at any of these.
    ///
    /// # Re-swept at 260 on 2026-08-12, and the old sweep was measuring a bug
    ///
    /// That sweep ran on one circuit and, worse, on a build where craft fell
    /// through the track: the twelve-circuit benchmark managed two clean laps
    /// at the time and the respawn count did not respond to grip at all, which
    /// is the signature of a number that is not answering the question asked of
    /// it. With the hover fixed - see `oag_physics::hover::sweep` and
    /// `FAST_PROBE_SPEED` - all twelve circuits lap cleanly and the sweep
    /// finally measures driving. A lone Ace, every circuit, five minutes each:
    ///
    /// | value | clean laps | recoveries | mean clean lap |
    /// | --- | --- | --- | --- |
    /// | 120 | 12 | 1 | 43.5s |
    /// | 180 | 12 | 2 | 40.3s |
    /// | **260** | **12** | **2** | **39.2s** |
    /// | 340 | 12 | 8 | 39.1s |
    /// | 440 | 12 | 5 | 39.2s |
    /// | 560 | 11 | 6 | 38.1s |
    /// | 700 | 11 | 9 | 38.0s |
    ///
    /// **260 is the knee.** 340 buys a tenth of a second and quadruples the
    /// recoveries; 440 is slower than 340 despite believing in more grip; past
    /// 560 a circuit stops managing a clean lap at all. The lap time keeps
    /// drifting down after that only because a craft that is recovered mid-lap
    /// does not count that lap, so the survivors are a flattering sample - which
    /// is exactly why the recovery column is next to it.
    ///
    /// `sweep_grip` in `race_ground_truth.rs` is the harness; it is
    /// `#[ignore]`d and gated on `OAG_SWEEP`.
    pub lateral_accel: f32,
    /// How far ahead the speed target looks for the sharpest bend, as a multiple
    /// of the lookahead. A corner has to be seen before it is entered.
    pub brake_lookahead: f32,
    /// Ceiling on the chord the curvature estimator measures over, in units, or
    /// `None` for no ceiling at all.
    ///
    /// **This is the estimator's resolution, and without a ceiling it is tied
    /// to speed.** Every caller of `Line::max_curvature` passes half its own
    /// lookahead as the span, so at 80 units/s `Line::curvature`'s chord triple
    /// covers ~72 units of track. A corner shorter than that is averaged with
    /// the straights either side of it and comes back smaller than it is:
    /// `07_Track`'s tightest arc is ~50 units long and reads 0.0155-0.0186
    /// where the local value is 0.047, understating it 2.5-3x and putting its
    /// own peak thirty samples early. A driver cannot slow for a corner it
    /// cannot see.
    ///
    /// # Eleven, measured - and ten was measured and rejected
    ///
    /// `sweep_curvature_span` in `crates/game/tests/ai_span_sweep.rs` is the
    /// harness, and it reports **two boards** because one of them is not
    /// enough:
    ///
    /// - **Solo**: a lone Ace over the twelve forward circuits, 18,000 ticks
    ///   each. Shield retained rather than lap time, because the failure this
    ///   knob reaches is a craft that laps *cleanly* while grinding down a
    ///   wall - `07_Track` banks 49.8s while shedding 34-35 shield doing it,
    ///   and every clean/round/respawn column reads that as a pass.
    /// - **Field**: the two committed field ground-truth fixtures exactly as
    ///   they set themselves up, reporting the quantities they assert on -
    ///   `untimed` (opponents past lap 2 with no lap time, which must be zero)
    ///   and opponent shield as a fraction of the pool after a minute, whose
    ///   floors are 0.70 on the mean and 0.45 on the **worst**.
    ///
    /// | span | solo total | resp | clean | mean lap | 07 laps | untimed | field mean | field worst |
    /// | --- | --- | --- | --- | --- | --- | --- | --- | --- |
    /// | 4 | 689.31 | 3 | 11 | 43.6s | 4 | - | - | - |
    /// | 5 | 738.77 | 1 | 12 | 43.3s | 4 | 0 | 0.90 | 0.63 |
    /// | 6 | 726.01 | 1 | 12 | 42.8s | 4 | 0 | 0.78 | **0.29** |
    /// | 7 | 715.32 | 1 | 12 | 42.7s | 4 | 0 | 0.78 | **0.25** |
    /// | 8 | 709.29 | 7 | 11 | 43.5s | 4 | **1** | 0.91 | 0.78 |
    /// | 9 | 709.51 | 1 | 12 | 42.7s | 4 | 0 | 0.83 | **0.35** |
    /// | 10 | 725.91 | 1 | 12 | 42.8s | 4 | **1** | 0.84 | **0.41** |
    /// | **11** | **705.68** | **1** | **12** | **42.7s** | **4** | **0** | **0.88** | **0.64** |
    /// | 12 | 704.18 | 1 | 12 | 42.6s | 4 | 0 | 0.90 | 0.63 |
    /// | 13 | 675.59 | 1 | 12 | 42.5s | 4 | **1** | 0.91 | 0.81 |
    /// | 14 | 688.94 | 1 | 12 | 42.4s | 4 | 0 | 0.83 | 0.45 |
    /// | 15 | 683.72 | 1 | 12 | 42.4s | 3 | 0 | 0.87 | 0.69 |
    /// | 16 | 669.70 | 1 | 12 | 42.2s | 3 | 0 | 0.84 | 0.57 |
    /// | 18 | 649.39 | 1 | 12 | 42.2s | 3 | 0 | 0.93 | 0.78 |
    /// | 20 | 660.42 | 1 | 12 | 42.2s | 3 | 0 | 0.88 | 0.77 |
    /// | 25 | 620.39 | 1 | 12 | 42.0s | 3 | 0 | 0.86 | 0.76 |
    /// | 32 | 620.28 | 1 | 12 | 42.2s | 3 | - | - | - |
    /// | none | 613.52 | 1 | 12 | 42.2s | 3 | 0 | 0.83 | 0.50 |
    ///
    /// **Ten was chosen first, on the solo board alone, and it turns two green
    /// tests red.** That is the most useful row on this table: a span can be
    /// clean on all twelve solo circuits and still wedge a craft that is being
    /// shoved by seven others, and `just` does not run the disc-backed suite,
    /// so nothing in the ordinary gate catches it. The field columns exist
    /// because of that.
    ///
    /// The criterion the value was picked with, in order:
    ///
    /// 1. **Hard gates.** All twelve circuits keep a clean lap; solo respawns
    ///    do not exceed the baseline's 1; `13_Track`'s end shield stays above
    ///    its 2.90 baseline; and both field tests stay green. That admits
    ///    `none`, 5, 11, 12, 15, 16, 18, 20 and 25.
    /// 2. **A band from the line's own geometry, not from the totals.** Below
    ///    ~7 all three of `Line::curvature`'s chords can fall inside one path
    ///    segment - every circuit carries two seams shaped `0.30, 0.11, ~7.0`
    ///    units, and consecutive samples are 0.89-1.77 apart, so a 4-5 unit
    ///    chord spans 3-5 samples and reads sample spacing as curvature. Above
    ///    ~15 the triple covers `3 * span` and can no longer resolve `07`'s
    ///    ~50-unit arc, which is exactly where the sweep puts `07` back to
    ///    dying on lap 3. That leaves 11 and 12.
    /// 3. **Maximise the solo total**, which picks 11 over 12 by 1.5 shield on
    ///    a board of twelve - which is to say they are the same number.
    ///
    /// **Read step 3 as a tie-break and nothing more.** 11 and 12 pass the
    /// field gate because a craft happened not to wedge, and 10 and 13 fail it
    /// for the same reason in reverse; nothing about the estimator explains
    /// that ordering. The field worst does cluster - `{6,7}` at 0.25-0.29,
    /// `{9,10}` at 0.35-0.41, `{5,11,12}` at 0.63-0.64 - so adjacent spans
    /// agree and it is one craft switching between basins rather than a coin
    /// flip, but the basin a span lands in is not a property of the span. Both
    /// failure modes - a craft wedging on a wall, and a craft ground down in
    /// traffic - are open items with their own measurements pending, and this
    /// number is worth re-sweeping once they land.
    ///
    /// `None` is kept rather than folded away so the uncapped row of that table
    /// stays re-runnable; it is the harness's own check on itself.
    pub curvature_span: Option<f32>,
    /// Fraction over target at which the airbrakes come on, rather than merely
    /// lifting off.
    pub brake_margin: f32,
    /// How much of the corridor either side of the line a driver may spend on
    /// [`Personality`]'s bias and drift, as a fraction of the room on that side.
    ///
    /// **Well under one on purpose.** The corridor's own edge is the last thing
    /// between an opponent and the scenery, so the clamp against it is a
    /// backstop and this is what actually decides how wide the field runs. A
    /// driver aiming at the edge would be relying on the clamp, and the clamp
    /// only knows about the point being aimed at, not about where the craft
    /// ends up while it gets there.
    pub corridor_use: f32,
    /// The lowest both-sides airbrake command that counts as braking.
    ///
    /// **`oag_physics::controls::update` gates the brake on both inputs being
    /// strictly positive and never reads their level**, so an epsilon command
    /// on both sides buys the whole of the deceleration at almost no cost in
    /// grip - `max(L, R)` is what the grip coefficient reads. That is faithful
    /// for the PSP's digital shoulder buttons and degenerate for the analog
    /// axis `oag_gameplay`'s input snapshot admits, and this is the floor that
    /// keeps the driver out of it.
    pub brake_floor: f32,
    /// Extra both-sides brake per unit of overspeed, as a fraction of target.
    ///
    /// **It buys no extra deceleration.** See [`Tuning::brake_floor`]: the
    /// brake ramps at one rate whatever the command. What it spends is grip,
    /// so this is how fast a driver gives up cornering to shed speed it should
    /// not have had.
    pub brake_gain: f32,
    /// Turn-rate error, in radians per second, below which no differential
    /// airbrake is applied at all.
    ///
    /// Keeps the differential out of the small-signal regime entirely, so the
    /// loop linearised about the line is the one `tests/closed_loop.rs`
    /// measured, unchanged.
    pub trail_deadband: f32,
    /// Differential airbrake per radian per second of turn-rate error past
    /// [`Tuning::trail_deadband`].
    pub trail_gain: f32,
    /// Ceiling on the differential.
    pub trail_max: f32,
    /// How saturated the steering command must be, as a fraction of full lock,
    /// before the differential engages.
    ///
    /// **The gate that makes this safe.** Below it the steering loop still has
    /// authority of its own, and a second path acting in parallel with a loop
    /// that already has authority is precisely the oscillation this crate was
    /// rewritten to remove. Above it the loop has run out of lock and the yaw
    /// the differential adds is authority the loop cannot produce at all.
    pub trail_saturation: f32,
    /// How often a driver misses a braking point, per tick, while it is at one.
    ///
    /// **Zero for a driver that never errs**, which is what the hardest
    /// difficulty sets it to - see [`Difficulty::mistakes`]. Scaled by the
    /// difficulty rather than read from it, so the driver never learns that
    /// difficulties exist: a level is a transformation of the tunables, not a
    /// parameter the controller branches on.
    ///
    /// [`Difficulty::mistakes`]: crate::Difficulty::mistakes
    pub mistake_rate: f32,
    /// How long a driver takes to notice a craft arriving beside, in front of
    /// or behind it, in ticks.
    ///
    /// **Zero for a driver that reacts on the frame**, which is what the
    /// hardest difficulty sets it to - see [`Difficulty::reaction_ticks`], and
    /// [`super::reflex`] for what a channel does while the clock runs. An
    /// integer because it counts ticks and rides in the world snapshot; the
    /// same reason `Driver::mistake` is one.
    ///
    /// [`Difficulty::reaction_ticks`]: crate::Difficulty::reaction_ticks
    pub reaction_ticks: u16,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            look_min: 20.0,
            look_speed: 0.35,
            look_max: 90.0,
            rate_gain: 5.0,
            max_turn_rate: 1.8,
            lateral_accel: 260.0,
            brake_lookahead: 2.5,
            curvature_span: Some(11.0),
            brake_margin: 0.05,
            corridor_use: 0.6,
            brake_floor: 0.35,
            brake_gain: 2.0,
            trail_deadband: 0.15,
            trail_gain: 1.2,
            trail_max: 0.6,
            trail_saturation: 0.85,
            // **Zero, because this is the competent driver.** Erring is a
            // degradation, so the rate belongs to `Difficulty` and arrives by
            // `Difficulty::tune`; a default that errs would make every caller
            // that has not chosen a difficulty - the closed-loop harness, a
            // replay - quietly non-deterministic in its driving.
            mistake_rate: 0.0,
            // **Zero for the same reason, and it is load-bearing.** A default
            // that took a tick to notice anything would change what every
            // existing closed-loop assertion measures, and none of them chose
            // a difficulty. See `super::reflex`.
            reaction_ticks: 0,
        }
    }
}
