//! Turning a craft's state and a line into the controls a craft is flown with.
//!
//! # The plant, and why the controller has the shape it has
//!
//! `oag_physics::engine::steering` feeds `Accumulators::local_angular`, which is
//! **torque**: steering commands yaw *acceleration*, behind the first-order lag
//! of `controls::ramp_steering`. Proportional feedback on heading around that
//! (a double integrator with a lag) overshoots forever, which is what this
//! crate's first version did, wall to wall.
//!
//! So the geometry produces a **target turn rate** and the loop is closed on
//! the rate the craft has: proportional feedback on a rate is derivative
//! feedback on a heading, the damping the plant needs. `tests/closed_loop.rs` is
//! the regression and was watched failing first.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_physics::{ShipControls, ShipState};

use crate::field::Field;
use crate::line::{Aim, Line};
use crate::noise::{roll, wobble};
use crate::pilot::Pilot;

pub use avoidance::LOOKAHEAD as AVOIDANCE_LOOKAHEAD;
use pace::{airbrakes, corner_target, curvature_span, throttle, track_peak_curvature, trail};
pub use pads::LOOKAHEAD as PAD_LOOKAHEAD;
pub use personality::Personality;
use planned::plan_slack;
pub use reflex::Reflex;
pub use tuning::Tuning;

/// Everything a driver reads that is not its own craft.
///
/// A bundle so a new channel is one line per call site, not signature churn.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// The line to follow, and the corridor around it.
    pub line: &'a Line,
    /// The constants the whole field shares.
    pub tuning: &'a Tuning,
    /// The character this craft is drawn from.
    pub pilot: &'a Pilot,
    /// What this craft can see of the rest of the grid.
    pub field: &'a Field,
    /// The yaw rate **this craft's own hull** can sustain, or `None` to fall
    /// back on [`Tuning::max_turn_rate`] alone.
    ///
    /// The one per-craft number here: `Tuning` is shared by the grid while
    /// `<Turning amount>` is authored per team (1.30 to 1.80). See
    /// [`hull_yaw_ceiling`](pace::hull_yaw_ceiling) and [`pace::corner_target`].
    /// `None` is for a caller with no craft to hand (a synthetic test, a probe).
    pub yaw_ceiling: Option<f32>,
    /// The speed plan for this line and craft, when the race built one (see
    /// [`crate::plan`]). `None` keeps the corner model (`pace::corner_target`).
    pub plan: Option<&'a crate::SpeedPlan>,
}

impl<'a> Context<'a> {
    /// A context with the balanced pilot, for callers that have not got one.
    #[must_use]
    pub fn new(line: &'a Line, tuning: &'a Tuning) -> Self {
        Self {
            line,
            tuning,
            pilot: &Pilot::BALANCED,
            field: &Field::EMPTY,
            yaw_ceiling: None,
            plan: None,
        }
    }
}

