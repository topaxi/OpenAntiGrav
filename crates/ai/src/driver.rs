//! Turning a craft's state and a line into the controls a craft is flown with.
//!
//! # The plant, and why the controller has the shape it has
//!
//! `oag_physics::engine::steering` feeds `Accumulators::local_angular`, which is
//! **torque**. So the steering input commands yaw *acceleration* - not a yaw
//! rate, and certainly not a heading. `controls::ramp_steering` puts a
//! first-order lag in front of that, because the steering state moves toward its
//! target at a finite rate rather than jumping.
//!
//! A controller that sets steering in proportion to how far off the line it is
//! is therefore proportional feedback around a double integrator with a lag. It
//! overshoots, corrects, overshoots the other way, and keeps doing it. **That is
//! what the first version of this crate did, and it drove wall to wall.** No
//! gain fixes it; the loop has to be closed one derivative in.
//!
//! So: the geometry produces a **target turn rate**, and the loop is closed on
//! the turn rate the craft actually has. Proportional feedback on a rate is
//! derivative feedback on a heading, which is the damping the plant needs.
//! `tests/closed_loop.rs` is the regression, and it was watched failing before
//! this was written.

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
/// A bundle rather than a widening argument list, so a later stage adding a
/// channel - what the other craft are doing, where the pads are - is one line
/// at each call site instead of a signature churn through every test.
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
    /// **The one per-craft number in here**, and the axis that decides whether
    /// the AI accommodates each team: `Tuning` is one set of constants for the
    /// whole grid, while `<Turning amount>` is authored per team and spans
    /// 1.30 to 1.80 across the disc's eight. See
    /// [`hull_yaw_ceiling`](pace::hull_yaw_ceiling) for the derivation and the
    /// table, and [`pace::corner_target`] for how it combines with the
    /// permission.
    ///
    /// `None` is exactly the behaviour from before this field existed, which is
    /// what lets a caller with no craft to hand - a synthetic closed-loop test,
    /// a probe - stay on the old reading rather than invent a hull.
    pub yaw_ceiling: Option<f32>,
    /// The speed plan for this line and this craft's handling, when the race
    /// built one - see [`crate::plan`]. `None` keeps the corner model
    /// (`pace::corner_target`), which is what every synthetic test and any
    /// line without a verified plan drives on.
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
/// Three `u32`s, and therefore `Copy`, `Eq` and free of allocation - which is
/// what lets it sit on a `Ship` inside the world snapshot rather than beside it.
/// A driver whose state lived in the composition root would be invisible to a
/// replay, and two runs of the same race would not be the same race. See
/// [ADR-0003](../../../docs/architecture/adr/0003-no-ecs.md).
///
/// **The [`Personality`] is not stored here**, it is derived from
/// [`Self::seed`] each tick. Storing it would put six `f32`s in the snapshot
/// that never change and cost the type its `Eq`, to save a handful of integer
/// operations per craft per tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Driver {
    /// Where on the line this craft was last found.
    ///
    /// The seed for the next windowed search, so the search stays local and a
    /// craft cannot latch onto a section of track stacked above or below it.
    pub index: u32,
    /// Which driver this is, as far as [`Personality::from_seed`] is concerned.
    ///
    /// **Zero means the plain line-follower** - see that function - so a
    /// `Driver::default()` behaves exactly as it did before personalities
    /// existed, and a caller opts a craft into having a character by giving it
    /// a seed. The caller derives it from the race's own seed and the craft's
    /// slot, so the field is the same field on every replay of that race.
    pub seed: u32,
    /// Ticks this driver has driven, which is the argument its wander is a
    /// function of.
    ///
    /// A counter rather than a clock: `oag_core::TickClock` is the only time
    /// the simulation may read, and this counts the ticks that reached *this
    /// driver*, so a craft that spends thirty ticks wrecked and released comes
    /// back where it left off instead of somewhere its own history cannot
    /// explain.
    pub phase: u32,
    /// Where this driver was placed last tick, `1`-based; `0` before it has ever
    /// been placed.
    ///
    /// The whole of what an overtake detector needs: a place that got *worse* is
    /// a craft that was just passed. A `u8` because the grid is eight, and an
    /// integer because this type is `Eq` and lives in the world snapshot.
    ///
    /// **Zero is not first.** A craft that has never been placed has not just
    /// been overtaken, and reading it as first would provoke the whole grid on
    /// the tick the standings first resolve.
    pub place: u8,
    /// Ticks of provocation still to run, counting down.
    ///
    /// **A countdown rather than a level with a decay rate**, because a rate
    /// would be an `f32` on a type that must stay `Eq`. How provokable a pilot
    /// is becomes how many ticks an overtake adds - see
    /// [`Personality::provocation_ticks`] - which is the same knob from the
    /// other end.
    pub provocation: u16,
    /// A fingerprint of the pilot this craft is flying.
    ///
    /// **Hashed, unlike `Ship::handling`, and the difference is the whole
    /// point.** Handling comes off the player's own disc and is the same on
    /// every machine that has that disc, so hashing it would make the gate
    /// depend on which ship was picked rather than on what the simulation did.
    /// A pilot can come out of `<config dir>/oag/pilots/`, which **differs
    /// between machines by design**. Left out, two machines running "the same
    /// race" with different `winston.toml` files would produce identical hashes
    /// for different races - a gate claiming an agreement it does not have,
    /// which is worse than no gate at all. In it, they visibly disagree.
    ///
    /// A digest of the **resolved numbers, not of the name**: two files both
    /// called `winston` that say different things must not agree, and that is
    /// exactly the case this exists to catch.
    ///
    /// `0` for [`Driver::default`], and no built-in pilot digests to zero, so a
    /// plain line-follower stays distinguishable from a balanced one.
    ///
    /// **It says "these differ", not *which* file differs.** Thirty-two bits
    /// cannot be inverted, so the composition root logs every roster entry's
    /// name and digest at startup; without that a divergence is merely
    /// mysterious instead of silent, which is an improvement but not the whole
    /// of one.
    pub pilot: u32,
    /// Ticks left of a braking point this driver is in the middle of missing.
    ///
    /// **The whole of mistake injection, and the cheap half is the only half
    /// there is.** `docs/gameplay/ai.md` calls recovery the expensive part of a
    /// mistake, on the grounds that a driver which runs wide and then carries on
    /// as though nothing happened is noise rather than an error. That is true of
    /// a controller that has to be *told* how to recover - and this one does not:
    /// it follows a line, so the recovery is the thing it was already doing. A
    /// mistake here is a driver holding the throttle through a corner it should
    /// have braked for; running wide and scrabbling back is then automatic.
    ///
    /// An integer, because this type is `Eq` and lives in the world snapshot,
    /// for the reason [`Self::provocation`] gives.
    pub mistake: u16,
    /// The highest [`pace::corner_target`] curvature seen since the line last
    /// went straight, as `f32::to_bits`.
    ///
    /// **What tells [`pace::trail`] the corner is opening up rather than still
    /// being entered**, which `speed < target` used to and, measured on Talon's
    /// Junction, does not - see that function's own doc for the two real-track
    /// families this replaced it for. Bits rather than the float itself, for
    /// the reason [`Self::provocation`] gives; `0` decodes to `0.0`, which is
    /// the value a straight resets it to, so [`Driver::default`] needs nothing
    /// special.
    pub peak_curvature: u32,
    /// What this driver has noticed of the craft around it, and what it is
    /// still in the middle of noticing.
    ///
    /// Reaction latency, and the one piece of driver state that is about
    /// *perception* rather than about driving - see [`Reflex`]. Integers, for
    /// the reason [`Self::provocation`] gives; `Reflex::IDLE` for a driver that
    /// has not ticked yet, which is not the same as one that has looked and
    /// seen nobody.
    pub reflex: Reflex,
    /// Whether this driver has already made its mind up about a barrel roll
    /// during the flight it is in.
    ///
    /// **One decision per airborne window, which is what makes
    /// [`Personality::roll_chance`] a probability rather than a rate.** Rolled
    /// every tick it would be neither: a chance of a tenth would fire in the
    /// first second of any long jump, and the axis would do far more than its
    /// number says.
    ///
    /// Set on the first tick past [`Personality::roll_airtime`] whichever way
    /// the decision went - a driver that decided *not* to roll this jump does
    /// not get to reconsider - and cleared the moment the craft is on the
    /// ground again. A `bool` because this type is `Eq` and lives in the world
    /// snapshot, for the reason [`Self::provocation`] gives.
    pub roll_decided: bool,
    /// Which line it is on and what it decided at the last fork: [`crate::branch`].
    pub branching: crate::branch::Branching,
}

