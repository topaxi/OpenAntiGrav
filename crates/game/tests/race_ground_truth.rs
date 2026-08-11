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
//! capture's. Contact generation is no longer the gap it was - the ten box sample
//! points, the angular half of the response, per-triangle contacts and the original's
//! single-sided rejection are all in `oag_physics::wall` now, each measured against
//! the whole-lap scenario on its own commit. What the lap run shows instead is a ship
//! that gets **wedged**: it reaches about 40 units/s by tick 400, stops at one place
//! on the circuit for a thousand ticks and then reverses, and its angular velocity
//! averages 21 rad/s over the lap. See `docs/physics/force-balance-ground-truth.md`,
//! and nothing here is tuned to hide it.

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

/// The grid measurement's own scenario: [`Options::opponents`] forced on.
///
/// `Mode::SingleRace` now fields a grid on its own, so the override is no longer
/// the only way to get eight craft - but it is still the right one *here*,
/// because it puts a full grid on the default mode and so keeps the grid
/// measurements independent of which mode is being raced. The tests that want
/// the AI driving use [`load_single_race`] instead. Every other test in this
/// file wants a solo ship, which is what [`load`] gives.
fn load_with_opponents() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
        opponents: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// A single race: the one mode that fields a grid because the mode says so,
/// rather than because a test override forced one.
fn load_single_race() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
        mode: oag_race::Mode::SingleRace,
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
fn a_ship_spawns_on_the_track_and_flies_along_it() {
    let Some(loaded) = load() else { return };
    let bound = envelope(&loaded);
    let handling = loaded.setup.handling;
    let mut race = race::Race::start(loaded.setup);

    // The spawn itself: on the track's authored `Start Position`, the suspension's
    // resting height above the surface, the right way up, and with the mass the force
    // law will read also in the body the integrator divides by.
    //
    // The height assertion below is worth more than it looks now that the spawn takes
    // its height off the *collision mesh* under the slot while this reads it off the
    // *spline*: agreeing to a hundredth means those two surfaces agree there, which
    // nothing made them do.
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
        // No saved language: these boot a fresh install every time.
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_game::frontend::Leg::LogoFmv,
        movie: Some(boot::DEFAULT_BOOT_MOVIE.to_string()),
        cache: std::env::temp_dir().join("oag-race-handoff"),
        audio_cache: oag_game::boot::default_audio_cache_dir(),
        extent: movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
    })
    .expect("loading the boot sequence");

    let mut frontend = boot.frontend;
    let mut input = Input::new();
    let dt = 1.0 / 60.0;

    // START skips the intro through `LogoFMVRedirectScreen`, cross picks a
    // language, and START again presses through `Show Logo`.
    input.begin_frame(1 << button::START);
    frontend.update(dt, &mut input, None);
    input.begin_frame(0);
    frontend.update(dt, &mut input, None);
    assert!(
        frontend.machine().is(states::LANGUAGE_SELECTION),
        "got as far as {:?}",
        frontend.machine().current()
    );

    input.begin_frame(1 << button::CROSS);
    frontend.update(dt, &mut input, None);
    assert!(frontend.machine().is(states::SHOW_LOGO));

    // `Show Logo` is the disc's PRESS START screen and it sits between the
    // picker and everything after it, so the front end is not finished until
    // START goes through it.
    input.begin_frame(1 << button::START);
    frontend.update(dt, &mut input, None);
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
/// `docs/physics/force-balance-ground-truth.md`.
///
/// **The "contact generation and the missing `cross(r, impulse)` angular response"
/// this used to name as the remaining gaps are both closed**, and were already
/// closed when that sentence was written - `oag_physics::wall` has the ten box
/// sample points, the angular share of the denominator, the `0.1`-scaled angular
/// application, per-triangle contacts and the original's single-sided rejection.
/// What a whole-lap comparison diverges on now is an open question rather than a
/// known omission.
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

