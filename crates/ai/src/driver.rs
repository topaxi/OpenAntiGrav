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
    /// Multiplier on [`Tuning::trail_gain`]: how readily this driver spends
    /// grip on rotating the craft when the steering has run out of lock.
    pub trail: f32,
    /// Multiplier on [`Tuning::corridor_use`]: how much of the corridor this
    /// driver is willing to use at all.
    pub width: f32,
    /// Bias toward the inside of the corner ahead, signed by its curvature, in
    /// the same fraction-of-the-room units as [`Personality::line_bias`].
    ///
    /// Unlike the bias, this **swaps sides with the corner**, which is what a
    /// driver taking a racing line does and what a fixed lean cannot express.
    pub inside: f32,
    /// How readily this driver moves out of the way of a craft behind it.
    pub courtesy: f32,
    /// How readily this driver moves to cover one.
    pub defence: f32,
    /// How early it lifts off for a craft close ahead.
    pub caution: f32,
    /// How readily it throws the craft sideways at a rival level with it.
    pub ram: f32,
    /// How many ticks of provocation being overtaken is worth.
    pub provocation_ticks: f32,
    /// How readily it puts a weapon in the air once it has a target.
    pub trigger: f32,
}

impl Personality {
    /// The driver that was here before personalities were: shares the field's
    /// tuning exactly and drives the authored line.
    /// **The identity, and every axis added later has to keep it one.** An
    /// appended multiplier is neutral at one and an appended bias at zero, so
    /// this stays the driver that was here before any of it existed - which is
    /// what every exact assertion written against seed zero is measuring.
    pub const NEUTRAL: Self = Self {
        line_bias: 0.0,
        wander: 0.0,
        wander_rate: 0.0,
        look: 1.0,
        commitment: 1.0,
        patience: 1.0,
        trail: 1.0,
        width: 1.0,
        inside: 0.0,
        courtesy: 0.0,
        defence: 0.0,
        caution: 0.0,
        ram: 0.0,
        provocation_ticks: 0.0,
        trigger: 0.0,
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
        Self::from_pilot_seed(&Pilot::BALANCED, seed)
    }

    /// Derives a personality from a pilot and a seed.
    ///
    /// Seed zero is [`Self::NEUTRAL`] whatever the pilot, with **zero** draws,
    /// for the reason [`Self::from_seed`] gives.
    #[must_use]
    pub fn from_pilot_seed(pilot: &Pilot, seed: u32) -> Self {
        if seed == 0 {
            return Self::NEUTRAL;
        }
        Self::from_pilot(pilot, &mut Rng::new(u64::from(seed)))
    }

    /// Draws a personality out of a pilot's ranges.
    ///
    /// # The draw order is frozen
    ///
    /// **Draws one to seven are what shipped before pilots existed, in this
    /// order, for ever; a new axis appends after them and never goes between.**
    /// [`Pilot::BALANCED`] holds the spans those seven used, so this reproduces
    /// the pre-pilot personality bit for bit for every seed - which is what
    /// made introducing pilots a change to no behaviour and no world hash.
    /// `pilot::tests::the_balanced_pilot_reproduces_the_personality_that_shipped_before_pilots_existed`
    /// is that claim, against literals captured before the change.
    ///
    /// Sequential lets rather than a closure in a struct literal, because the
    /// *order* of the draws is what makes this reproducible and a struct
    /// literal's field order is a thing a later edit moves without thinking.
    /// Two craft that drew the same values in a different order are two
    /// different craft.
    ///
    /// Takes the generator rather than a seed so a caller can prove two pilots
    /// consume the same draws by comparing `Rng::snapshot` afterwards.
    #[must_use]
    pub fn from_pilot(pilot: &Pilot, rng: &mut Rng) -> Self {
        // Rarely near zero: the middle of the corridor is where everyone would
        // be anyway.
        let magnitude = pilot.line_bias.draw(rng);
        let line_bias = magnitude * pilot.lean.sign(rng);
        let wander = pilot.wander.draw(rng);
        // Drawn as a period and stored as a rate. Two and a half to seven
        // seconds a step at 60 Hz for the balanced pilot - slower than a
        // corner, so the drift is something a driver *is* rather than something
        // that happens to it mid-bend.
        let wander_rate = 1.0 / pilot.wander_period.draw(rng);
        let look = pilot.look.draw(rng);
        let commitment = pilot.commitment.draw(rng);
        let patience = pilot.patience.draw(rng);
        let trail = pilot.trail.draw(rng);
        let width = pilot.width.draw(rng);
        let inside = pilot.inside.draw(rng);
        let courtesy = pilot.courtesy.draw(rng);
        let defence = pilot.defence.draw(rng);
        let caution = pilot.caution.draw(rng);
        let ram = pilot.ram.draw(rng);
        let provocation_ticks = pilot.provocation_ticks.draw(rng);
        let trigger = pilot.trigger.draw(rng);

        Self {
            line_bias,
            wander,
            wander_rate,
            look,
            commitment,
            patience,
            trail,
            width,
            inside,
            courtesy,
            defence,
            caution,
            ram,
            provocation_ticks,
            trigger,
        }
    }
}