/// How much of the line either side of the last index a driver looks at.
///
/// Wide enough that a craft knocked sideways by a collision still finds itself,
/// narrow enough that the search cannot cross to a stacked section. Eight craft
/// pay it every tick, so it is also the cost that matters.
const SEARCH_WINDOW: usize = 48;

/// The most provocation a driver can be carrying, in ticks - ten seconds.
///
/// A ceiling rather than an accumulator without one: a craft having a bad race
/// would otherwise bank enough grievance to spend the rest of it at maximum.
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

/// And the one that picks which way it goes. A stream of its own rather than a second use of [`ROLL_STREAM`]: sharing it
/// would tie which way a driver rolls to how readily it rolls at all, so a
/// pilot with a high [`Personality::roll_chance`] would only ever roll one way.
const ROLL_SIDE_STREAM: u32 = 5;

/// How often a driver at full `trigger` will fire once it has a target, per
/// tick.
///
/// **This is a rate, not a probability.** Rolled every tick, so at `1.0` the
/// median wait once a target is in the cone is about thirteen ticks - a fifth
/// of a second - and at `0.2` about a second and a half. Read as "a five per
/// cent chance" it looks far too small; read as a delay it is what a driver
/// taking a moment to line up looks like.
const TRIGGER_RATE: f32 = 0.05;