/// Which way the shipped `Wall` triangles are wound, measured rather than assumed.
///
/// **This is the measurement that gates single-sided wall contacts.**
/// `Collision_BoxAgainstMesh` (`0x08815cd4`) does not flip a triangle normal; it
/// *rejects* the sample point when `dot(boxCentre - sample, n) <= 0`, so the raw
/// winding normal is what reaches the contact - see
/// `docs/ghidra/functions/psp-pulse-usa/collision.md`. `oag_physics::wall` flips
/// instead, which is safe under either winding and is why nobody has had to know
/// this. Reproducing the original's rejection is only correct if the shipped
/// walls actually face the circuit, and nothing in `docs/` says they do.
///
/// So: for every `Wall` triangle on the track, take its raw winding normal
/// `(b - a) x (c - a)` and ask whether it points toward the nearest point of the
/// track's own spline. A wall that faces the circuit answers yes.
///
/// The assertion is deliberately loose. Some walls genuinely face away - a
/// barrier with a driveable side and a scenery side, a tunnel mouth, geometry
/// past a junction where "nearest spline point" is the wrong reference - and a
/// track is not obliged to be tidy. What would sink single-siding is a *mixed*
/// result, so the claim is that one answer dominates, and the printed fraction
/// is the number a reader should take away rather than the pass.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_track_s_walls_are_wound_toward_the_circuit() {
    let Some(loaded) = load() else { return };
    let spline = &loaded.setup.spline;
    assert!(!spline.is_empty(), "the track produced no spline samples");

    let centres: Vec<Vec3> = (0..spline.len())
        .filter_map(|index| spline.sample(index))
        .map(|sample| Vec3::from_array(sample.pos))
        .collect();

    let mut inward = 0usize;
    let mut outward = 0usize;
    for collider in loaded.setup.collision.colliders() {
        if collider.surface() != oag_physics::Surface::Wall {
            continue;
        }
        for index in 0..collider.triangle_count() {
            let Some([a, b, c]) = collider.triangle(index) else {
                continue;
            };
            let normal = (b - a).cross(c - a);
            if normal.length_squared() <= 0.0 {
                continue;
            }
            let centroid = (a + b + c) / 3.0;
            let Some(nearest) = centres.iter().copied().min_by(|p, q| {
                (*p - centroid)
                    .length_squared()
                    .total_cmp(&(*q - centroid).length_squared())
            }) else {
                continue;
            };
            if normal.dot(nearest - centroid) > 0.0 {
                inward += 1;
            } else {
                outward += 1;
            }
        }
    }

    let total = inward + outward;
    assert!(total > 0, "the track has no wall triangles to measure");
    let fraction = inward as f32 / total as f32;
    println!(
        "{inward}/{total} wall triangles ({:.1}%) are wound toward the nearest spline point",
        fraction * 100.0
    );
    assert!(
        !(0.3..=0.7).contains(&fraction),
        "the shipped wall winding is mixed ({:.1}% inward), so a single-sided \
         contact gate cannot be implemented from it without more evidence",
        fraction * 100.0
    );
}

/// Where the original's own craft stands at the start of a Talon's Junction time
/// trial, read off `data/traces/talons-junction-standing-start.csv` at tick 0.
///
/// **Written in rather than read from the file, deliberately.** `data/` is
/// gitignored and derived captures under it have gone missing once already, taking
/// three documents' citations with them; a start pose is four numbers and the
/// point of this test is that they outlive the CSV. The heading is the recorded
/// craft basis, read left-up-forward - see `HANDOVER.md` on why the column names
/// cannot be taken at face value.
const CAPTURED_START: [f32; 3] = [6.07941, -50.06461, -195.9884];
const CAPTURED_FORWARD: [f32; 3] = [0.9998092, 0.004672341, -0.01896589];
const CAPTURED_LEFT: [f32; 3] = [-0.0188732, -0.01914353, -0.9996386];

/// The authored grid slot points where the original's craft points.
///
/// This is the check that says `Start Position` is read correctly rather than
/// merely parsed, and it is the *heading* that says it. Position cannot: the
/// authored slot and the pose a time trial starts from are 139.7 units apart, and
/// a distance that large is as consistent with a misread matrix as with a grid
/// laid out behind the line. A heading agreeing to a degree is not.
///
/// So the assertions are split by what each one can carry:
///
/// - **Heading, asserted tightly.** The slot's forward is within 1.2 degrees of
///   the craft's, and its left within 1.6 of the craft's left. Read the rows in
///   any other order, or take the recorded basis at its column names, and these
///   are tens of degrees out.
/// - **Position, asserted only in the slot's own frame**, where the separation
///   decomposes into 137.9 units *along* the slot's forward, 22.4 to its left and
///   0.8 up. Those are the three numbers a grid hypothesis has to explain; none
///   of them is asserted as a value, only in sign and rough scale, because what
///   lays out the other seven slots has not been read.
///
/// The vertical figure is the one to be most careful with: 0.8 is the gap between
/// two *authored* heights on a track that falls away, not a ride height. See
/// `oag_gameplay::spawn::Pose::from_start_position` for why the spawn takes its
/// height off the collision geometry instead.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_authored_slot_points_where_the_original_starts() {
    let Some(loaded) = load() else { return };
    let slot = loaded
        .setup
        .start_position
        .expect("16_Track authors a Start Position");

    let forward = Vec3::from_array(slot.forward);
    let left = Vec3::from_array(slot.left);
    let heading = forward
        .dot(Vec3::from_array(CAPTURED_FORWARD))
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees();
    let sideways = left
        .dot(Vec3::from_array(CAPTURED_LEFT))
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees();

    let offset = Vec3::from_array(CAPTURED_START) - Vec3::from_array(slot.position);
    println!(
        "the authored slot is {:.2} units from the original's start pose: {:.2} ahead, \
         {:.2} left, {:.2} up; heading agrees to {heading:.2} degrees and left to \
         {sideways:.2}",
        offset.length(),
        offset.dot(forward),
        offset.dot(left),
        offset.dot(Vec3::from_array(slot.up)),
    );

    assert!(
        heading < 3.0,
        "the authored slot faces {heading:.2} degrees away from where the original's craft \
         faces at the start line, so the matrix rows are not being read as left-up-forward"
    );
    assert!(
        sideways < 3.0,
        "the authored slot's left is {sideways:.2} degrees off the craft's left"
    );

    // Behind the line, not past it: the slot is upstream of where a time trial
    // begins, which is what a grid slot is and what a start-line marker is not.
    assert!(
        offset.dot(forward) > 50.0,
        "the original starts {:.2} units along the slot's own forward, which is not behind it",
        offset.dot(forward)
    );
    // And across the track from it, by about a track column rather than a
    // rounding error - the two are not the same slot.
    assert!(
        offset.dot(left).abs() > 5.0,
        "the original starts {:.2} units to the slot's side, so the two may be one slot",
        offset.dot(left)
    );
}

