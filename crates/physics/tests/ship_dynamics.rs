//! Structural invariants of one ship over many ticks.
//!
//! Every assertion here is about *structure*: something is finite, something is
//! bounded, something is symmetric, something is exactly zero, something is
//! quantised to three values. **Nothing here asserts a speed, a height or a turn
//! rate as though it were known**, because the force law comes from
//! `docs/physics/README.md`, which is static analysis that has never been run
//! under an emulator. A test that pinned a settling height would be pinning this
//! crate's own arithmetic and calling it a measurement. Behavioural comparison
//! against the original is M3's job; see
//! `docs/reverse-engineering/verification-protocol.md`.
//!
//! The parameter sets below are arbitrary round numbers chosen so the arithmetic
//! is checkable by hand. **They are not values from any ship**, and no handling
//! data is reproduced anywhere in this repository - see
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.

use std::cell::Cell;

use oag_core::math::Vec3;
use oag_physics::collide::{Ray, RaycastHit, Surface, TriangleSoup};
use oag_physics::params::{
    Airbrake, Antigrav, Brakes, Dimensions, Engine, Physical, Pitch, Turning,
};
use oag_physics::{
    Body, CollisionWorld, Environment, Handling, Raycaster, ShipControls, ShipState, Sideshift,
    step,
};

/// A fixed 60 Hz tick, which is what this project simulates at always. The
/// original integrates a measured delta instead; see
/// `docs/architecture/adr/0007-fixed-timestep-vs-original.md`.
const TICK: f32 = 1.0 / 60.0;

fn flat_floor(surface: Surface) -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-2000.0, 0.0, -2000.0],
            [-2000.0, 0.0, 2000.0],
            [2000.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        surface,
        0,
    ));
    world
}

/// Arbitrary round numbers, in the **already-scaled in-memory form** that
/// `oag_physics::Handling` holds: `airbrake.amount` and `airbrake.slidegrip` are what
/// the loader's `1e-4` would have stored, `engine.amount` what its `1e-3` would have,
/// and `brakes.amount` is negative because the loader's factor is `-0.01`. See
/// `oag_physics::params`.
///
/// **Not values from any ship.**
fn fixture() -> Handling {
    Handling {
        airbrake: Airbrake {
            // XML 20, 100.
            amount: 0.002,
            drag: 5.0,
            falloff: 400.0,
            gain: 800.0,
            turn: 3.0,
            slidegrip: 0.01,
            sideshift: 6.0,
        },
        antigrav: Antigrav {
            grip_air: 1.0,
            grip_ground: 2.0,
            landing_rebound: 2.0,
            rebound: 1.0,
            rebound_jump_time: 0.0,
            ride_height: 20.0,
        },
        brakes: Brakes {
            amount: -0.5,
            falloff: 200.0,
            gain: 400.0,
        },
        engine: Engine {
            accelcap: 20.0,
            amount: 0.4,
            // Dead in the original, so deliberately non-zero here: a ramp that took
            // effect would show up as a throttle that lags.
            falloff: 1.0,
            gain: 1.0,
            turbo: 0.0,
        },
        physical: Physical {
            flight_gravity: 10.0,
            mass: 1.0,
            normal_gravity: 10.0,
            track_gravity: 0.0,
        },
        pitch: Pitch {
            pitch_air: 0.02,
            pitch_ground: 0.005,
            pitch_damping: 3.0,
            // The hover target is `ride_height` plus this, so zero keeps the target
            // inside the probe's own reach.
            antigrav_height_adjust: 0.0,
        },
        turning: Turning {
            amount: 0.01,
            falloff: 200.0,
            gain: 400.0,
        },
        dimensions: Dimensions {
            height: 1.0,
            length: 4.0,
            width: 2.0,
            ..Dimensions::default()
        },
    }
}

fn ship_at(height: f32, handling: &Handling) -> ShipState {
    ShipState {
        body: Body {
            position: Vec3::new(0.0, height, 0.0),
            // The integrator divides by the body's mass and the force law
            // multiplies by the parameter set's. Keeping them equal is the
            // caller's job, which is this.
            mass: handling.physical.mass,
            ..Body::default()
        },
        ..ShipState::default()
    }
}