/// How much corridor room a driver wants on the side it is shifting toward,
/// measured **from where the craft is** rather than from the line.
///
/// A ram that puts the rammer into the wall is not aggression, it is a bug with
/// a personality - and this was `3.0`, less than half of the **median 8.2
/// units** a shift is measured to cover on `16_Track`, so the gate did not mean
/// what it said. Twelve *and* the craft's own room took the rammer ending up
/// outside the corridor from 54 of 208 to 11 of 138; neither half does much
/// alone. Sweep, method and the two remainders are in `docs/gameplay/ai.md`,
/// guarded by `crates/game/tests/ram_ground_truth.rs`.
///
/// **Ours, and measured against one hull.** The reach scales with
/// `handling.airbrake.sideshift / mass` - `450` against `1` on Pulse - which
/// this crate cannot see: `Context` carries no handling.
const RAM_CLEARANCE: f32 = 12.0;

/// The turn, in radians across the stretch between a craft and its aim point,
/// at which [`Personality::inside`] is asking for all the room it is allowed.
///
/// `Line::bend` returns the turn itself, which is a tenth of a radian on the
/// sort of corner a craft takes at speed - so without a scale here an `inside`
/// of one would be worth a tenth of the corridor and the axis would read as
/// broken rather than as subtle. Ours, and a feel knob: about nine degrees.
const FULL_BEND: f32 = 0.15;

impl Driver {
    /// A driver with a character of its own, from a seed.
    ///
    /// See [`Personality::from_seed`] for what the seed decides, and for why
    /// zero is the plain line-follower.
    #[must_use]
    pub fn seeded(seed: u32) -> Self {
        Self {
            seed,
            ..Self::default()
        }
    }

    /// The driver for one slot of a race.
    ///
    /// **The race's own seed and the craft's slot, and nothing else**, so the
    /// field has the same eight characters every time that race is replayed and
    /// a different eight in the next race. Slot zero is the player's and gets
    /// the plain line-follower, which costs nothing and means a caller that
    /// seeds the whole array cannot accidentally give the player a personality
    /// nothing reads.
    #[must_use]
    pub fn for_slot(race_seed: u64, slot: u32) -> Self {
        if slot == 0 {
            return Self::default();
        }
        // Multiply the slot into the high bits before mixing, so slot 1 and slot
        // 2 of the same race are not neighbouring seeds.
        let mixed = race_seed ^ u64::from(slot).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let seed = Rng::new(mixed).next_u32();
        // Zero is the plain driver, and a one-in-four-billion race should not
        // quietly field one.
        Self::seeded(if seed == 0 { 1 } else { seed })
    }

    /// This driver's character. Derived, not stored - see [`Self::seed`].
    #[must_use]
    pub fn personality(&self, pilot: &Pilot) -> Personality {
        Personality::from_pilot_seed(pilot, self.seed)
    }