/// What a driver carries from one tick to the next.
///
/// Plain integers, so it is `Copy` and `Eq` and sits on a `Ship` inside the
/// world snapshot: state beside the world would be invisible to a replay. See
/// [ADR-0003](../../../docs/architecture/adr/0003-no-ecs.md).
///
/// **The [`Personality`] is not stored**, it is derived from [`Self::seed`]
/// each tick: storing six unchanging `f32`s would cost the type its `Eq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Driver {
    /// Where on the line this craft was last found: the seed for the next
    /// windowed search, so it stays local and cannot latch onto stacked track.
    pub index: u32,
    /// Which driver this is, as far as [`Personality::from_seed`] is concerned.
    ///
    /// **Zero is the plain line-follower**, so `Driver::default()` has no
    /// character. The caller derives it from the race seed and the slot, so a
    /// replay fields the same field.
    pub seed: u32,
    /// Ticks this driver has driven: the argument its wander is a function of.
    ///
    /// Not a clock (`oag_core::TickClock` is the only time the simulation may
    /// read): a craft wrecked for thirty ticks resumes where it left off.
    pub phase: u32,
    /// Where this driver was placed last tick, `1`-based; `0` before it has
    /// been placed.
    ///
    /// A place that got worse is a craft just passed. **Zero is not first**:
    /// reading it as first would provoke the grid when standings first resolve.
    pub place: u8,
    /// Ticks of provocation still to run, counting down.
    ///
    /// A countdown rather than a level with a decay rate, which would be an
    /// `f32` on an `Eq` type. See [`Personality::provocation_ticks`].
    pub provocation: u16,
    /// A fingerprint of the pilot this craft is flying.
    ///
    /// **Hashed, unlike `Ship::handling`, on purpose.** Handling comes off the
    /// player's disc and matches on every machine with that disc. A pilot can
    /// come out of `<config dir>/oag/pilots/`, which **differs between machines
    /// by design**: left out, two machines running "the same race" with
    /// different `winston.toml` files would hash identically, a gate claiming
    /// an agreement it does not have.
    ///
    /// A digest of the **resolved numbers, not of the name**. `0` for
    /// [`Driver::default`]; no built-in pilot digests to zero.
    ///
    /// It says "these differ", not *which*: 32 bits cannot be inverted, so the
    /// composition root logs every roster entry's name and digest at startup.
    pub pilot: u32,
    /// Ticks left of a braking point this driver is in the middle of missing.
    ///
    /// The whole of mistake injection. `docs/gameplay/ai.md` calls recovery the
    /// expensive half, but this controller follows a line, so recovery is what
    /// it was already doing: a mistake is holding the throttle through a corner
    /// it should have braked for. An integer, for [`Self::provocation`]'s reason.
    pub mistake: u16,
    /// The highest [`pace::corner_target`] curvature seen since the line last
    /// went straight, as `f32::to_bits`.
    ///
    /// Tells [`pace::trail`] the corner is opening up rather than being
    /// entered, which `speed < target` does not on Talon's Junction (see that
    /// function). Bits for [`Self::provocation`]'s reason; `0` is `0.0`.
    pub peak_curvature: u32,
    /// What this driver has noticed of the craft around it, and what it is
    /// still noticing: reaction latency, see [`Reflex`]. Integers, for
    /// [`Self::provocation`]'s reason; `Reflex::IDLE` before the first tick is
    /// not the same as having looked and seen nobody.
    pub reflex: Reflex,
    /// Whether this driver has already decided about a barrel roll during the
    /// flight it is in.
    ///
    /// One decision per airborne window, which is what makes
    /// [`Personality::roll_chance`] a probability rather than a rate. Set on the
    /// first tick past [`Personality::roll_airtime`] whichever way it went, and
    /// cleared on landing.
    pub roll_decided: bool,
    /// Which line it is on and what it decided at the last fork: [`crate::branch`].
    pub branching: crate::branch::Branching,
}

/// How much of the line either side of the last index a driver looks at.
///
/// Wide enough to find a craft knocked sideways, narrow enough not to cross to
/// a stacked section. Eight craft pay it every tick.
const SEARCH_WINDOW: usize = 48;

/// The most provocation a driver can carry, in ticks (ten seconds), so a craft
/// having a bad race cannot bank grievance forever.
const PROVOCATION_MAX: u16 = 600;

/// The noise stream ramming decisions are rolled against.
const RAM_STREAM: u32 = 1;

/// How often a driver at full `ram` takes a shot at a rival alongside: once a second.
const RAM_RATE: f32 = 1.0 / 60.0;

/// The noise stream mistakes are rolled against.
const MISTAKE_STREAM: u32 = 3;

/// How long a missed braking point lasts. Half a second, which at racing speed
/// is comfortably past the point the driver should have lifted.
const MISTAKE_TICKS: u16 = 30;

/// The noise stream weapon decisions are rolled against.
const WEAPON_STREAM: u32 = 2;

/// The noise stream the barrel roll's commit decision is rolled against.
const ROLL_STREAM: u32 = 4;

