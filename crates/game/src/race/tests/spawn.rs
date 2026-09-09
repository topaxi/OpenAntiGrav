//! `Race::start`: where a craft is put, what mass and inertia it is given,
//! and that it stays there.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`.

use super::*;

/// A field of one reports **no** place, rather than "first of one".
///
/// Which is every solo mode: a time trial, a speed lap and a Zone run all grid
/// the player alone, and so does a single race on a track with no authored
/// `Start Position`. `1 / 1` is arithmetic rather than a standing.
///
/// The other half - a real place from a real field - needs a grid, so it is
/// `the_players_place_reaches_the_hud` in the ground-truth suite, which CI never
/// runs. This is the half that *can* run everywhere, and it exists because the
/// readout carried a hardcoded `place: 0` for months: a future edit that puts a
/// constant back fails a test that always runs rather than one that needs a
/// disc image.
#[test]
fn a_field_of_one_reports_no_place() {
    let race = Race::start(setup(Handling::ZERO));
    assert_eq!(race.sim.world.ship_count, 1);

    let readout = race.readout();
    assert_eq!(readout.place, 0);
    // Known regardless of the field size. The HUD is what decides not to draw a
    // field size with no place beside it - see `oag_game::hud::text_for`.
    assert_eq!(readout.ships, 1);
    // The *table* still says first, because a lone craft leads the race it is
    // in. The readout is where that stops being reported as a standing, so a
    // gate that only consulted `places()` would not pass this.
    assert_eq!(race.player_place(), 1);
}

/// A ship must arrive on the track with its mass in the body, or the hover
/// spring and the integrator disagree about how heavy it is.
#[test]
fn a_started_race_puts_one_ship_on_the_spline_with_its_mass_set() {
    let handling = Handling {
        physical: oag_physics::params::Physical {
            mass: 7.5,
            ..Default::default()
        },
        antigrav: oag_physics::params::Antigrav {
            // Chosen so the resulting spawn height cannot coincide with
            // `track::HOVER_LIFT`, which the assertion below distinguishes it from.
            ride_height: 8.0,
            ..Default::default()
        },
        pitch: oag_physics::params::Pitch {
            antigrav_height_adjust: 0.5,
            ..Default::default()
        },
        ..Handling::ZERO
    };
    let setup = setup(handling);
    let start = *setup.spline.start().expect("a first sample");
    let race = Race::start(setup);

    assert_eq!(race.sim.world.ship_count, 1);
    assert!(race.ship().active);
    assert_eq!(race.ship().physics.body.mass, 7.5);
    assert_eq!(race.ship().handling.physical.mass, 7.5);
    assert_eq!(race.dt(), 1.0 / 60.0);

    // Lifted off the surface line by the height its own suspension holds it at,
    // which is neither `track::HOVER_LIFT` nor the probe's full reach; see
    // `spawn_height`. This fixture has no gravity, so the sag is zero and the rest
    // height is the target itself: `ride_height` scaled by the measured
    // `TARGET_GLOBAL_SCALE`, with the `antigrav_height_adjust` of 0.5 contributing
    // nothing because `craft+0x74` reads zero in the running game.
    assert_eq!(
        spawn_height(&handling),
        8.0 * oag_physics::hover::TARGET_GLOBAL_SCALE
    );
    assert_ne!(spawn_height(&handling), track::HOVER_LIFT);
    // Well inside the probes' reach, which is what gives the suspension travel.
    assert!(spawn_height(&handling) < handling.antigrav.ride_height);
    let up = (-Vec3::from_array(start.down)).normalize();
    let offset = race.ship().physics.body.position - Vec3::from_array(start.pos);
    assert!(
        (offset.dot(up) - spawn_height(&handling)).abs() < 1e-4,
        "{offset}"
    );
}