    /// Chooses this tick's controls, and advances the driver's place on the line.
    ///
    /// Returns released controls when there is no line to follow: a track whose
    /// spline produced nothing leaves the opponents sitting on the grid, which is
    /// what this engine did before there was an AI at all.
    pub fn drive(&mut self, state: &ShipState, ctx: &Context<'_>) -> ShipControls {
        let (line, tuning) = (ctx.line, ctx.tuning);
        if line.is_empty() {
            return ShipControls::default();
        }

        // **What this driver has noticed, not what the caller measured.** At
        // the default zero latency the two are the same field; above it a craft
        // that has just arrived is not in this one yet. Advanced before it is
        // read, so a wait that runs out this tick is acted on this tick.
        self.reflex.advance(ctx.field, tuning.reaction_ticks);
        let noticed = self.reflex.filter(ctx.field);
        let ctx = &Context {
            field: &noticed,
            ..*ctx
        };

        let personality = self.personality(ctx.pilot);
        // On a plan, a driver's line is a handicap: see `Personality::spent`
        // and `planned::plan_slack`. Off one, untouched - not even multiplied
        // by one, which would move the corner model's bits.
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

        // Forward speed rather than speed: a craft sliding sideways at 60 is not
        // approaching its corner at 60, and the lookahead is about how far away
        // the corner is in time.
        let speed = body.linear_velocity.dot(forward).max(0.0);
        let look =
            (tuning.look_min + tuning.look_speed * speed).min(tuning.look_max) * personality.look;

        let steer = self.steering(state, ctx, look, speed, &personality);
        let window = look * tuning.brake_lookahead * personality.patience;
        let step = curvature_span(tuning, look);
        let curvature =
            line.max_curvature_stepped(index, window, pace::curvature_chord(tuning, step), step);
        // **The speed plan, when the race built one for this line**, and the
        // corner model otherwise. The plan is what our own physics showed the
        // craft can carry, braking zones included (`crate::plan`); a level and
        // a pilot handicap it by [`plan_margin`] rather than by believing in
        // less grip, so an Ace drives the plan and a Novice drives under it.
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
            // **No lateral grip to spend a brake on.** `throttle`'s own doc
            // says what the symmetric brake buys: cornering grip, traded for
            // deceleration. Off the ground there is no cornering grip at
            // all, so the same command that is a sensible trade on the
            // track is a pure loss of the forward speed a landing needs.
            // Found via a unit test with no craft or track at all -
            // `time_airborne` was previously read nowhere in this function,
            // so a grounded and an airborne fixture produced byte-identical
            // controls on an ordinary corner - not by observing this on any
            // one circuit; measured afterward not to be the cause of
            // `13_Track`'s own Novice jump pathology, which this branch does
            // not change. Chosen, not measured, no confidence score: this is
            // our own driver's behaviour, not a recovered one - see
            // `docs/gameplay/ai.md#the-jump-clearing-failure-the-mechanism-a-real-bug-that-turned-out-not-to-be-it-and-why-this-is-where-the-chase-stops`.
            (1.0, 0.0)
        } else if followed.is_some() {
            // The plan's own follower, the law it was learned with.
            crate::plan::longitudinal(speed, target)
        } else {
            throttle(speed, target, tuning)
        };
        // **No lift on the run-up to a gap**: the clearing speed is the one
        // thing a craft cannot buy back in the air. Chosen, not measured.
        let thrust = if line.is_takeoff(index) {
            thrust
        } else {
            thrust * self.caution(ctx, &personality)
        };
        // Neither timer is a **momentary** hit flag: both are seconds still
        // running, so a craft is "recovering" for as long as either does,
        // which is exactly the window the differential buys nothing in - see
        // `trail`'s own doc.
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
            // Every tick, not only the ones a roll is asked for: it also gates
            // the *gesture*, which this driver's own steering can complete by
            // accident. See `oag_physics::barrel_roll::within_budget`.
            roll_shield_floor: personality.roll_floor,
            ..ShipControls::default()
        }
    }

    /// Whether the line for `distance` ahead of this driver allows `speed`.
    ///
    /// The same cornering limit [`throttle`] brakes against, asked as a
    /// question about a speed the craft does not have yet. **A boost is what
    /// wants to know.** The driver's own braking horizon is built from the speed
    /// it is doing now, so a craft that is about to double its speed outruns
    /// what it looked at: on a real track a Turbo fired at 126 puts a craft
    /// through the next corner at 270 having only ever checked 160 units ahead,
    /// and that is a craft in the scenery. Measured, on `16_Track` - see
    /// `docs/gameplay/ai.md`.
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
    /// Pure pursuit gives the curvature of the arc from the craft to a point on
    /// the line: `k = 2 * e / L^2`, where `e` is how far off the craft's nose the
    /// aim point sits and `L` is the distance to it. That curvature times the
    /// craft's speed is the yaw rate needed to get there, and the loop is closed
    /// on the difference between that and the rate the craft has.
    ///
    /// **`L` is the measured distance to the aim point, not the requested
    /// lookahead.** They differ on a curve, and `k` divides by its square, so
    /// using the request scales the whole command wrong exactly where the corner
    /// is.
    ///
    /// Cross-track error needs no term of its own: the aim point is on the line
    /// and the craft is not, so the vector between them already carries it. The
    /// first version added a second, separate cross-track term on top, which
    /// double-counted the same error.
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
        // **The drift moves the aim point, not the command.** Noise added to the
        // steering output is noise inside a rate loop with a gain of five and
        // nothing filtering it; noise on the point being aimed at is a driver
        // choosing a slightly different line, which is what a driver who is not
        // a machine actually does. The controller stays the one
        // `tests/closed_loop.rs` measured.
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

        // Positive yaw about the craft's own up axis turns it **left** - the
        // right-hand rule, with forward on `-Z`. Working in "rate of turning to
        // the right" keeps every sign here the same as the steering input's.
        // The craft's own up, never world up, or this is wrong the moment the
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
    /// **Rolled only where it would otherwise brake.** A mistake on a straight
    /// is not a mistake, it is nothing at all - the throttle was already open -
    /// so rolling everywhere would spend the whole rate on ticks where it
    /// cannot show, and the setting would do far less than its number suggests.
    /// Gating it on the braking point also makes the rate legible: it is per
    /// tick *of braking*, so roughly once a second spent slowing down.
    fn blunder(&mut self, tuning: &Tuning, speed: f32, target: f32) {
        if self.mistake > 0 {
            self.mistake -= 1;
            return;
        }
        // **Seed zero never errs**, for the same reason it has no personality:
        // `Driver::default()` is the plain line-follower every exact assertion
        // in this crate and in `oag-trace`'s replay path is written against,
        // and a driver that occasionally sails through a corner is not one of
        // those. See `Personality::from_seed`.
        if self.seed == 0 || tuning.mistake_rate <= 0.0 || speed <= target {
            return;
        }
        if roll(self.seed, self.phase, MISTAKE_STREAM) < tuning.mistake_rate {
            self.mistake = MISTAKE_TICKS;
        }
    }

    /// Advances the grudge: notices being overtaken, and lets it cool.
    ///
    /// **A place that got worse is a craft that was just passed**, which is the
    /// whole of the detector. Both places have to be real: `0` means nothing
    /// has placed this craft yet, and treating that as first would provoke the
    /// entire grid on the tick the standings first resolve.
    ///
    /// What provocation does **not** touch is
    /// [`Personality::commitment`]. It is the obvious thing to raise and the
    /// wrong one: that axis is documented as the one where over what the hull
    /// can hold is a driver in the wall, and an angry AI that drives into the
    /// scenery reads as a bug rather than as character. It scales what a driver
    /// does to *other craft*, not what it asks of its own.
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

    /// How much of its thrust this driver keeps when it is closing on a craft
    /// in front, `0.0..=1.0`.
    ///
    /// **A lift, never a brake.** Braking hard mid-corner for a craft ahead
    /// spends the grip that was holding the corner, so the cure would put the
    /// craft in the scenery rather than into the back of a rival. Lifting off
    /// is what a driver actually does, and the speed target still owns the
    /// braking.
    ///
    /// This axis exists because [`Self::social`] and
    /// [`Personality::inside`] both sometimes move craft *toward* each other,
    /// and nothing else in this controller reacts to a closing gap at all.
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
    /// craft behind, in the same fraction-of-the-room units as everything else
    /// in [`Self::drift`].
    ///
    /// **One term rather than two, and that is the design.** Courtesy moves
    /// away from the side a rival is on and defence moves toward it; as two
    /// separately gated terms they fight each other and the craft jitters
    /// between them. As two signs of one number a pilot simply sits somewhere
    /// on the axis, and `defence - courtesy` is where.
    ///
    /// Continuous in the gap, so it needs no slew limiter and no state on the
    /// driver: it fades in as a rival closes and fades out as it drops away.
    ///
    /// # A touching rival is `alongside`, not `behind`, and this used to go blind there
    ///
    /// **Found from play, 2026-09-07: two craft that are actually touching
    /// produced *no* lateral response from either driver at all**, which reads
    /// as sticking - `oag_physics::pair::resolve` pushes them apart by its own
    /// small positional correction, and both drivers' steering just aims back
    /// at the same corridor point a tick later, fighting it. The cause: this
    /// function read only [`Field::behind`], and `crates/game`'s own
    /// `field_for` classifies a rival as [`Field::alongside`] - a **different**
    /// channel this function never read - the moment it is within
    /// `ALONGSIDE_GAP` (16 units of track distance) and `ALONGSIDE_WIDTH` (12
    /// lateral). A real hull touch (`oag_physics::pair::overlap`, measured
    /// around 3-6 units on a realistic hull) is always well inside both, so a
    /// rival close enough to actually be touching had *already left* the one
    /// bucket this term reacted to, and the yield/cover lean it was building
    /// as the rival closed **dropped to exactly zero at the moment contact
    /// started** - confirmed directly: `social` returned the same value for a
    /// touching alongside rival as for `Field::EMPTY`.
    ///
    /// **The fix is to read `alongside` too, preferring it over `behind`** -
    /// an alongside rival is the more urgent one whenever both are present,
    /// since it is the one already in or entering contact. This is not a
    /// widened *range*: `field_for` builds a [`Rival`] identically for every
    /// channel (`gap`, `offset` and `closing` are the same computation, only
    /// the bucket differs), and this function already takes `gap.abs()`, so
    /// nothing here has to change to accept the new source. **No new
    /// constant is introduced**, and none of `AWARENESS_RANGE`,
    /// `SOCIAL_MIN_GAP`, `ASTERN_DEADBAND`, `CLOSING_FULL`, `SOCIAL_MAX` or
    /// `YIELD_MAX` moved.
    ///
    /// `ALONGSIDE_GAP` (16) is a little *past* `SOCIAL_MIN_GAP` (14), so an
    /// alongside rival between 14 and 16 units away is not automatically
    /// forced to yield-only by gap alone - blocking can still fire there.
    /// That is not a hole this change opens: a real hull touch
    /// (`oag_physics::pair::overlap`, around 3-6 units on a realistic hull)
    /// is well inside `SOCIAL_MIN_GAP` regardless, so blocking still cannot
    /// fire at actual contact - the only place it can fire under `alongside`
    /// is the few units between 14 and 16, where nothing is touching yet,
    /// which is exactly "a block is something you do early" already
    /// documented below, just reached from a different channel. So this is
    /// **chosen, not measured** only in the sense that reusing these
    /// constants for a second channel is a judgement call, not a fit - no
    /// confidence score, because there is nothing recovered to score
    /// against: the original has no such term at all (this driver's social
    /// behaviour is `oag_ai`'s own, not a ported one).
    ///
    /// Verified against `crates/game`'s own probe methodology (an env-gated
    /// instrument on `resolve_craft_pairs`, run and reverted): before this
    /// change, a real 8-craft `VENOM` race's pair `(4,7)` resolved on **90**
    /// consecutive ticks with `vn_before` negative on all 90; after, the
    /// longest streak and the count of streaks past ten ticks are the
    /// acceptance test - see the ground-truth test this change adds.
    ///
    /// Three things it will not do:
    ///
    /// - **Take over the whole line.** Capped at [`SOCIAL_MAX`], because
    ///   provocation can push `defence` past the entire aim budget on its own -
    ///   see that constant for what that looked like from the cockpit.
    /// - **Swerve into somebody already there.** Below [`SOCIAL_MIN_GAP`] it
    ///   stops: a block is something you do early.
    /// - **Act mid-corner.** Both are straight-line manoeuvres; where the
    ///   corridor is a couple of metres wide and both craft are at the grip
    ///   limit, moving sideways on purpose is how two craft end up in the
    ///   scenery. The gate is the same normalised `bend` the inside line uses.
    /// - **React equally to a craft holding station and one closing.** Closing
    ///   earns the rest of the lean on top of [`PRESENCE_SHARE`]; proximity
    ///   alone is what triggers it, because a craft sitting on another's tail
    ///   at matched pace is exactly when yielding matters.
    /// - **Guess a side from noise.** A rival directly astern has an offset of
    ///   about zero and its *sign* is meaningless, so inside a deadband the
    ///   side comes from the corner ahead instead - yielding toward its
    ///   outside, which is where an overtaker does not want to be.
    fn social(&self, ctx: &Context<'_>, personality: &Personality, bend: f32) -> f32 {
        let touching = ctx.field.alongside;
        let Some(rival) = touching.or(ctx.field.behind) else {
            return 0.0;
        };
        // **Provocation scales covering, not yielding.** A driver that has just
        // been passed defends harder; it does not become more polite. Applied
        // to the defence side alone so a shy pilot stays shy however cross it
        // is - see `Self::stew`.
        // Clamped *before* the scale below, so provocation raises the lean
        // until it is total and then stops, rather than running past the budget
        // the aim point is summed into.
        let lean =
            (personality.defence * (1.0 + self.provoked()) - personality.courtesy).clamp(-1.0, 1.0);

        let gap = rival.gap.abs();
        if gap >= AWARENESS_RANGE {
            return 0.0;
        }
        // Inside the close range only the yielding half survives - see
        // [`SOCIAL_MIN_GAP`] - **and below that, everybody yields at least
        // [`CONTACT_FLOOR`]**, whatever their personality says. See that
        // constant.
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
        // `bend` is already normalised by `FULL_BEND` and clamped, so this is
        // one at a corner worth the name and zero on a straight.
        let corner_gate = 1.0 - bend.abs().min(1.0);

        // A rival dead astern gives a sign that is rounding noise. Fall back to
        // the corner: its outside is the side an overtaker least wants.
        let side = if rival.offset.abs() > ASTERN_DEADBAND {
            rival.offset.signum()
        } else if bend.abs() > f32::EPSILON {
            -bend.signum()
        } else {
            return 0.0;
        };

        // **Two budgets, because the two halves fail differently.** Blocking is
        // the half that swerves into somebody and is kept on a tight rein;
        // yielding cannot, and gets a looser one - but not the whole corridor,
        // or a craft hands over the entire track and reads as having given up.
        let budget = if lean > 0.0 { SOCIAL_MAX } else { YIELD_MAX };
        lean * side * pressure * corner_gate * budget
    }

    /// How far off the authored line this driver is aiming, as a world-space
    /// vector across it.
    ///
    /// A constant lean plus a slow drift, both measured as a fraction of the
    /// room the corridor gives **on the side being leant toward** - the two
    /// sides are not the same width, and on a real track they are often nothing
    /// like it. Scaled by [`Tuning::corridor_use`] so the corridor's own edge
    /// stays a backstop, and clamped against it anyway.
    ///
    /// Zero when the line carries no corridor. There is nothing to be off the
    /// line *by* - no lateral axis and no bound - and guessing one from the
    /// line's own shape and world up is exactly the mistake `docs/formats/track.md`
    /// records: it is wrong the moment the track rolls. A line with no corridor
    /// is a synthetic one, and every craft on it drives it exactly.
    fn drift(&self, aim: &Aim, look: f32, ctx: &Context<'_>, personality: &Personality) -> Vec3 {
        let Some(frame) = aim.corridor else {
            return Vec3::ZERO;
        };
        let phase = self.phase as f32 * personality.wander_rate;
        // The seed is the driver's, so two craft with the same wander amplitude
        // still wander apart.
        let wobbled = personality.wander * wobble(self.seed, phase);
        // **Which way the corner goes, not how hard.** A fixed lean is a side
        // of the line; an inside line swaps sides with the corner, so this is
        // the one axis that has to read the geometry rather than just the
        // craft. It fades out on a straight, where `bend` goes to zero.
        // **From the craft's own index, not the aim point's.** The aim point is
        // already `look` downtrack, and `bend` walks three spans past whatever
        // it is given, so measuring from there would bias toward a corner two
        // lookaheads away rather than the one being entered. A third of the
        // lookahead per span puts the three samples between the craft and its
        // aim point, which is the stretch this offset is steering through.
        let bend = (ctx
            .line
            .bend(self.index as usize, look / 3.0, frame.lateral)
            / FULL_BEND)
            .clamp(-1.0, 1.0);
        let inside = personality.inside * bend;
        let social = self.social(ctx, personality, bend);
        // **Summed with the rest rather than overriding them**, so a driver
        // dodging a mine is still driving a line. It has the largest budget of
        // the terms here and it still has to win the argument; see
        // [`Driver::avoidance`].
        let avoidance = self.avoidance(ctx.field.hazard);
        // On a plan the bias, wander and inside line arrive already spent -
        // see `Personality::spent`. Avoidance and the social terms - yielding,
        // blocking, the contact floor - are about other craft, not character,
        // and are never scaled: a lone craft has nobody to answer.
        let wanted =
            (personality.line_bias + wobbled + inside + social + avoidance).clamp(-1.0, 1.0);
        // The terms above are fractions of the room, clamped once here; the pad
        // pull is in units, and the corridor clamp backstops both.
        let offset = wanted * frame.room(wanted) * ctx.tuning.corridor_use * personality.width;
        let offset = Self::toward_pad(offset, ctx.field.pad, avoidance != 0.0);
        frame.lateral * frame.clamp(offset)
    }
}