/// The reference capture is on `track.vex`, not `track_reversed.vex`.
///
/// **This is the `01_Track` trap one level down, and it had to be measured.** M3
/// settled the track by casting the recording's own positions against every
/// track's *collision geometry* and finding one where all of them land - see
/// `oag_game::race::DEFAULT_TRACK`. A forward layout and its reverse share that
/// geometry, so that test could not tell the two apart, and something has to.
///
/// What made the question live rather than theoretical: `track_reversed.vex`'s
/// authored slot sits **2.2 units** from where the original's craft starts, `y`
/// agreeing to 0.0096, and **4.011 units** above its own collision surface
/// against a resting craft height of 4.002-4.009. Three coincidences pointing at
/// the reversed layout, against `track.vex`'s slot at 139.7 units. Only the
/// heading separates them, so the heading is what this asks.
///
/// It is not close: the recorded craft faces **along** `track.vex`'s tangent and
/// **against** the reversed variant's, by construction of the pair. So the
/// coincidences are exactly that - the two layouts' grids bracket one shared
/// start line, and the reversed grid's authored slot happens to land near the
/// forward grid's pole.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_captured_start_pose_faces_along_the_forward_layout() {
    let Some(image) = image() else { return };
    let forward = Vec3::from_array(CAPTURED_FORWARD);
    let at = Vec3::from_array(CAPTURED_START);

    let mut readings = Vec::new();
    for variant in ["track.vex", "track_reversed.vex"] {
        let name = format!(r"Data\Environments\16_Track\{variant}");
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            track: name.clone(),
            class: SpeedClass::Venom,
            ..race::Options::default()
        })
        .expect("loading the race");

        let (_, sample, _) = loaded
            .setup
            .spline
            .nearest(at)
            .expect("a resampled spline has samples");
        let tangent = Vec3::from_array(sample.tangent).normalize_or_zero();
        let along = forward.dot(tangent);
        println!("{variant}: the recorded craft faces the tangent at {along:.4}");
        readings.push((variant, along));
    }

    assert!(
        readings[0].1 > 0.9,
        "the capture faces {:.4} along track.vex's tangent, so the reference scenario is not          on the layout race::DEFAULT_TRACK names",
        readings[0].1
    );
    assert!(
        readings[1].1 < -0.9,
        "the capture faces {:.4} along track_reversed.vex's tangent, which should be the          negation of the forward layout's",
        readings[1].1
    );
}

/// A ship spawned on the authored slot is where the hover law wants it.
///
/// The spawn takes its height off the collision surface under the slot rather
/// than out of the slot's own `y`, and this is what that buys: both probes in
/// contact on the very first tick, on the track the reference scenario uses.
/// Asserting it here rather than trusting the arithmetic, because the authored
/// height ranges over six units across the shipped tracks and a spawn that only
/// works on one of them would still pass every unit test.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ship_spawned_on_the_authored_slot_starts_in_contact() {
    let Some(loaded) = load() else { return };
    let handling = loaded.setup.handling;
    let colliders = loaded.setup.collision.clone();
    let slot = loaded
        .setup
        .start_position
        .expect("16_Track authors a Start Position");
    let mut race = race::Race::start(loaded.setup);

    let start = race.telemetry();
    println!(
        "spawned at {:?}, {:.3} above the slot's own authored y",
        start.position,
        start.position.y - slot.position[1]
    );

    // Facing where the slot says, still, after the pose has been through a
    // quaternion and back out of the body.
    let forward = race.ship().physics.body.forward();
    assert!(
        forward.dot(Vec3::from_array(slot.forward)) > 0.999,
        "the spawned ship faces {forward} and the slot says {:?}",
        slot.forward
    );

    // One tick, so the hover probes have run.
    let mut held = race::HeldButtons::new(0);
    race.tick(&held.snapshot());
    assert_eq!(
        race.ship().physics.grounded,
        1.0,
        "both hover probes must reach the track from the authored slot, got {}: {}",
        race.ship().physics.grounded,
        race::describe(&race.telemetry())
    );

    // And the surface really is under it, at the height the spawn claims.
    let origin = start.position + Vec3::Y * 20.0;
    let hit = oag_physics::Raycaster::raycast(
        &colliders,
        oag_physics::Ray::new(origin, Vec3::NEG_Y, 80.0),
        None,
        false,
    )
    .expect("the authored slot has track under it");
    let above = start.position.y - hit.point.y;
    assert!(
        (above - race::spawn_height(&handling)).abs() < 0.01,
        "the ship spawned {above:.3} above the surface, wanted {:.3}",
        race::spawn_height(&handling)
    );
}