/// The stream for which way a roll goes: separate from [`ROLL_STREAM`], else a
/// pilot with a high [`Personality::roll_chance`] would only roll one way.
const ROLL_SIDE_STREAM: u32 = 5;

/// How often a driver at full `trigger` fires once it has a target, per tick.
///
/// **A rate, not a probability**: at `1.0` the median wait is about thirteen
/// ticks, at `0.2` about a second and a half.
const TRIGGER_RATE: f32 = 0.05;

/// How much corridor room a driver wants on the side it is shifting toward,
/// measured **from where the craft is**, not from the line.
///
/// Was `3.0`, under half the median 8.2 units a shift covers on `16_Track`.
/// Twelve plus the craft's own room took the rammer ending up outside the
/// corridor from 54 of 208 to 11 of 138; neither half does much alone. Sweep:
/// `docs/gameplay/ai.md`, guarded by `crates/game/tests/ram_ground_truth.rs`.
///
/// **Ours, and measured against one hull.** Reach scales with
/// `handling.airbrake.sideshift / mass` (`450` against `1` on Pulse), which
/// `Context` cannot see.
const RAM_CLEARANCE: f32 = 12.0;

/// The turn, in radians across the stretch to the aim point, at which
/// [`Personality::inside`] asks for all the room it is allowed.
///
/// `Line::bend` is about a tenth of a radian on a corner taken at speed, so
/// without a scale `inside` would read as broken. Ours, a feel knob: about nine
/// degrees.
const FULL_BEND: f32 = 0.15;

impl Driver {
    /// A driver with a character of its own, from a seed; see
    /// [`Personality::from_seed`] (zero is the plain line-follower).
    #[must_use]
    pub fn seeded(seed: u32) -> Self {
        Self {
            seed,
            ..Self::default()
        }
    }

    /// The driver for one slot of a race, from the race seed and slot alone, so
    /// replays field the same eight characters. Slot zero is the player's and
    /// gets the plain line-follower.
    #[must_use]
    pub fn for_slot(race_seed: u64, slot: u32) -> Self {
        if slot == 0 {
            return Self::default();
        }
        // Mix the slot into the high bits so slots 1 and 2 are not neighbours.
        let mixed = race_seed ^ u64::from(slot).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let seed = Rng::new(mixed).next_u32();
        // Zero is the plain driver; do not field one by accident.
        Self::seeded(if seed == 0 { 1 } else { seed })
    }

    /// This driver's character. Derived, not stored - see [`Self::seed`].
    #[must_use]
    pub fn personality(&self, pilot: &Pilot) -> Personality {
        Personality::from_pilot_seed(pilot, self.seed)
    }

