//! Drives a ship into a **real track's `Reset` geometry**, out of a real disc
//! image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(reset_zone_ground_truth)'
//! ```
//!
//! # Why a real track is the only place this can be checked
//!
//! The synthetic tests in `oag_physics::reset` and in `race.rs` build their own
//! reset planes, so they know the geometry in advance. A shipped reset volume
//! supplies its own shape, and two properties of it are not knowable from here:
//!
//! - **Whether the track has one at all.** The census in
//!   `docs/formats/collision.md` counts 26 `Reset Collision` nodes across 40 PSP
//!   tracks, so **not every track has one** - and the default race track,
//!   `16_Track`, is one of the ones that does not. Measured here: 66 wall, 123
//!   floor, 7 mag-floor and **zero** reset colliders. So this test picks its own
//!   track from [`CANDIDATES`] rather than using the default, and says which.
//! - **Whether a reset volume is masked by other geometry.** It is what
//!   `oag_physics::reset` bypasses the nearest-hit query for; a real track is
//!   where that bypass either matters or does not.

use std::path::{Path, PathBuf};

use oag_core::math::Vec3;
use oag_game::race;
use oag_gameplay::input::InputSnapshot;
use oag_physics::{SpeedClass, Surface};

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Tracks to try, in order, until one has `Reset` geometry.
///
/// **The default race track has none.** Measured on the USA PSP disc:
/// `16_Track` carries 66 wall, 123 floor and 7 mag-floor colliders and *zero*
/// reset ones, so the feature cannot be exercised there at all. The rest of this
/// list is tracks measured to have some, so the test has something to fly into.
/// It is a list rather than one name because a different region's disc need not
/// agree, and a test that hard-codes one track fails as "the respawn is broken".
const CANDIDATES: &[&str] = &[
    race::DEFAULT_TRACK,
    r"Data\Environments\03_Track\track.vex",
    r"Data\Environments\05_Track\track.vex",
    r"Data\Environments\10_Track\track.vex",
];

fn load(track: &str) -> Option<race::Loaded> {
    let image = image()?;
    race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
        track: track.to_string(),
        ..race::Options::default()
    })
    .ok()
}

fn reset_colliders(world: &oag_physics::CollisionWorld) -> usize {
    world
        .colliders()
        .iter()
        .filter(|c| c.surface() == Surface::Reset && c.triangle_count() > 0)
        .count()
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn touching_a_real_reset_volume_respawns_the_ship() {
    if image().is_none() {
        return;
    }

    let mut chosen = None;
    for track in CANDIDATES {
        let Some(loaded) = load(track) else {
            continue;
        };
        let count = reset_colliders(&loaded.setup.collision);
        println!("{track}: {count} reset collider(s)");
        if count > 0 {
            chosen = Some((*track, loaded));
            break;
        }
    }

    let Some((track, loaded)) = chosen else {
        panic!(
            "none of the candidate tracks has Reset geometry, so the respawn \
             could not be exercised. Candidates: {CANDIDATES:?}"
        );
    };
    println!("flying into {track}");
    let setup = loaded.setup;

    // The census, because it is the finding either way: how much of each class
    // this track actually carries.
    for surface in [
        Surface::Wall,
        Surface::Floor,
        Surface::MagFloor,
        Surface::Reset,
    ] {
        let count = setup
            .collision
            .colliders()
            .iter()
            .filter(|c| c.surface() == surface)
            .count();
        let triangles: usize = setup
            .collision
            .colliders()
            .iter()
            .filter(|c| c.surface() == surface)
            .map(oag_physics::TriangleSoup::triangle_count)
            .sum();
        println!("{surface:?}: {count} collider(s), {triangles} triangle(s)");
    }

    let reset = setup
        .collision
        .colliders()
        .iter()
        .find(|c| c.surface() == Surface::Reset && c.triangle_count() > 0)
        .map(|c| (c.collider(), c.triangle(0)));

    let Some((collider, Some([a, b, c]))) = reset else {
        panic!("the chosen track lost its reset collider between the two passes");
    };

    let centroid = (a + b + c) / 3.0;
    let normal = (b - a)
        .cross(c - a)
        .try_normalize()
        .expect("the first reset triangle is degenerate");
    println!("reset collider {collider}, triangle at {centroid:?}, normal {normal:?}");

    let mut race = race::Race::start(setup);
    assert_eq!(race.respawns(), 0);

    // Put the ship clear of the trigger and fire it straight through. Which side
    // of the triangle it starts on does not matter: the swept ray crosses the
    // plane either way.
    {
        let body = &mut race.world.ships[0].physics.body;
        body.position = centroid + normal * 20.0;
        body.linear_velocity = -normal * 600.0;
        body.angular_velocity = Vec3::ZERO;
    }

    for _ in 0..20 {
        race.tick(&InputSnapshot::default());
        if race.respawns() > 0 {
            break;
        }
    }

    assert!(
        race.respawns() > 0,
        "flying through a real Reset triangle did not respawn the ship"
    );

    // And it came back somewhere on the track rather than to the origin or to a
    // non-finite pose.
    let position = race.ship().physics.body.position;
    assert!(position.is_finite());
    let distance = race
        .spline()
        .distance_to(position)
        .expect("the track has samples");
    println!("respawned {distance} from the nearest spline sample");
    assert!(
        distance < race.spline().max_half_width() * 2.0,
        "respawned {distance} from the spline, which is off the track"
    );
}
