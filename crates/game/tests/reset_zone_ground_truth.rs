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

fn disc(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

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

fn image() -> Option<PathBuf> {
    disc("pulse-psp-usa.chd")
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
    load_from(&image()?, Some(track))
}

/// Loads a race off any disc, taking the source's own default circuit when
/// `track` is `None` - which is what the Pure case wants, since naming a Pulse
/// circuit against a Pure disc finds nothing.
fn load_from(source: &Path, track: Option<&str>) -> Option<race::Loaded> {
    race::load(&race::Options {
        source: source.display().to_string(),
        class: SpeedClass::Venom,
        track: track.map(str::to_string),
        ..race::Options::default()
    })
    .ok()
}

/// Fires the craft through `soup`'s first triangle and returns how far from the
/// spline it came back, or `None` if it never respawned.
///
/// The body of what [`touching_a_real_reset_volume_respawns_the_ship`] always
/// did, lifted out so a second title can be put through the identical check
/// rather than a similar one.
fn fly_through_first_reset(setup: race::Setup) -> Option<f32> {
    let reset = setup
        .collision
        .colliders()
        .iter()
        .find(|c| c.surface() == Surface::Reset && c.triangle_count() > 0)
        .map(|c| (c.collider(), c.triangle(0)));

    let Some((collider, Some([a, b, c]))) = reset else {
        panic!("this track has no non-empty reset collider");
    };
    let centroid = (a + b + c) / 3.0;
    let normal = (b - a)
        .cross(c - a)
        .try_normalize()
        .expect("the first reset triangle is degenerate");
    println!("reset collider {collider}, triangle at {centroid:?}, normal {normal:?}");

    let mut race = race::Race::start(setup);
    assert_eq!(race.respawns(), 0);

    // Clear of the trigger and straight through it. Which side it starts on does
    // not matter: the swept ray crosses the plane either way.
    {
        let body = &mut race.world.ships[0].physics.body;
        body.position = centroid + normal * 20.0;
        body.linear_velocity = -normal * 600.0;
        body.angular_velocity = Vec3::ZERO;
    }

    for _ in 0..20 {
        race.tick(&InputSnapshot::default());
        if race.respawns() > 0 {
            let position = race.ship().physics.body.position;
            assert!(position.is_finite());
            return race.spline().distance_to(position);
        }
    }
    None
}

/// **Pure's reset surface is authored differently from Pulse's, and it works.**
///
/// The class identity was settled by the class-name table both executables carry
/// (`docs/formats/pure-status.md`), not by behaviour, and the raycast evidence
/// behind it left a real residual: Pure's `Reset Collision` covers **99.9 %** of
/// its spline at a consistent ~10.7 units below the road, where Pulse's covers
/// **5.8 %** at ~21.5 and is placed at the points a craft can leave. So Pure
/// carries a continuous under-surface and Pulse carries patches.
///
/// That difference is exactly the kind that makes a class *identity* correct and
/// the *behaviour* still untested, which is what this closes: flying through
/// Pure's surface has to respawn the craft on Pure's track, through the same
/// `oag_physics::reset` path and with no Pure-specific branch anywhere.
///
/// Its own circuit and its own default team, because a Pulse track name resolves
/// to nothing in Pure's archives.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pures_reset_surface_respawns_the_ship_too() {
    let Some(source) = disc("pure-psp-usa.chd") else {
        return;
    };
    let loaded = load_from(&source, None).expect("a Pure race loads");
    let setup = loaded.setup;

    // The census first, because the shape of Pure's collision is the finding and
    // it should be in the output whether the assertion below passes or fails.
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
        println!("{surface:?}: {count} collider(s)");
    }
    // Pure ships neither string in its executable, so this is a claim about the
    // title rather than about one circuit.
    assert_eq!(
        setup
            .collision
            .colliders()
            .iter()
            .filter(|c| c.surface() == Surface::MagFloor)
            .count(),
        0,
        "Pure names no Mag Floor Collision class at all"
    );

    let max_half_width = {
        let race = race::Race::start(setup.clone());
        race.spline().max_half_width()
    };
    let distance = fly_through_first_reset(setup)
        .expect("flying through Pure's reset surface did not respawn the ship");
    println!("respawned {distance} from the nearest spline sample");
    assert!(
        distance < max_half_width * 2.0,
        "respawned {distance} from the spline, which is off the track"
    );
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
