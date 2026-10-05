//! Flies a ship at a **real track's wall**, out of a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(wall_collision_ground_truth)'
//! ```
//!
//! # What this is for that the synthetic tests are not
//!
//! `oag_physics::wall`'s unit tests and
//! `crates/physics/tests/ship_dynamics.rs` both fire a ship at a hand-written
//! plane, so both know the wall's winding, scale and normal in advance. A shipped
//! wall supplies all three itself, and the response law reads all three:
//!
//! - **Winding.** A collision triangle's normal is not guaranteed to face the
//!   ship, which is why `oag_physics::wall` flips it against the probe direction.
//!   A synthetic quad can be authored the right way round by accident; a track
//!   cannot.
//! - **Scale.** The hull box comes from the ship's own `<Misc>` and the wall
//!   comes from the track. Nothing in the repository checks that those two are in
//!   the same units until they are put in the same scene.
//! - **Density.** Real walls are many small triangles rather than one large quad,
//!   so a probe can leave one triangle and miss the next.
//!
//! Everything asserted is structural - a side, a sign, a finite number - for the
//! reason `crates/game/tests/race_ground_truth.rs` gives at length: the response
//! law is an implementation choice awaiting M3, so pinning its magnitudes here
//! would pin this project's own arithmetic and call it a measurement.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_physics::{Body, Environment, ShipControls, ShipState, Surface, step};
use oag_raceplay as race;

/// A fixed 60 Hz tick, matching the rest of the simulation.
const TICK: f32 = 1.0 / 60.0;

/// How fast the ship is thrown at the wall, in units per second.
///
/// Fast enough that it crosses a zero-thickness wall shell inside one tick, so
/// the swept query is the thing under test and not the hull probes.
const APPROACH_SPEED: f32 = 150.0;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn load() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(loaded)
}

/// A wall triangle from the track, as `(centroid, unit normal)`.
///
/// Takes the first triangle of the first `Wall` collider rather than searching
/// for a convenient one: any wall must work, and picking the agreeable one would
/// be the test grading itself.
fn a_wall_triangle(world: &oag_physics::CollisionWorld) -> Option<(Vec3, Vec3)> {
    let collider = world
        .colliders()
        .iter()
        .find(|c| c.surface() == Surface::Wall && c.triangle_count() > 0)?;
    let [a, b, c] = collider.triangle(0)?;
    let centroid = (a + b + c) / 3.0;
    let normal = (b - a).cross(c - a).try_normalize()?;
    Some((centroid, normal))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ship_thrown_at_a_real_track_wall_does_not_pass_through_it() {
    let Some(loaded) = load() else {
        return;
    };
    let setup = loaded.setup;

    let Some((centroid, normal)) = a_wall_triangle(&setup.collision) else {
        panic!("the track has no wall collider with triangles in it");
    };
    println!("wall triangle at {centroid:?}, normal {normal:?}");
    println!(
        "hull {:?} x {:?} x {:?}",
        setup.handling.dimensions.width,
        setup.handling.dimensions.height,
        setup.handling.dimensions.length
    );
    assert!(
        setup.handling.dimensions.width > 0.0 && setup.handling.dimensions.length > 0.0,
        "the ship's <Misc> dimensions came through as zero, so nothing can collide"
    );

    // Start clear of the wall on the normal's side and fly straight at it. Which
    // side that is does not matter: the response flips the normal to face
    // whatever approaches.
    let standoff = setup.handling.dimensions.length.max(4.0);
    let start = centroid + normal * standoff;

    let mut state = ShipState {
        body: Body {
            position: start,
            linear_velocity: -normal * APPROACH_SPEED,
            mass: setup.handling.physical.mass,
            ..Body::default()
        },
        ..ShipState::default()
    };

    // Signed distance from the wall plane along the normal: positive is the side
    // the ship started on.
    let side_of = |p: Vec3| (p - centroid).dot(normal);
    assert!(side_of(start) > 0.0);

    let mut deepest = f32::MAX;
    let mut turned_round = false;
    for tick in 0..120 {
        step(
            &mut state,
            &ShipControls::default(),
            &setup.handling,
            &Environment::default(),
            &setup.collision,
            TICK,
        );
        let side = side_of(state.body.position);
        deepest = deepest.min(side);
        turned_round |= state.body.linear_velocity.dot(normal) > 0.0;
        assert!(
            state.body.position.is_finite(),
            "tick {tick}: position went non-finite"
        );
    }

    println!("deepest penetration {deepest}, turned round {turned_round}");

    // The hull's own half-extent is how far past the plane the *centre* may
    // legitimately sit, and gravity plus hover will have moved the ship off the
    // triangle by the end, so this is generous on purpose: the failure being
    // caught is a ship that sails on through, not one that scrapes.
    let allowance = -setup.handling.dimensions.length;
    assert!(
        deepest > allowance,
        "the ship went through the wall: reached {deepest}, allowed {allowance}"
    );
    assert!(
        turned_round,
        "the ship never gained any velocity back along the wall normal"
    );
}