/// The spawn height is the spring's rest height plus the probes' own drop,
/// not the spring's target and not the probe's reach.
///
/// Round numbers chosen so the sag is checkable by hand, and **not** a ship's:
/// two probes give a gradient of `2 * 0.3 * HOVER_K * (normal_gravity +
/// track_gravity)`, and a grounded craft carries `normal_gravity +
/// track_gravity` (gravity plus `oag_physics::hover::DOWNFORCE_SCALE`'s
/// downforce), so the sag is exactly `1.25` whatever the two gravities are.
/// The centre of mass then sits a probe drop above that. `mass` is
/// deliberately not 1, to pin that it cancels.
#[test]
fn a_ship_spawns_at_the_height_its_own_suspension_holds_it_at() {
    let handling = Handling {
        physical: oag_physics::params::Physical {
            mass: 3.0,
            normal_gravity: 5.0,
            track_gravity: 80.0,
            flight_gravity: 95.0,
        },
        antigrav: oag_physics::params::Antigrav {
            ride_height: 5.5,
            ..Default::default()
        },
        ..Handling::ZERO
    };

    let gradient = 2.0 * 0.3 * oag_physics::hover::HOVER_K * 85.0;
    let target = 5.5 * oag_physics::hover::TARGET_GLOBAL_SCALE;
    let drop = oag_physics::hover::PROBE_DROP_RAW * oag_physics::hover::TARGET_GLOBAL_SCALE;
    let expected = target - 85.0 / gradient + drop;
    assert!(
        (spawn_height(&handling) - expected).abs() < 1e-5,
        "{} was not {expected}",
        spawn_height(&handling)
    );

    // Real travel in **both** directions, which is the property that matters
    // and the one the old geometry could not have. A probe reaches exactly as
    // far as the target (`oag_physics::hover::probe`), so its travel is the
    // whole `target`: the resting probe sits `1.25` up from full compression
    // and `1.25` short of losing the ground. That symmetry is the recovered
    // model's, not a tuning: both numbers are the same sag.
    let probe_rest = spawn_height(&handling) - drop;
    assert!(probe_rest > 0.0 && probe_rest < target);
    assert!(
        (probe_rest - (target - 1.25)).abs() < 1e-5,
        "the probe rests at {probe_rest}, not 1.25 below its {target} target"
    );

    // And mass really does cancel.
    let heavier = Handling {
        physical: oag_physics::params::Physical {
            mass: 50.0,
            ..handling.physical
        },
        ..handling
    };
    assert_eq!(spawn_height(&heavier), spawn_height(&handling));
}

/// The tensor is the recovered box and does **not** depend on the ship.
///
/// This test used to assert the opposite: that the tensor was built from
/// `<Misc width/height/length>` at `<Physical mass>`, with roll the cheapest
/// axis because the hull is long. Both halves are now refuted -
/// `Body_SetBoxInertia`'s single call site passes the code literal
/// `(12, 8, 12)` at a mass of `0.9`, the hull dimensions go to the collider
/// instead, and the box is square in plan so **pitch and roll are equal** and
/// yaw is the odd axis out. Kept as a test rather than deleted because "the
/// inertia varies per craft" is exactly the assumption that would come back.
#[test]
fn the_inertia_tensor_is_the_recovered_box_and_is_the_same_for_every_ship() {
    let inertia = box_inertia();

    assert!((inertia.x - 15.6).abs() < 0.01, "pitch was {}", inertia.x);
    assert!((inertia.y - 21.6).abs() < 0.01, "yaw was {}", inertia.y);
    assert!((inertia.z - 15.6).abs() < 0.01, "roll was {}", inertia.z);

    // Square in plan, so the two attitude axes cost the same and yaw is the
    // hardest. A hull-derived tensor could not produce this.
    assert_eq!(inertia.x, inertia.z);
    assert!(inertia.y > inertia.x);
    assert_ne!(inertia, Vec3::ONE);
}

