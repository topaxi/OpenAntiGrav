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

use crate::line::{Frame, Line};
use crate::noise::wobble;

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
    pub max_turn_rate: f32,
    /// The lateral acceleration a craft is assumed to hold through a corner.
    ///
    /// Sets the speed target: on a corner of curvature `k` the target is
    /// `sqrt(lateral_accel / k)`, the standard cornering limit. Raising it makes
    /// a driver commit harder and, past what the hull can hold, into the wall.
    ///
    /// **Deliberately conservative**, because cornering here is steering and a
    /// speed target and nothing else. The original's own `<Controller>` has no
    /// airbrake term either - see the note on [`Tuning`] - which is consistent
    /// with that and is *not* evidence for it.
    pub lateral_accel: f32,
    /// How far ahead the speed target looks for the sharpest bend, as a multiple
    /// of the lookahead. A corner has to be seen before it is entered.
    pub brake_lookahead: f32,
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
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            look_min: 20.0,
            look_speed: 0.35,
            look_max: 90.0,
            rate_gain: 5.0,
            max_turn_rate: 1.2,
            lateral_accel: 55.0,
            brake_lookahead: 2.5,
            brake_margin: 0.05,
            corridor_use: 0.6,
            brake_floor: 0.35,
            brake_gain: 2.0,
            trail_deadband: 0.15,
            trail_gain: 1.2,
            trail_max: 0.6,
            trail_saturation: 0.85,
        }
    }
}

/// What makes one driver drive unlike the next.
///
/// **Every field is a departure from the [`Tuning`] the whole field shares**,
/// not a replacement for it: a multiplier of one or a bias of zero gives back
/// exactly the driver that was here before this existed. That is deliberate -
/// the shared tuning is what `tests/closed_loop.rs` measured and it stays the
/// centre of the distribution, so the field is spread around a controller that
/// is known to be stable rather than around eight untested ones.
///
/// It is derived from a seed rather than stored, so a driver stays three
/// `u32`s and a replay reproduces it without carrying it. See
/// [`Personality::from_seed`].
///
/// This is the first piece of the skill vector `docs/gameplay/ai.md` describes.
/// Three of its axes are here - line noise, grip used through a corner, how
/// early the braking is - and the ones that are not are named in
/// [what is missing](crate#what-a-personality-does-not-cover).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Personality {
    /// Which side of the line this driver sits on and how far, as a fraction of
    /// the room the corridor gives on that side. Negative is left.
    ///
    /// This is the one that breaks the queue. Eight craft tracking one line to
    /// the centimetre drive nose to tail because there is nowhere else for them
    /// to be; eight craft each holding a different part of the corridor look
    /// like a field.
    pub line_bias: f32,
    /// How much of the corridor this driver spends drifting about its bias, in
    /// the same units. Zero is a driver on rails.
    pub wander: f32,
    /// How fast that drift moves, in noise steps per tick. Small: a wander that
    /// completes in under a couple of seconds reads as a twitch rather than as
    /// a driver.
    pub wander_rate: f32,
    /// Multiplier on the lookahead. Above one is a driver who aims further down
    /// the road, cuts more corner and is smoother; below one is one who chases
    /// the line and looks busier doing it.
    pub look: f32,
    /// Multiplier on [`Tuning::lateral_accel`], so on the speed a corner is
    /// taken at.
    ///
    /// **This is what strings the field out.** Cornering speed goes as the
    /// square root of it, so a spread of a few per cent here is a spread of a
    /// couple of per cent in corner speed - small on one corner, a gap by the
    /// end of a lap. It is also the axis that must not be generous: over what
    /// the hull can hold is not a faster driver, it is a driver in the wall.
    pub commitment: f32,
    /// Multiplier on [`Tuning::brake_lookahead`] - how far ahead this driver
    /// starts worrying about a corner. Below one is a late braker.
    pub patience: f32,
}

impl Personality {
    /// The driver that was here before personalities were: shares the field's
    /// tuning exactly and drives the authored line.
    pub const NEUTRAL: Self = Self {
        line_bias: 0.0,
        wander: 0.0,
        wander_rate: 0.0,
        look: 1.0,
        commitment: 1.0,
        patience: 1.0,
    };