/// A raycaster that counts queries, so a test can see how many times the force
/// law ran.
struct CountingRaycaster {
    inner: CollisionWorld,
    casts: Cell<u32>,
}

impl CountingRaycaster {
    fn new(inner: CollisionWorld) -> Self {
        Self {
            inner,
            casts: Cell::new(0),
        }
    }
}

impl Raycaster for CountingRaycaster {
    fn raycast(&self, ray: Ray, skip: Option<u32>, include_reset: bool) -> Option<RaycastHit> {
        self.casts.set(self.casts.get() + 1);
        self.inner.raycast(ray, skip, include_reset)
    }
}

/// The counter-intuitive half of the specified integrator: forces are evaluated
/// **once per frame at full `dt`**, and the three sub-steps reuse that one
/// evaluation. Two probes therefore mean two raycasts per tick, not six.
#[test]
fn forces_are_evaluated_once_per_frame_and_not_once_per_sub_step() {
    let handling = fixture();
    let raycaster = CountingRaycaster::new(flat_floor(Surface::Floor));
    let mut state = ship_at(4.0, &handling);

    step(
        &mut state,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &raycaster,
        TICK,
    );
    assert_eq!(raycaster.casts.get(), 2);

    for _ in 0..9 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &raycaster,
            TICK,
        );
    }
    assert_eq!(raycaster.casts.get(), 20);
}

/// A ship let go above a floor settles at *some* finite height. Which height is
/// deliberately not asserted; that it neither diverges nor falls through is.
#[test]
fn a_ship_dropped_onto_a_floor_settles_at_a_finite_height() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    let mut state = ship_at(15.0, &handling);

    for tick in 0..3000 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
        assert!(
            state.body.position.is_finite(),
            "position diverged on tick {tick}: {:?}",
            state.body.position
        );
    }

    let height = state.body.position.y;
    assert!(
        height > 0.0,
        "the ship fell through the floor, ending at {height}"
    );
    assert!(
        height < handling.antigrav.ride_height,
        "the ship never came down, ending at {height}"
    );
    assert!(
        state.body.linear_velocity.length() < 1.0,
        "the ship never settled, still moving at {}",
        state.body.linear_velocity.length()
    );
    assert!(state.is_grounded());
}

/// The divergence check. With no input, an explicit Euler integrator on a stiff
/// spring is exactly where energy injection would show up, and it would show up
/// as an amplitude that grows every bounce.
#[test]
fn a_ship_with_no_input_does_not_gain_energy_over_thousands_of_ticks() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    let mut state = ship_at(12.0, &handling);

    let mut peak_speed = 0.0f32;
    let mut peak_height = state.body.position.y;
    let mut peak_spin = 0.0f32;

    for tick in 0..5000 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );

        peak_speed = peak_speed.max(state.body.linear_velocity.length());
        peak_height = peak_height.max(state.body.position.y);
        peak_spin = peak_spin.max(state.body.angular_velocity.length());

        assert!(
            state.body.position.is_finite() && state.body.linear_velocity.is_finite(),
            "diverged on tick {tick}"
        );
    }

    // A ship released below its hover target rises to it and no further: the probes
    // are `ride_height` long, so contact - and with it every lifting term - is lost
    // above that height.
    assert!(peak_speed < 100.0, "peak speed was {peak_speed}");
    assert!(
        peak_height <= handling.antigrav.ride_height,
        "peak height was {peak_height}"
    );
    // Spin is bounded but **not** zero, and that is the weathervane torque doing its
    // job: it turns the nose toward the direction of travel, and a ship settling onto
    // its air cushion is travelling vertically. Nothing rolls it, though.
    assert!(peak_spin < 1.0, "peak spin was {peak_spin}");
    assert_eq!(state.body.up().x, 0.0);
}