    /// Chooses this tick's controls and advances the driver's place on the line.
    ///
    /// Returns released controls when there is no line, which leaves opponents
    /// on the grid.
    pub fn drive(&mut self, state: &ShipState, ctx: &Context<'_>) -> ShipControls {
        let (line, tuning) = (ctx.line, ctx.tuning);
        if line.is_empty() {
            return ShipControls::default();
        }

        // What this driver has noticed, not what the caller measured. Advanced
        // before it is read so a wait that runs out this tick is acted on now.
        self.reflex.advance(ctx.field, tuning.reaction_ticks);
        let noticed = self.reflex.filter(ctx.field);
        let ctx = &Context {
            field: &noticed,
            ..*ctx
        };

        let personality = self.personality(ctx.pilot);
        // On a plan a line is a handicap (`Personality::spent`, `plan_slack`).
        // Off one it is untouched, not multiplied by one: that moves bits.
        let personality = if ctx.plan.is_some_and(|plan| plan.len() == line.len()) {
            personality.spent(plan_slack(ctx, &personality))
        } else {
            personality
        };
        let body = &state.body;
        let forward = body.forward();

        let index = line.nearest(body.position, self.index as usize, SEARCH_WINDOW);
        self.index = index as u32;
        // Only ticks this driver actually drove. See [`Self::phase`].
        self.phase = self.phase.wrapping_add(1);
        self.stew(ctx, &personality);

        // Forward speed: a craft sliding sideways is not approaching its corner
        // at that speed.
        let speed = body.linear_velocity.dot(forward).max(0.0);
        let look =
            (tuning.look_min + tuning.look_speed * speed).min(tuning.look_max) * personality.look;

        let steer = self.steering(state, ctx, look, speed, &personality);
        let window = look * tuning.brake_lookahead * personality.patience;
        let step = curvature_span(tuning, look);
        let curvature =
            line.max_curvature_stepped(index, window, pace::curvature_chord(tuning, step), step);
        // The plan when the race built one, else the corner model. A level and
        // a pilot handicap the plan by [`plan_margin`], not by believing in
        // less grip.
        let followed = ctx.plan.filter(|plan| plan.len() == line.len());
        let target = match followed {
            Some(plan) => planned::target(
                plan,
                index,
                speed,
                tuning,
                &personality,
                line.takeoff_within(index, planned::RUN_UP_REACH),
            ),
            None => corner_target(curvature, tuning, &personality, ctx.yaw_ceiling),
        };
        // Every tick, saturated or not - see [`track_peak_curvature`]'s own doc.
        self.peak_curvature =
            track_peak_curvature(curvature, f32::from_bits(self.peak_curvature), tuning).to_bits();
        self.blunder(tuning, speed, target);
        let (thrust, brake) = if self.mistake > 0 {
            // Sailing through it. See [`Self::mistake`].
            (1.0, 0.0)
        } else if state.time_airborne > 0.0 {
            // No lateral grip airborne, so a symmetric brake is pure loss of the
            // forward speed a landing needs. Chosen, not measured, no confidence
            // score: our own behaviour. See `docs/gameplay/ai.md`, "The
            // jump-clearing failure".
            (1.0, 0.0)
        } else if followed.is_some() {
            // The plan's own follower, the law it was learned with.
            crate::plan::longitudinal(speed, target)
        } else {
            throttle(speed, target, tuning)
        };
        // No lift on the run-up to a gap: clearing speed cannot be bought back
        // in the air. Chosen, not measured.
        let thrust = if line.is_takeoff(index) {
            thrust
        } else {
            thrust * self.caution(ctx, &personality)
        };
        // Both timers are seconds still running, so "recovering" lasts while
        // either does: the window the differential buys nothing in (see `trail`).
        let recovering = state.slowdown_timer > 0.0 || state.stun_timer > 0.0;
        let differential = trail(
            &steer,
            curvature,
            f32::from_bits(self.peak_curvature),
            recovering,
            tuning,
            &personality,
        );
        let (airbrake_left, airbrake_right) = if followed.is_some() {
            crate::plan::airbrakes(brake, differential)
        } else {
            airbrakes(brake, differential, tuning.brake_floor)
        };

        ShipControls {
            steer_x: steer.command,
            thrust,
            airbrake_left,
            airbrake_right,
            sideshift: self.ram(state, ctx, &personality),
            roll_request: self.wants_to_roll(state, &personality),
            // Every tick: it also gates the gesture, which this driver's own
            // steering can complete by accident. See
            // `oag_physics::barrel_roll::within_budget`.
            roll_shield_floor: personality.roll_floor,
            ..ShipControls::default()
        }
    }

    /// Whether the line for `distance` ahead allows `speed`.
    ///
    /// The cornering limit [`throttle`] brakes against, asked about a speed the
    /// craft does not have yet: **a boost wants to know**. The braking horizon
    /// is built from the current speed, so a Turbo at 126 puts a craft through
    /// the next corner at 270 having checked 160 units (measured on `16_Track`,
    /// `docs/gameplay/ai.md`).
    #[must_use]
    pub fn allows_speed(&self, ctx: &Context<'_>, speed: f32, distance: f32) -> bool {
        if ctx.line.is_empty() || distance <= 0.0 {
            return true;
        }
        let look = ctx.tuning.look_min + ctx.tuning.look_speed * speed;
        let step = curvature_span(ctx.tuning, look);
        let curvature = ctx.line.max_curvature_stepped(
            self.index as usize,
            distance,
            pace::curvature_chord(ctx.tuning, step),
            step,
        );
        speed
            <= corner_target(
                curvature,
                ctx.tuning,
                &self.personality(ctx.pilot),
                ctx.yaw_ceiling,
            )
    }

