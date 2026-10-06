//! Structural invariants of one ship over many ticks.
//!
//! Every assertion is about *structure*: something is finite, bounded, symmetric, exactly zero,
//! or quantised to three values. **Nothing asserts a speed, height or turn rate as though it
//! were known**: a test pinning a settling height would pin this crate's own arithmetic and call
//! it a measurement. Behavioural comparison against the original is M3's job
//! (`docs/reverse-engineering/verification-protocol.md`).
//!
//! The parameter sets are arbitrary round numbers chosen so the arithmetic is checkable by hand.
//! **They are not values from any ship**
//! (`docs/architecture/adr/0006-no-copyrighted-content.md`).

use std::cell::Cell;

use oag_core::math::Vec3;
use oag_physics::collide::{Ray, RaycastHit, Surface, TriangleSoup};
use oag_physics::params::{
    Airbrake, Antigrav, Brakes, Dimensions, Engine, Physical, Pitch, SpeedupPads, Turning,
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
/// `oag_physics::Handling` holds: `airbrake.amount`/`slidegrip` are what the
/// loader's `1e-4` would have stored, `engine.amount` what its `1e-3` would
/// have, and `brakes.amount` is negative because the loader's factor is
/// `-0.01`. See `oag_physics::params`. **Not values from any ship.**
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
        // Zero on purpose here and for the three roll fields: nothing hands
        // `evaluate` a pad hit, so a magnitude could only mask a stray boost.
        speedup_pads: SpeedupPads::default(),
        speedpad_jump: 0.0,
        roll_cost: 0.0,
        roll_speed: 0.0,
        roll_turbotime: 0.0,
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

/// The counter-intuitive half of the integrator: forces are evaluated **once per frame at full
/// `dt`** and the three sub-steps reuse that evaluation, so two probes mean two raycasts per
/// force evaluation, not six. Asserted against `forces::evaluate`, not `step`, which also runs
/// the wall constraint with probes of its own: counting `step` would measure both and pin
/// neither.
#[test]
fn forces_are_evaluated_once_per_frame_and_not_once_per_sub_step() {
    let handling = fixture();
    let raycaster = CountingRaycaster::new(flat_floor(Surface::Floor));
    let mut state = ship_at(4.0, &handling);

    for _ in 0..10 {
        state.body.clear_accumulators();
        oag_physics::forces::evaluate(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &raycaster,
            TICK,
        );
        oag_physics::integrate(&mut state.body, TICK);
    }
    assert_eq!(raycaster.casts.get(), 20);
}

/// The whole per-frame query cost is the same every tick: whatever `step` costs in raycasts it
/// must not grow with the sub-step count or drift between ticks (ten ticks cost ten times one,
/// where a force per sub-step would cost three times that). It is only *constant* because
/// nothing in this world responds to the wall constraint (a swept-query hit short-circuits the
/// eight hull probes), so after adding a wall to the fixture a failure is the fixture changing.
#[test]
fn the_query_cost_of_a_tick_does_not_grow_with_the_sub_steps() {
    let handling = fixture();
    let raycaster = CountingRaycaster::new(flat_floor(Surface::Floor));
    let mut state = ship_at(4.0, &handling);

    let tick = |state: &mut _, raycaster: &CountingRaycaster| {
        step(
            state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            raycaster,
            TICK,
        );
    };

    // One warm-up tick, so the ship is moving and the swept query is live throughout.
    tick(&mut state, &raycaster);
    let before = raycaster.casts.get();
    tick(&mut state, &raycaster);
    let per_tick = raycaster.casts.get() - before;
    assert!(
        per_tick >= 2,
        "the two hover probes at least, got {per_tick}"
    );

    for _ in 0..9 {
        tick(&mut state, &raycaster);
    }
    assert_eq!(raycaster.casts.get() - before, per_tick * 10);
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

/// The divergence check: with no input, explicit Euler on a stiff spring is where energy
/// injection would show, as an amplitude growing every bounce.
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

    // A ship released below its hover target rises to it and no further: the probes are
    // `ride_height` long, so contact (and every lifting term) is lost above that height.
    assert!(peak_speed < 100.0, "peak speed was {peak_speed}");
    assert!(
        peak_height <= handling.antigrav.ride_height,
        "peak height was {peak_height}"
    );
    // Spin is bounded but **not** zero: the weathervane turns the nose toward the direction of
    // travel and a ship settling onto its cushion travels vertically. Nothing rolls it.
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

/// A ship spawned inside the floor is pushed back out by the penetration-escape constraint and
/// stays out, without a velocity change doing the work.
///
/// The spawn height moved with the recovered probe geometry and the assertion did not: the
/// probes hang `1.125` below the centre of mass (`oag_physics::hover::probe_offsets`), so a
/// centre `0.1` above the floor puts both a unit *underneath* it with nothing to escape from (as
/// in the original, whose ray starts at the probe and points down). The ship is placed where the
/// constraint applies: probes `0.1` into the floor, a centre `0.1 + 1.125` above it.
#[test]
fn a_ship_pressed_into_the_floor_never_ends_up_below_it() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    let probe_drop = oag_physics::hover::PROBE_DROP_RAW * oag_physics::hover::TARGET_GLOBAL_SCALE;
    let mut state = ship_at(0.1 + probe_drop, &handling);
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
            state.body.position.y > probe_drop,
            "a probe ended up at {} on tick {tick}",
            state.body.position.y - probe_drop
        );
    }
}