/// `grounded` is a contact count over two and nothing else, so it can only ever
/// hold one of three values however the ship is moving.
#[test]
fn groundedness_only_ever_holds_one_of_three_values() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    let mut state = ship_at(19.0, &handling);
    state.body.linear_velocity = Vec3::new(0.0, -30.0, 0.0);

    for tick in 0..2000 {
        step(
            &mut state,
            &ShipControls {
                steer_x: 0.5,
                airbrake_left: 1.0,
                ..ShipControls::default()
            },
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );

        assert!(
            state.grounded == 0.0 || state.grounded == 0.5 || state.grounded == 1.0,
            "grounded was {} on tick {tick}",
            state.grounded
        );
        assert!(
            state.grounded_prev == 0.0 || state.grounded_prev == 0.5 || state.grounded_prev == 1.0,
            "grounded_prev was {} on tick {tick}",
            state.grounded_prev
        );
    }
}

/// A ship spawned inside the floor is pushed back out by the penetration-escape
/// constraint and stays out, without a velocity change doing the work.
#[test]
fn a_ship_pressed_into_the_floor_never_ends_up_below_it() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    let mut state = ship_at(0.1, &handling);
    state.body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);

    for tick in 0..600 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
        assert!(
            state.body.position.y > 0.0,
            "ended up at {} on tick {tick}",
            state.body.position.y
        );
    }
}

/// With every tunable zeroed there is no gravity, no suspension - `ride_height`
/// is zero, so the probe segments have no length and cannot find the floor - and
/// no airbrake response. A ship at rest must therefore stay exactly where it is,
/// bit for bit, whatever the pilot does.
#[test]
fn a_zero_parameter_ship_at_rest_never_accelerates() {
    let world = flat_floor(Surface::Floor);
    let mut state = ship_at(10.0, &Handling::ZERO);
    state.body.mass = 1.0;
    let before = state.body;

    let controls = ShipControls {
        steer_x: 1.0,
        steer_y: -1.0,
        thrust: 1.0,
        airbrake_left: 1.0,
        airbrake_right: 0.5,
        sideshift: Sideshift::Right,
    };

    for _ in 0..600 {
        step(
            &mut state,
            &controls,
            &Handling::ZERO,
            &Environment::default(),
            &world,
            TICK,
        );
    }

    assert_eq!(state.body.position, before.position);
    assert_eq!(state.body.linear_velocity, Vec3::ZERO);
    assert_eq!(state.body.angular_velocity, Vec3::ZERO);
    assert_eq!(state.body.orientation, before.orientation);
    assert_eq!(state.grounded, 0.0);
}

/// Mirroring the airbrakes mirrors the trajectory. Reflection through the `x = 0`
/// plane negates X and leaves Y and Z, and it holds **exactly** rather than within
/// a tolerance, because no term in the force law treats one side preferentially and
/// float negation is exact.
#[test]
fn mirrored_airbrake_input_produces_a_mirrored_trajectory() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);

    let mut left_heavy = ship_at(19.0, &handling);
    let mut right_heavy = ship_at(19.0, &handling);
    // Forward is -Z.
    left_heavy.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);
    right_heavy.body.linear_velocity = Vec3::new(0.0, 0.0, -60.0);

    let braking_left = ShipControls {
        steer_x: -0.5,
        airbrake_left: 1.0,
        ..ShipControls::default()
    };
    let braking_right = ShipControls {
        steer_x: 0.5,
        airbrake_right: 1.0,
        ..ShipControls::default()
    };

    // Five seconds, which is bounded deliberately: the recovered roll stiffness and
    // roll damping put this model just outside the explicit-Euler stability limit
    // (see `a_ship_rolled_off_level_is_pulled_back_toward_level`), so both ships
    // eventually tumble. They tumble *in mirror image*, and the mirror assertions
    // below keep holding - but once a coordinate reaches `NaN`, `NaN != NaN` makes
    // them vacuous rather than true. The horizon stops short of that.
    for tick in 0..300 {
        step(
            &mut left_heavy,
            &braking_left,
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
        step(
            &mut right_heavy,
            &braking_right,
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );

        assert_eq!(
            left_heavy.body.position.x, -right_heavy.body.position.x,
            "x diverged on tick {tick}"
        );
        assert_eq!(
            left_heavy.body.position.y, right_heavy.body.position.y,
            "y diverged on tick {tick}"
        );
        assert_eq!(
            left_heavy.body.position.z, right_heavy.body.position.z,
            "z diverged on tick {tick}"
        );
        assert_eq!(left_heavy.grounded, right_heavy.grounded);
        assert_eq!(
            left_heavy.body.angular_velocity.y,
            -right_heavy.body.angular_velocity.y
        );
    }

    // The two did actually go somewhere, or the assertions above are vacuous.
    assert_ne!(left_heavy.body.position.x, 0.0);
}