impl Default for Personality {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

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

/// How often a driver at full `ram` will take a shot at a rival alongside, per
/// tick. About once a second.
const RAM_RATE: f32 = 1.0 / 60.0;

/// The noise stream mistakes are rolled against.
const MISTAKE_STREAM: u32 = 3;

/// How long a missed braking point lasts. Half a second, which at racing speed
/// is comfortably past the point the driver should have lifted.
const MISTAKE_TICKS: u16 = 30;

/// The noise stream weapon decisions are rolled against.
const WEAPON_STREAM: u32 = 2;

/// How often a driver at full `trigger` will fire once it has a target, per
/// tick.
///
/// **This is a rate, not a probability.** Rolled every tick, so at `1.0` the
/// median wait once a target is in the cone is about thirteen ticks - a fifth
/// of a second - and at `0.2` about a second and a half. Read as "a five per
/// cent chance" it looks far too small; read as a delay it is what a driver
/// taking a moment to line up looks like.
const TRIGGER_RATE: f32 = 0.05;

/// How far a forward weapon is worth firing, in units.
const WEAPON_RANGE: f32 = 200.0;

/// And how close is too close.
///
/// `oag_gameplay::projectile::blast` damages **every** craft in radius,
/// including the one that fired, so a rocket let go at point-blank is a rocket
/// fired at yourself.
const WEAPON_MIN_RANGE: f32 = 20.0;

/// How far off the nose a target may sit, as a cosine.
///
/// A cosine and never an angle: `docs/architecture/determinism.md` forbids the
/// transcendental, and `Rival::cos_bearing` is a dot product the caller already
/// had. About twenty degrees.
const WEAPON_CONE: f32 = 0.94;

/// How bent the road between here and the target may be before a shot is not
/// worth taking.
const WEAPON_CURVATURE: f32 = 1.0 / 400.0;

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

        let personality = self.personality(ctx.pilot);
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
        let curvature = line.max_curvature(index, window, look * 0.5);
        let target = corner_target(curvature, tuning, &personality);
        self.blunder(tuning, speed, target);
        let (thrust, brake) = if self.mistake > 0 {
            // Sailing through it. See [`Self::mistake`].
            (1.0, 0.0)
        } else {
            throttle(speed, target, tuning)
        };
        let thrust = thrust * self.caution(ctx, &personality);
        let differential = trail(&steer, speed, target, tuning, &personality);
        let (airbrake_left, airbrake_right) = airbrakes(brake, differential, tuning.brake_floor);