/// With every tunable zeroed there is no gravity, no suspension (`ride_height`
/// is zero, so the probes have no length) and no airbrake response either.
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
        // Every gesture input too, the barrel roll's taps included.
        shift_modifier: true,
        shift_tap_left: true,
        shift_tap_right: true,
        roll_tap_left: true,
        roll_tap_right: true,
        roll_request: Some(oag_physics::barrel_roll::TapDirection::Right),
        roll_shield_floor: 0.2,
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

    // Five seconds, bounded on purpose: the recovered roll stiffness and damping put this model
    // just outside the explicit-Euler stability limit (see
    // `a_ship_rolled_off_level_is_pulled_back_toward_level`), so both ships eventually tumble in
    // mirror image, but once a coordinate reaches `NaN`, `NaN != NaN` makes the mirror assertions
    // vacuous. The horizon stops short of that.
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

/// A mag floor is ordinary floor with a different tag, so a ship must behave identically over one,
/// bit for bit. Confidence 92 on that being the whole of the difference; the dedicated mag
/// probe's hold is exercised in the ceiling tests below.
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

/// Nothing in the model commands roll, so a level ship over a level floor never acquires any,
/// exactly. **Not "never pitches"**: the weathervane turns the nose toward the direction of
/// *travel* in three dimensions, so a ship settling onto its cushion (vertical velocity) gets a
/// pitch term out of it, a property of the recovered term. Roll differs: `cross(forward,
/// velocity)` has no roll component while the velocity stays in the ship's vertical plane, and no
/// control input writes an angular Z.
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

