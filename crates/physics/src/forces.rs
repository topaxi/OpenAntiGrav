//! One frame of force evaluation, and nothing else.
//!
//! # Why this is a separate module from the integrator
//!
//! `docs/physics/README.md` specifies the craft integrator as
//!
//! ```text
//! dt = clamp(measured_dt, 0, 0.06666)
//! evaluate all forces once at full dt
//! integrate 3 explicit Euler sub-steps of dt/3
//! ```
//!
//! **Forces are evaluated once, at full `dt`, not once per sub-step.** That is
//! counter-intuitive - it is not what a textbook sub-stepped integrator does, and
//! it is not what anyone reaching for "more accuracy" would write - so the
//! split is structural rather than a matter of discipline: this module cannot
//! sub-step because it has no loop and no notion of a sub-step, and
//! [`crate::integrate`] cannot re-evaluate forces because it only sees a
//! [`Body`](crate::ship::Body).
//!
//! # Groundedness is stale for every control term
//!
//! This is the single most likely thing to get wrong, and it is worth more than a
//! comment. In the original, hover is **step 8 of 15** and it clears the contact
//! flag on entry, so the terms that run before it see the *previous* frame's
//! groundedness and the terms after it see this frame's:
//!
//! | Reads | Sees |
//! | --- | --- |
//! | engine, brakes, pitch, quadratic drag, gravity, hover's own load factor | last frame |
//! | lateral grip, weathervane torque | this frame |
//!
//! Confidence 84, from `docs/ghidra/functions/psp-pulse/engine.md`. The obvious
//! reimplementation - resolve contacts, then apply forces - gets a different answer
//! on every takeoff and landing frame, in five terms simultaneously. So [`evaluate`]
//! binds the two values to separately named locals and every call site takes one
//! explicitly; no term reads groundedness out of the state for itself.
//!
//! # What is here and what is deliberately absent
//!
//! Present: the control force law from
//! `docs/ghidra/functions/psp-pulse/engine.md` (engine, brakes, steering, pitch and
//! the passive terms), the two-probe air cushion from `docs/physics/README.md`, and
//! the airbrakes with lateral grip and the sideshift.
//!
//! **Absent**, each because implementing it would mean inventing a trigger nobody
//! has decoded: the track-section force (`FUN_08848f9c`, a guess at confidence 45),
//! the four-corner hover variant and the auto-speed law behind its selector, turbo,
//! the magnetic hold, and every flag-gated engine and steering variant. See
//! `docs/physics/README.md`'s "what is implemented" section for the full list.

use oag_core::math::Vec3;

use crate::airbrake::{self, AirbrakeForces};
use crate::collide::Raycaster;
use crate::hover::{self, Hover};
use crate::params::Handling;
use crate::ship::{ShipControls, ShipState};
use crate::{controls, engine, passive};

/// The world outside the ship, as the force law sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    /// The track's own up axis at the ship.
    ///
    /// **Consumed by nothing yet.** Gravity acts on world `.y` only, and
    /// `track_gravity` reaches the force law solely through the hover spring's
    /// calibration, so this is not the gravity direction. It is kept because the
    /// magstrip probe casts along `-5 * (shipUp - trackGravityUp)` and will need it
    /// when the magnetic hold is implemented.
    pub track_up: Vec3,
    /// The per-speed-class scale on `normal_gravity`, from the table at
    /// `0x08ab0dcc`.
    ///
    /// **The table's values were not read**, so the identity is the default, and it
    /// is an input rather than a constant because it belongs to the speed class,
    /// which this crate does not otherwise know about. Confidence 78 that the table
    /// exists and is indexed by class; what is in it is a guess awaiting M3.
    pub class_gravity_scale: f32,
    /// The ship's own collider index, so its probes do not hit itself.
    pub self_collider: Option<u32>,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            track_up: Vec3::Y,
            class_gravity_scale: 1.0,
            self_collider: None,
        }
    }
}

