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
    ///
    /// **Ours, and this project's own** - the disc authors a per-class
    /// `LookAheadSecs` (`docs/ghidra/functions/psp-pulse-usa/ai-stats.md`,
    /// `AiStats_ParseController`), but that page's own header says its units
    /// are not determined and its consumer is not identified: a lookahead in
    /// an unknown formula is not a value this crate's differently-shaped pure
    /// pursuit could read off the disc without inventing the missing half.
    /// So this stays a project constant, chosen and swept the way
    /// [`Tuning::max_turn_rate`] and [`Tuning::lateral_accel`] were.
    ///
    /// # Lowered from `0.35`, and 07 unaffected
    ///
    /// **`13_Track`'s driver was measured carrying lateral momentum across a
    /// wide S-bend for tens of samples before its own corridor went
    /// negative** - see `docs/gameplay/ai.md`'s convergence section for the
    /// full trace. The pure-pursuit correction is
    /// `curvature = 2 * offset / distance^2`, and `distance` grows with this
    /// constant, so a longer lookahead answers a given lateral error with a
    /// gentler request - exactly backwards from what a craft already outside
    /// its corridor needs. `Tuning::rate_gain` was swept first and ruled
    /// out: at `10.0` and `20.0` the loop is already saturated at full lock
    /// through these sites, so more gain buys nothing there and instead
    /// breaks the field - `opponent_weapons_ground_truth`'s floors fail at
    /// `10.0` (worst opponent shield `0.00`, a craft destroyed) and again at
    /// `20.0`.
    ///
    /// Swept by `sweep_look_speed` and `sweep_rate_gain` in
    /// `crates/game/tests/ai_look_sweep.rs` - a lone Ace, twelve forward
    /// circuits, 18,000 ticks each - against the two committed field fixtures
    /// (`lap_times_ground_truth::clocks`'s and
    /// `opponent_weapons_ground_truth::single_race`'s own setups, tuned
    /// through [`crate::Difficulty::tune`] exactly as `Race::start` does):
    ///
    /// | value | 13 end shield | 07 end shield | solo total | field mean | field worst |
    /// | --- | --- | --- | --- | --- | --- |
    /// | 0.35 (old) | 6.52 | 0.00 | 705.68 | 0.90 | 0.70 |
    /// | 0.33 | 17.86 | 0.00 | 774.28 | 0.86 | 0.75 |
    /// | 0.32 | 24.17 | 0.00 | 788.97 | 0.83 | 0.54 |
    /// | **0.30 (shipped)** | **39.07** | **0.00** | **863.10** | **0.73** | **0.00 (fails 0.45)** |
    /// | 0.25 | 59.50 | 0.00 | 932.20 | 0.92 | 0.67 |
    /// | 0.22 | 67.55 | 0.00 | 937.78 | 0.66 (fails 0.70) | 0.20 (fails 0.45) |
    /// | 0.20 | 63.45 | 0.00 | 934.48 | 0.68 (fails 0.70) | 0.00 (fails 0.45) |
    /// | 0.15 | 58.02 | 0.00 | 914.41 | 0.85 | 0.50 |
    ///
    /// # Re-swept 2026-09-07, after `oag_physics::pair::overlap`'s correction
    ///
    /// **The three left-hand columns (`13`, `07`, solo total) are
    /// re-measured, not merely re-quoted, and reproduce to the digit against
    /// the row above them.** A lone craft never touches
    /// `oag_physics::pair::overlap` - there is nothing else on the circuit
    /// to collide with - so nothing about the corrected narrowphase (confidence
    /// 92, not touched by this re-sweep) can have moved a number
    /// this harness only ever measured on one craft. That is worth stating
    /// plainly: the convergence story this table exists to tell - `13`
    /// rising from `6.52` to `39.07`-`67.55` as `look_speed` falls, `07`
    /// stuck at `0.00` throughout - is exactly as solid today as it was
    /// before the correction landed.
    ///
    /// **The two field columns moved, and they no longer mean what they did.**
    /// Both are a single seed (the project's fixed `SEED = 1`, unchanged from
    /// before), and the corrected narrowphase changes which axis wins a
    /// contact tie on an already-detected overlap - enough, over eight craft
    /// and 3,600 ticks, to redirect one craft's whole later trajectory and
    /// every RNG draw it gates. At `0.30` that redirection now seats an
    /// opponent inside a Plasma blast it did not stand in before: `worst`
    /// reads `0.00`, failing the very floor this value was chosen to clear.
    /// **Measured over 16 seeds rather than trusting the one this table is
    /// pinned to** (`crates/game/tests/weapon_floor_sweep.rs`,
    /// `sweep_worst_shield_over_seeds` and `sweep_lap_completion_over_seeds`):
    /// mean-of-means shield is `0.68` at `0.30` against `0.72` at `0.22` and
    /// `0.71` at `0.20`; mean-of-worsts is `0.32`/`0.38`/`0.30`; a depleted
    /// craft appears in `2/16`, `2/16` and `3/16` seeds respectively; opponent
    /// lap completion (no weapon RNG at all) posts `2/16`, `2/16` and `3/16`
    /// short seeds. **None of mean shield, worst shield, depleted-craft count
    /// or lap completion separates `0.22` or `0.20` - both independently
    /// measured above as genuinely wrecking the field - from the shipped
    /// `0.30` any more.** `0.22` in fact scores better than `0.30` on every
    /// field column but one. See `opponent_weapons_ground_truth.rs`'s own doc
    /// comment for the full account; that test no longer serves as an AI
    /// tuning guard because of this finding, and does not try to.
    ///
    /// # `0.30`'s original justification does not survive this, and the
    /// constant was not moved anyway
    ///
    /// **`0.30` was chosen over the total-maximising `0.25` (and `0.22`,
    /// which scores higher still) entirely on field-floor cliff distance** -
    /// the paragraph above this one, before the correction, read "`0.22` and
    /// `0.20`... both fail the field floors outright... distance from that
    /// cliff is worth more here than the last few points of `13`'s number."
    /// **That reasoning is gone**: the field floor the cliff was measured
    /// against no longer discriminates any of these values from one another,
    /// so there is no field-side evidence left that `0.22` or `0.25` costs
    /// anything `0.30` does not also cost. On solo evidence alone - the only
    /// evidence this correction left standing - `0.22` is the best row on the
    /// board (`937.78` solo total, `67.55` on `13`), with `0.25` close behind
    /// and `0.30` well back on both.
    ///
    /// **This is reported rather than acted on.** Moving the constant on this
    /// finding alone would be retuning on the strength of a guard that no
    /// longer exists: the field board was what stood between "a value that
    /// scores well solo" and "a value that wrecks a real race with other
    /// craft on it," and nothing here has replaced that check for a *tuning*
    /// regression - `opponent_weapons_ground_truth`'s rebuilt assertion is
    /// deliberately coarse (catches the field collapsing, not catches a
    /// tuning nudge) precisely because the finer-grained version is what this
    /// section just showed cannot be built from this scenario any more. `0.30`
    /// stays the shipped value until someone builds the replacement guard or
    /// rules that solo evidence is enough on its own.
    ///
    /// **`07_Track` is unchanged at every value tested** - `0.00` end shield
    /// in every row, the same craft destroyed at the same eventual state as
    /// the old default. Its per-lap loss does fall (28-30 down to 22-25 at
    /// `0.30`), but the end state does not move, so this constant neither
    /// fixes nor regresses 07's own wall-grind problem - a separate, still-open
    /// question about the damage-charging model rather than about
    /// convergence.
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
    ///
    /// # It used to do a second job, and that job is now the hull's
    ///
    /// Until 2026-09-12 this same constant was also the kinematic corner limit
    /// in [`pace::corner_target`] - `max_turn_rate / curvature`, the fastest a
    /// craft can hold a line it has to yaw along. **It was wrong there for
    /// every craft on the disc.** The rate a hull achieves is `steer *
    /// Turning.amount / (5 * I_yy)`; `I_yy` is a code literal shared by every
    /// craft (see [`oag_physics::forces::YAW_INVERSE_INERTIA`]) and
    /// `<Turning amount>` is authored per team, giving ceilings of **1.204 to
    /// 1.667 rad/s** - none of them 1.8. So that limit was asking every craft
    /// for a corner speed it could not rotate at, by 7 % on Feisar and 33 % on
    /// Triakis and Piranha.
    ///
    /// [`pace::corner_target`] now reads
    /// [`pace::hull_yaw_ceiling`](super::pace::hull_yaw_ceiling) and takes the
    /// smaller of it and this, so **this is the permission and nothing else**:
    /// it caps what a driver is *allowed* to want, which is what keeps
    /// [`crate::Difficulty::tune`]'s ladder meaningful, and
    /// [`Driver::steering`](crate::Driver::steering)'s clamp on the requested
    /// rate is the one the sweep above measures. Read the flat table as
    /// evidence about a permission that has stopped binding, **not** as
    /// evidence that 1.8 was ever the right corner limit. The full table and
    /// the board it was measured on are in `docs/gameplay/ai.md`, "The
    /// clean-Ace board".
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
    ///
    /// # The sweep that chose eleven no longer measures the shipped function
    ///
    /// Recorded 2026-09-12. Every row of the table above was taken with
    /// [`pace::corner_target`]'s kinematic limit evaluated at
    /// [`Self::max_turn_rate`]'s 1.8. It is now evaluated at the flown craft's
    /// own hull ceiling, 1.204-1.667 depending on team - so the sweep measured
    /// a different function from the one shipping, and its criterion ("the best
    /// point in a band bounded by two field tests going red at span 10") is no
    /// longer supported by it. **Nothing is broken**: both field ground-truth
    /// tests are green at eleven today, and that was checked rather than
    /// assumed. But the *choice* wants re-establishing, and the work that would
    /// do it is the estimator change the Outpost 7 thread's step 6 names -
    /// which has to re-sweep the span anyway.
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
    ///
    /// **Lowered from `0.15` to `0.05` on 2026-09-11**, alongside
    /// [`Tuning::trail_gain`] and [`Tuning::trail_saturation`] - see the
    /// latter's own doc for why the three move together. At the old value this
    /// sat only `0.02` rad/s below the turn-rate error
    /// [`Tuning::trail_saturation`] itself implies (`0.85 / rate_gain` = `0.17`
    /// at the defaults), so the moment the gate opened there was almost no
    /// `rate_error - trail_deadband` left to turn into a command: measured on
    /// a live `--race --autopilot` run of Talon's Junction's own U-turn, the
    /// differential peaked at **7.67** of the airbrake's `0..=100` range.
    pub trail_deadband: f32,
    /// Differential airbrake per radian per second of turn-rate error past
    /// [`Tuning::trail_deadband`].
    ///
    /// **Raised from `1.2` to `3.0` on 2026-09-11**, the other half of
    /// [`Tuning::trail_deadband`]'s move - see that field's own doc for the
    /// measurement neither alone explains.
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
    ///
    /// **Lowered from `0.85` to `0.7` on 2026-09-11, alongside
    /// [`Tuning::trail_deadband`] and [`Tuning::trail_gain`].** Reported from
    /// play: with `driver::pace::trail`'s corner-entry gate fixed (see that
    /// function's own doc), the differential fired on Talon's Junction's own
    /// U-turn but was reported as imperceptible - measured at 19 engaged
    /// ticks over the corner and a mean magnitude of 4.6 of the airbrake's
    /// `0..=100` range, because `0.85`'s implied turn-rate threshold left
    /// [`Tuning::trail_deadband`] almost no headroom to compute a command
    /// from. Three points swept on the same live run and the same real-disc
    /// regression that caught the original gate bug
    /// (`opponent_weapons_ground_truth::a_field_racing_with_real_pads_does_not_mine_itself_to_death`):
    ///
    /// | `trail_saturation` | engaged ticks | mean magnitude | mean-of-means energy | depleted |
    /// | --- | --- | --- | --- | --- |
    /// | `0.85` (old) | 19 | 4.6 | 0.67 | 0 |
    /// | `0.6` | 58 | 23.9 | 0.58 | 3 |
    /// | **`0.7`** | **41** | **25.2** | **0.67** | **1** |
    ///
    /// `0.6` engages for longer and no stronger, but spends enough grip across
    /// a full seven-opponent field with real weapons to measurably cost the
    /// energy floor the field test guards - `0.7` is the point on this sweep
    /// that gets back to more than double the old engagement with the old
    /// energy margin intact. `trail_deadband` and `trail_gain` hold the
    /// engaged magnitude in the 20s at every point on the sweep; only this
    /// field decides how much of the corner reaches it. Zero respawns on the
    /// live run at every point swept, including `0.6` - the corner was never
    /// missed, only the field cost moved.
    pub trail_saturation: f32,
    /// Curvature below which [`pace::trail`] treats the line as straight,
    /// whatever [`pace::corner_target`] says about it.
    ///
    /// **Fitted, not derived**, against a real two-lap capture of Talon's
    /// Junction - see `docs/gameplay/ai.md`, "Airbrakes, and what a
    /// differential one actually does": curvature on its straights sat at
    /// 0.0003-0.0005 (chord noise off three sampled points that are never
    /// *quite* collinear), against 0.0056 and up everywhere the driver was
    /// actually saturated fighting a real bend. `0.001` sits in the gap
    /// between the two, well clear of either. This is what `!target.is_finite()`
    /// was reaching for and could not get, because no disc track's `target` is
    /// ever literally infinite - see [`pace::trail`].
    ///
    /// [`pace::trail`]: super::pace::trail
    /// [`pace::corner_target`]: super::pace::corner_target
    pub trail_curvature_floor: f32,
    /// Fraction of [`Driver::peak_curvature`](crate::Driver::peak_curvature)
    /// curvature may fall to before [`pace::trail`] reads it as corner exit.
    ///
    /// **Fitted against the same capture.** Past the apex of Talon's Junction's
    /// tightest bend, curvature ran 0.0274 down to 0.0117 over 63 ticks as the
    /// craft pulled away - a real decline, not the ~5e-5 tick-to-tick noise the
    /// chord estimate carries mid-corner. `0.7` sits well inside that decline
    /// and well outside the noise, so a corner has to have genuinely opened up
    /// before this gates the differential back off.
    ///
    /// [`pace::trail`]: super::pace::trail
    pub trail_exit_decay: f32,
    /// Per-tick leak on [`Driver::peak_curvature`](crate::Driver::peak_curvature),
    /// the high-water mark [`trail_exit_decay`](Self::trail_exit_decay) is read
    /// against.
    ///
    /// **`1.0` latches the mark for the whole race, which is what it used to do
    /// and it was a bug.** `pace::track_peak_curvature` resets the mark when
    /// the curvature falls to [`Self::trail_curvature_floor`], meaning "the
    /// line went straight" - and measured on `07_Track` over 6,000 ticks the
    /// smallest windowed curvature all run is `0.00127` against a floor of
    /// `0.00100`, so **the reset never fires on this disc's geometry**. The
    /// mark became a running maximum over the whole race, latching on the
    /// circuit's tightest corner, and the exit gate it feeds then meant "this
    /// is not the tightest corner seen so far" - true almost everywhere after
    /// lap one. That gate alone rejected 74.8 % of ticks and the differential
    /// fired on 3.78 %.
    ///
    /// A leak makes "exit" mean the corner has opened up since its tightest
    /// point *recently*. The value is a half-life: at `0.99` the mark halves in
    /// about 69 ticks, a little over a second, which is the scale of a corner.
    ///
    /// # And it is nonetheless shipped at `1.0`, which is the latch
    ///
    /// **The leak works as a mechanism and does not pay as a tuning.** Swept by
    /// `sweep_trail_peak_decay` over the whole 48-row board, lone Ace, charge
    /// capped at each row's pool:
    ///
    /// | decay | contact ticks | charged | end pool | respawns | eliminated | clean laps | mean lap |
    /// | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
    /// | **1.0** | **10,924** | **2,277.3** | **2,191.6** | 35 | **11** | 44/48 | 38.88s |
    /// | 0.998 | 14,228 | 2,373.4 | 2,057.1 | 40 | 10 | 44/48 | 38.95s |
    /// | 0.995 | 13,902 | 2,317.3 | 2,129.6 | 34 | 12 | 43/48 | 38.76s |
    /// | 0.99 | 14,973 | 2,339.5 | 2,112.7 | 34 | 12 | 43/48 | 38.72s |
    /// | 0.95 | 13,917 | 2,302.2 | 2,181.8 | 34 | 11 | 45/48 | 38.72s |
    ///
    /// Every leak value costs contact ticks and end-of-run shield against the
    /// latch, and buys **0.16 s** of mean lap. On `07_Track` the differential
    /// goes from firing on 3.78 % of ticks to 11.58 % - three times as much
    /// airbrake - and the board gets worse.
    ///
    /// **Why, and it is the useful half of the result**: the differential is
    /// spent *reactively*, as a rescue once [`Self::trail_saturation`] says the
    /// steering loop has already run out of authority. `pace::trail`'s own doc
    /// records that it "cuts lateral grip exactly as hard as holding both sides
    /// would", so spending it without also **banking** the higher corner speed
    /// it permits is a pure loss of grip. The speed is only banked if
    /// `pace::corner_target` raises its target *because* the differential is
    /// planned - `v = omega_steer / (k - C)` - and it does not. Until it does,
    /// more differential is more grip spent for nothing, which is what the table
    /// measures.
    ///
    /// So `1.0` here is **not** an endorsement of the latch. It is the value
    /// that changes no behaviour while the constant, the sweep and the
    /// measurement exist for the pass that makes the differential a plan. See
    /// `pace::track_peak_curvature` and `docs/gameplay/ai.md`.
    pub trail_peak_decay: f32,
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
            look_speed: 0.30,
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
            trail_deadband: 0.05,
            trail_gain: 3.0,
            trail_max: 0.6,
            trail_saturation: 0.7,
            trail_curvature_floor: 0.001,
            trail_exit_decay: 0.7,
            trail_peak_decay: 1.0,
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