/// A ship rolled slightly off a flat floor is pulled **toward** level by the surface-alignment
/// torque.
///
/// Asserted: the roll shrinks over the first quarter of the alignment oscillation's period, the
/// whole-ship consequence of the torque's direction (with the page's literal sign it grows from
/// the first tick). The direction itself is pinned unconditionally, on the torque, by
/// `oag_physics::hover`'s
/// `the_alignment_torque_points_from_the_ships_up_axis_toward_the_surface_normal`.
///
/// Long-run convergence now holds, by measurement and not by tuning. This test once argued roll
/// could not converge: with the transcribed gain `400` against `Ship_ApplyAngularDamping`'s
/// `-2.0`, explicit Euler is outside its stability limit and the oscillation grows. Two
/// corrections retired that: the bound is not `h <= c / k` on the sub-step but
/// `c >= (2/3) * k * H` on the frame (acceleration computed once per frame), a 2.22-fold
/// shortfall, not 11 %; and the gain is not 400 but about **20.2**, measured by rolling the
/// original `0.25 rad` through PPSSPP's debugger (a damped cosine crossing zero at frame 21,
/// `omega = 4.49 rad/s`; damping near 1.6). Trace, caveats and the unresolved gain-or-inertia
/// question: `oag_physics::hover::ALIGNMENT_GAIN` and `docs/physics/README.md`, "Alignment gain:
/// the measurements behind the numbers".
///
/// So this asserts what the original does: a monotone, non-oscillatory return toward level on
/// the measured timescale.
#[test]
fn a_ship_rolled_off_level_is_pulled_back_toward_level() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    // Resting height recomputed for the recovered geometry: the probes hang `1.125` below the
    // centre and the spring rests where it carries the load, `normal_gravity / (0.8 *
    // (normal_gravity + track_gravity))` below the `0.75 * ride_height` target, `1.25` here. At
    // the old `19.0` both probes are above their own reach (the reach is the target) and it falls.
    let mut state = ship_at(15.0 - 1.25 + 1.125, &handling);
    state.body.orientation = oag_core::math::Quat::from_rotation_z(0.02);

    let initial_roll = state.body.up().x.abs();
    let mut roll = initial_roll;

    // The measured quarter period is about 21 ticks, so this window is inside the first monotone
    // descent and the roll must fall on every tick.
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

    // The measured cosine reaches 0.5 at `omega * t = pi / 3`, about 14 ticks at 4.49 rad/s. This
    // fixture's own suspension adds stiffness the perturbed original lacked, so it lands a little
    // above; the bound pins the *shape* (a substantial monotone decay over a quarter period)
    // without pretending to reproduce the trace.
    assert!(
        roll < initial_roll * 0.65,
        "roll only came down from {initial_roll} to {roll} in 14 ticks"
    );
}

/// The landing window drives `landing_rebound` in place of `rebound` for its first 0.2 s, and the
/// clock behind it is armed **in the air**, not on touchdown: `Ship_UpdateCraft` (`0x08849df0`)
/// runs the airborne clock and `Ship_HoverTwoPoint` zeroes the landing clock once it passes
/// `rebound_jump_time`, so on touchdown the landing clock is already at zero and starts counting
/// (`ShipState::time_airborne`).
#[test]
fn the_landing_clock_is_armed_in_the_air_and_runs_from_touchdown() {
    // The fixture's `rebound_jump_time` is zero, so any flight at all arms it.
    let handling = fixture();
    let world = flat_floor(Surface::Floor);

    // Well above `ride_height`, so the probes find nothing.
    let mut state = ship_at(60.0, &handling);

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
    assert!(!state.is_grounded());
    assert!(state.time_airborne > 0.0, "the airborne clock did not run");
    assert_eq!(
        state.time_since_landing, 0.0,
        "a flight past `rebound_jump_time` should have armed the landing response"
    );

    // Now drop it into contact. The landing clock starts from the zero it was
    // armed at, and the airborne clock is the one that resets.
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
    assert_eq!(state.time_airborne, 0.0);
    assert_eq!(state.time_since_landing, TICK);

    // And it keeps counting while it stays in contact.
    step(
        &mut state,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        TICK,
    );
    assert_eq!(state.time_since_landing, TICK * 2.0);
}

/// A hop shorter than `rebound_jump_time` never arms the landing response. Rough ground flickers
/// in and out of contact constantly; resetting the landing clock on every touchdown edge (this
/// crate's behaviour until `rebound_jump_time` was read) applies `landing_rebound` almost
/// permanently instead of on real landings.
#[test]
fn a_hop_shorter_than_rebound_jump_time_never_arms_the_landing_response() {
    let mut handling = fixture();
    handling.antigrav.rebound_jump_time = 1.0;
    let world = flat_floor(Surface::Floor);

    let mut state = ship_at(10.0, &handling);
    for _ in 0..10 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
    }
    assert!(state.is_grounded(), "the fixture should settle in contact");
    let settled = state.time_since_landing;

    // Lift it clear for a quarter of a second: a real hop, well short of the parameter's second.
    state.body.position.y = 60.0;
    for _ in 0..15 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
    }
    assert!(!state.is_grounded());
    assert!(
        state.time_airborne > 0.2 && state.time_airborne < 1.0,
        "the hop should be shorter than `rebound_jump_time`, was {}",
        state.time_airborne
    );
    assert_eq!(
        state.time_since_landing, settled,
        "a short hop must leave the landing clock alone"
    );
}