/// The whole grid lands on the track, in the formation measured off the
/// original, and every craft is over solid geometry.
///
/// The unit tests in `oag_gameplay::spawn` pin the *arithmetic* against an
/// identity node; this pins it against a real track, where each slot is dropped
/// onto its own footprint and the surface is neither flat nor straight over the
/// grid's 138 units. A layout that is right in the abstract and buries the front
/// row is still wrong.
/// The AI's own composition check: eight craft, driven, on a real circuit.
///
/// Everything under this has synthetic tests - `oag-ai`'s controller against a
/// circle, `race::tests` against a straight - and none of them can see the thing
/// that actually goes wrong: whether the racing line built from the *disc's* own
/// `racing_line` offsets runs where the collision geometry is, and whether a
/// craft flown along it stays on the track rather than understeering into the
/// first wall.
///
/// **Deliberately not a lap-time assertion.** Nothing here claims the field is
/// fast, or that it is as fast as the original's - there is no measurement of
/// the original's opponents to compare against, and the speed law is this
/// project's own rather than a recovery. What is asserted is that they left the
/// grid, went the right way, made progress and were still on the track after ten
/// seconds. See `docs/gameplay/ai.md`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_ai_drives_the_field_along_the_track() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let bound = envelope(&loaded);
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(
        race.ship_count(),
        8,
        "a single race should field a full grid without an override"
    );

    let start: Vec<_> = race
        .world
        .ships
        .iter()
        .map(|ship| ship.physics.body.position)
        .collect();
    let heading: Vec<_> = race
        .world
        .ships
        .iter()
        .map(|ship| ship.physics.body.forward())
        .collect();

    // Ten seconds, the same window `a_ship_stays_on_the_track_for_ten_seconds`
    // uses, and with the player released so that what moves is the AI.
    for _ in 0..600 {
        race.tick(&oag_gameplay::InputSnapshot::default());
    }

    for slot in 1..8 {
        let ship = &race.world.ships[slot];
        let moved = ship.physics.body.position - start[slot];

        assert!(
            moved.length() > 10.0,
            "opponent {slot} moved {:.2} units in ten seconds, so it is not being driven",
            moved.length()
        );
        assert!(
            moved.dot(heading[slot]) > 0.0,
            "opponent {slot} went backwards along the grid's own forward"
        );
        assert!(
            ship.driver.index != 0,
            "opponent {slot}'s driver never left the line's first point"
        );

        let distance = race
            .spline()
            .distance_to(ship.physics.body.position)
            .expect("the track has samples");
        assert!(
            distance < bound,
            "opponent {slot} is {distance:.2} from the track, past the {bound:.2} envelope"
        );
        assert!(
            ship.physics.body.position.is_finite(),
            "opponent {slot} left the world"
        );
    }
}