    /// The steering command, as a turn-rate error.
    ///
    /// Pure pursuit: the arc to a point on the line has curvature
    /// `k = 2 * e / L^2` (`e` the aim point's offset from the nose, `L` its
    /// distance). Times speed that is the yaw rate needed, and the loop closes
    /// on its difference from the rate the craft has.
    ///
    /// **`L` is the measured distance, not the requested lookahead**: they
    /// differ on a curve and `k` divides by the square. Cross-track error needs
    /// no term of its own, the aim point already carries it (a second term
    /// double-counted it).
    fn steering(
        &self,
        state: &ShipState,
        ctx: &Context<'_>,
        look: f32,
        speed: f32,
        personality: &Personality,
    ) -> Steer {
        let (line, tuning) = (ctx.line, ctx.tuning);
        let body = &state.body;
        let aim = line.aim(self.index as usize, look);
        // The drift moves the aim point, not the command: noise on a rate loop
        // with gain five is unfiltered, noise on the aim is a different line.
        let aim = aim.point + self.drift(&aim, look, ctx, personality);
        let to_aim = aim - body.position;
        let distance = to_aim.length();
        if distance <= f32::EPSILON {
            return Steer::STRAIGHT;
        }

        // Positive is to the craft's right.
        let offset = to_aim.dot(body.right());
        let curvature = 2.0 * offset / (distance * distance);
        let wanted =
            (curvature * speed.max(1.0)).clamp(-tuning.max_turn_rate, tuning.max_turn_rate);

        // Positive yaw about the craft's up turns it left (right-hand rule,
        // forward on -Z); using "turning right" keeps signs equal to the
        // steering input's. Craft up, never world up: wrong once the track rolls.
        // track rolls.
        let actual = -body.angular_velocity.dot(body.up());

        let rate_error = wanted - actual;
        Steer {
            command: (tuning.rate_gain * rate_error).clamp(-1.0, 1.0),
            rate_error,
        }
    }

    /// Decides whether this driver is about to miss a braking point, and
    /// counts down one it is already missing.
    ///
    /// Rolled only where it would otherwise brake: a mistake on a straight
    /// shows nothing and would waste the rate. The rate is per tick *of
    /// braking*, roughly once a second spent slowing.
    fn blunder(&mut self, tuning: &Tuning, speed: f32, target: f32) {
        if self.mistake > 0 {
            self.mistake -= 1;
            return;
        }
        // Seed zero never errs, like its missing personality: `Driver::default()`
        // is what every exact assertion here and in `oag-trace` is written
        // against. See `Personality::from_seed`.
        if self.seed == 0 || tuning.mistake_rate <= 0.0 || speed <= target {
            return;
        }
        if roll(self.seed, self.phase, MISTAKE_STREAM) < tuning.mistake_rate {
            self.mistake = MISTAKE_TICKS;
        }
    }

    /// Advances the grudge: notices being overtaken, and lets it cool.
    ///
    /// A worse place is a craft just passed. Both places must be real: `0`
    /// means unplaced. Provocation does **not** touch
    /// [`Personality::commitment`]: an angry AI driving into the scenery reads
    /// as a bug. It scales what a driver does to *other craft*.
    fn stew(&mut self, ctx: &Context<'_>, personality: &Personality) {
        let now = ctx.field.place;
        if now != 0 && self.place != 0 && now > self.place {
            let sting = personality.provocation_ticks as u16;
            self.provocation = self.provocation.saturating_add(sting).min(PROVOCATION_MAX);
        }
        self.provocation = self.provocation.saturating_sub(1);
        self.place = now;
    }

    /// How provoked this driver is, `0.0..=1.0`.
    fn provoked(&self) -> f32 {
        f32::from(self.provocation) / f32::from(PROVOCATION_MAX)
    }