/// A settled ship with no sideshift is unchanged by a frame the outer clock reports as zero.
///
/// **Narrower than "a zero-length frame changes nothing", deliberately**: the integrator is a
/// no-op at `dt = 0` but force evaluation still runs, and two effects land outside the
/// accumulators (the penetration-escape teleport and the sideshift's velocity change), so a
/// penetrating ship or a held sideshift does move. Whether the original gates those on the delta
/// is unrecorded, so nothing here gates them and the test says only what it can see.
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

/// A sideshift is a **force**, it lasts its own timer, and it is grounded-only.
///
/// Replaces `a_sideshift_is_a_one_shot_change_in_velocity`, which pinned the old guess of a
/// velocity change applied once. `Ship_UpdateAirbrakes`' tail calls `Body_AddForceWorld` while a
/// per-side timer runs, so it is an ordinary world force for
/// `oag_physics::airbrake::SIDESHIFT_DURATION`, divided by mass by the integrator (listing and
/// direction: `oag_physics::airbrake::sideshift_force`). Still pinned: it is not in
/// `Evaluated::airbrake`'s own force (a separate call in the same function), and it does not push
/// for ever.
#[test]
fn a_sideshift_is_a_grounded_force_that_lasts_its_own_timer() {
    let handling = fixture();
    let world = flat_floor(Surface::Floor);
    let target = oag_physics::hover::target_height(&handling, 0.0, 0.0);
    let drop = oag_physics::hover::PROBE_DROP_RAW * oag_physics::hover::TARGET_GLOBAL_SCALE;
    let mut state = ship_at(target - 1.25 + drop, &handling);
    // A grounded craft: the sideshift reads last frame's contact flag.
    state.grounded = 1.0;

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
    // Not part of the airbrake block's own force, by design.
    assert_eq!(shifted.airbrake.world_force, Vec3::ZERO);
    assert!(state.body.linear_velocity.x > 0.0);

    // It keeps pushing after the input has gone, which the one-shot shape did not.
    let after_one = state.body.linear_velocity.x;
    step(
        &mut state,
        &ShipControls::default(),
        &handling,
        &Environment::default(),
        &world,
        TICK,
    );
    assert!(state.body.linear_velocity.x > after_one);

    // And it stops: the push lasts 0.2 s, so the lateral speed peaks inside it and is lower
    // afterwards rather than growing without bound.
    let mut peak: f32 = 0.0;
    let mut samples = Vec::new();
    for _ in 0..60 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
        let lateral = state.body.linear_velocity.dot(state.body.right());
        peak = peak.max(lateral);
        samples.push(lateral);
    }
    assert!(
        samples.last().copied().unwrap() < peak,
        "lateral speed never came off its peak {peak}: {samples:?}"
    );
}

