//! Flies a ship on a real track, out of a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all
//! ```
//!
//! # What this is for
//!
//! Every layer under a race has its own tests against synthetic data. What none of
//! them can check is whether the *composition* is right: whether the collision
//! geometry a ship hovers on is the same geometry the spline runs along, whether the
//! handling parameters arrive scaled exactly once, whether a spawn pose built from a
//! `.vex` frame puts a ship the right way up. Those only fail on real data, so this
//! is the only place they can be asserted. The same goes for the two halves of the
//! binary: [`the_front_end_hands_off_into_a_driveable_race`] boots the front end off
//! the disc, picks a language and flies what `Launch Game` starts, which is the
//! whole of what a player does and needs no GPU to check.
//!
//! Everything asserted is structural: something is finite, something is bounded by a
//! number the *track itself* supplies, something changed. Nothing here asserts a
//! speed, a height or a settling point as though it were known - the force law is
//! transcribed static analysis that has never been run against the original, so a
//! test that pinned a speed would be pinning this project's own arithmetic and
//! calling it a measurement. See `crates/physics/tests/ship_dynamics.rs`, which says
//! the same thing at its own level.
//!
//! # The ship now stays on the track
//!
//! [`a_ship_stays_on_the_track_for_ten_seconds`] is the assertion, and it used to be
//! the opposite one: `the_ship_does_not_stay_on_the_track_yet` recorded the ship
//! leaving the envelope at about tick **257** and said to delete itself when that
//! stopped happening. It stopped. The property is now pinned the right way round, so
//! the same measurement catches a regression instead of an improvement.
//!
//! **Two wrong diagnoses on the way there, both recorded rather than forgotten.** The
//! first was a suspension that dropped a probe: on the *previous* default track the
//! ship was grounded on 83 of 120 ticks with both probes on only 6. The default track
//! was the bug - see [`oag_game::race::DEFAULT_TRACK`] - and the right one is grounded
//! on 120 of 120 with both probes throughout. The second was a missing speed
//! equilibrium, inferred from captures that held 23.6-25.1 units/s; the standing-start
//! capture inverted it. The force law is right, those captures were in sustained wall
//! contact the whole way, and what was missing was the contact response that reads it.
//!
//! What is left is **not** on the track-holding axis: the speed is still far above any
//! capture's, and the trace comparison still diverges on contact generation and on the
//! angular half of the contact response. See
//! `docs/physics/force-balance-ground-truth.md`, and nothing here is tuned to hide it.

use std::path::{Path, PathBuf};

use oag_core::math::Vec3;
use oag_game::frontend::states;
use oag_game::{boot, movie, race};
use oag_gameplay::input::{Input, button};
use oag_physics::{Raycaster, SpeedClass};

/// Ticks the per-tick suspension assertions cover: two seconds at the fixed 60 Hz.
///
/// Short on purpose, because this is the test that checks *every* tick against a hover
/// target and a probe count. Ten seconds of the same scenario is covered by
/// [`a_ship_stays_on_the_track_for_ten_seconds`], which asserts the envelope and
/// finiteness rather than the suspension.
const TICKS: u32 = 120;

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