    /// Derives a personality from a seed.
    ///
    /// **Seed zero is [`Self::NEUTRAL`], as a special case rather than by
    /// accident**: `Rng::new(0)` remaps a zero seed, so it would otherwise draw
    /// an ordinary personality and every test written against the shared tuning
    /// would start measuring a random driver instead. It is also what makes
    /// [`Driver::default`] the plain line-follower it has always been.
    ///
    /// The draws are in a fixed order off [`oag_core::Rng`], so this is a pure
    /// function of the seed on every platform: no clock, no OS entropy, and no
    /// draw from the world's own generator - a personality taken from that
    /// stream would move every later pickup roll, and which craft has which
    /// character would depend on how many pickups had been drawn.
    #[must_use]
    pub fn from_seed(seed: u32) -> Self {
        if seed == 0 {
            return Self::NEUTRAL;
        }
        let mut rng = Rng::new(u64::from(seed));
        // Sequential lets rather than a closure in a struct literal, because the
        // *order* of the draws is what makes this reproducible and a struct
        // literal's field order is a thing a later edit moves without thinking.
        // Two craft that drew the same values in a different order are two
        // different craft.

        // Both signs, and rarely near zero: the middle of the corridor is where
        // everyone would be anyway.
        let magnitude = spread(&mut rng, 0.25, 0.85);
        let line_bias = if rng.next_f32() < 0.5 {
            -magnitude
        } else {
            magnitude
        };
        let wander = spread(&mut rng, 0.10, 0.30);
        // Two and a half to seven seconds a step at 60 Hz. Slower than a corner,
        // so the drift is something a driver *is* rather than something that
        // happens to it mid-bend.
        let wander_rate = 1.0 / spread(&mut rng, 150.0, 420.0);
        let look = spread(&mut rng, 0.85, 1.15);
        let commitment = spread(&mut rng, 0.93, 1.05);
        let patience = spread(&mut rng, 0.85, 1.20);

        Self {
            line_bias,
            wander,
            wander_rate,
            look,
            commitment,
            patience,
        }
    }
}

impl Default for Personality {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

/// One draw, scaled into `low..high`.
fn spread(rng: &mut Rng, low: f32, high: f32) -> f32 {
    low + (high - low) * rng.next_f32()
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
}

/// How much of the line either side of the last index a driver looks at.
///
/// Wide enough that a craft knocked sideways by a collision still finds itself,
/// narrow enough that the search cannot cross to a stacked section. Eight craft
/// pay it every tick, so it is also the cost that matters.
const SEARCH_WINDOW: usize = 48;

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
    pub fn personality(&self) -> Personality {
        Personality::from_seed(self.seed)
    }