        ShipControls {
            steer_x: steer.command,
            thrust,
            airbrake_left,
            airbrake_right,
            sideshift: self.ram(state, ctx, &personality),
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
        let span = (ctx.tuning.look_min + ctx.tuning.look_speed * speed) * 0.5;
        let curvature = ctx.line.max_curvature(self.index as usize, distance, span);
        speed <= corner_target(curvature, ctx.tuning, &self.personality(ctx.pilot))
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
    /// Whether this driver would put a forward weapon in the air this tick, and
    /// at whom.
    ///
    /// The target slot comes back even though a Rocket is unguided and will not
    /// use it, because the *decision* is the part worth testing and a guided
    /// weapon will want it. A caller that only needs "yes" reads `is_some`.
    ///
    /// Five gates:
    ///
    /// 1. There is a craft ahead at all.
    /// 2. It is inside [`WEAPON_RANGE`] and outside [`WEAPON_MIN_RANGE`] - the
    ///    near bound matters, because the blast catches the firer too.
    /// 3. It is inside the cone, by [`Rival::cos_bearing`].
    /// 4. **The road between here and there is straight enough**, by the same
    ///    `max_curvature` the Turbo gate uses. A rocket round a corner is a
    ///    rocket in a wall, and using the same notion of "is this a straight"
    ///    keeps the two decisions consistent rather than inventing a second.
    /// 5. The trigger roll - see [`TRIGGER_RATE`], which is a rate and not a
    ///    probability.
    ///
    /// **The roll does not touch the world's generator.** It is
    /// `noise::roll(seed, phase, WEAPON_STREAM)`, a pure function of this
    /// driver's own seed and tick count, for the reason
    /// `Personality::from_pilot` gives: a draw from that stream would move every
    /// later pickup roll and make *which craft shoots* depend on how many
    /// pickups had been handed out.
    #[must_use]
    pub fn wants_to_fire(&self, ctx: &Context<'_>) -> Option<u8> {
        let personality = self.personality(ctx.pilot);
        if personality.trigger <= 0.0 {
            return None;
        }
        let target = ctx.field.ahead?;
        if target.range <= WEAPON_MIN_RANGE || target.range > WEAPON_RANGE {
            return None;
        }
        if target.cos_bearing < WEAPON_CONE {
            return None;
        }
        let span = (ctx.tuning.look_min + ctx.tuning.look_speed * target.range) * 0.5;
        if ctx
            .line
            .max_curvature(self.index as usize, target.range, span)
            > WEAPON_CURVATURE
        {
            return None;
        }
        let appetite = personality.trigger * (1.0 + self.provoked());
        if roll(self.seed, self.phase, WEAPON_STREAM) >= appetite * TRIGGER_RATE {
            return None;
        }
        Some(target.slot)
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
        let Some(rival) = ctx.field.behind else {
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
        if lean == 0.0 {
            return 0.0;
        }

        let gap = rival.gap.abs();
        if gap >= AWARENESS_RANGE {
            return 0.0;
        }
        // Inside the close range only the yielding half survives - see
        // [`SOCIAL_MIN_GAP`].
        let lean = if gap < SOCIAL_MIN_GAP {
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
        let wanted = (personality.line_bias + wobbled + inside + social).clamp(-1.0, 1.0);
        // Every term above is in the same fraction-of-the-room units and is
        // clamped once here, so no term can fight the corridor: the clamp below
        // is the backstop and not the mechanism.
        let offset = wanted * frame.room(wanted) * ctx.tuning.corridor_use * personality.width;
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
fn trail(
    steer: &Steer,
    speed: f32,
    target: f32,
    tuning: &Tuning,
    personality: &Personality,
) -> f32 {
    if speed < target
        || steer.command.abs() < tuning.trail_saturation
        || steer.rate_error.abs() <= tuning.trail_deadband
    {
        return 0.0;
    }
    let past = steer.rate_error.abs() - tuning.trail_deadband;
    let magnitude = (past * tuning.trail_gain * personality.trail).min(tuning.trail_max);
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

mod ram;

#[cfg(test)]
mod tests;