fn load() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// The track's own scale for "still roughly on the track".
///
/// Two of the widest half-width the track has anywhere, which on the default track
/// is tens of units. Derived from the data rather than picked, so it means the same
/// thing on a wider track, and generous on purpose: this is meant to catch a ship
/// that has left, not to grade the driving.
fn envelope(loaded: &race::Loaded) -> f32 {
    loaded.setup.spline.max_half_width() * 2.0
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ship_spawns_on_the_spline_and_flies_along_it() {
    let Some(loaded) = load() else { return };
    let bound = envelope(&loaded);
    let handling = loaded.setup.handling;
    let mut race = race::Race::start(loaded.setup);

    // The spawn itself: on the racing line, the suspension's target height above the
    // surface, the right way up, and with the mass the force law will read also in
    // the body the integrator divides by.
    let start = race.telemetry();
    assert!(start.position.is_finite(), "{:?}", start.position);
    assert_eq!(race.ship().physics.body.mass, handling.physical.mass);
    assert!(
        (start.height_above_spline - race::spawn_height(&handling)).abs() < 0.01,
        "spawned at {} above the spline, wanted {}",
        start.height_above_spline,
        race::spawn_height(&handling)
    );
    // `up` from a `.vex` frame is the surface normal negated, and getting that
    // backwards buries the ship. On this track's first sample the surface is roughly
    // level, so the ship's own up must have a positive world y.
    assert!(
        race.ship().physics.body.up().y > 0.5,
        "the ship is not the right way up: {}",
        race.ship().physics.body.up()
    );

    let mut held = race::HeldButtons::new(1 << button::CROSS);
    let mut grounded_ticks = 0u32;
    // Counted apart, because one probe in contact and two are different situations:
    // a single probe is a pitch torque applied every tick.
    let mut both_probes = 0u32;
    let mut worst_distance = 0.0f32;
    let mut peak_speed = 0.0f32;

    for tick in 0..TICKS {
        let snapshot = held.snapshot();
        race.tick(&snapshot);
        let telemetry = race.telemetry();

        assert!(
            telemetry.position.is_finite(),
            "tick {tick}: {}",
            race::describe(&telemetry)
        );
        assert!(
            telemetry.spline_distance < bound,
            "tick {tick}: {bound:.1} units off the spline is off the track: {}",
            race::describe(&telemetry)
        );

        if race.ship().physics.grounded > 0.0 {
            grounded_ticks += 1;
        }
        if race.ship().physics.grounded == 1.0 {
            both_probes += 1;
        }
        worst_distance = worst_distance.max(telemetry.spline_distance);
        peak_speed = peak_speed.max(telemetry.speed);
    }

    let end = race.telemetry();
    println!("{}", race::describe(&end));
    println!(
        "grounded on {grounded_ticks}/{TICKS} tick(s), both probes on {both_probes}, \
         worst spline distance {worst_distance:.2} of {bound:.1}, peak speed {peak_speed:.2}"
    );

    // It hovered: a probe reached the collision geometry on some tick. Without this
    // the ship could be in free fall and everything above would still hold.
    assert!(
        grounded_ticks > 0,
        "no hover probe ever reached the track's collision geometry"
    );
    // And it hovered on *both* probes, every tick. This is the pin on
    // `race::DEFAULT_TRACK`: against the track this scenario was previously flown on,
    // the same run held both probes on 6 of these 120 ticks and one on 83, which reads
    // as a suspension that cannot hold a ship and was actually a ship being flown over
    // the wrong track's collision geometry. Strict on purpose - a single dropped probe
    // here is a regression worth looking at, and this test never runs in CI.
    assert_eq!(
        both_probes, TICKS,
        "both hover probes were in contact on only {both_probes} of {TICKS} tick(s); \
         see race::DEFAULT_TRACK"
    );
    // And it went somewhere. Thrust held for two seconds against an engine that
    // reaches tens of units per second cannot leave it where it started.
    let travelled = (end.position - start.position).length();
    assert!(
        travelled > 10.0,
        "the ship travelled {travelled:.2} units in {TICKS} ticks: {}",
        race::describe(&end)
    );
    // And it went *forwards*. Three separate conventions have to agree for this to
    // hold - the spline's tangent, the rotation `Pose::from_sample` builds from it,
    // and the body axis the engine pushes along - and any one of them inverted would
    // still satisfy every assertion above.
    let forward = race.ship().physics.body.forward();
    let along = race.ship().physics.body.linear_velocity.dot(forward);
    assert!(
        along > 0.0,
        "thrust drove the ship backwards along its own forward axis: {along:.2}"
    );
    assert_eq!(race.world.tick, u64::from(TICKS));
}

/// Ticks the handoff test flies. Half of [`TICKS`], and for the same reason: the
/// question is whether `Launch Game` produces a driveable ship at all, not how long
/// one stays driveable.
const HANDOFF_TICKS: u32 = 60;

/// `Launch Game` is what puts a player in a ship, so the two halves have to
/// compose: the boot sequence has to reach that state off a real disc, and what it
/// hands off to has to be a race that spawns, hovers and drives.
///
/// The window does exactly this, in this order, with a GPU in the middle. Nothing
/// here needs one, which is the point: if this passes and the window does not, the
/// fault is in the window.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_end_hands_off_into_a_driveable_race() {
    let Some(image) = image() else { return };

    // The same load the binary does, without the transcode: what is under test is
    // the sequencing and the handoff, not the picture.
    let boot = boot::load(&boot::Options {
        source: image.display().to_string(),
        leg: oag_game::frontend::Leg::LogoFmv,
        movie: boot::DEFAULT_BOOT_MOVIE.to_string(),
        cache: std::env::temp_dir().join("oag-race-handoff"),
        extent: movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
    })
    .expect("loading the boot sequence");

    let mut frontend = boot.frontend;
    let mut input = Input::new();
    let dt = 1.0 / 60.0;

    // START skips the intro, and cross picks a language, which fires `Launch Game`.
    input.begin_frame(1 << button::START);
    frontend.update(dt, &mut input);
    input.begin_frame(0);
    frontend.update(dt, &mut input);
    assert!(
        frontend.machine().is(states::LANGUAGE_SELECTION),
        "got as far as {:?}",
        frontend.machine().current()
    );

    input.begin_frame(1 << button::CROSS);
    frontend.update(dt, &mut input);
    assert!(frontend.machine().is(states::LAUNCH_GAME));
    assert!(
        frontend.is_finished(),
        "reaching Launch Game is what starts a race"
    );

    // And this is what the window does next.
    let Some(loaded) = load() else { return };
    let bound = envelope(&loaded);
    let mut race = race::Race::start(loaded.setup);
    let mut held = race::HeldButtons::new(1 << button::CROSS);
    let mut grounded_ticks = 0u32;

    for tick in 0..HANDOFF_TICKS {
        let snapshot = held.snapshot();
        race.tick(&snapshot);
        let telemetry = race.telemetry();
        assert!(
            telemetry.position.is_finite(),
            "tick {tick}: {}",
            race::describe(&telemetry)
        );
        assert!(
            telemetry.spline_distance < bound,
            "tick {tick}: {bound:.1} units off the spline is off the track: {}",
            race::describe(&telemetry)
        );
        if race.ship().physics.grounded > 0.0 {
            grounded_ticks += 1;
        }
    }

    println!("{}", race::describe(&race.telemetry()));
    assert!(
        grounded_ticks > 0,
        "no hover probe reached the track in {HANDOFF_TICKS} ticks"
    );
    let forward = race.ship().physics.body.forward();
    let along = race.ship().physics.body.linear_velocity.dot(forward);
    assert!(
        along > 0.0,
        "the ship the front end handed off to does not drive forwards: {along:.2}"
    );
}