    /// How much of its thrust this driver keeps when closing on a craft ahead,
    /// `0.0..=1.0`.
    ///
    /// **A lift, never a brake**: braking mid-corner spends the grip holding
    /// it. The speed target still owns braking. Exists because [`Self::social`]
    /// and [`Personality::inside`] sometimes move craft toward each other and
    /// nothing else reacts to a closing gap.
    fn caution(&self, ctx: &Context<'_>, personality: &Personality) -> f32 {
        if personality.caution <= 0.0 {
            return 1.0;
        }
        let Some(rival) = ctx.field.ahead else {
            return 1.0;
        };
        if rival.gap <= 0.0 || rival.gap >= CAUTION_RANGE || rival.closing <= 0.0 {
            return 1.0;
        }
        let closeness = (CAUTION_RANGE - rival.gap) / CAUTION_RANGE;
        let closing = (rival.closing / CLOSING_FULL).clamp(0.0, 1.0);
        (1.0 - personality.caution * closeness * closing).clamp(0.0, 1.0)
    }

    /// Yielding and blocking, as **one signed lean** toward or away from the
    /// craft behind, in the fraction-of-the-room units of [`Self::drift`].
    ///
    /// One term, not two: separately gated courtesy and defence fight and the
    /// craft jitters; as two signs of one number a pilot sits at
    /// `defence - courtesy`. Continuous in the gap, so no slew limiter or state.
    ///
    /// # A touching rival is `alongside`, not `behind`
    ///
    /// Found from play 2026-09-07: two touching craft produced no lateral
    /// response, since this read only [`Field::behind`] while `field_for`
    /// buckets a rival as [`Field::alongside`] inside `ALONGSIDE_GAP` (16) and
    /// `ALONGSIDE_WIDTH` (12). The lean dropped to zero at contact, and
    /// `oag_physics::pair::resolve`'s nudge fought both drivers' aim, which
    /// read as sticking. So `alongside` is read too and preferred, being the
    /// more urgent. No new constant, and none of the existing ones moved.
    ///
    /// `ALONGSIDE_GAP` is a little past [`SOCIAL_MIN_GAP`] (14), so blocking
    /// can still fire between 14 and 16, before anything touches; a real hull
    /// touch (3-6 units, `oag_physics::pair::overlap`) is well inside.
    /// **Chosen, not measured**, no confidence score: the original has no
    /// social term. Before the fix a real 8-craft `VENOM` race's pair `(4,7)`
    /// resolved on 90 consecutive ticks; the acceptance test is the longest
    /// streak (see the ground-truth test added with it).
    ///
    /// It will not:
    ///
    /// - **Take over the whole line**: capped at [`SOCIAL_MAX`].
    /// - **Swerve into somebody already there**: below [`SOCIAL_MIN_GAP`] it
    ///   stops, a block is something you do early.
    /// - **Act mid-corner**: both are straight-line manoeuvres. The gate is the
    ///   normalised `bend` the inside line uses.
    /// - **React equally to a craft holding station and one closing**: closing
    ///   earns the rest of the lean on top of [`PRESENCE_SHARE`].
    /// - **Guess a side from noise**: a rival dead astern has a meaningless
    ///   offset sign, so inside a deadband the side comes from the corner ahead
    ///   (yield toward its outside).
    fn social(&self, ctx: &Context<'_>, personality: &Personality, bend: f32) -> f32 {
        let touching = ctx.field.alongside;
        let Some(rival) = touching.or(ctx.field.behind) else {
            return 0.0;
        };
        // Provocation scales covering, not yielding: a passed driver defends
        // harder, it does not get more polite (see `Self::stew`). Clamped before
        // the budget scale so the lean saturates rather than overrunning the
        // aim budget.
        let lean =
            (personality.defence * (1.0 + self.provoked()) - personality.courtesy).clamp(-1.0, 1.0);

        let gap = rival.gap.abs();
        if gap >= AWARENESS_RANGE {
            return 0.0;
        }
        // Inside the close range only yielding survives ([`SOCIAL_MIN_GAP`]),
        // and everybody yields at least [`CONTACT_FLOOR`] when alongside.
        let lean = if touching.is_some() && gap < SOCIAL_MIN_GAP {
            lean.min(-CONTACT_FLOOR)
        } else if gap < SOCIAL_MIN_GAP {
            lean.min(0.0)
        } else {
            lean
        };
        if lean == 0.0 {
            return 0.0;
        }
        let closeness = (AWARENESS_RANGE - gap) / AWARENESS_RANGE;
        let closing = (rival.closing / CLOSING_FULL).clamp(0.0, 1.0);
        let pressure = closeness * (PRESENCE_SHARE + (1.0 - PRESENCE_SHARE) * closing);
        // `bend` is normalised by `FULL_BEND` and clamped: one at a corner,
        // zero on a straight.
        let corner_gate = 1.0 - bend.abs().min(1.0);

        // Dead astern the sign is noise: use the corner's outside instead.
        let side = if rival.offset.abs() > ASTERN_DEADBAND {
            rival.offset.signum()
        } else if bend.abs() > f32::EPSILON {
            -bend.signum()
        } else {
            return 0.0;
        };

        // Two budgets, since the halves fail differently: blocking swerves into
        // somebody and is kept tight; yielding gets more, but not the corridor,
        // or a craft reads as having given up.
        let budget = if lean > 0.0 { SOCIAL_MAX } else { YIELD_MAX };
        lean * side * pressure * corner_gate * budget
    }