/// Under a ceiling the probes still find the surface, because they cast along the **ship's own up
/// axis**, not world up, and the spring they produce acts along that axis.
///
/// **This does not assert the ship is held there.** The old test asserted an inverted ship stayed
/// on its ceiling, behaviour the recovered force law lacks (`docs/ghidra/functions/psp-pulse-usa/engine.md`):
/// the inline gravity writes **world `.y` only**, `track_gravity` reaches the force law solely
/// through the hover spring's *magnitude*, and the spring always pushes the ship *away* from the
/// surface it found, so nothing pulls a ship toward a ceiling and an inverted ship falls off.
/// Inverted sections here are magstrips, whose hold is [`oag_physics::maglock`]. This test is
/// unchanged by it on purpose: the ceiling is tagged [`Surface::Floor`] and the mag probe accepts
/// only [`Surface::MagFloor`], so the blend stays zero. The companion below is the same ship under
/// a *mag* ceiling.
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
    // A ceiling section: `down` points up, out of the surface the ship hangs
    // under. The mag probe needs it to have a direction to cast along at all.
    let env = Environment {
        track_sample: Some(oag_physics::TrackSample {
            position: Vec3::new(0.0, -oag_physics::maglock::SPLINE_LIFT, 0.0),
            down: Vec3::Y,
        }),
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

    // Both probes found the ceiling along the ship's own up axis, where a world-up
    // probe would have found nothing at all. The distance is `12` from the centre
    // of mass minus the `1.125` the probes hang toward it - the recovered offset,
    // which on an inverted ship points at the surface rather than away from it.
    assert_eq!(evaluated.hover.contacts, 2);
    assert!((evaluated.hover.probes[0].height - (12.0 - 1.125)).abs() < 1e-4);

    // And the force they produce lies along that axis rather than along world up.
    let force = evaluated.hover.probes[0].force;
    assert_ne!(force, Vec3::ZERO);
    assert!(
        force.cross(up).length() < 1e-3,
        "force {force:?} was not along the ship's up axis {up:?}"
    );
}

/// The other half of the test above: under a **mag** ceiling the ship stays. Same geometry, pose
/// and entry point, one tag changed. Asserted through [`step`] and not `maglock` directly, because
/// the likely break is the wiring: the hold is not a force, so nothing in the accumulators would
/// miss it if the call disappeared.
///
/// It pins the property a torque implementation would fail: the attitude is slaved to the surface
/// while the **angular velocity stays whatever it was**
/// (`docs/physics/cornering-ground-truth.md` for the lap that measured it).
#[test]
fn a_ship_under_a_mag_ceiling_is_held_there_without_any_angular_velocity() {
    // A shorter ride height than the shared fixture's: the mag probe's reach is a fixed ~10
    // units (`5 * |up - down|`) while the hover probes' is `ride_height`. At 20 the hold would
    // park the ship four units beyond its own probe's reach and chatter; the original's ride
    // heights are around 5.5, where the two are consistent.
    let handling = Handling {
        antigrav: Antigrav {
            ride_height: 5.0,
            ..fixture().antigrav
        },
        ..fixture()
    };

    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-2000.0, 0.0, 2000.0],
            [-2000.0, 0.0, -2000.0],
            [2000.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        Surface::MagFloor,
        0,
    ));

    let mut state = ship_at(-4.0, &handling);
    // Inverted, but a fifth of a radian out of true, so the hold has something to
    // correct and a "does nothing" implementation cannot pass.
    state.body.orientation = oag_core::math::Quat::from_rotation_z(core::f32::consts::PI - 0.2);
    let env = Environment {
        track_sample: Some(oag_physics::TrackSample {
            position: Vec3::new(0.0, -oag_physics::maglock::SPLINE_LIFT, 0.0),
            down: Vec3::Y,
        }),
        ..Environment::default()
    };

    for _ in 0..30 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &env,
            &world,
            TICK,
        );
    }

    assert_eq!(state.mag_lock_blend, 1.0, "the strip never locked");
    // Not exact, and the residue is the mechanism: the hold runs inside force evaluation and the
    // integrator then rotates the basis by the angular velocity the frame's torques left, so the
    // ship sits a milliradian off the axis. A hold that ran after the integrator would read
    // exactly `NEG_Y` and be in the wrong place.
    let up = state.body.up();
    assert!(
        (up - Vec3::NEG_Y).length() < 1e-2,
        "the hold left up at {up:?}, not slaved to the ceiling's normal"
    );
    // Held near the strip rather than falling away from it: `0.8 * target`, and
    // the ship hangs *below* a ceiling at y = 0.
    let target = oag_physics::hover::target_height(&handling, state.mag_lock_blend, 0.0);
    assert!(
        (state.body.position.y + 0.8 * target).abs() < 0.5,
        "the ship sits at y = {}, not {} below the strip",
        state.body.position.y,
        0.8 * target
    );
}