/// Ten seconds at full throttle, on the track and finite the whole way.
///
/// **This replaces a recorded negative result.** `the_ship_does_not_stay_on_the_track_yet`
/// asserted the opposite - that the ship *left* the envelope inside 600 ticks - and
/// carried an instruction to delete it when it started failing. It did, so the
/// property is pinned the right way round here rather than dropped: a regression that
/// threw the ship off the track again would otherwise go unnoticed.
///
/// What changed is the wall contact response in `oag_physics::wall`, matched to the
/// `3.67 %`-per-frame speed loss measured off a real capture. Before it, the ship left
/// the envelope at about tick **257**, not by falling off the surface but by carrying
/// far more speed into a corner than the corner would take.
///
/// Measured on the reference scenario - Assegai, Venom, [`race::DEFAULT_TRACK`], thrust
/// held from the first tick - over the full 600:
///
/// | | |
/// | --- | --- |
/// | Worst distance from the spline | **27.2** of a 114.0 envelope, at tick 520 |
/// | Grounded | **600 of 600** ticks |
/// | Respawns | **0** - it stays on by driving, not by being put back |
/// | Non-finite | never |
///
/// The assertion is the envelope and finiteness only. **The speed is still not the
/// original's** - this run peaks at 122 against a capture's 23.6-25.1 - and nothing
/// here asserts it, because a clean straight has no equilibrium to assert and the
/// captures that looked like one were in sustained wall contact. See
/// `docs/physics/force-balance-ground-truth.md`, whose remaining gaps - contact
/// generation and the missing `cross(r, impulse)` angular response - are what a trace
/// comparison still diverges on.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ship_stays_on_the_track_for_ten_seconds() {
    let Some(loaded) = load() else { return };
    let bound = envelope(&loaded);
    let mut race = race::Race::start(loaded.setup);
    let mut held = race::HeldButtons::new(1 << button::CROSS);

    let mut worst_distance: f32 = 0.0;
    let mut worst_at = 0u32;
    let mut top_speed: f32 = 0.0;
    let mut grounded_ticks = 0u32;
    for tick in 0..600u32 {
        let snapshot = held.snapshot();
        race.tick(&snapshot);
        let telemetry = race.telemetry();
        assert!(
            telemetry.position.is_finite(),
            "the ship went non-finite at tick {tick}"
        );
        if telemetry.spline_distance > worst_distance {
            worst_distance = telemetry.spline_distance;
            worst_at = tick;
        }
        top_speed = top_speed.max(telemetry.speed);
        if telemetry.grounded > 0.0 {
            grounded_ticks += 1;
        }
    }

    println!(
        "worst spline distance {worst_distance:.1} of {bound:.1} at tick {worst_at}, \
         top speed {top_speed:.1}, grounded on {grounded_ticks} of 600, \
         {} respawn(s)",
        race.respawns()
    );
    assert!(
        worst_distance < bound,
        "the ship left the {bound:.1}-unit envelope at tick {worst_at}"
    );
}

