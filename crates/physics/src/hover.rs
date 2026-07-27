//! The two-probe air cushion, and the grounded-only terms that come with it.
//!
//! Specified in `docs/physics/README.md` at confidence **91** for the force law,
//! from static analysis that **has never been run**. Every number in this module
//! is transcribed from that page, including the parts that look wrong, because a
//! transcription can be corrected against a trace in M3 and an improvement
//! cannot.
//!
//! The three things most likely to be "fixed" by mistake, all of them deliberate:
//!
//! - **Stiffness carries no handling parameter.** It is
//!   `mass * 0.3 * (normal_gravity + track_gravity)`, calibrated against gravity.
//!   Ship-to-ship differences come from the gravity values, not from a spring
//!   constant, and there is no spring constant to look for.
//! - **`rebound` and `landing_rebound` scale damping only**, never the spring.
//!   Damping is a *multiplier* on the spring term rather than a summand, so at
//!   the height where the spring is zero the damping is zero too.
//! - **`ride_height` is both the raycast length and the spring's target.** This
//!   corrects `docs/physics/README.md`, which said it "never appears in the force
//!   law": `docs/ghidra/functions/psp-pulse/engine.md` traced the offset chain from
//!   the parser through `craft+0x70` and `craft+0x2f0` into the spring, at
//!   confidence 88. See [`target_height`].
//!
//! And one absence: **nothing here pitches the ship to the track**. Pitch and
//! roll response is entirely emergent from applying two forces at two points,
//! and the surface-alignment torque explicitly projects its pitch component out.
//! Adding an explicit pitch-to-track term would change the character of the whole
//! model.

use oag_core::math::Vec3;

use crate::collide::{Ray, Raycaster};
use crate::forces::Environment;
use crate::params::Handling;
use crate::ship::ShipState;

/// The global hover stiffness scale, `K`.
///
/// The static image holds this global as `1.0`, but **ship construction
/// overwrites it with `1.3333` before any race runs**, so `1.0` is never the
/// value the force law sees. Confidence **90**, and it scales hover stiffness
/// directly, which makes it one of the first things worth confirming with a
/// runtime read once M3's harness can break on a global: a factor of 1.33 on the
/// suspension is the difference between a ship that skims and one that scrapes.
///
/// Written as the decimal the original holds rather than as `4.0 / 3.0`, which is
/// a different `f32`.
pub const HOVER_K: f32 = 1.333_3;

/// How long after touchdown `landing_rebound` replaces `rebound`, in seconds.
pub const LANDING_WINDOW: f32 = 0.2;

/// How much a full magstrip lock raises the hover target height.
///
/// `target = (...) * (1 + 0.2 * magLockBlend) * K2`.
pub const TARGET_MAG_LOCK_GAIN: f32 = 0.2;

/// The global scale on the hover target height, `K2` at `0x08ab0e1c`.
///
/// **Its value was not read.** The identity is used so that it cannot silently
/// scale the target, and it is named rather than dropped so that the multiplication
/// is visible when someone does read it. A guess awaiting M3; unlike [`HOVER_K`],
/// there is not even a static-image value to go on.
pub const TARGET_GLOBAL_SCALE: f32 = 1.0;

/// The largest reduction the leap timer can make to the hover target height.
///
/// `leapAdjust = min(craft+0x2e0, 4.0)`, subtracted from the target while that
/// timer runs.
pub const LEAP_ADJUST_MAX: f32 = 4.0;