/// Two runs of one race are one race, opponents included.
///
/// The drivers hold their state on the ships and therefore inside the world
/// snapshot, which is the property that makes this true - see
/// `oag_gameplay::Ship::driver`. It is asserted on real geometry because that is
/// where a divergence would come from: the collision query, not the controller.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_driven_field_replays_identically() {
    let (Some(first), Some(second)) = (load_single_race(), load_single_race()) else {
        return;
    };
    let mut first = race::Race::start(first.setup);
    let mut second = race::Race::start(second.setup);

    for _ in 0..300 {
        first.tick(&oag_gameplay::InputSnapshot::default());
        second.tick(&oag_gameplay::InputSnapshot::default());
    }

    for slot in 0..8 {
        assert_eq!(
            first.world.ships[slot].physics.body.position,
            second.world.ships[slot].physics.body.position,
            "slot {slot} diverged between two runs of the same race"
        );
        assert_eq!(
            first.world.ships[slot].driver, second.world.ships[slot].driver,
            "slot {slot}'s driver diverged between two runs of the same race"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_whole_grid_lands_on_the_track() {
    let Some(loaded) = load_with_opponents() else {
        return;
    };
    let bound = envelope(&loaded);
    let colliders = loaded.setup.collision.clone();
    let race = race::Race::start(loaded.setup);

    assert_eq!(race.ship_count(), 8, "16_Track should field a full grid");
    let ships: Vec<_> = race.world.ships.iter().filter(|s| s.active).collect();
    assert_eq!(ships.len(), 8);

    // The player is array index 0 and grid slot 8, so it is the rearmost. Every
    // opponent is ahead of it along its own forward axis.
    let player = ships[0].physics.body;
    let forward = player.forward();
    for (index, ship) in ships.iter().enumerate().skip(1) {
        let ahead = (ship.physics.body.position - player.position).dot(forward);
        assert!(
            ahead > 0.0,
            "ship {index} is {ahead:.2} along the player's forward, so it is behind them"
        );
    }

    // Two staggered columns, across the player's own **left**. Slot 8 is the
    // player and sits on the node's line; the odd slots sit off it. A whole unit of
    // tolerance because each slot was re-dropped onto its own footprint and the
    // track rolls under the grid.
    let left = -player.right();
    for (index, ship) in ships.iter().enumerate() {
        let slot = if index == 0 { 8 } else { index as u8 };
        let lateral = (ship.physics.body.position - player.position).dot(left);
        let expected = if slot % 2 == 1 {
            oag_gameplay::GRID_COLUMN_OFFSET
        } else {
            0.0
        };
        assert!(
            (lateral - expected).abs() < 1.0,
            "slot {slot} sits {lateral:.2} across the grid, expected about {expected:.1}"
        );
    }

    // And every one of them is over the track rather than over a gap, inside a
    // wall or out in the scenery. Two independent checks, because either alone
    // passes somewhere a race never goes: near the driveable line, and with
    // collision geometry underneath.
    for (index, ship) in ships.iter().enumerate() {
        let position = ship.physics.body.position;
        let distance = race
            .spline()
            .distance_to(position)
            .expect("the track has samples");
        assert!(
            distance < bound,
            "ship {index} is {distance:.2} from the driveable line, \
             outside the {bound:.2} envelope"
        );
        let origin = position + Vec3::Y * 20.0;
        let ray = oag_physics::Ray::new(origin, Vec3::NEG_Y, 80.0);
        assert!(
            oag_physics::Raycaster::raycast(&colliders, ray, None, false).is_some(),
            "ship {index} has no collision surface under it"
        );
    }
}

/// Where the original's own eight craft sit on Talon's Junction's grid.
///
/// Read out of PPSSPP's memory on 2026-08-10 while the countdown held them in
/// place, slot 1 first: `psp-drive.py menu --single-race`, then the racer table
/// at `0x08b34420`. Fourteen samples agreed to a thousandth, so these are the
/// original's numbers rather than one noisy frame. Method and the rest of the
/// measurement: `docs/ghidra/functions/psp-pulse-usa/grid.md`.
///
/// Slot 1 is worth recognising: `(6.125, -50.064, -195.945)` is where a *time
/// trial* starts, which is the other end of the same grid.
const ORIGINAL_GRID: [[f32; 3]; 8] = [
    [6.125, -50.064, -195.945],
    [-13.537, -49.709, -175.897],
    [-33.428, -50.121, -195.817],
    [-53.161, -49.696, -175.727],
    [-72.998, -50.026, -195.615],
    [-92.777, -49.651, -175.494],
    [-112.565, -49.847, -195.362],
    [-132.304, -49.577, -175.223],
];

/// Our grid is the original's grid, slot for slot, on the same track.
///
/// The strongest check available on this layout, and much stronger than the
/// envelope test above: it compares against the thing itself rather than against
/// a bound. Every slot has to land, so a formation that is right on average and
/// wrong at one end fails here.
///
/// **Three units of tolerance, and where it goes.** Our anchor is the authored
/// `Start Position` node dropped onto the collision mesh, and that lands 1.84
/// units from the original's own slot 8 - so about two units of the budget is
/// spent before the layout is consulted at all. The rest covers each slot being
/// re-dropped onto its own footprint, which the original may not do the same way.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn our_grid_is_the_originals_grid() {
    let Some(loaded) = load_with_opponents() else {
        return;
    };
    let race = race::Race::start(loaded.setup);
    assert_eq!(race.ship_count(), 8);

    // Array index 0 is the player and grid slot 8; indices 1..=7 are slots 1..=7.
    let slot_of = |index: usize| if index == 0 { 8 } else { index };
    let mut worst = 0.0f32;
    for (index, ship) in race
        .world
        .ships
        .iter()
        .enumerate()
        .filter(|(_, ship)| ship.active)
    {
        let slot = slot_of(index);
        let want = Vec3::from_array(ORIGINAL_GRID[slot - 1]);
        let got = ship.physics.body.position;
        let error = (got - want).length();
        worst = worst.max(error);
        println!("slot {slot}: ours {got:?}, original {want:?}, {error:.3} apart");
        assert!(
            error < 3.0,
            "slot {slot} is {error:.3} from where the original puts it"
        );
    }
    println!("worst slot: {worst:.3} units");
}

/// The weapon pads are drawn where they trigger.
///
/// Two independent decodes of the same nodes have to agree: `mesh::build_weapon_pads`
/// reads them as geometry through the mesh path, and `oag_formats::pads::volumes`
/// reads them as trigger boxes through the payload path. Neither knows about the
/// other, so a transform applied in one and not the other shows up here and
/// nowhere else - and it is exactly the mistake that is easy to make, because a
/// pad's payload is a *mesh* payload and its placement comes from the parent
/// chain rather than from itself.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_weapon_pads_are_drawn_where_they_trigger() {
    let Some(loaded) = load() else { return };
    let model = loaded
        .weapon_pad_model
        .as_ref()
        .expect("16_Track authors Weapon Pad geometry");
    assert!(!model.indices.is_empty());

    let volumes = &loaded.setup.weapon_pads;
    assert!(
        !volumes.is_empty(),
        "the track decodes no weapon pad volumes"
    );
    println!(
        "{} weapon pad volume(s), {} triangles drawn",
        volumes.len(),
        model.indices.len() / 3
    );

    // Every trigger volume has drawn geometry sitting in it. Ten units of slack
    // because a pad's mesh is a flat plate and its volume is a box around the
    // craft that crosses it, so the two are the same *place* rather than the
    // same extent.
    for (index, pad) in volumes.iter().enumerate() {
        let centre = Vec3::from_array(pad.centre());
        let nearest = model
            .vertices
            .iter()
            .map(|v| (Vec3::from_array(v.position) - centre).length())
            .fold(f32::INFINITY, f32::min);
        assert!(
            nearest < 10.0,
            "weapon pad {index} triggers at {centre:?} and the nearest drawn \
             vertex is {nearest:.2} away"
        );
    }
}