/// A mag floor is ordinary floor with a different tag, so a ship must behave
/// identically over one, bit for bit. Confidence 92 on that being the whole of
/// the difference; the magnetic hold that a dedicated probe would additionally
/// trigger is **not implemented**, because its force law was not decoded.
#[test]
fn a_mag_floor_is_indistinguishable_from_a_floor_to_the_suspension() {
    let handling = fixture();
    let floor = flat_floor(Surface::Floor);
    let mag = flat_floor(Surface::MagFloor);

    let mut over_floor = ship_at(12.0, &handling);
    let mut over_mag = ship_at(12.0, &handling);

    for _ in 0..1200 {
        step(
            &mut over_floor,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &floor,
            TICK,
        );
        step(
            &mut over_mag,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &mag,
            TICK,
        );
    }

    assert_eq!(over_floor.body, over_mag.body);
}

/// A wall is not hoverable and a reset collider is skipped by ordinary raycasts,
/// so neither holds a ship up.
#[test]
fn a_ship_over_a_wall_or_a_reset_collider_is_never_grounded() {
    let handling = fixture();

    for surface in [Surface::Wall, Surface::Reset] {
        let world = flat_floor(surface);
        let mut state = ship_at(10.0, &handling);

        for _ in 0..120 {
            step(
                &mut state,
                &ShipControls::default(),
                &handling,
                &Environment::default(),
                &world,
                TICK,
            );
            assert!(!state.is_grounded(), "{surface:?} held the ship up");
        }

        assert!(state.body.position.y < 10.0, "the ship did not fall");
    }
}

/// Nothing in the model commands roll, so a level ship over a level floor never
/// acquires any - exactly, not approximately.
///
/// **Not "never pitches", which this test used to claim.** The weathervane torque
/// turns the nose toward the direction of *travel* in three dimensions, so a ship with
/// any vertical velocity gets a nose-down or nose-up term out of it, and a ship
/// settling onto its air cushion has vertical velocity. That pitch is a property of the
/// recovered term, not a bug. Roll is different: `cross(forward, velocity)` has no roll
/// component while the velocity stays in the ship's own vertical plane, and no control
/// input writes an angular Z at all.
#[test]
fn a_level_ship_over_a_level_floor_never_rolls() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    let mut state = ship_at(19.0, &handling);

    for tick in 0..1200 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );

        assert_eq!(
            state.body.angular_velocity.z, 0.0,
            "acquired roll on tick {tick}"
        );
        assert_eq!(state.body.up().x, 0.0, "rolled on tick {tick}");
    }

    // Still the right way up, and still level in roll.
    assert!(state.body.up().y > 0.9);
}