/// The hover spring's target height.
///
/// ```text
/// target = (ride_height + offset - min(leapTimer, 4.0)) * (1 + 0.2 * magLockBlend) * K2
/// ```
///
/// **`ride_height` is the primary term.** `docs/physics/README.md` said it "never
/// appears in the force law"; `docs/ghidra/functions/psp-pulse/engine.md` traced the
/// chain - the parser stores it at `+0x94`, `craft+0x70` points at it,
/// `Ship_UpdateCraft` builds `craft+0x2f0` from it, and `Ship_HoverTwoPoint` springs
/// against that - at confidence 88. It is still *also* the raycast length, so a ship
/// cannot probe further than the height it is trying to hold, and the two uses are
/// deliberately kept as one field rather than split.
///
/// **The additive `offset` is a guess.** The slot is `craft+0x74` and **nothing was
/// found that writes it**. [`crate::params::Pitch::antigrav_height_adjust`] is the
/// obvious candidate by name and is what is passed here, but a search for readers of
/// that field in the craft path found none - a weak negative, at confidence 50 for
/// "parsed but never consumed". If it turns out to be something else, this is the
/// one line to change. A guess awaiting M3.
#[must_use]
pub fn target_height(handling: &Handling, mag_lock_blend: f32, leap_timer: f32) -> f32 {
    let leap_adjust = leap_timer.clamp(0.0, LEAP_ADJUST_MAX);
    let base = handling.antigrav.ride_height + handling.pitch.antigrav_height_adjust - leap_adjust;

    base * (1.0 + TARGET_MAG_LOCK_GAIN * mag_lock_blend) * TARGET_GLOBAL_SCALE
}

/// The probe height, in units, below which penetration escape teleports the body.
///
/// Not a tunable and not part of the spring, which reads `targetHeight - h` at
/// every height. See [`HoverProbe::escape`].
pub const PENETRATION_LIMIT: f32 = 1.0;

/// The surface-alignment torque gain, grounded only.
///
/// Applied as `ALIGNMENT_GAIN * cross(up, avgNormal)` with the right-axis
/// component projected out, so it levels roll and yaw but deliberately not pitch.
///
/// # Why this is `+400` where the page writes `-400`
///
/// `docs/physics/README.md` writes `-400 * cross(up, avgNormal)` and, in the same
/// sentence, describes a term "which levels roll and yaw". **Those two statements
/// contradict each other**, and the contradiction is settled by arithmetic rather
/// than by reading the prose more carefully - which is the lesson `HANDOVER.md`
/// draws from every other disagreement this project has had: look for the
/// relation that must hold.
///
/// Let `omega = up x n`. A torque along `omega` grows angular velocity along
/// `omega`, and the up axis then moves as
///
/// ```text
/// d(up)/dt = omega x up = (up x n) x up
///          = n * (up . up) - up * (n . up)      [ (a x b) x c = b(a.c) - a(b.c) ]
///          = n - up * (n . up)                  [ up is a unit vector ]
/// ```
///
/// which is exactly the component of `n` perpendicular to `up`: it points *from*
/// `up` *toward* `n`. So `+k * cross(up, n)` aligns and `-k * cross(up, n)`
/// diverges, and **no magnitude can change that**. The gain, the inertia tensor,
/// the probe offsets and the target height are all unrecovered, and not one of
/// them can flip the sign of that derivation. The page's own prose asserts
/// levelling as part of a force law scored at confidence 91, so it is the literal
/// `-400` that has to be the transcription error: either the operands were
/// transposed when the page was written, `cross(avgNormal, up)`, or the sign was.
/// Behaviour is the stronger of the two claims, being what a trace would actually
/// show, where operand order is a detail of how somebody wrote down a decompiled
/// expression.
///
/// Simulation corroborates it. With the literal `-400`, a ship placed on a flat
/// floor with a 0.02 rad roll grows to a full tumble in under a second of
/// simulated time at 60 Hz - the signature of anti-alignment, not of a mistuned
/// gain.
///
/// **A third explanation has since become the leading one: handedness.**
/// [`crate::passive::WEATHERVANE_GROUND`] turned out to have exactly the same shape -
/// a negative coefficient on a cross product that must be positive for the behaviour
/// the same evidence page describes - and two independent transcription errors of one
/// shape is a poor explanation where a systematic frame difference is a good one.
/// `docs/ghidra/functions/psp-pulse/engine.md` lists handedness as not determined. If
/// that is what this is, the original's expression is correct as written *in its own
/// frame*, nothing is a typo, and this crate is simply obliged to flip every cross
/// product it inherits from that path. Which is a prediction, and a cheap one to
/// check against the next such term recovered.
///
/// **Only the sign is settled. The magnitude is not.** `400` is transcribed and
/// stays M3 work, along with everything else on that page. Confidence **84** on
/// the direction, which is the rubric's ceiling for decompilation-only evidence:
/// the arithmetic is compulsory, but the decompilation as recorded is
/// self-contradictory, no call site was re-read, and nothing is runtime-verified.
/// See the resolved-contradiction note in `docs/physics/README.md` for what would
/// raise it.
pub const ALIGNMENT_GAIN: f32 = 400.0;