/// **The pickup chain, on the disc's own track and the disc's own weapon
/// table.** The unit tests in `oag_game::race` run it against a synthetic pad
/// and a hand-written table, so they check the logic against this project's
/// reading of both formats. This checks it against the shipped bytes: a real
/// `Weapon Pad` volume out of `16_Track`, real `<Pickupodds>` out of
/// `WeaponStats_Race.xml`, and the real `<WeaponPad refresh_time>` out of
/// `HandlingStats.xml`.
///
/// Three things it pins, in order of how easily each could break silently:
///
/// 1. **A single race arms the pads and the three solo modes do not.** This is
///    the one part of the pickup system that is wholly recovered - a
///    weapons-off race in the original hides every pad and zeroes the trigger
///    list's count - so it is the part with a right answer to fail against.
/// 2. **Crossing one grants a pickup**, which is this project's reading rather
///    than a ported branch: no grant call site has been found. What is checked
///    here is that the chain reaches the inventory at all, not that the
///    original does the same.
/// 3. **The class's authored odds resolve.** `HandlingStats.xml` spells a speed
///    class `VENOM` and `WeaponStats_Race.xml` spells it `Venom`, so a lookup
///    that matched case-sensitively would find no table and hand out nothing -
///    a failure that looks exactly like "the pad did not fire".
///
/// See `docs/gameplay/pickups.md`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_weapon_pad_on_the_disc_hands_out_a_pickup_in_a_single_race() {
    let Some(image) = image() else { return };

    // Where to put the craft: the first weapon pad the track authors, lifted to
    // the pad's own centre. Taken from a plain load so the position comes from
    // the same decode `the_weapon_pads_are_drawn_where_they_trigger` checks.
    let Some(scouted) = load() else { return };
    let pad = *scouted
        .setup
        .weapon_pads
        .first()
        .expect("16_Track authors weapon pads");
    let centre = Vec3::from_array(pad.centre());
    println!("first weapon pad at {centre:?}");

    let race_at_pad_seeded = |mode: oag_race::Mode, seed: Option<u64>| {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: SpeedClass::Venom,
            mode,
            seed,
            pose: Some(race::PoseRequest::SplineAligned {
                position: centre,
                yaw: 0.0,
            }),
            ..race::Options::default()
        })
        .expect("loading the race");
        race::Race::start(loaded.setup)
    };
    let race_at_pad = |mode: oag_race::Mode| race_at_pad_seeded(mode, None);

    // A single race: the craft starts inside the volume, so the first tick is
    // the entry edge.
    let mut single = race_at_pad(oag_race::Mode::SingleRace);
    single.tick(&Default::default());
    let granted = single.ship_pickup();
    let granted = granted.expect(
        "a craft standing on a real weapon pad was handed nothing - either the \
         trigger missed the volume or the class's <Pickupodds> did not resolve",
    );
    assert!(
        oag_gameplay::pickup::IMPLEMENTED.contains(&granted),
        "the pad handed out {granted:?}, which nothing can fire or absorb"
    );
    println!("granted {granted:?} off the disc's own odds");

    // **The magnitude check below is about the Turbo specifically**, so it needs
    // a crossing that draws one. That used to be automatic - `IMPLEMENTED` held
    // a single weapon - and stopped being so the moment it held more. Scanning
    // seeds rather than pinning a magic one, because a magic constant would go
    // quietly wrong the next time `IMPLEMENTED` grows and the walk order shifts,
    // which is the failure this whole comment exists to prevent.
    let turbo_seed = (0..64u64).find(|&seed| {
        let mut race = race_at_pad_seeded(oag_race::Mode::SingleRace, Some(seed));
        race.tick(&Default::default());
        race.ship_pickup() == Some(oag_formats::weapons::Weapon::Turbo)
    });
    let turbo_seed = turbo_seed.expect(
        "no seed in 0..64 drew a Turbo from this class's authored odds - either \
         the table weights it at zero or the draw is broken",
    );
    println!("seed {turbo_seed} draws a Turbo");

    // **And firing it visibly accelerates the craft, on the disc's own
    // numbers.** This is the assertion the first version of this feature
    // needed and did not have: it wired the neighbouring `1.2` multiplier
    // instead of the turbo add, every unit test passed because a ratio is a
    // ratio, and the effect was imperceptible in a real race because `1.2`
    // applied to a thrust already clamped to `0.5 * speed + accelcap` is a few
    // units of force. Comparing *speed after a second* against a control makes
    // the magnitude the thing under test rather than the ratio.
    let mut control = race_at_pad_seeded(oag_race::Mode::SingleRace, Some(turbo_seed));
    let mut fired = race_at_pad_seeded(oag_race::Mode::SingleRace, Some(turbo_seed));
    let mut control_buttons = Input::new();
    let mut fired_buttons = Input::new();
    const CROSS: u32 = 1 << button::CROSS;
    const SQUARE: u32 = 1 << button::SQUARE;

    let snapshot = |buttons: &mut Input, mask: u32| {
        buttons.begin_frame(mask);
        oag_gameplay::InputSnapshot {
            buttons: *buttons,
            ..Default::default()
        }
    };

    // Both hold thrust; one of them also presses fire on the tick after the
    // pickup lands.
    control.tick(&snapshot(&mut control_buttons, CROSS));
    fired.tick(&snapshot(&mut fired_buttons, CROSS));
    control.tick(&snapshot(&mut control_buttons, CROSS));
    fired.tick(&snapshot(&mut fired_buttons, CROSS | SQUARE));
    assert_eq!(fired.ship_pickup(), None, "firing must spend the pickup");
    assert!(
        fired.ship().physics.turbo_timer > 0.0,
        "firing must arm the turbo"
    );

    for _ in 0..60 {
        control.tick(&snapshot(&mut control_buttons, CROSS));
        fired.tick(&snapshot(&mut fired_buttons, CROSS));
    }
    let speed = |race: &race::Race| race.ship().physics.body.linear_velocity.length();
    let (boosted, plain) = (speed(&fired), speed(&control));
    println!("after one second: {boosted:.1} boosted against {plain:.1} plain");
    // Twice the control's speed. A deliberately coarse bar: what it is there to
    // reject is a boost that is technically applied and imperceptible, so it
    // has to be far above measurement noise rather than just above zero.
    assert!(
        boosted > plain * 2.0,
        "a fired turbo left the craft at {boosted:.1} against {plain:.1} - that \
         is not a turbo"
    );

    // And the three weapons-off modes hand out nothing from the same spot, for
    // any number of ticks. Zone is the one with no free turbo either, so it is
    // the clean negative; a time trial and a speed lap are checked for the
    // *pad* specifically by standing still, which never completes a lap.
    for mode in [
        oag_race::Mode::TimeTrial,
        oag_race::Mode::SpeedLap,
        oag_race::Mode::Zone,
    ] {
        let mut race = race_at_pad(mode);
        for _ in 0..120 {
            race.tick(&Default::default());
        }
        assert_eq!(
            race.ship_pickup(),
            None,
            "{mode:?} handed out a pickup, and the original arms no pads for it"
        );
    }
}