/// The four accumulators the original keeps on the craft, zeroed every frame.
///
/// `craft+0x320`, `+0x330`, `+0x340` and `+0x350`, drained into the rigid body at the
/// bottom of `Ship_UpdateCraft`. Reproduced as four separate vectors rather than
/// collapsed into one, because which accumulator a term writes is *evidence*: it is
/// how the local and world frames were told apart in the first place, and collapsing
/// them would throw that away.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Accumulators {
    /// Body-local force. Engine thrust and lift, and lateral grip.
    pub local_force: Vec3,
    /// World force. Brakes, the airbrake slide and lateral terms, gravity, drag,
    /// rolling resistance, the hover downforce and vertical damping.
    pub world_force: Vec3,
    /// Body-local angular. Steering yaw, pitch, airbrake yaw, bank-to-yaw and
    /// angular damping.
    ///
    /// **Whether the angular accumulators hold torque or angular acceleration is not
    /// determined.** Nothing in any term visibly divides by an inertia,
    /// `docs/physics/README.md` calls this `angAccelLocal`, and the angular damping
    /// term's shape - a coefficient times angular velocity - is dimensionally an
    /// acceleration. So this crate treats both angular accumulators as **angular
    /// acceleration**, which is a pick awaiting M3. One consequence is worth
    /// knowing: under that reading the inertia tensor has no effect on any of these
    /// terms at all, because the drain multiplies by exactly what the integrator
    /// then divides by.
    pub local_angular: Vec3,
    /// World angular. The surface-alignment torque and the weathervane torque.
    pub world_angular: Vec3,
}

/// Everything one force evaluation produced, for inspection.
///
/// The forces have already been accumulated onto the body; this is what a test or a
/// debug overlay wants to look at afterwards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evaluated {
    /// The four accumulators, as drained onto the body.
    pub accumulators: Accumulators,
    /// The air cushion's output.
    pub hover: Hover,
    /// The airbrake path's output, excluding lateral grip and the sideshift.
    pub airbrake: AirbrakeForces,
    /// The engine's body-local contribution.
    pub engine: engine::EngineForce,
    /// The brake's world-space force.
    pub brakes: Vec3,
    /// Lateral grip, as a body-local force.
    pub lateral_grip: Vec3,
    /// Gravity, as a world-space force.
    pub gravity: Vec3,
    /// Quadratic drag, as a world-space force.
    pub drag: Vec3,
    /// Rolling resistance, as a world-space force.
    pub rolling_resistance: Vec3,
    /// Vertical damping, as a world-space force.
    pub vertical_damping: Vec3,
    /// The weathervane torque, world angular.
    pub weathervane: Vec3,
    /// Angular damping, body-local angular.
    pub angular_damping: Vec3,
    /// Body-local yaw from the steering.
    pub steering: f32,
    /// Body-local pitch from the pitch axis.
    pub pitch: f32,
    /// `dot(velocity, forward)`, signed, which four terms branch on.
    pub forward_speed: f32,
    /// The groundedness every control term saw: **last** frame's.
    pub control_grounded: f32,
    /// The groundedness lateral grip and the weathervane saw: this frame's.
    pub contact_grounded: f32,
}