/// The bank-to-yaw coupling gain, grounded only.
///
/// `angularLocal.y += 30 * right.y * (1 - magLockBlend)`. The magstrip factor comes
/// from `docs/ghidra/functions/psp-pulse/engine.md`'s per-component enumeration of
/// the hover epilogue's writes; `docs/physics/README.md` records the term without it.
pub const BANK_TO_YAW_GAIN: f32 = 30.0;

/// How strongly the grounded downforce opposes the hover spring.
///
/// `docs/physics/README.md` records a downforce "along the ground normal opposing
/// the hover spring" and **does not record its magnitude**. The shape is
/// therefore implemented and the coefficient is **deliberately zero**: this crate
/// ships no invented constant, because a plausible-looking `0.05` here would be
/// indistinguishable from a recovered value six months from now. M3 fills it in.
pub const DOWNFORCE_SCALE: f32 = 0.0;

/// What one probe found and what it contributed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoverProbe {
    /// Where the probe sits, in world space.
    pub point: Vec3,
    /// The force to apply at [`Self::point`], already including damping and the
    /// magstrip blend. Zero when the probe found no hoverable surface.
    pub force: Vec3,
    /// Whether the probe found a [hoverable](crate::collide::Surface::is_hoverable)
    /// surface within `ride_height`.
    pub contact: bool,
    /// The hit surface's normal, or [`Vec3::ZERO`] without contact.
    pub normal: Vec3,
    /// `dot(probeWorld - rayHit, up)`: how far the probe sits above the surface
    /// measured along the ship's own up axis, not along the surface normal.
    pub height: f32,
    /// Position correction for the penetration-escape constraint, or
    /// [`Vec3::ZERO`].
    ///
    /// When `h < 1.0` on a hoverable surface the original **teleports** the body
    /// by `up * (1 - h)` with no velocity change. That is penetration escape for
    /// the last unit only; it is not the hover mechanism and must not be mistaken
    /// for one.
    pub escape: Vec3,
}

impl HoverProbe {
    /// A probe that found nothing.
    fn miss(point: Vec3) -> Self {
        Self {
            point,
            force: Vec3::ZERO,
            contact: false,
            normal: Vec3::ZERO,
            height: 0.0,
            escape: Vec3::ZERO,
        }
    }
}

/// Everything the air cushion produced this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hover {
    /// The two probes, front first.
    pub probes: [HoverProbe; 2],
    /// How many probes are in contact, `0..=2`.
    pub contacts: u32,
    /// Mean of the normals under the probes in contact, normalised, or
    /// [`Vec3::ZERO`] when none are.
    pub average_normal: Vec3,
    /// The grounded-only surface-alignment torque, world space, pitch projected
    /// out.
    pub alignment_torque: Vec3,
    /// The grounded-only downforce along the ground normal, world space.
    pub downforce: Vec3,
    /// The grounded-only bank-to-yaw coupling, as a body-local angular
    /// acceleration.
    pub local_angular_acceleration: Vec3,
    /// The penetration-escape position correction for the whole body.
    pub escape: Vec3,
}

impl Hover {
    /// A frame with both probes in the air.
    fn airborne(probes: [HoverProbe; 2]) -> Self {
        Self {
            probes,
            contacts: 0,
            average_normal: Vec3::ZERO,
            alignment_torque: Vec3::ZERO,
            downforce: Vec3::ZERO,
            local_angular_acceleration: Vec3::ZERO,
            escape: Vec3::ZERO,
        }
    }
}

/// Where the two probes sit in body space.
///
/// Half a hull length fore and aft of the centre of mass, from
/// `<Misc length/>`. The page says the hull length sets where the probes sit and
/// does not say with what factor, so the symmetric half-length placement is a
/// **guess awaiting M3**; it is at least symmetric, which is the property the
/// tests depend on. The front probe is first.
#[must_use]
pub fn probe_offsets(handling: &Handling) -> [Vec3; 2] {
    let half = handling.dimensions.length * 0.5;

    // Body forward is -Z; see `Body::forward`.
    [Vec3::new(0.0, 0.0, -half), Vec3::new(0.0, 0.0, half)]
}