/// A ship at rest on flat ground stays at rest.
///
/// This is the assertion whose absence let a ship fall through the floor with no
/// input held: on the shipped parameters the placeholder inertia tensor turned
/// single-probe contact into a 38 rad/s pitch oscillator that explicit Euler grew
/// about 5 % a tick, and the ship inverted and left the world by tick 250 while the
/// player was not touching the controls. Reverting either the tensor or the spawn
/// height fails this within two seconds of simulated time.
///
/// A flat floor and no track, deliberately: the same failure reproduced with no
/// spline, no collision soup and no camera, so this pins the force law rather than
/// the composition. Ten seconds is long enough that 5 %-per-tick growth would be
/// astronomically visible.
#[test]
fn a_ship_at_rest_on_flat_ground_stays_at_rest() {
    use oag_physics::{Body, ShipControls, ShipState, Surface, TriangleSoup};

    // The shape of the observed parameters. Round numbers, and not a ship's.
    let handling = Handling {
        physical: oag_physics::params::Physical {
            mass: 1.0,
            normal_gravity: 5.0,
            track_gravity: 80.0,
            flight_gravity: 95.0,
        },
        antigrav: oag_physics::params::Antigrav {
            ride_height: 5.5,
            rebound: 0.6,
            landing_rebound: 0.5,
            ..Default::default()
        },
        dimensions: oag_physics::params::Dimensions {
            width: 5.0,
            height: 3.5,
            length: 13.0,
            ..Default::default()
        },
        pitch: oag_physics::params::Pitch {
            pitch_damping: 3.0,
            // Set, and required not to matter: see `hover::target_height`.
            antigrav_height_adjust: 1.0,
            ..Default::default()
        },
        ..Handling::ZERO
    };

    let mut floor = CollisionWorld::new();
    floor.push(TriangleSoup::new(
        vec![
            [-5000.0, 0.0, -5000.0],
            [-5000.0, 0.0, 5000.0],
            [5000.0, 0.0, 5000.0],
            [5000.0, 0.0, -5000.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        Surface::Floor,
        0,
    ));

    let start = spawn_height(&handling);
    let mut state = ShipState {
        body: Body {
            position: Vec3::new(0.0, start, 0.0),
            mass: handling.physical.mass,
            inertia: box_inertia(),
            ..Body::default()
        },
        grounded: 1.0,
        ..ShipState::default()
    };

    let mut worst_tilt = 0.0f32;
    let mut airborne = 0u32;
    for _ in 0..600 {
        let evaluated = oag_physics::step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &floor,
            1.0 / 60.0,
        );
        if evaluated.hover.contacts == 0 {
            airborne += 1;
        }
        let tilt = state.body.up().dot(Vec3::Y).clamp(-1.0, 1.0).acos();
        worst_tilt = worst_tilt.max(tilt);
    }

    assert_eq!(
        airborne, 0,
        "the ship left the ground on {airborne} tick(s)"
    );
    assert!(
        worst_tilt < 0.01,
        "the ship tilted to {worst_tilt} rad with no input held"
    );
    assert!(
        (state.body.position.y - start).abs() < 0.5,
        "the ship drifted from {start} to {} with no input held",
        state.body.position.y
    );
    assert!(
        state.body.linear_velocity.length() < 1.0,
        "the ship was moving at {} with no input held",
        state.body.linear_velocity.length()
    );
}

/// With no collision geometry there is nothing to hover on, so the ship must
/// fall - and must fall *finitely*. The cheapest possible check that the force
/// law is really being driven from here.
#[test]
fn a_ship_over_nothing_falls_and_stays_finite() {
    let handling = Handling {
        physical: oag_physics::params::Physical {
            mass: 1.0,
            flight_gravity: 10.0,
            normal_gravity: 10.0,
            track_gravity: 10.0,
        },
        ..Handling::ZERO
    };
    let mut race = Race::start(setup(handling));
    let start = race.ship().physics.body.position;

    let mut held = HeldButtons::new(0);
    for _ in 0..60 {
        let snapshot = held.snapshot();
        race.tick(&snapshot);
    }

    let now = race.ship().physics.body.position;
    assert!(now.is_finite(), "{now}");
    assert!(now.y < start.y, "nothing to hover on, so it must fall");
    assert_eq!(race.ship().physics.grounded, 0.0);
    assert_eq!(race.sim.world.tick, 60);
    assert!(race.telemetry().spline_distance.is_finite());
}

/// A slot pointing back down its own track is turned round.
///
/// The synthetic straight runs along `+x`, so a slot authored facing `-x` is
/// the state nine of Wipeout HD's 28 circuit files are actually in: the node
/// was dragged to the other end for the reversed build and never re-authored.
/// Its ground-truth counterpart is
/// `crates/game/tests/spawn_heading_ground_truth.rs`, which CI never runs -
/// this is the half that runs everywhere.
#[test]
fn a_slot_facing_back_down_its_own_track_is_turned_round() {
    let mut setup = setup(hulled_handling());
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, 1.0],
        up: [0.0, 1.0, 0.0],
        forward: [-1.0, 0.0, 0.0],
    });
    let race = Race::start(setup);

    let forward = race.ship().physics.body.forward();
    assert!(
        forward.x > 0.9,
        "the track runs along +x and the craft must too: {forward}"
    );
    // Still level: only the heading is taken from the spline.
    assert!(forward.y.abs() < 1.0e-3, "{forward}");
}