/// How far a rival has to be behind before it stops being this driver's
/// problem, in units along the track.
///
/// Ours. Wide enough that a craft closing at a real speed difference is seen
/// before it arrives, narrow enough that a driver is not defending against
/// somebody a corner away.
pub const AWARENESS_RANGE: f32 = 120.0;

/// The closing speed, in units a second, at which a rival is taken as closing
/// as hard as it is going to.
///
/// Small on purpose. Two craft racing each other differ by a few units a
/// second, not by tens, so a threshold set at "obviously catching up" leaves
/// the term reading as zero for the whole of a real battle.
const CLOSING_FULL: f32 = 8.0;

/// How much of the social lean a rival gets for merely being close, before any
/// of it is earned by closing.
///
/// **Not zero, and that is the point.** A craft sitting on another's tail at
/// matched pace is exactly when yielding matters, and a term gated purely on
/// closing speed would do nothing there - a shy driver would only move over for
/// someone who was going to get past anyway.
const PRESENCE_SHARE: f32 = 0.5;

/// How far off-centre a rival has to sit before the *sign* of its offset means
/// anything. Ours, and about a hull's width.
const ASTERN_DEADBAND: f32 = 2.0;

/// How much of the aim budget a fully committed **block** is worth.
///
/// Yielding is not scaled by it - see the use site.
///
/// **A scale on the term, not a clamp over it.** A clamp was tried first and
/// swallowed provocation whole: an aggressive pilot sits near the top of the
/// lean range already, so clamping made a provoked driver and a calm one
/// identical. Scaling keeps the ordering - being passed still makes a driver
/// cover harder - while making it impossible for the term to own the line.
///
/// **Reported from play: opponents were turning almost ninety degrees into the
/// player to block, hitting them, and then hitting a wall.** The cause was that
/// `defence` is scaled by provocation, so an aggressive pilot that had just
/// been passed reached about 1.9 - nearly twice the whole `-1..1` budget the
/// aim point is clamped into. It therefore saturated that clamp on its own,
/// pinning the craft to the corridor edge on the rival's side and squeezing
/// every other term - the racing line's own bias, the inside line - out of the
/// sum entirely. Covering a line is worth part of the corridor; it is not worth
/// all of it, and it is certainly not worth the wall.
const SOCIAL_MAX: f32 = 0.45;