/// A rocket fired on a real track flies and detonates on the track's own
/// geometry.
///
/// # What this is for, and what the unit tests cannot do
///
/// `oag_gameplay::projectile`'s own tests fly rockets down a synthetic corridor:
/// one quad, authored by the test, at a distance the test chose. **Every
/// number in them is invented**, including the geometry. This is the only check
/// that a rocket at the disc's own authored speed, launched from the disc's own
/// hull dimensions, meets the disc's own collision soup at all: a launch offset
/// slightly inside the hull, a class speed read from the wrong attribute, or a
/// swept step long enough to tunnel through a real wall would all pass every
/// unit test and fail here.
///
/// The craft stands on the first weapon pad the track authors, as
/// `a_weapon_pad_on_the_disc_hands_out_a_pickup_in_a_single_race` does, and the
/// seed is scanned for a Rocket draw for the reason that test gives at length.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_rocket_fired_on_a_real_track_flies_and_detonates() {
    let Some(image) = image() else { return };
    let Some(scouted) = load() else { return };
    let pad = *scouted
        .setup
        .weapon_pads
        .first()
        .expect("16_Track authors weapon pads");
    let centre = Vec3::from_array(pad.centre());

    let race_at_pad = |seed: u64| {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: SpeedClass::Venom,
            mode: oag_race::Mode::SingleRace,
            seed: Some(seed),
            pose: Some(race::PoseRequest::SplineAligned {
                position: centre,
                yaw: 0.0,
            }),
            ..race::Options::default()
        })
        .expect("loading the race");
        race::Race::start(loaded.setup)
    };

    // The class's authored odds decide this, so scan rather than pin - see the
    // Turbo test's own comment.
    let rocket_seed = (0..64u64)
        .find(|&seed| {
            let mut race = race_at_pad(seed);
            race.tick(&Default::default());
            race.ship_pickup() == Some(oag_formats::weapons::Weapon::Rocket)
        })
        .expect(
            "no seed in 0..64 drew a Rocket from this class's authored odds - either the \
             table weights it at zero or the draw is broken",
        );
    println!("seed {rocket_seed} draws a Rocket");

    let mut race = race_at_pad(rocket_seed);
    let mut buttons = Input::new();
    let snapshot = |buttons: &mut Input, mask: u32| {
        buttons.begin_frame(mask);
        oag_gameplay::InputSnapshot {
            buttons: *buttons,
            ..Default::default()
        }
    };
    const SQUARE: u32 = 1 << button::SQUARE;

    race.tick(&snapshot(&mut buttons, 0));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_formats::weapons::Weapon::Rocket)
    );

    let muzzle = race.ship().physics.body.position;
    race.tick(&snapshot(&mut buttons, SQUARE));
    assert_eq!(race.ship_pickup(), None, "firing must spend the pickup");
    assert_eq!(
        race.world.projectiles.live(),
        1,
        "firing a rocket on a real track put nothing in the air"
    );

    // It has to *travel*, which is what says the class speed was read at all,
    // and it has to stop, which is what says it met the real collision soup
    // rather than flying through it to the lifetime cap.
    let mut ticks = 1;
    let mut furthest = 0.0f32;
    while race.world.projectiles.live() > 0 {
        let flying = race.world.projectiles.slots[0].position;
        furthest = furthest.max((flying - muzzle).length());
        race.tick(&snapshot(&mut buttons, 0));
        ticks += 1;
        assert!(ticks < 1200, "the rocket never stopped");
    }
    println!("the rocket flew {furthest:.1} units over {ticks} tick(s)");
    assert!(
        furthest > 10.0,
        "the rocket travelled {furthest:.1} units, which is not flight"
    );
    // 10 seconds at any authored class speed is thousands of units, so a rocket
    // that merely aged out would show a far larger distance than a wall on a
    // real circuit can be from a weapon pad.
    let seconds = ticks as f32 * race.dt();
    assert!(
        seconds < oag_gameplay::projectile::MAX_FLIGHT_SECONDS,
        "the rocket aged out after {seconds:.2} s instead of hitting the track"
    );
}