/// The damping multiplier's rebound coefficient for this frame.
///
/// ```text
/// reb = timeSinceLanding >= 0.2
///         ? rebound
///         : 0.5 * (rebound * t + landing_rebound * (1 - 5*t))
/// ```
///
/// **This is transcribed and it is internally odd.** Taking `t` as
/// `time_since_landing` in seconds, the branch is discontinuous at `t = 0.2`,
/// where the blend evaluates to `0.1 * rebound` and the other arm to `rebound`,
/// and `rebound * t` mixes a coefficient with a time. `1 - 5*t` does reach zero
/// exactly at the window's end, which is the one part that reads as intended, so
/// the most likely explanation is a factor lost when the expression was written
/// down rather than in the original. Recorded, kept literal, flagged for M3.
#[must_use]
pub fn rebound_coefficient(handling: &Handling, time_since_landing: f32) -> f32 {
    if time_since_landing >= LANDING_WINDOW {
        handling.antigrav.rebound
    } else {
        let t = time_since_landing;
        0.5 * (handling.antigrav.rebound * t + handling.antigrav.landing_rebound * (1.0 - 5.0 * t))
    }
}

/// Runs one probe.
///
/// `target_height` is the height the spring pulls toward, from [`target_height`]. It
/// stays a parameter rather than a lookup because one term of it, the additive
/// offset, is still a guess.
///
/// `grounded_prev` is last frame's groundedness, which the spring scales by
/// `0.75 * grounded_prev + 0.25`. Last frame's, not this frame's: the spring is
/// evaluated before the new contact count exists.
#[must_use]
pub fn probe<R: Raycaster + ?Sized>(
    state: &ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    local_offset: Vec3,
    target_height: f32,
) -> HoverProbe {
    let body = &state.body;
    let up = body.up();
    let point = body.position + body.orientation * local_offset;

    // Cast along the ship's **own** up axis rather than world gravity. This is
    // the whole reason magstrips and inversions work at all.
    let ray = Ray::new(point, -up, handling.antigrav.ride_height);

    let Some(hit) = raycaster.raycast(ray, env.self_collider, false) else {
        return HoverProbe::miss(point);
    };
    // Contact is accepted only for surface types 1 (Floor) and 3 (Mag Floor).
    if !hit.surface.is_hoverable() {
        return HoverProbe::miss(point);
    }

    let height = (point - hit.point).dot(up);

    // The page writes this as `bodyLinearVel + M * cross(probeLocal, angularVel)`,
    // which is the negation of the conventional rigid-body point velocity. The
    // conventional form is used here, because `omega x r` is what makes a
    // descending nose damp rather than pump; the discrepancy is recorded as an
    // unresolved sign rather than assumed to be a typo.
    let velocity = body.velocity_at(point);
    let normal_velocity = velocity.dot(hit.normal);

    let gravity = handling.physical.normal_gravity + handling.physical.track_gravity;
    let load = 0.75 * state.grounded_prev + 0.25;
    // Grouped left to right, deliberately: the operation order is part of the
    // result. No `mul_add` here or anywhere.
    let spring = handling.physical.mass * 0.3 * (target_height - height) * HOVER_K * load * gravity;

    let damping = (-0.1 * normal_velocity).clamp(-1.0, 2.0);
    let rebound = rebound_coefficient(handling, state.time_since_landing);

    // The magstrip blend fades the ordinary suspension out entirely; the magnetic
    // hold that is meant to replace it is **not implemented**, because its force
    // law was not decoded (confidence 82 on the blend alone). So `mag_lock_blend`
    // is consumed here and never driven by this crate: a caller that sets it to
    // 1.0 gets a ship with no suspension, which is the honest behaviour until the
    // hold block is read.
    let force = up * spring * (1.0 + rebound * damping) * (1.0 - state.mag_lock_blend);

    let escape = if height < PENETRATION_LIMIT {
        up * (PENETRATION_LIMIT - height)
    } else {
        Vec3::ZERO
    };

    HoverProbe {
        point,
        force,
        contact: true,
        normal: hit.normal,
        height,
        escape,
    }
}