/// And how much a full yield is worth.
///
/// **More than a block and far less than the corridor.** Yielding cannot swerve
/// into anybody, so it does not need the block's tight rein - but a craft that
/// hands over the whole track is not being courteous, it is abandoning the
/// racing line, and it looks like it has given up rather than let somebody
/// through. What is wanted is a bit of room to be passed in, so this is a lean
/// rather than a move.
const YIELD_MAX: f32 = 0.55;

/// How close a rival behind has to get before **blocking** stops.
///
/// You cover a line *early*, while there is still room to do it smoothly. Past
/// this the rival is already at your gearbox, and moving across is no longer a
/// block - it is a swerve into somebody, which ends with both craft in the
/// scenery and the blocker further back than if it had done nothing.
///
/// **Yielding is deliberately not gated on it**, and the first version of this
/// gate was wrong for exactly that reason: it suppressed the whole term, and
/// since a packed grid is *permanently* inside fourteen units, craft stopped
/// getting out of each other's way at the only range where it matters. They
/// bumped instead, and the whole field lost about an eighth of its pace -
/// measured, 119 against 95 at elite. Moving away from somebody who is already
/// there is never the wrong thing to do.
const SOCIAL_MIN_GAP: f32 = 14.0;

/// The minimum yield every driver gives a rival it is **already alongside**,
/// whatever its personality says.
///
/// **Chosen, not measured. No confidence score** - the original has no social
/// term at all, so there is nothing to recover this from, and opponent
/// behaviour is a design axis on this project rather than a fidelity one.
///
/// # The hole this fills
///
/// [`Driver::social`]'s lean is `defence - courtesy`, and inside
/// [`SOCIAL_MIN_GAP`] only its yielding half survives. So **every driver whose
/// defence is at least its courtesy contributes exactly zero at contact** -
/// including the neutral baseline `Personality::from_seed(0)` gives, where both
/// are zero. Two such craft that touch have nothing steering either of them
/// apart: `oag_physics::pair::resolve` nudges them, both drivers aim straight
/// back at the same corridor point, and they grind along together. That is the
/// residue `craft_sticking_ground_truth`'s own header records as left over
/// after the `alongside` fix - the fix made a touching rival *visible* to this
/// term; this makes the term *say something* when it sees one.
///
/// Small on purpose. It is a floor, not a policy: an aggressive pilot still
/// covers, a shy one still yields harder, and all this does is stop the
/// aggressive one contributing nothing at the one range where contact is
/// already happening. Scaled by [`YIELD_MAX`], pressure and the corner gate
/// like any other yield, so it fades out mid-corner rather than steering two
/// craft off a bend they are both at the grip limit on.
const CONTACT_FLOOR: f32 = 0.25;

/// How close a craft ahead has to be before a driver lifts for it, in units
/// along the track. Shorter than [`AWARENESS_RANGE`]: a craft two seconds up
/// the road is not something to lift for.
const CAUTION_RANGE: f32 = 45.0;

/// The steering command, and the turn-rate error it was computed from.
///
/// **The error travels with the command because [`trail`] has to be a function
/// of the same signal the rate loop closed on.** Feeding it `command` instead
/// would be a path from the loop's own output back into the plant it drives,
/// which is a second loop around the first - the shape this crate was rewritten
/// to remove.
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