/// A solo mode grids the player on slot 1, not slot 8.
///
/// The bug this guards: the player used to be placed directly on the authored
/// `Start Position` node, which is grid slot 8 - the back - on every mode,
/// including the three solo ones where there is no field to be at the back
/// *of*. See `Race::start`'s own comment on `solo_slot_one` for the evidence
/// (a live time-trial capture) and `docs/ghidra/functions/psp-pulse-usa/grid.md`
/// for why slot 8 is right for a full grid.
///
/// The synthetic straight runs along `+x`; the node is planted at its very
/// start, so slot 1 - `GRID_ROW_PITCH * 7` ahead of it - lands well past where
/// the node itself sits, and slot 8 (the pre-fix behaviour) would not move at
/// all.
#[test]
fn a_solo_mode_starts_on_slot_one_not_slot_eight() {
    let mut setup = setup(hulled_handling());
    assert!(
        !setup.mode.has_opponents(),
        "the fixture's default mode must be a solo one for this test to mean anything"
    );
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, 1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let race = Race::start(setup);

    assert_eq!(race.sim.world.ship_count, 1, "a solo mode fields one ship");
    let position = race.ship().physics.body.position;
    assert!(
        position.x > 50.0,
        "slot 1 is far ahead of the node along the track's own +x, not on top of \
         it: {position}"
    );
}

/// The counterpart to the test above: a full grid still anchors the player on
/// the authored node itself - slot 8 - unmoved by the solo-mode fix.
#[test]
fn a_full_grid_still_starts_the_player_on_slot_eight() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, 1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let race = Race::start(setup);

    assert_eq!(
        race.sim.world.ship_count, 8,
        "a full grid fields eight ships"
    );
    let position = race.ship().physics.body.position;
    assert!(
        position.x.abs() < 5.0,
        "slot 8 is the node itself, not ahead of it: {position}"
    );
}

/// A slot that agrees with its track keeps its **own** heading, not the
/// spline's rounding of it.
///
/// The pair to the test above, and the one that makes it mean something: the
/// authored value is the one to prefer wherever it is maintained, so a craft on
/// a slot deliberately angled a few degrees off the tangent must come out at
/// that angle. Every circuit file on both PSP discs is in this state.
#[test]
fn a_slot_that_agrees_keeps_its_own_heading() {
    let angled = oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, 1.0],
        up: [0.0, 1.0, 0.0],
        // About 11 degrees off the straight's own `+x`, and well inside the
        // band that counts as agreement.
        forward: [0.98058, 0.0, 0.19612],
    };
    let mut setup = setup(hulled_handling());
    setup.start_position = Some(angled);
    let race = Race::start(setup);

    let forward = race.ship().physics.body.forward();
    assert!(
        (forward.z - angled.forward[2]).abs() < 1.0e-3,
        "the authored angle survives: {forward}"
    );
}