/// The spline and the collision geometry have to be the same track.
///
/// Nothing under this test can catch a mismatch: `oag-formats` decodes each of them
/// correctly in isolation, and a ship hovering over geometry a hundred units from
/// the spline it spawned on would look like a physics bug rather than a loading one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_collision_geometry_is_under_the_spline() {
    let Some(loaded) = load() else { return };
    let handling = loaded.setup.handling;
    let spline = &loaded.setup.spline;
    let collision = &loaded.setup.collision;

    assert!(!spline.is_empty(), "the track produced no spline samples");
    assert!(
        !collision.colliders().is_empty(),
        "the track produced no colliders"
    );

    // Cast down the surface normal at a spread of samples along the track, from one
    // probe reach above the surface line to one below it. `ride_height` is the length
    // the ship's own probes use, so a miss here is a place a ship on the spline would
    // find nothing to hover on.
    let reach = handling.antigrav.ride_height;
    let mut hits = 0;
    let mut probes = 0;
    let step = (spline.len() / 200).max(1);
    for index in (0..spline.len()).step_by(step) {
        let Some(sample) = spline.sample(index) else {
            continue;
        };
        let up = (-Vec3::from_array(sample.down)).normalize_or_zero();
        let from = Vec3::from_array(sample.pos) + up * reach;
        probes += 1;
        if collision
            .raycast(oag_physics::Ray::new(from, -up, reach * 2.0), None, false)
            .is_some()
        {
            hits += 1;
        }
    }

    println!("{hits}/{probes} downward probes found collision geometry within {reach} units");
    // Not all of them: a junction's samples overlap, the spline runs through
    // scenery-only stretches, and the two decoders were never promised to agree
    // everywhere. Most of them is the claim.
    assert!(
        hits * 2 > probes,
        "only {hits} of {probes} samples have anything under them within \
         {reach} units; the spline and the collision geometry are not the same track"
    );
}