/// The field is placed, which is what makes it a race rather than a parade.
///
/// **The property, not a result.** Nothing here asserts who wins - that is the
/// AI's speed law and this project's own, so a pinned finishing order would be
/// pinning our own arithmetic. What is asserted is that the places form a valid
/// permutation, that the player's place agrees with the table, and that being
/// further round the circuit is what earns a better one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_field_is_placed_by_how_far_round_it_is() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..900 {
        race.tick(&oag_gameplay::InputSnapshot::default());
    }

    let places = race.places();
    let mut seen: Vec<u8> = places.to_vec();
    seen.sort_unstable();
    assert_eq!(
        seen,
        (1..=8).collect::<Vec<u8>>(),
        "the places are not a permutation of 1..=8: {places:?}"
    );
    assert_eq!(race.player_place(), places[0]);

    // The craft in first is the one furthest round, since nobody has finished a
    // three-lap race in fifteen seconds.
    let leader = places
        .iter()
        .position(|&place| place == 1)
        .expect("somebody is first");
    let course = race.course().expect("16_Track closes");
    let furthest = (0..8)
        .max_by(|&a, &b| {
            race.world.ships[a]
                .standing
                .distance(course)
                .total_cmp(&race.world.ships[b].standing.distance(course))
        })
        .expect("eight craft");
    assert_eq!(
        leader, furthest,
        "slot {leader} is placed first but slot {furthest} is further round"
    );
}