/// A vertical quad in the plane `x = at`, tall and wide enough that a ship cannot go round it,
/// tagged with whatever surface the caller wants (the two tests below need the *same* geometry
/// under two tags).
///
/// The winding is load-bearing: hull contacts are single-sided, so the raw `(b - a) x (c - a)`
/// normal has to face the ship, which approaches from `x < at`. The other way round the quad is
/// a back face the hull probes ignore, as the original's do.
fn vertical_quad_at(at: f32, surface: Surface) -> TriangleSoup {
    TriangleSoup::new(
        vec![
            [at, -500.0, -500.0],
            [at, 500.0, -500.0],
            [at, 500.0, 500.0],
            [at, -500.0, 500.0],
        ],
        vec![[0, 2, 1], [0, 3, 2]],
        Vec::new(),
        surface,
        1,
    )
}

fn wall_at(at: f32) -> TriangleSoup {
    vertical_quad_at(at, Surface::Wall)
}

/// The behaviour this whole constraint exists for, through the real entry point: a ship flown at
/// a wall ends on the near side and turns round. The `oag_physics::wall` unit tests pin one call
/// to `resolve`; this pins that `step` invokes it, the wiring a refactor would silently drop. Only
/// the sign and side are asserted, for the reason in this file's header.
#[test]
fn a_ship_flown_at_a_wall_ends_up_on_the_near_side_of_it_and_turns_round() {
    const WALL_X: f32 = 60.0;

    let handling = fixture();
    let mut world = flat_floor(Surface::Floor);
    world.push(wall_at(WALL_X));

    let mut state = ship_at(4.0, &handling);
    state.body.linear_velocity = Vec3::new(120.0, 0.0, 0.0);

    let mut reversed = false;
    for tick in 0..600 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );

        // Half the hull width is 1.0, so the centre may never pass the wall.
        assert!(
            state.body.position.x <= WALL_X,
            "tick {tick}: the ship went through the wall, x = {}",
            state.body.position.x
        );
        assert!(state.body.position.is_finite(), "tick {tick}");
        reversed |= state.body.linear_velocity.x < 0.0;
    }

    assert!(reversed, "the ship never bounced back off the wall");
}

/// A floor inside the hull pushes it and charges no shield: the original's
/// narrowphase reads no surface type and damages only a positive-friction
/// contact (`0x08815cd4`, `0x08842648`; see `oag_physics::wall`). The quad is
/// inside the hull and out of the hover probes' reach.
#[test]
fn a_hoverable_surface_inside_the_hull_pushes_it_and_charges_no_shield() {
    let mut handling = fixture();
    handling.dimensions.shield = 100.0; // the fixture's zero pool is vacuous
    let run = |beside: Option<Surface>| {
        let mut world = flat_floor(Surface::Floor);
        if let Some(surface) = beside {
            world.push(vertical_quad_at(0.5, surface));
        }
        let mut state = ship_at(4.0, &handling);
        state.shield = 100.0;
        for _ in 0..120 {
            step(
                &mut state,
                &ShipControls::default(),
                &handling,
                &Environment::default(),
                &world,
                TICK,
            );
        }
        state
    };

    let alone = run(None);
    for surface in [Surface::Floor, Surface::MagFloor] {
        let beside = run(Some(surface));
        assert_ne!(alone.body, beside.body, "{surface:?} did not push");
        assert_eq!(alone.shield, beside.shield, "{surface:?} charged");
    }

    // The control: tagged `Wall`, the same quad pushes and charges.
    let walled = run(Some(Surface::Wall));
    assert_ne!(alone.body, walled.body);
    assert!(walled.shield < alone.shield, "{}", walled.shield);
}