    /// Chooses this tick's controls, and advances the driver's place on the line.
    ///
    /// Returns released controls when there is no line to follow: a track whose
    /// spline produced nothing leaves the opponents sitting on the grid, which is
    /// what this engine did before there was an AI at all.
    pub fn drive(&mut self, state: &ShipState, line: &Line, tuning: &Tuning) -> ShipControls {
        if line.is_empty() {
            return ShipControls::default();
        }

        let personality = self.personality();
        let body = &state.body;
        let forward = body.forward();

        let index = line.nearest(body.position, self.index as usize, SEARCH_WINDOW);
        self.index = index as u32;
        // Only ticks this driver actually drove. See [`Self::phase`].
        self.phase = self.phase.wrapping_add(1);

        // Forward speed rather than speed: a craft sliding sideways at 60 is not
        // approaching its corner at 60, and the lookahead is about how far away
        // the corner is in time.
        let speed = body.linear_velocity.dot(forward).max(0.0);
        let look =
            (tuning.look_min + tuning.look_speed * speed).min(tuning.look_max) * personality.look;

        let steer = self.steering(state, line, look, speed, tuning, &personality);
        let window = look * tuning.brake_lookahead * personality.patience;
        let curvature = line.max_curvature(index, window, look * 0.5);
        let target = corner_target(curvature, tuning, &personality);
        let (thrust, brake) = throttle(speed, target, tuning);
        let differential = trail(&steer, speed, target, tuning);
        let (airbrake_left, airbrake_right) = airbrakes(brake, differential, tuning.brake_floor);

        ShipControls {
            steer_x: steer.command,
            thrust,
            airbrake_left,
            airbrake_right,
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
    pub fn allows_speed(&self, line: &Line, tuning: &Tuning, speed: f32, distance: f32) -> bool {
        if line.is_empty() || distance <= 0.0 {
            return true;
        }
        let span = (tuning.look_min + tuning.look_speed * speed) * 0.5;
        let curvature = line.max_curvature(self.index as usize, distance, span);
        speed <= corner_target(curvature, tuning, &self.personality())
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
        line: &Line,
        look: f32,
        speed: f32,
        tuning: &Tuning,
        personality: &Personality,
    ) -> Steer {
        let body = &state.body;
        let aim = line.aim(self.index as usize, look);
        // **The drift moves the aim point, not the command.** Noise added to the
        // steering output is noise inside a rate loop with a gain of five and
        // nothing filtering it; noise on the point being aimed at is a driver
        // choosing a slightly different line, which is what a driver who is not
        // a machine actually does. The controller stays the one
        // `tests/closed_loop.rs` measured.
        let aim = aim.point + self.drift(aim.corridor, tuning, personality);
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
    fn drift(&self, corridor: Option<Frame>, tuning: &Tuning, personality: &Personality) -> Vec3 {
        let Some(frame) = corridor else {
            return Vec3::ZERO;
        };
        let phase = self.phase as f32 * personality.wander_rate;
        // The seed is the driver's, so two craft with the same wander amplitude
        // still wander apart.
        let wobbled = personality.wander * wobble(self.seed, phase);
        let wanted = (personality.line_bias + wobbled).clamp(-1.0, 1.0);
        let offset = wanted * frame.room(wanted) * tuning.corridor_use;
        frame.lateral * frame.clamp(offset)
    }
}

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

/// The fastest the corner ahead can be taken, or infinity where there is no
/// corner.
///
/// `sqrt(lateral_accel / curvature)` is the ordinary cornering limit.
/// [`Personality::commitment`] scales the grip a driver assumes it has, so the
/// eight targets are eight different numbers and the field does not lift off
/// and brake in unison. It is under the square root, so a spread of a few per
/// cent in commitment is half that in speed.
///
/// Infinity rather than an option, because every caller wants "is this speed
/// allowed", and `speed <= f32::INFINITY` is the right answer for a straight
/// without a branch of its own.
fn corner_target(curvature: f32, tuning: &Tuning, personality: &Personality) -> f32 {
    if curvature <= f32::EPSILON {
        return f32::INFINITY;
    }
    (tuning.lateral_accel * personality.commitment / curvature).sqrt()
}

/// Thrust, and the brake held on **both** sides, from the speed the corner
/// ahead allows.
///
/// Past [`Tuning::brake_margin`] the command climbs from
/// [`Tuning::brake_floor`] with the overspeed rather than snapping to one.
///
/// **This does not ramp the deceleration.** `oag_physics::controls::update`
/// gates `ShipState::brake` on both inputs being strictly positive and its ramp
/// rate never reads their level, so a command of `0.35` and a command of `1.0`
/// slow the craft at exactly the same rate. What climbs with the command is
/// `max(L, R)`, which is what the lateral-grip coefficient reads. So this is a
/// dial on **how much cornering grip the deceleration is bought with**, and a
/// driver only a little over its target keeps the grip it is about to need.
fn throttle(speed: f32, target: f32, tuning: &Tuning) -> (f32, f32) {
    if speed <= target {
        return (1.0, 0.0);
    }
    // Finite, because `speed <= target` already returned for an infinite one.
    let overspeed = speed / target - 1.0;
    if overspeed <= tuning.brake_margin {
        return (0.0, 0.0);
    }
    let brake = (tuning.brake_floor + (overspeed - tuning.brake_margin) * tuning.brake_gain)
        .clamp(tuning.brake_floor, 1.0);
    (0.0, brake)
}

/// The differential airbrake: how much harder one side is held than the other,
/// positive when the extra braking goes on the **right**.
///
/// # This is not a brake
///
/// Braking one side alone yaws the nose toward that side, pushes the body away
/// from it, adds a little forward speed, and cuts lateral grip exactly as hard
/// as holding both sides would - and it engages no deceleration at all, because
/// that needs both. Three of those four are the wrong sign for what "trail
/// braking" usually means. What it actually buys is **yaw authority, paid for
/// in grip**, which is why it is spent only where the steering loop has run out
/// of authority of its own.
///
/// Three gates, each doing a different job:
///
/// - **Saturation.** Below [`Tuning::trail_saturation`] of full lock the rate
///   loop still has authority, and a second path in parallel with it is the
///   oscillation this crate was rewritten to remove. Above it the loop is
///   asking for more than the steering input can deliver.
/// - **Deadband.** Keeps it out of the small-signal regime, so the loop
///   linearised about the line is provably the one `tests/closed_loop.rs`
///   measured.
/// - **Overspeed.** Never below the corner's target speed, which is corner
///   exit - where the grip is wanted for accelerating and where the forward
///   slide term is at its largest. On a straight `target` is infinite and this
///   gate is what keeps the differential off it.
///
/// No slew limiting here, and none needed: this is a *target*, and
/// `oag_physics::controls::update` ramps the airbrake states toward it at
/// `Airbrake::gain`/`falloff`. The plant is the rate limiter, so the driver
/// needs no state of its own to remember.
fn trail(steer: &Steer, speed: f32, target: f32, tuning: &Tuning) -> f32 {
    if speed < target
        || steer.command.abs() < tuning.trail_saturation
        || steer.rate_error.abs() <= tuning.trail_deadband
    {
        return 0.0;
    }
    let past = steer.rate_error.abs() - tuning.trail_deadband;
    let magnitude = (past * tuning.trail_gain).min(tuning.trail_max);
    // Positive `rate_error` is a craft that wants to turn further right, and a
    // nose-right yaw needs `imbalance = L - R` negative - so the **right** side
    // is the one braked. Getting this backwards is what `5ad69f3` shipped for
    // months; `the_differential_brakes_the_side_the_nose_is_turning_toward`
    // pins it, and `oag_physics::airbrake`'s own header settles the sign.
    if steer.rate_error >= 0.0 {
        magnitude
    } else {
        -magnitude
    }
}

/// The two airbrake commands, from the symmetric brake and the differential.
///
/// The two sides are an interval of width `differential` slid to sit as near
/// the symmetric brake as it will go, rather than the brake plus and minus half
/// of it. Two reasons, and both are about not losing the yaw where it is most
/// needed:
///
/// - **A craft braking flat out has no headroom above.** Adding to one side
///   alone would clip against `1.0` and deliver nothing, exactly in the corner
///   the driver is most in trouble in. Sliding the interval down instead keeps
///   the imbalance the caller asked for.
/// - **The low side must stay strictly positive whenever the brake is on**,
///   because both sides positive is the only thing that engages
///   `ShipState::brake`. Dropping one to zero mid-corner would silently cancel
///   the deceleration. `floor` is the limit it may slide to; below it the
///   differential is what shrinks, never the brake.
///
/// With the brake off, `floor` is zero and one side rises from nothing: yaw
/// authority and no deceleration, which is what a differential airbrake
/// physically is.
fn airbrakes(brake: f32, differential: f32, floor: f32) -> (f32, f32) {
    let limit = if brake > 0.0 { floor } else { 0.0 };
    let width = differential.abs().min(1.0 - limit);
    let low = brake.clamp(limit, 1.0 - width);
    let high = low + width;
    if differential >= 0.0 {
        (low, high)
    } else {
        (high, low)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::{Quat, Vec3};
    use oag_physics::Body;

    fn straight() -> Line {
        Line::new(
            (0..64)
                .map(|step| Vec3::new(0.0, 0.0, -10.0 * step as f32))
                .collect(),
        )
    }

    /// A craft at the origin pointing down `-Z`, which is [`Body::forward`].
    fn craft(position: Vec3, speed: f32) -> ShipState {
        ShipState {
            body: Body {
                position,
                orientation: Quat::IDENTITY,
                linear_velocity: Vec3::new(0.0, 0.0, -speed),
                ..Body::default()
            },
            ..ShipState::default()
        }
    }

    #[test]
    fn no_line_means_no_input() {
        let mut driver = Driver::default();
        let controls = driver.drive(
            &craft(Vec3::ZERO, 0.0),
            &Line::default(),
            &Tuning::default(),
        );
        assert_eq!(controls, ShipControls::default());
    }

    #[test]
    fn a_craft_on_a_straight_holds_full_thrust_and_no_steering() {
        let mut driver = Driver::default();
        let controls = driver.drive(&craft(Vec3::ZERO, 50.0), &straight(), &Tuning::default());
        assert_eq!(controls.thrust, 1.0);
        assert!(controls.steer_x.abs() < 1e-3, "steer {}", controls.steer_x);
        assert_eq!(controls.airbrake_left, 0.0);
        assert_eq!(controls.airbrake_right, 0.0);
    }

    /// The sign is the whole thing: a craft to the *right* of its line must steer
    /// *left*, and getting it backwards drives the field into the outside wall on
    /// the first corner - which is exactly the trap the grid's lateral offset
    /// already sprung once. See `docs/ghidra/functions/psp-pulse-usa/grid.md`.
    #[test]
    fn a_craft_beside_the_line_steers_back_toward_it() {
        let tuning = Tuning::default();

        let mut driver = Driver::default();
        let right_of_line =
            driver.drive(&craft(Vec3::new(6.0, 0.0, 0.0), 40.0), &straight(), &tuning);
        assert!(
            right_of_line.steer_x < 0.0,
            "steer {} should be left of centre",
            right_of_line.steer_x
        );

        let mut driver = Driver::default();
        let left_of_line = driver.drive(
            &craft(Vec3::new(-6.0, 0.0, 0.0), 40.0),
            &straight(),
            &tuning,
        );
        assert!(
            left_of_line.steer_x > 0.0,
            "steer {} should be right of centre",
            left_of_line.steer_x
        );
    }

    /// The damping, and the reason the rewrite happened: a craft already turning
    /// the way the geometry wants must be asked for *less* lock, not the same.
    #[test]
    fn a_craft_already_turning_is_asked_for_less_lock() {
        let tuning = Tuning::default();
        let mut state = craft(Vec3::new(6.0, 0.0, 0.0), 40.0);

        let mut driver = Driver::default();
        let still = driver.drive(&state, &straight(), &tuning);

        // Yawing left, which is positive about the craft's own up axis, and is
        // the direction the controller wants to go.
        state.body.angular_velocity = Vec3::new(0.0, 0.3, 0.0);
        let mut driver = Driver::default();
        let turning = driver.drive(&state, &straight(), &tuning);
        assert!(
            turning.steer_x > still.steer_x,
            "already turning: {} should be less left lock than {}",
            turning.steer_x,
            still.steer_x
        );
    }

    /// Braking uses both sides while the steering loop still has authority. A
    /// single airbrake yaws the craft, so tying one to the steering below
    /// saturation would close a second loop around it - see [`trail`].
    #[test]
    fn braking_is_symmetric_until_the_steering_loop_saturates() {
        let tuning = Tuning::default();
        let line = Line::new(
            (0..128)
                .map(|step| {
                    let angle = std::f32::consts::TAU * step as f32 / 128.0;
                    Vec3::new(60.0 * angle.cos(), 0.0, 60.0 * angle.sin())
                })
                .collect(),
        );
        // Well over the corner's target, so the brakes are on throughout; the
        // craft's own turn rate is swept to find the one that leaves the
        // steering loop short of lock. `actual` is `-angular_velocity.y` while
        // the orientation is the identity, so this is sweeping the rate error
        // through zero.
        let mut unsaturated = 0;
        for step in -200..=200 {
            let mut state = craft(line.point(0), 200.0);
            state.body.angular_velocity = Vec3::new(0.0, step as f32 * 0.02, 0.0);
            let controls = Driver::default().drive(&state, &line, &tuning);
            if controls.steer_x.abs() >= tuning.trail_saturation {
                continue;
            }
            unsaturated += 1;
            assert_eq!(
                controls.airbrake_left, controls.airbrake_right,
                "unsaturated at steer {}, but the airbrakes differ",
                controls.steer_x
            );
            assert!(
                controls.airbrake_left > 0.0,
                "a 60-unit circle at 200 needs brakes"
            );
        }
        assert!(
            unsaturated > 0,
            "the sweep never left the loop unsaturated, so this proves nothing"
        );
    }

    /// The other half of the pair above, and the feature itself: once the loop
    /// is out of lock and the craft is over the corner's speed, the brakes stop
    /// being symmetric and the extra goes on the side it is turning toward.
    #[test]
    fn a_saturated_driver_over_its_corner_speed_brakes_asymmetrically() {
        let tuning = Tuning::default();
        let line = Line::new(
            (0..128)
                .map(|step| {
                    let angle = std::f32::consts::TAU * step as f32 / 128.0;
                    Vec3::new(60.0 * angle.cos(), 0.0, 60.0 * angle.sin())
                })
                .collect(),
        );
        let controls = Driver::default().drive(&craft(line.point(0), 200.0), &line, &tuning);
        assert!(
            controls.steer_x.abs() >= tuning.trail_saturation,
            "a 60-unit circle at 200 should have the loop at lock, got {}",
            controls.steer_x
        );
        assert_ne!(controls.airbrake_left, controls.airbrake_right);
        // Steering left means the nose is going left, so the left side carries
        // the extra.
        if controls.steer_x < 0.0 {
            assert!(controls.airbrake_left > controls.airbrake_right);
        } else {
            assert!(controls.airbrake_right > controls.airbrake_left);
        }
    }

    /// The sign that `5ad69f3` shipped backwards for months. Positive rate
    /// error is a craft wanting to turn further right; a nose-right yaw needs
    /// `imbalance = L - R` negative, so the **right** side is braked.
    #[test]
    fn the_differential_brakes_the_side_the_nose_is_turning_toward() {
        let tuning = Tuning::default();
        let hard_right = Steer {
            command: 1.0,
            rate_error: 1.0,
        };
        let hard_left = Steer {
            command: -1.0,
            rate_error: -1.0,
        };
        // Over the target, so the overspeed gate is open.
        assert!(trail(&hard_right, 100.0, 50.0, &tuning) > 0.0);
        assert!(trail(&hard_left, 100.0, 50.0, &tuning) < 0.0);

        let (left, right) = airbrakes(
            0.0,
            trail(&hard_right, 100.0, 50.0, &tuning),
            tuning.brake_floor,
        );
        assert!(right > left, "turning right brakes the right side");
    }

    #[test]
    fn the_differential_is_dead_inside_its_deadband() {
        let tuning = Tuning::default();
        let saturated_but_settled = Steer {
            command: 1.0,
            rate_error: tuning.trail_deadband,
        };
        assert_eq!(trail(&saturated_but_settled, 100.0, 50.0, &tuning), 0.0);
    }

    #[test]
    fn the_differential_waits_for_the_steering_loop_to_run_out_of_lock() {
        let tuning = Tuning::default();
        let unsaturated = Steer {
            command: tuning.trail_saturation - 0.01,
            rate_error: 1.0,
        };
        assert_eq!(trail(&unsaturated, 100.0, 50.0, &tuning), 0.0);
    }

    /// Below the corner's target speed is corner exit, where the grip is wanted
    /// for accelerating. A straight has an infinite target, so this is also
    /// what keeps the differential off one.
    #[test]
    fn the_differential_never_acts_on_corner_exit() {
        let tuning = Tuning::default();
        let hard = Steer {
            command: 1.0,
            rate_error: 1.0,
        };
        assert_eq!(trail(&hard, 49.0, 50.0, &tuning), 0.0);
        assert_eq!(trail(&hard, 300.0, f32::INFINITY, &tuning), 0.0);
    }

    /// A yaw request must never drop a side to zero while braking: both sides
    /// strictly positive is the only thing that engages `ShipState::brake`.
    #[test]
    fn a_differential_never_cancels_the_brake_it_is_layered_on() {
        let floor = Tuning::default().brake_floor;
        for brake in [floor, 0.5, 0.8, 1.0] {
            for differential in [-1.0f32, -0.6, -0.1, 0.0, 0.1, 0.6, 1.0] {
                let (left, right) = airbrakes(brake, differential, floor);
                assert!(
                    left > 0.0 && right > 0.0,
                    "brake {brake} with differential {differential} broke the \
                     both-held gate: {left}, {right}"
                );
                assert!((0.0..=1.0).contains(&left) && (0.0..=1.0).contains(&right));
            }
        }
    }

    /// The reason the interval slides rather than being clipped: a craft
    /// braking flat out has no headroom above, and that is the corner it most
    /// needs to rotate in.
    #[test]
    fn a_differential_survives_a_craft_already_braking_flat_out() {
        let floor = Tuning::default().brake_floor;
        let (left, right) = airbrakes(1.0, 0.6, floor);
        assert!((right - left - 0.6).abs() < 1.0e-6, "got {left}, {right}");
        assert_eq!(right, 1.0);
        assert!(left >= floor);
    }

    /// With the brake off, one side rises from nothing: yaw and no
    /// deceleration, which is what a differential airbrake physically is.
    #[test]
    fn a_differential_with_no_brake_engages_no_brake() {
        let (left, right) = airbrakes(0.0, 0.4, Tuning::default().brake_floor);
        assert_eq!(left, 0.0);
        assert!((right - 0.4).abs() < 1.0e-6);
    }

    #[test]
    fn a_straight_has_no_speed_limit() {
        let tuning = Tuning::default();
        let target = corner_target(0.0, &tuning, &Personality::NEUTRAL);
        assert_eq!(target, f32::INFINITY);
        assert_eq!(throttle(500.0, target, &tuning), (1.0, 0.0));
    }

    #[test]
    fn a_corner_taken_too_fast_brakes_and_taken_slowly_does_not() {
        let tuning = Tuning::default();
        let target = corner_target(0.01, &tuning, &Personality::NEUTRAL);
        assert_eq!(throttle(target * 0.5, target, &tuning), (1.0, 0.0));
        assert_eq!(throttle(target * 2.0, target, &tuning), (0.0, 1.0));
    }

    /// The finding that makes a proportional brake worth having: the command
    /// level is not a deceleration, it is how much cornering grip the
    /// deceleration is bought with. See [`Tuning::brake_floor`].
    #[test]
    fn a_brake_climbs_with_the_overspeed_and_never_starts_below_the_floor() {
        let tuning = Tuning::default();
        let target = 100.0;

        // Inside the margin: lift off, but do not touch the airbrakes.
        assert_eq!(throttle(target * 1.02, target, &tuning), (0.0, 0.0));

        // Just past it: braking begins at the floor, not at an epsilon.
        let (_, just_past) = throttle(target * 1.051, target, &tuning);
        assert!(
            (just_past - tuning.brake_floor).abs() < 0.01,
            "braking should start at the floor, got {just_past}"
        );

        // Further past it: more grip spent, monotonically, up to full.
        let (_, further) = throttle(target * 1.2, target, &tuning);
        assert!(further > just_past);
        assert_eq!(throttle(target * 3.0, target, &tuning).1, 1.0);
    }

    #[test]
    fn a_driver_never_brakes_below_the_floor() {
        let tuning = Tuning::default();
        let target = 100.0;
        for step in 0..400 {
            let speed = target * (1.0 + step as f32 * 0.01);
            let (_, brake) = throttle(speed, target, &tuning);
            assert!(
                brake == 0.0 || brake >= tuning.brake_floor,
                "speed {speed} gave a brake of {brake}, between zero and the floor"
            );
        }
    }

    /// The turn rate the geometry asks for is bounded, or a craft thrown clear of
    /// the track holds full lock through the whole recovery.
    #[test]
    fn the_requested_turn_rate_is_clamped() {
        let tuning = Tuning {
            rate_gain: 1.0,
            ..Tuning::default()
        };
        let mut driver = Driver::default();
        // Absurdly far off the line, at speed: the raw curvature is enormous.
        let controls = driver.drive(
            &craft(Vec3::new(500.0, 0.0, 0.0), 150.0),
            &straight(),
            &tuning,
        );
        assert!(
            controls.steer_x >= -1.0,
            "steer {} left the input range",
            controls.steer_x
        );
        assert!(controls.steer_x.abs() <= tuning.max_turn_rate * tuning.rate_gain + 1e-3);
    }

    /// A straight with a corridor 8 units either side of the line.
    fn straight_with_corridor() -> Line {
        let points: Vec<Vec3> = (0..64)
            .map(|step| Vec3::new(0.0, 0.0, -10.0 * step as f32))
            .collect();
        let corridor = points
            .iter()
            .map(|_| Frame {
                // The line runs along `-Z`, so the driver's right is `-X`.
                lateral: Vec3::NEG_X,
                left: -8.0,
                right: 8.0,
            })
            .collect();
        Line::with_corridor(points, corridor)
    }

    /// Seed zero has to stay the driver every other test in this file measures.
    #[test]
    fn seed_zero_is_the_plain_line_follower() {
        assert_eq!(Personality::from_seed(0), Personality::NEUTRAL);
        assert_eq!(Driver::default().personality(), Personality::NEUTRAL);
        assert_ne!(Personality::from_seed(1), Personality::NEUTRAL);
    }

    /// The seed is the whole character, so it must decide it completely.
    #[test]
    fn a_personality_is_a_pure_function_of_its_seed() {
        for seed in [1u32, 2, 99, 0xdead_beef] {
            assert_eq!(Personality::from_seed(seed), Personality::from_seed(seed));
        }
        assert_ne!(Personality::from_seed(1), Personality::from_seed(2));
    }

    /// Every axis has to land inside the range its documentation claims, or a
    /// craft is handed a lookahead or a grip budget nothing tested.
    #[test]
    fn every_personality_stays_within_its_stated_range() {
        for seed in 1..2_000u32 {
            let p = Personality::from_seed(seed);
            assert!(
                (0.25..=0.85).contains(&p.line_bias.abs()),
                "seed {seed}: line_bias {}",
                p.line_bias
            );
            assert!((0.10..=0.30).contains(&p.wander), "seed {seed}");
            assert!(
                (1.0 / 420.0..=1.0 / 150.0).contains(&p.wander_rate),
                "seed {seed}"
            );
            assert!((0.85..=1.15).contains(&p.look), "seed {seed}");
            assert!((0.93..=1.05).contains(&p.commitment), "seed {seed}");
            assert!((0.85..=1.20).contains(&p.patience), "seed {seed}");
        }
    }

    /// Both sides of the line get used. A bias that only ever went one way
    /// would put the whole field on one side of the track, which is the same
    /// queue in a different place.
    #[test]
    fn the_field_leans_both_ways() {
        let left = (1..200u32)
            .filter(|&seed| Personality::from_seed(seed).line_bias < 0.0)
            .count();
        assert!((60..140).contains(&left), "{left} of 199 leant left");
    }

    /// The point of the whole change: two seeded craft in the same place aim at
    /// different points, and a craft with no seed aims at the line.
    #[test]
    fn two_drivers_aim_at_different_parts_of_the_corridor() {
        let tuning = Tuning::default();
        let line = straight_with_corridor();
        let state = craft(Vec3::ZERO, 60.0);

        let offset = |seed: u32| {
            let mut driver = Driver::seeded(seed);
            driver.drive(&state, &line, &tuning);
            driver
                .drift(line.aim(0, 40.0).corridor, &tuning, &driver.personality())
                .dot(Vec3::NEG_X)
        };

        let one = offset(1);
        let two = offset(2);
        assert!(
            (one - two).abs() > 1.0,
            "two seeded drivers aim {one} and {two} across the line, which is the same place"
        );
        assert_eq!(offset(0), 0.0, "an unseeded driver leaves the line alone");
    }

    /// And the corridor is a bound, not a suggestion.
    #[test]
    fn the_drift_stays_inside_the_corridor() {
        let tuning = Tuning::default();
        let line = straight_with_corridor();
        let frame = line.aim(0, 40.0).corridor;
        let room = 8.0 * tuning.corridor_use;

        for seed in 1..500u32 {
            let mut driver = Driver::seeded(seed);
            let personality = driver.personality();
            // A minute of driving, at the ticks the wander is a function of.
            for tick in 0..3_600 {
                driver.phase = tick;
                let across = driver.drift(frame, &tuning, &personality).dot(Vec3::NEG_X);
                assert!(
                    across.abs() <= room + 1e-3,
                    "seed {seed} at tick {tick} aimed {across} across a corridor of {room}"
                );
            }
        }
    }

    /// A line with no corridor has no room to spend, and every craft on it
    /// drives it exactly - which is what keeps the synthetic tests meaningful.
    #[test]
    fn a_line_without_a_corridor_is_driven_exactly() {
        let tuning = Tuning::default();
        let state = craft(Vec3::new(6.0, 0.0, 0.0), 40.0);

        let mut plain = Driver::default();
        let mut seeded = Driver::seeded(7);
        let a = plain.drive(&state, &straight(), &tuning);
        let b = seeded.drive(&state, &straight(), &tuning);
        assert_eq!(a.steer_x, b.steer_x);
    }

    /// The drift has to move, or the field is eight fixed lines rather than
    /// eight drivers.
    #[test]
    fn a_driver_drifts_over_time() {
        let tuning = Tuning::default();
        let line = straight_with_corridor();
        let frame = line.aim(0, 40.0).corridor;

        let mut driver = Driver::seeded(3);
        let personality = driver.personality();
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        for tick in 0..3_600 {
            driver.phase = tick;
            let across = driver.drift(frame, &tuning, &personality).dot(Vec3::NEG_X);
            low = low.min(across);
            high = high.max(across);
        }
        assert!(high - low > 0.5, "drifted over a range of {}", high - low);
    }

    /// Commitment is the axis that strings the field out, so it has to reach
    /// the speed target.
    #[test]
    fn a_committed_driver_carries_more_speed_through_a_corner() {
        let tuning = Tuning::default();
        let timid = Personality {
            commitment: 0.9,
            ..Personality::NEUTRAL
        };
        let brave = Personality {
            commitment: 1.1,
            ..Personality::NEUTRAL
        };
        // A speed between the two targets: one lifts, the other does not.
        let speed = corner_target(0.01, &tuning, &Personality::NEUTRAL);
        let (brave_thrust, brave_brake) =
            throttle(speed, corner_target(0.01, &tuning, &brave), &tuning);
        let (timid_thrust, timid_brake) =
            throttle(speed, corner_target(0.01, &tuning, &timid), &tuning);
        assert_eq!((brave_thrust, brave_brake), (1.0, 0.0));
        assert_eq!(timid_thrust, 0.0, "the timid driver is over its own target");
        assert!(timid_brake > 0.0, "and far enough over it to brake");
    }

    /// The wander argument is the driver's own tick count, so it has to advance
    /// when the driver drives and only then.
    #[test]
    fn the_phase_counts_the_ticks_this_driver_drove() {
        let mut driver = Driver::seeded(4);
        let state = craft(Vec3::ZERO, 40.0);
        for expected in 1..=5 {
            driver.drive(&state, &straight(), &Tuning::default());
            assert_eq!(driver.phase, expected);
        }
        // No line, no drive, no tick.
        driver.drive(&state, &Line::default(), &Tuning::default());
        assert_eq!(driver.phase, 5);
    }

    /// The index is the search seed, so it has to survive the call.
    #[test]
    fn the_driver_remembers_where_it_was() {
        let mut driver = Driver::default();
        let line = straight();
        driver.drive(
            &craft(Vec3::new(0.0, 0.0, -200.0), 40.0),
            &line,
            &Tuning::default(),
        );
        assert_eq!(driver.index, 20);
    }
}