/// Runs both probes and the grounded-only terms.
#[must_use]
pub fn evaluate<R: Raycaster + ?Sized>(
    state: &ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    target_height: f32,
) -> Hover {
    let offsets = probe_offsets(handling);
    let probes = [
        probe(state, handling, env, raycaster, offsets[0], target_height),
        probe(state, handling, env, raycaster, offsets[1], target_height),
    ];

    let contacts = probes.iter().filter(|probe| probe.contact).count() as u32;
    if contacts == 0 {
        return Hover::airborne(probes);
    }

    let mut normal_sum = Vec3::ZERO;
    let mut spring_along_up = 0.0f32;
    let up = state.body.up();
    for probe in &probes {
        if probe.contact {
            normal_sum += probe.normal;
            spring_along_up += probe.force.dot(up);
        }
    }
    let average_normal = (normal_sum / (contacts as f32)).normalize_or_zero();

    let mut alignment_torque = up.cross(average_normal) * ALIGNMENT_GAIN;
    // Project the right-axis component out, which is what removes pitch and
    // leaves roll and yaw. Written as a subtraction of the projection rather
    // than as a basis change, so there is one operation to check.
    let right = state.body.right();
    alignment_torque -= right * alignment_torque.dot(right);

    let downforce = -average_normal * spring_along_up * DOWNFORCE_SCALE;

    // Bank-to-yaw: a body-local yaw term proportional to how far the right axis
    // has tipped out of the world horizontal, and cancelled by a magstrip lock
    // like the rest of the suspension.
    let local_angular_acceleration = Vec3::new(
        0.0,
        BANK_TO_YAW_GAIN * right.y * (1.0 - state.mag_lock_blend),
        0.0,
    );

    // Two probes both penetrating would each ask for a teleport, and applying
    // both would move the body twice as far as either wanted. The larger
    // correction is taken instead. The original applies them in sequence against
    // a position that moves as it goes, which cannot be reproduced without
    // re-casting between probes; this is a choice, not a finding.
    let mut escape = Vec3::ZERO;
    for probe in &probes {
        if probe.escape.length_squared() > escape.length_squared() {
            escape = probe.escape;
        }
    }

    Hover {
        probes,
        contacts,
        average_normal,
        alignment_torque,
        downforce,
        local_angular_acceleration,
        escape,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::{CollisionWorld, Surface, TriangleSoup};
    use crate::ship::Body;

    fn flat_floor() -> CollisionWorld {
        let mut world = CollisionWorld::new();
        world.push(TriangleSoup::new(
            vec![
                [-500.0, 0.0, -500.0],
                [-500.0, 0.0, 500.0],
                [500.0, 0.0, 0.0],
            ],
            vec![[0, 1, 2]],
            Vec::new(),
            Surface::Floor,
            0,
        ));
        world
    }

    /// Arbitrary round numbers, chosen so the arithmetic is checkable by hand.
    /// **Not recovered values**: no handling data is reproduced in this
    /// repository, and none of these came from a ship.
    fn test_handling() -> Handling {
        Handling {
            antigrav: crate::params::Antigrav {
                ride_height: 20.0,
                rebound: 1.0,
                ..crate::params::Antigrav::default()
            },
            physical: crate::params::Physical {
                mass: 1.0,
                normal_gravity: 10.0,
                ..crate::params::Physical::default()
            },
            dimensions: crate::params::Dimensions {
                length: 4.0,
                ..crate::params::Dimensions::default()
            },
            ..Handling::ZERO
        }
    }

    fn state_at(height: f32) -> ShipState {
        ShipState {
            body: Body {
                position: Vec3::new(0.0, height, 0.0),
                ..Body::default()
            },
            grounded_prev: 1.0,
            ..ShipState::default()
        }
    }

    #[test]
    fn a_probe_at_the_target_height_produces_no_force() {
        let world = flat_floor();
        let handling = test_handling();
        let target = 5.0;
        let state = state_at(target);

        let probe = probe(
            &state,
            &handling,
            &Environment::default(),
            &world,
            Vec3::ZERO,
            target,
        );

        assert!(probe.contact);
        assert_eq!(probe.height, target);
        assert_eq!(probe.force, Vec3::ZERO);
    }

    /// The damping is a multiplier on the spring rather than a summand, so at the
    /// height where the spring vanishes no amount of vertical velocity produces a
    /// force. Counter-intuitive, and exactly what the page specifies.
    #[test]
    fn damping_cannot_produce_a_force_where_the_spring_is_zero() {
        let world = flat_floor();
        let handling = test_handling();
        let target = 5.0;
        let mut state = state_at(target);
        state.body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);

        let probe = probe(
            &state,
            &handling,
            &Environment::default(),
            &world,
            Vec3::ZERO,
            target,
        );
        assert_eq!(probe.force, Vec3::ZERO);
    }

    #[test]
    fn a_probe_below_the_target_height_is_pushed_along_the_ships_up_axis() {
        let world = flat_floor();
        let handling = test_handling();
        let probe = probe(
            &state_at(3.0),
            &handling,
            &Environment::default(),
            &world,
            Vec3::ZERO,
            5.0,
        );
        assert!(probe.force.y > 0.0);
        assert_eq!(probe.force.x, 0.0);
        assert_eq!(probe.force.z, 0.0);
    }

    #[test]
    fn a_probe_above_the_target_height_is_pulled_back_down() {
        let world = flat_floor();
        let handling = test_handling();
        let probe = probe(
            &state_at(8.0),
            &handling,
            &Environment::default(),
            &world,
            Vec3::ZERO,
            5.0,
        );
        assert!(probe.force.y < 0.0);
    }

    /// `ride_height` is the raycast length and nothing else, so a probe further
    /// above the floor than `ride_height` simply finds nothing.
    #[test]
    fn a_probe_beyond_ride_height_finds_no_surface() {
        let world = flat_floor();
        let handling = test_handling();
        let probe = probe(
            &state_at(handling.antigrav.ride_height + 1.0),
            &handling,
            &Environment::default(),
            &world,
            Vec3::ZERO,
            5.0,
        );
        assert!(!probe.contact);
        assert_eq!(probe.force, Vec3::ZERO);
    }

    #[test]
    fn a_wall_is_not_a_hoverable_surface() {
        let mut world = CollisionWorld::new();
        world.push(TriangleSoup::new(
            vec![
                [-500.0, 0.0, -500.0],
                [-500.0, 0.0, 500.0],
                [500.0, 0.0, 0.0],
            ],
            vec![[0, 1, 2]],
            Vec::new(),
            Surface::Wall,
            0,
        ));

        let probe = probe(
            &state_at(3.0),
            &test_handling(),
            &Environment::default(),
            &world,
            Vec3::ZERO,
            5.0,
        );
        assert!(!probe.contact);
    }

    #[test]
    fn a_mag_floor_hovers_exactly_like_a_floor() {
        let handling = test_handling();
        let state = state_at(3.0);

        let mut floor = CollisionWorld::new();
        floor.push(TriangleSoup::new(
            vec![
                [-500.0, 0.0, -500.0],
                [-500.0, 0.0, 500.0],
                [500.0, 0.0, 0.0],
            ],
            vec![[0, 1, 2]],
            Vec::new(),
            Surface::Floor,
            0,
        ));
        let mut mag = CollisionWorld::new();
        mag.push(TriangleSoup::new(
            vec![
                [-500.0, 0.0, -500.0],
                [-500.0, 0.0, 500.0],
                [500.0, 0.0, 0.0],
            ],
            vec![[0, 1, 2]],
            Vec::new(),
            Surface::MagFloor,
            0,
        ));

        let a = probe(
            &state,
            &handling,
            &Environment::default(),
            &floor,
            Vec3::ZERO,
            5.0,
        );
        let b = probe(
            &state,
            &handling,
            &Environment::default(),
            &mag,
            Vec3::ZERO,
            5.0,
        );
        assert_eq!(a.force, b.force);
    }

    #[test]
    fn a_full_magstrip_blend_cancels_the_ordinary_suspension() {
        let world = flat_floor();
        let handling = test_handling();
        let mut state = state_at(3.0);
        state.mag_lock_blend = 1.0;

        let probe = probe(
            &state,
            &handling,
            &Environment::default(),
            &world,
            Vec3::ZERO,
            5.0,
        );
        assert!(probe.contact);
        assert_eq!(probe.force, Vec3::ZERO);
    }

    #[test]
    fn penetration_escape_engages_only_within_the_last_unit() {
        let world = flat_floor();
        let handling = test_handling();

        let clear = probe(
            &state_at(1.5),
            &handling,
            &Environment::default(),
            &world,
            Vec3::ZERO,
            5.0,
        );
        assert_eq!(clear.escape, Vec3::ZERO);

        let deep = probe(
            &state_at(0.25),
            &handling,
            &Environment::default(),
            &world,
            Vec3::ZERO,
            5.0,
        );
        assert_eq!(deep.escape, Vec3::new(0.0, 0.75, 0.0));
    }

    #[test]
    fn both_probes_reach_the_floor_under_a_level_ship() {
        let world = flat_floor();
        let handling = test_handling();
        let hover = evaluate(
            &state_at(3.0),
            &handling,
            &Environment::default(),
            &world,
            5.0,
        );
        assert_eq!(hover.contacts, 2);
        assert_eq!(hover.average_normal, Vec3::Y);
    }

    /// A level ship over a level floor has nothing to align to, which is why the
    /// alignment torque's sign cannot be pinned by a flat-floor test.
    #[test]
    fn a_level_ship_over_a_level_floor_gets_no_alignment_torque() {
        let world = flat_floor();
        let hover = evaluate(
            &state_at(3.0),
            &test_handling(),
            &Environment::default(),
            &world,
            5.0,
        );
        assert_eq!(hover.alignment_torque, Vec3::ZERO);
    }

    /// The alignment torque points in the **aligning** direction.
    ///
    /// This is the invariant the whole `-400` versus `+400` question reduces to,
    /// and it is asserted directly on the torque rather than through a simulation:
    /// a torque along `+cross(up, n)` moves the up axis along `n - up * (n . up)`,
    /// which points from `up` toward `n`. Nothing about the gain's magnitude, the
    /// inertia tensor, the probe placement or the target height can change the sign
    /// of that dot product, which is exactly why the sign is settleable while the
    /// magnitude is not. See [`ALIGNMENT_GAIN`].
    #[test]
    fn the_alignment_torque_points_from_the_ships_up_axis_toward_the_surface_normal() {
        let world = flat_floor();
        let handling = test_handling();

        for angle in [-0.6f32, -0.02, 0.02, 0.6] {
            let mut state = state_at(3.0);
            state.body.orientation = oag_core::math::Quat::from_rotation_z(angle);

            let hover = evaluate(&state, &handling, &Environment::default(), &world, 5.0);
            assert!(hover.contacts > 0);

            let aligning = state.body.up().cross(hover.average_normal);
            assert!(
                hover.alignment_torque.dot(aligning) > 0.0,
                "torque {:?} was not aligning at a roll of {angle}",
                hover.alignment_torque
            );
        }
    }

    /// The documented property of the alignment torque is the absence of a pitch
    /// component, which is independent of its direction, so it gets its own test.
    #[test]
    fn the_alignment_torque_never_has_a_pitch_component() {
        let world = flat_floor();
        let handling = test_handling();

        // Rolled and yawed away from level, so `cross(up, normal)` is non-zero.
        let mut state = state_at(3.0);
        state.body.orientation =
            oag_core::math::Quat::from_rotation_z(0.2) * oag_core::math::Quat::from_rotation_y(0.4);

        let hover = evaluate(&state, &handling, &Environment::default(), &world, 5.0);
        assert!(hover.contacts > 0);
        assert_ne!(hover.alignment_torque, Vec3::ZERO);

        let right = state.body.right();
        assert!(
            hover.alignment_torque.dot(right).abs() < 1e-5,
            "pitch component was {}",
            hover.alignment_torque.dot(right)
        );
    }

    #[test]
    fn an_airborne_ship_gets_none_of_the_grounded_terms() {
        let world = flat_floor();
        let handling = test_handling();
        let hover = evaluate(
            &state_at(handling.antigrav.ride_height + 10.0),
            &handling,
            &Environment::default(),
            &world,
            5.0,
        );
        assert_eq!(hover.contacts, 0);
        assert_eq!(hover.alignment_torque, Vec3::ZERO);
        assert_eq!(hover.downforce, Vec3::ZERO);
        assert_eq!(hover.local_angular_acceleration, Vec3::ZERO);
        assert_eq!(hover.escape, Vec3::ZERO);
    }

    #[test]
    fn the_rebound_coefficient_leaves_the_landing_window_at_the_plain_rebound() {
        let handling = Handling {
            antigrav: crate::params::Antigrav {
                rebound: 2.0,
                landing_rebound: 7.0,
                ..crate::params::Antigrav::default()
            },
            ..Handling::ZERO
        };

        assert_eq!(rebound_coefficient(&handling, LANDING_WINDOW), 2.0);
        assert_eq!(rebound_coefficient(&handling, 10.0), 2.0);
        // Inside the window, at touchdown, only `landing_rebound` contributes.
        assert_eq!(rebound_coefficient(&handling, 0.0), 3.5);
    }

    /// Two penetrating probes each ask for a teleport, and applying both would
    /// move the body by their sum, which is further than either wanted. The larger
    /// wins. That is a choice made here and not a finding, so it is pinned.
    #[test]
    fn two_penetrating_probes_move_the_body_by_the_deeper_one_and_not_by_their_sum() {
        // A floor at y = 0 under the front probe and a floor at y = -0.5 under the
        // rear one, both wound to face up.
        let mut world = CollisionWorld::new();
        world.push(TriangleSoup::new(
            vec![
                [-100.0, 0.0, -100.0],
                [-100.0, 0.0, -0.1],
                [100.0, 0.0, -0.1],
                [100.0, 0.0, -100.0],
                [-100.0, -0.5, 0.1],
                [-100.0, -0.5, 100.0],
                [100.0, -0.5, 100.0],
                [100.0, -0.5, 0.1],
            ],
            vec![[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]],
            Vec::new(),
            Surface::Floor,
            0,
        ));

        let handling = test_handling();
        let hover = evaluate(
            &state_at(0.2),
            &handling,
            &Environment::default(),
            &world,
            5.0,
        );

        assert_eq!(hover.contacts, 2);
        // Front probe 0.2 above its floor, rear probe 0.7 above its own.
        assert_eq!(hover.probes[0].escape, Vec3::new(0.0, 0.8, 0.0));
        assert_eq!(hover.probes[1].escape, Vec3::new(0.0, 0.3, 0.0));
        assert_eq!(hover.escape, Vec3::new(0.0, 0.8, 0.0));
    }

    /// `ride_height` is the primary term of the hover target, which is the correction
    /// `docs/ghidra/functions/psp-pulse/engine.md` made to `docs/physics/README.md`.
    #[test]
    fn the_hover_target_is_built_from_ride_height() {
        let handling = Handling {
            antigrav: crate::params::Antigrav {
                ride_height: 12.0,
                ..crate::params::Antigrav::default()
            },
            pitch: crate::params::Pitch {
                antigrav_height_adjust: 3.0,
                ..crate::params::Pitch::default()
            },
            ..Handling::ZERO
        };

        assert_eq!(target_height(&handling, 0.0, 0.0), 15.0);
    }

    /// A magstrip lock raises the target by a fifth, and the leap timer lowers it by up
    /// to four units while it runs.
    #[test]
    fn the_hover_target_responds_to_the_magstrip_blend_and_the_leap_timer() {
        let handling = Handling {
            antigrav: crate::params::Antigrav {
                ride_height: 10.0,
                ..crate::params::Antigrav::default()
            },
            ..Handling::ZERO
        };

        assert_eq!(target_height(&handling, 1.0, 0.0), 12.0);
        assert_eq!(target_height(&handling, 0.0, 1.5), 8.5);
        // Clamped at four, however long the timer says.
        assert_eq!(target_height(&handling, 0.0, 100.0), 6.0);
        // And a spent timer takes nothing off.
        assert_eq!(target_height(&handling, 0.0, 0.0), 10.0);
    }

    /// `Handling::ZERO` has `ride_height = 0.0`, so the probe segment has zero
    /// length and cannot hit anything. That is the mechanism behind the
    /// crate-level "nothing accelerates" invariant, so it is pinned here too.
    #[test]
    fn a_zero_handling_ship_never_finds_the_floor() {
        let world = flat_floor();
        let hover = evaluate(
            &state_at(0.0),
            &Handling::ZERO,
            &Environment::default(),
            &world,
            0.0,
        );
        assert_eq!(hover.contacts, 0);
    }
}