/// The suspension carries `normal_gravity + track_gravity`, not `normal_gravity`.
///
/// The hover downforce (`oag_physics::hover::DOWNFORCE_SCALE`) presses the craft onto the surface
/// with `track_gravity * mass * grounded` and the spring is calibrated against the *sum*, so the
/// equilibrium compression is
///
/// ```text
/// (normal_gravity + track_gravity) / (2 * 0.3 * HOVER_K * (normal_gravity + track_gravity)) = 1.25
/// ```
///
/// **whatever the split**, so this fixture's round numbers still pin the recovered behaviour.
/// Carrying gravity alone (the crate before the downforce was read) rests the ship at
/// `10 / 32 = 0.3125`, four times shallower: the number this test keeps out. The load matters
/// beyond height: the spring's damper multiplies the spring magnitude, so attitude damping scales
/// with what the suspension carries.
#[test]
fn a_settled_ship_rests_1_25_below_its_target_because_of_the_downforce() {
    let handling = Handling {
        physical: Physical {
            normal_gravity: 10.0,
            track_gravity: 30.0,
            ..fixture().physical
        },
        ..fixture()
    };
    let world = flat_floor(Surface::Floor);
    let target = oag_physics::hover::target_height(&handling, 0.0, 0.0);
    let drop = oag_physics::hover::PROBE_DROP_RAW * oag_physics::hover::TARGET_GLOBAL_SCALE;

    let mut state = ship_at(target - 1.25 + drop, &handling);
    let mut last = None;
    for _ in 0..240 {
        let evaluated = step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
        last = Some(evaluated);
    }

    let height = last.unwrap().hover.probes[0].height;
    assert!(
        (height - (target - 1.25)).abs() < 1e-2,
        "the probe settled at {height}, not 1.25 below its {target} target"
    );
    assert_eq!(state.grounded, 1.0);
}

/// A pitched craft comes back **without ringing**, the property the downforce restored.
///
/// `docs/physics/angular-velocity-column.md` measures the original answering a held pitch input
/// with extrema `+0.397 -0.080 +0.017 -0.007` rad/s (each about a fifth of the last), while this
/// crate used to give `+0.431 -0.353 +0.287 -0.234`. The hover damper multiplies the spring
/// magnitude, so it was 17x too weak while the suspension carried `normal_gravity` alone.
///
/// Asserts the *shape*, successive pitch-rate extrema each at most a third of the one before, on
/// a fixture with its own numbers. It fails on the pre-downforce crate (ratio about `0.8`).
#[test]
fn a_pitched_craft_stops_ringing_within_two_swings() {
    let handling = Handling {
        physical: Physical {
            normal_gravity: 10.0,
            track_gravity: 30.0,
            ..fixture().physical
        },
        ..fixture()
    };
    let world = flat_floor(Surface::Floor);
    let target = oag_physics::hover::target_height(&handling, 0.0, 0.0);
    let drop = oag_physics::hover::PROBE_DROP_RAW * oag_physics::hover::TARGET_GLOBAL_SCALE;
    let mut state = ship_at(target - 1.25 + drop, &handling);
    state.body.angular_velocity = Vec3::new(0.4, 0.0, 0.0);

    let mut rates = Vec::new();
    for _ in 0..120 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
        let local = state.body.orientation.inverse() * state.body.angular_velocity;
        rates.push(local.x);
    }

    let mut extrema = Vec::new();
    for window in rates.windows(3) {
        let (before, here, after) = (window[0], window[1], window[2]);
        if (here > before && here >= after) || (here < before && here <= after) {
            extrema.push(here);
        }
    }
    assert!(
        extrema.len() >= 3,
        "only {} extrema in 120 ticks: {extrema:?}",
        extrema.len()
    );
    for pair in extrema.windows(2).take(2) {
        assert!(
            pair[1].abs() < pair[0].abs() * 0.34,
            "pitch rate went {} -> {}, which is a ring rather than a decay: {extrema:?}",
            pair[0],
            pair[1]
        );
    }
}