/// A ship rolled slightly off a flat floor is pulled **toward** level by the
/// surface-alignment torque.
///
/// # What is asserted
///
/// That the roll shrinks over the first quarter of the alignment oscillation's period.
/// That is the whole-ship consequence of the torque's direction, and it is the strongest
/// statement this model supports: with the sign as `docs/physics/README.md` writes it,
/// the roll grows from the first tick instead. The direction itself is pinned
/// unconditionally, on the torque rather than through a simulation, by
/// `oag_physics::hover`'s
/// `the_alignment_torque_points_from_the_ships_up_axis_toward_the_surface_normal`.
///
/// # Long-run convergence, which this used to say did not happen
///
/// It does now, and the change is a measurement rather than a tune. This comment used
/// to argue that roll could not converge: with the transcribed alignment gain of `400`
/// against `Ship_ApplyAngularDamping`'s `-2.0`, explicit Euler is outside its stability
/// limit and a rolled ship's oscillation grows until it tumbles. Two corrections
/// retired that.
///
/// The stability arithmetic itself was wrong in a way that made it look marginal. The
/// bound is not `h <= c / k` on the sub-step, because the acceleration is computed once
/// per frame and held across all three sub-steps, so the governing step is the frame.
/// The real condition is `c >= (2/3) * k * H`, and at `k = 400` the shortfall was
/// 2.22-fold, not 11 %.
///
/// And the gain is not 400. It was measured on the original by rolling the ship
/// `0.25 rad` through PPSSPP's debugger and tracing the recovery: a clean damped cosine
/// crossing zero at frame 21, so `omega = 4.49 rad/s` and the stiffness is about
/// **20.2**. The damping came out near 1.6 against the transcribed 2.0, so that
/// constant is roughly right and the gain was the outlier. See
/// `oag_physics::hover::ALIGNMENT_GAIN` for the trace and the caveats - in particular
/// that whether the factor of twenty is the gain itself or a moment of inertia this
/// crate's accumulators bypass is **not** resolved.
///
/// So this test now asserts what the original does: a monotone, non-oscillatory return
/// toward level, on the measured timescale rather than the transcribed one.
#[test]
fn a_ship_rolled_off_level_is_pulled_back_toward_level() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    let mut state = ship_at(19.0, &handling);
    state.body.orientation = oag_core::math::Quat::from_rotation_z(0.02);

    let initial_roll = state.body.up().x.abs();
    let mut roll = initial_roll;

    // The measured quarter period is about 21 ticks, so the whole of this window is
    // inside the first monotone descent and the roll must fall on every one of them.
    for tick in 0..14 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );

        let next = state.body.up().x.abs();
        assert!(
            next < roll,
            "roll grew from {roll} to {next} on tick {tick}, so the torque is not aligning"
        );
        roll = next;
    }

    // The measured cosine reaches 0.5 at `omega * t = pi / 3`, about 14 ticks at
    // 4.49 rad/s. This fixture sits on its own suspension as well, whose probes add
    // stiffness the perturbed original did not have, so it lands a little above that;
    // the bound is set where the *shape* is pinned - a substantial monotone decay over
    // a quarter period - without pretending the fixture reproduces the trace exactly.
    assert!(
        roll < initial_roll * 0.65,
        "roll only came down from {initial_roll} to {roll} in 14 ticks"
    );
}

/// The landing window drives `landing_rebound` in place of `rebound` for its
/// first 0.2 s, so the clock behind it has to actually reset on touchdown and
/// advance otherwise.
#[test]
fn the_landing_clock_resets_on_touchdown_and_advances_in_the_air() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);

    // Well above `ride_height`, so the probes find nothing.
    let mut state = ship_at(60.0, &handling);
    let airborne_before = state.time_since_landing;

    for _ in 0..30 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
    }
    assert!(state.time_since_landing > airborne_before);
    assert!(!state.is_grounded());

    // Now drop it into contact and watch the clock reset on the edge.
    state.body.position.y = 10.0;
    step(
        &mut state,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        TICK,
    );
    assert!(state.is_grounded());
    assert_eq!(state.time_since_landing, 0.0);

    // And that it does not reset again while it stays in contact.
    step(
        &mut state,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        TICK,
    );
    assert_eq!(state.time_since_landing, TICK);
}

/// A settled ship with no sideshift is unchanged by a frame the outer clock
/// reports as zero.
///
/// **Narrower than "a zero-length frame changes nothing", deliberately.** The
/// integrator is a no-op at `dt = 0`, but force evaluation still runs, and two of
/// its effects are applied outside the accumulators: the penetration-escape
/// teleport and the sideshift's velocity change. A penetrating ship or a held
/// sideshift therefore *does* move on a zero-length frame. Whether the original
/// gates those on the delta is not recorded, so nothing here gates them either,
/// and the test says only what it can see.
#[test]
fn a_settled_ship_is_unchanged_by_a_zero_length_frame() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    let mut state = ship_at(4.0, &handling);

    for _ in 0..60 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
    }

    let before = state;
    step(
        &mut state,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        0.0,
    );

    assert_eq!(state.body.position, before.body.position);
    assert_eq!(state.body.linear_velocity, before.body.linear_velocity);
    assert_eq!(state.body.orientation, before.body.orientation);
    assert_eq!(state.time_since_landing, before.time_since_landing);
}