/// Evaluates every force for one frame, at full `dt`, exactly once.
///
/// Mutates `state`: the control states, the contact bookkeeping, the accumulators on
/// the body, and the two things the original applies outside the accumulators
/// entirely - the sideshift's velocity change and the penetration-escape teleport.
///
/// The body's accumulators are **not** cleared here. [`crate::integrate::step`]
/// clears them at the top of the frame, so that after a step they still hold what was
/// applied.
///
/// The term order below follows `Ship_UpdateCraft`'s fifteen steps. Order does not
/// change a sum of forces, and it is kept anyway: it is what decides which
/// groundedness each term sees, and a reader comparing this against the evidence page
/// should not have to reorder anything in their head.
pub fn evaluate<R: Raycaster + ?Sized>(
    state: &mut ShipState,
    input: &ShipControls,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    dt: f32,
) -> Evaluated {
    // Last frame's contacts. Every control term reads this, and it is bound here,
    // once, before anything can overwrite it.
    let control_grounded = state.grounded;
    state.grounded_prev = control_grounded;
    let control_contact = control_grounded > 0.0;

    controls::update(state, input, handling, dt);

    let forward = state.body.forward();
    let up = state.body.up();
    let velocity = state.body.linear_velocity;
    let forward_speed = velocity.dot(forward);

    let mut acc = Accumulators::default();

    // 1. Engine, into the local force accumulator.
    let engine_force = engine::engine(state, handling, control_grounded, forward_speed);
    acc.local_force += engine_force.as_local_force();

    // 2. Brakes. Called only while the contact flag is set, so groundedness gates the
    //    whole term rather than scaling it: braking does nothing in the air.
    let brakes = if control_contact {
        engine::brakes(state, handling)
    } else {
        Vec3::ZERO
    };
    acc.world_force += brakes;

    // 3. Airbrakes. The sideshift they can also produce goes straight to the body,
    //    below, rather than through any accumulator.
    let airbrake = airbrake::evaluate(state, input, handling, forward_speed);
    acc.world_force += airbrake.world_force;
    acc.local_angular += airbrake.local_angular;

    // 4. Steering and 5. pitch, both body-local angular.
    let steering = engine::steering(state, handling);
    let pitch = engine::pitch(input, handling, control_contact);
    acc.local_angular.y += steering;
    acc.local_angular.x += pitch;

    // 6. Quadratic drag and 7. gravity.
    let drag = passive::quadratic_drag(velocity, forward_speed, control_contact);
    let gravity = passive::gravity(
        handling,
        state.body.mass,
        env.class_gravity_scale,
        control_grounded,
    );
    acc.world_force += drag;
    acc.world_force += gravity;

    // 8. Hover. Its load factor reads last frame's groundedness through
    //    `grounded_prev`, and it is what produces this frame's contact count.
    let hover = hover::evaluate(
        state,
        handling,
        env,
        raycaster,
        hover::target_height(handling, state.mag_lock_blend, state.leap_timer),
    );
    for probe in &hover.probes {
        if probe.contact {
            state.body.add_force_at_point(probe.force, probe.point);
        }
    }
    acc.world_force += hover.downforce;
    acc.local_angular += hover.local_angular_acceleration;
    acc.world_angular += hover.alignment_torque;

    // From here on, this frame's contacts.
    state.grounded = ShipState::quantise_grounded(hover.contacts);
    if control_grounded == 0.0 && state.grounded > 0.0 {
        state.time_since_landing = 0.0;
    } else {
        state.time_since_landing += dt;
    }
    let contact_grounded = state.grounded;
    let contact = contact_grounded > 0.0;

    // 9. Lateral grip, into the *local* force accumulator, and only once the leap
    //    timer has expired.
    let lateral_grip = if state.leap_timer > 0.0 {
        Vec3::ZERO
    } else {
        airbrake::lateral_grip(state, handling, contact_grounded)
    };
    acc.local_force += lateral_grip;

    // 11. Weathervane, 12. angular damping, 13. rolling resistance, 14. vertical
    //     damping. Step 10 is the dead in-air roll-levelling branch and is
    //     deliberately not ported; step 15 is the track-section force, which is not
    //     implemented.
    let weathervane = passive::weathervane(forward, velocity, contact);
    let local_angular_velocity = state.body.orientation.inverse() * state.body.angular_velocity;
    let angular_damping = passive::angular_damping(handling, local_angular_velocity);
    let rolling_resistance = passive::rolling_resistance(velocity, forward_speed);
    let vertical_damping = passive::vertical_damping(up, velocity, state.mag_lock_blend);

    acc.world_angular += weathervane;
    acc.local_angular += angular_damping;
    acc.world_force += rolling_resistance;
    acc.world_force += vertical_damping;

    drain(state, &acc);

    // Outside the accumulators, both of them deliberately.
    state.body.linear_velocity += airbrake::sideshift_velocity_delta(state, input, handling);
    state.body.position += hover.escape;

    Evaluated {
        accumulators: acc,
        hover,
        airbrake,
        engine: engine_force,
        brakes,
        lateral_grip,
        gravity,
        drag,
        rolling_resistance,
        vertical_damping,
        weathervane,
        angular_damping,
        steering,
        pitch,
        forward_speed,
        control_grounded,
        contact_grounded,
    }
}