    /// How far off the authored line this driver is aiming, as a world-space
    /// vector across it.
    ///
    /// A constant lean plus a slow drift, as a fraction of the room on the side
    /// being leant toward (the sides differ), scaled by
    /// [`Tuning::corridor_use`] and clamped against the corridor anyway.
    ///
    /// Zero when the line has no corridor (a synthetic line, driven exactly):
    /// guessing a lateral axis from world up is wrong once the track rolls
    /// (`docs/formats/track.md`).
    fn drift(&self, aim: &Aim, look: f32, ctx: &Context<'_>, personality: &Personality) -> Vec3 {
        let Some(frame) = aim.corridor else {
            return Vec3::ZERO;
        };
        let phase = self.phase as f32 * personality.wander_rate;
        // The seed is the driver's, so equal amplitudes still wander apart.
        let wobbled = personality.wander * wobble(self.seed, phase);
        // Which way the corner goes, not how hard: an inside line swaps sides
        // with the corner, so this axis reads geometry. Fades to zero on a
        // straight. Measured from the craft's own index, not the aim point's,
        // which is already `look` downtrack: three spans of `look / 3.0` sample
        // the stretch being steered through.
        let bend = (ctx
            .line
            .bend(self.index as usize, look / 3.0, frame.lateral)
            / FULL_BEND)
            .clamp(-1.0, 1.0);
        let inside = personality.inside * bend;
        let social = self.social(ctx, personality, bend);
        // Summed, not overriding, so a driver dodging a mine still drives a
        // line; avoidance has the largest budget and still has to win. See
        // [`Driver::avoidance`].
        let avoidance = self.avoidance(ctx.field.hazard);
        // On a plan the bias, wander and inside line arrive already spent
        // (`Personality::spent`). Avoidance and the social terms are about other
        // craft and are never scaled.
        let wanted =
            (personality.line_bias + wobbled + inside + social + avoidance).clamp(-1.0, 1.0);
        // Fractions of the room are clamped once here; the pad pull is in units,
        // and the corridor clamp backstops both.
        let offset = wanted * frame.room(wanted) * ctx.tuning.corridor_use * personality.width;
        let offset = Self::toward_pad(offset, ctx.field.pad, avoidance != 0.0);
        frame.lateral * frame.clamp(offset)
    }
}