/// A frame longer than the original will integrate is clamped, so an enormous
/// delta and the clamp itself must produce the same tick.
#[test]
fn an_overlong_frame_is_clamped_rather_than_integrated() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);

    let mut huge = ship_at(4.0, &handling);
    let mut clamped = ship_at(4.0, &handling);

    step(
        &mut huge,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        10.0,
    );
    step(
        &mut clamped,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        oag_physics::MAX_DT,
    );

    assert_eq!(huge.body, clamped.body);
}

/// A sideshift bypasses the force accumulators entirely and changes velocity
/// directly, and it is one-shot: the frame after, nothing more is added.
#[test]
fn a_sideshift_is_a_one_shot_change_in_velocity() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    let mut state = ship_at(4.0, &handling);

    let shifted = step(
        &mut state,
        &ShipControls {
            sideshift: Sideshift::Right,
            ..ShipControls::default()
        },
        &handling,
        &Environment::default(),
        &world,
        TICK,
    );
    // The impulse is not visible in the accumulated forces, by design.
    assert_eq!(shifted.airbrake.world_force, Vec3::ZERO);
    assert!(state.body.linear_velocity.x > 0.0);

    let after = state.body.linear_velocity.x;
    step(
        &mut state,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        TICK,
    );
    // Lateral grip is now working against it, so it must not have grown.
    assert!(state.body.linear_velocity.x <= after);
}

/// Under a ceiling, the probes still find the surface, because they cast along the
/// **ship's own up axis** rather than along world up, and the spring they produce acts
/// along that same axis.
///
/// # This does not assert that the ship is held there, because it is not
///
/// The test that used to live here asserted an inverted ship stayed on its ceiling. It
/// was asserting behaviour the recovered force law does not have, and
/// `docs/ghidra/functions/psp-pulse/engine.md` is what showed that: the inline gravity
/// term writes **world `.y` only**, `track_gravity` reaches the force law solely through
/// the hover spring's *magnitude*, and the hover spring always pushes the ship *away*
/// from the surface it found. So nothing in what has been recovered pulls a ship toward
/// a ceiling, and an inverted ship falls off.
///
/// Which is consistent rather than alarming: inverted sections in this game are
/// magstrips, and the magnetic hold that would do the holding is **not implemented**
/// because its force law was not decoded. This test therefore pins the part that is
/// recovered - the probe direction and the force axis - and deliberately stops there.
#[test]
fn a_ship_under_a_ceiling_probes_along_its_own_up_axis() {
    let handling = fixture();

    // A floor wound so that its normal points down, with the ship below it and rolled
    // 180 degrees so its own up axis points away from it.
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-2000.0, 0.0, 2000.0],
            [-2000.0, 0.0, -2000.0],
            [2000.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        Surface::Floor,
        0,
    ));

    let mut state = ship_at(-12.0, &handling);
    state.body.orientation = oag_core::math::Quat::from_rotation_z(core::f32::consts::PI);
    let env = Environment {
        track_up: Vec3::NEG_Y,
        ..Environment::default()
    };

    let up = state.body.up();
    assert!(up.y < -0.99, "the ship is not inverted: up is {up:?}");

    let evaluated = step(
        &mut state,
        &ShipControls::default(),
        &handling,
        &env,
        &world,
        TICK,
    );

    // Both probes found the ceiling, 12 units away along the ship's own up axis, and
    // a world-up probe would have found nothing at all.
    assert_eq!(evaluated.hover.contacts, 2);
    assert!((evaluated.hover.probes[0].height - 12.0).abs() < 1e-4);

    // And the force they produce lies along that axis rather than along world up.
    let force = evaluated.hover.probes[0].force;
    assert_ne!(force, Vec3::ZERO);
    assert!(
        force.cross(up).length() < 1e-3,
        "force {force:?} was not along the ship's up axis {up:?}"
    );
}