/// Drains the four accumulators onto the body.
///
/// The two force accumulators hold **forces**: gravity carries `mass` explicitly and
/// no other term does, exactly as in the original, so nothing here multiplies by
/// mass. The two angular accumulators are treated as angular **accelerations**, so
/// they are combined in the body frame and multiplied by the inertia tensor once -
/// see [`Accumulators::local_angular`] for why that reading was chosen and what it
/// costs.
fn drain(state: &mut ShipState, acc: &Accumulators) {
    let orientation = state.body.orientation;

    state.body.add_force(orientation * acc.local_force);
    state.body.add_force(acc.world_force);

    let mut local_angular = acc.local_angular;
    local_angular += orientation.inverse() * acc.world_angular;
    state
        .body
        .add_torque(orientation * (local_angular * state.body.inertia));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::{CollisionWorld, Surface, TriangleSoup};
    use crate::params::{Antigrav, Brakes, Engine, Physical};
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

    /// Arbitrary round numbers, in the scaled in-memory form. **Not recovered
    /// values.**
    fn test_handling() -> Handling {
        Handling {
            antigrav: Antigrav {
                ride_height: 6.0,
                rebound: 1.0,
                ..Antigrav::default()
            },
            physical: Physical {
                mass: 1.0,
                normal_gravity: 10.0,
                flight_gravity: 4.0,
                ..Physical::default()
            },
            ..Handling::ZERO
        }
    }

    fn ship_at(height: f32) -> ShipState {
        ShipState {
            body: Body {
                position: Vec3::new(0.0, height, 0.0),
                ..Body::default()
            },
            ..ShipState::default()
        }
    }

    /// The staleness invariant, asserted where it is observable: on the frame a ship
    /// first touches down, gravity must still be the **airborne** one even though the
    /// ship ends that frame grounded.
    #[test]
    fn the_frame_a_ship_lands_uses_airborne_gravity_and_grounded_grip() {
        let handling = test_handling();
        let world = flat_floor();
        let mut state = ship_at(4.0);
        state.body.linear_velocity = Vec3::new(3.0, -1.0, 0.0);
        state.grounded = 0.0;

        let evaluated = evaluate(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        assert_eq!(evaluated.control_grounded, 0.0);
        assert_eq!(evaluated.contact_grounded, 1.0);
        assert_eq!(state.grounded, 1.0);
        // `flight_gravity` is 4 and `normal_gravity` is 10, so this is unambiguous.
        assert_eq!(evaluated.gravity.y, -handling.physical.flight_gravity);
    }

    /// And the reverse on the frame it leaves the ground: gravity is still the
    /// grounded one.
    #[test]
    fn the_frame_a_ship_takes_off_uses_grounded_gravity() {
        let handling = test_handling();
        let world = flat_floor();
        // Above `ride_height`, so no probe can reach the floor this frame.
        let mut state = ship_at(handling.antigrav.ride_height + 5.0);
        state.grounded = 1.0;

        let evaluated = evaluate(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        assert_eq!(evaluated.control_grounded, 1.0);
        assert_eq!(evaluated.contact_grounded, 0.0);
        assert_eq!(evaluated.gravity.y, -handling.physical.normal_gravity);
    }

    /// The engine reads the same stale value, which is what makes a landing frame
    /// give a ship 20 % thrust even though it ends that frame on the ground.
    #[test]
    fn the_engine_reads_the_stale_groundedness_too() {
        let handling = Handling {
            engine: Engine {
                accelcap: 1000.0,
                amount: 0.4,
                ..Engine::default()
            },
            ..test_handling()
        };
        let world = flat_floor();
        let full_throttle = ShipControls {
            thrust: 1.0,
            ..ShipControls::default()
        };

        let mut landing = ship_at(4.0);
        landing.grounded = 0.0;
        let landing_eval = evaluate(
            &mut landing,
            &full_throttle,
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        let mut settled = ship_at(4.0);
        settled.grounded = 1.0;
        let settled_eval = evaluate(
            &mut settled,
            &full_throttle,
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        assert_eq!(landing_eval.contact_grounded, settled_eval.contact_grounded);
        assert!(
            landing_eval.engine.thrust < settled_eval.engine.thrust,
            "landing thrust {} was not below settled thrust {}",
            landing_eval.engine.thrust,
            settled_eval.engine.thrust
        );
    }

    /// Braking is gated on the stale contact flag, not scaled by it, so a ship that
    /// was airborne last frame gets no brake force at all this frame.
    #[test]
    fn braking_does_nothing_in_the_air() {
        let handling = Handling {
            brakes: Brakes {
                amount: -0.5,
                gain: 400.0,
                falloff: 200.0,
            },
            ..test_handling()
        };
        let world = flat_floor();
        let held = ShipControls {
            airbrake_left: 1.0,
            airbrake_right: 1.0,
            ..ShipControls::default()
        };

        let mut airborne = ship_at(4.0);
        airborne.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
        airborne.brake = 100.0;
        airborne.grounded = 0.0;
        let airborne_eval = evaluate(
            &mut airborne,
            &held,
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );
        assert_eq!(airborne_eval.brakes, Vec3::ZERO);

        let mut grounded = ship_at(4.0);
        grounded.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
        grounded.brake = 100.0;
        grounded.grounded = 1.0;
        let grounded_eval = evaluate(
            &mut grounded,
            &held,
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );
        assert!(grounded_eval.brakes.z > 0.0);
    }

    /// The leap timer suppresses lateral grip entirely while it runs.
    #[test]
    fn lateral_grip_is_suppressed_while_the_leap_timer_runs() {
        let handling = Handling {
            antigrav: Antigrav {
                grip_ground: 2.0,
                grip_air: 1.0,
                ..test_handling().antigrav
            },
            ..test_handling()
        };
        let world = flat_floor();

        let mut sliding = ship_at(4.0);
        sliding.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
        sliding.grounded = 1.0;
        let free = evaluate(
            &mut sliding,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );
        assert_ne!(free.lateral_grip, Vec3::ZERO);

        let mut leaping = ship_at(4.0);
        leaping.body.linear_velocity = Vec3::new(10.0, 0.0, -60.0);
        leaping.grounded = 1.0;
        leaping.leap_timer = 1.0;
        let held = evaluate(
            &mut leaping,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );
        assert_eq!(held.lateral_grip, Vec3::ZERO);
    }

    /// The world force accumulator holds forces, not accelerations: gravity is the
    /// only term carrying `mass`, so doubling the mass must not double the drag.
    #[test]
    fn only_gravity_scales_with_mass() {
        let handling = test_handling();
        let world = flat_floor();

        let mut light = ship_at(4.0);
        light.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
        light.grounded = 1.0;
        let light_eval = evaluate(
            &mut light,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        let mut heavy = ship_at(4.0);
        heavy.body.mass = 2.0;
        heavy.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
        heavy.grounded = 1.0;
        let heavy_eval = evaluate(
            &mut heavy,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        assert_eq!(heavy_eval.gravity, light_eval.gravity * 2.0);
        assert_eq!(heavy_eval.drag, light_eval.drag);
        assert_eq!(heavy_eval.rolling_resistance, light_eval.rolling_resistance);
    }

    /// The hover target is built from `ride_height`, which was previously believed
    /// not to reach the force law at all.
    #[test]
    fn the_hover_target_follows_ride_height() {
        let mut handling = test_handling();
        let world = flat_floor();

        let mut low = ship_at(4.0);
        low.grounded = 1.0;
        let low_eval = evaluate(
            &mut low,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        handling.antigrav.ride_height *= 2.0;
        let mut high = ship_at(4.0);
        high.grounded = 1.0;
        let high_eval = evaluate(
            &mut high,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            1.0 / 60.0,
        );

        // Same height and the same everything else, but a higher target means more
        // lift.
        assert!(
            high_eval.hover.probes[0].force.y > low_eval.hover.probes[0].force.y,
            "{:?} was not above {:?}",
            high_eval.hover.probes[0].force,
            low_eval.hover.probes[0].force
        );
    }

    /// The end-to-end version of the steering sign fix: holding right must swing
    /// the nose toward the ship's own right axis over real ticks of `evaluate` and
    /// `integrate`, not away from it. A unit-level assertion on
    /// `engine::steering`'s return value alone would not catch a sign error
    /// introduced anywhere downstream in how the accumulators are drained onto the
    /// body, which is exactly the layer `docs/ghidra/functions/psp-pulse/engine.md`
    /// left as an open question until it was traced for this fix.
    #[test]
    fn holding_right_turns_the_ship_toward_its_own_right_axis() {
        let handling = Handling {
            turning: crate::params::Turning {
                amount: 0.02,
                gain: 400.0,
                falloff: 200.0,
            },
            ..Handling::ZERO
        };
        let world = flat_floor();
        // High enough above the floor that no hover probe ever makes contact, so
        // only steering is in play.
        let mut state = ship_at(1000.0);
        let controls = ShipControls {
            steer_x: 1.0,
            ..ShipControls::default()
        };
        let initial_right = state.body.right();

        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            crate::integrate::step(
                &mut state,
                &controls,
                &handling,
                &Environment::default(),
                &world,
                dt,
            );
        }

        assert!(
            state.body.forward().dot(initial_right) > 0.0,
            "forward was {:?}, expected a component toward the initial right axis {initial_right:?}",
            state.body.forward()
        );
    }
}