/// How far behind a rival has to be before it stops being this driver's
/// problem, in units along the track. Ours: wide enough to see a closing craft
/// before it arrives, narrow enough not to defend against one a corner away.
pub const AWARENESS_RANGE: f32 = 120.0;

/// The closing speed, in units a second, at which a rival counts as closing as
/// hard as it will. Small on purpose: two racing craft differ by a few units a
/// second, and a "obviously catching up" threshold reads zero in a real battle.
const CLOSING_FULL: f32 = 8.0;

/// How much of the social lean a rival gets for merely being close.
///
/// **Not zero**: a craft on another's tail at matched pace is when yielding
/// matters, and a term gated purely on closing would do nothing there.
const PRESENCE_SHARE: f32 = 0.5;

/// How far off-centre a rival must sit before its offset's *sign* means
/// anything. Ours, about a hull's width.
const ASTERN_DEADBAND: f32 = 2.0;

/// How much of the aim budget a fully committed **block** is worth. Yielding is
/// not scaled by it.
///
/// **A scale, not a clamp**: a clamp swallowed provocation (an aggressive
/// pilot already sits near the top, so provoked and calm came out identical).
///
/// Reported from play: opponents turned almost ninety degrees into the player
/// to block, then hit a wall. `defence` scaled by provocation reached about
/// 1.9, nearly twice the `-1..1` budget, so it saturated the aim clamp alone
/// and squeezed every other term out of the sum.
const SOCIAL_MAX: f32 = 0.45;

/// And how much a full yield is worth.
///
/// **More than a block, far less than the corridor.** Yielding cannot swerve
/// into anybody, but a craft that hands over the whole track looks like it has
/// given up; what is wanted is room to be passed in.
const YIELD_MAX: f32 = 0.55;

/// How close a rival behind has to get before **blocking** stops: you cover a
/// line early, and past this a move across is a swerve into somebody.
///
/// **Yielding is deliberately not gated on it.** The first version suppressed
/// the whole term, and since a packed grid is permanently inside fourteen
/// units, craft stopped getting out of each other's way and the field lost
/// about an eighth of its pace (119 against 95 at elite, measured).
const SOCIAL_MIN_GAP: f32 = 14.0;

/// The minimum yield every driver gives a rival it is **already alongside**,
/// whatever its personality says.
///
/// **Chosen, not measured. No confidence score**: the original has no social
/// term, and opponent behaviour is a design axis here.
///
/// Inside [`SOCIAL_MIN_GAP`] only the yielding half of
/// [`Driver::social`]'s `defence - courtesy` survives, so every driver whose
/// defence is at least its courtesy (including `Personality::from_seed(0)`)
/// contributed zero at contact: two touching craft had nothing steering them
/// apart. That is the residue `craft_sticking_ground_truth`'s header records
/// after the `alongside` fix.
///
/// Small on purpose: a floor, not a policy. Scaled by [`YIELD_MAX`], pressure
/// and the corner gate, so it fades out mid-corner.
const CONTACT_FLOOR: f32 = 0.25;

/// How close a craft ahead has to be before a driver lifts for it, in units
/// along the track. Shorter than [`AWARENESS_RANGE`].
const CAUTION_RANGE: f32 = 45.0;

/// The steering command, and the turn-rate error it was computed from.
///
/// **The error travels with the command because [`trail`] must be a function
/// of the signal the rate loop closed on.** Feeding it `command` would be a
/// second loop around the first, the shape this crate was rewritten to remove.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Steer {
    /// The steering input, `-1.0..=1.0`, positive to the right.
    command: f32,
    /// Wanted turn rate minus actual, in radians per second, positive to the
    /// right.
    rate_error: f32,
}

impl Steer {
    /// No command and nothing left to correct.
    const STRAIGHT: Self = Self {
        command: 0.0,
        rate_error: 0.0,
    };
}

mod avoidance;
mod pace;
mod pads;
pub use pace::hull_yaw_ceiling;
mod personality;
mod planned;
mod ram;
mod reflex;
mod roll;
mod tuning;
mod weapons;

#[cfg(test)]
mod tests;
