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
//! was the bug - see [`oag_raceplay::DEFAULT_TRACK`] - and the right one is grounded
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

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_game::{boot, movie};
use oag_gameplay::{PlayerInputs, input::Button, input::Input};
use oag_physics::Raycaster;
use oag_raceplay::{self as race, catalogue};
use oag_ui::frontend::states;

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
        class: "VENOM".to_string(),
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
        class: "VENOM".to_string(),
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

/// Ticks the handoff test flies: the 272-tick countdown, where nothing is driven
/// (`grid-state.md`: a craft held on the grid does not move), then 60 more, half
/// the two seconds `ship_spawn_ground_truth.rs` drives. The question is whether
/// `Launch Game` produces a driveable ship, not how long one stays driveable.
const HANDOFF_TICKS: u32 = oag_race::state::COUNTDOWN_TICKS as u32 + 60;

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
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: Some(boot::DEFAULT_BOOT_MOVIE.to_string()),
        cache: std::env::temp_dir().join("oag-race-handoff"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    })
    .expect("loading the boot sequence");

    let mut frontend = boot.frontend;
    let mut input = Input::new();
    let dt = 1.0 / 60.0;

    // START skips the intro through `LogoFMVRedirectScreen`, cross picks a
    // language, and START again presses through `Show Logo`.
    input.begin_frame(Button::Start.bit());
    frontend.update(dt, &mut input, None);
    input.begin_frame(0);
    frontend.update(dt, &mut input, None);
    assert!(
        frontend.machine().is(states::LANGUAGE_SELECTION),
        "got as far as {:?}",
        frontend.machine().current()
    );

    input.begin_frame(Button::Cross.bit());
    frontend.update(dt, &mut input, None);
    assert!(frontend.machine().is(states::SHOW_LOGO));

    // `Show Logo` is the disc's PRESS START screen and it sits between the
    // picker and everything after it, so the front end is not finished until
    // START goes through it.
    input.begin_frame(Button::Start.bit());
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
    let mut held = race::HeldButtons::new(Button::Cross.bit());
    let mut grounded_ticks = 0u32;

    for tick in 0..HANDOFF_TICKS {
        let snapshot = held.snapshot();
        race.tick(&PlayerInputs::single(snapshot));
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
    let mut held = race::HeldButtons::new(Button::Cross.bit());

    let mut worst_distance: f32 = 0.0;
    let mut worst_at = 0u32;
    let mut top_speed: f32 = 0.0;
    let mut grounded_ticks = 0u32;
    for tick in 0..600u32 {
        let snapshot = held.snapshot();
        race.tick(&PlayerInputs::single(snapshot));
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
/// `oag_raceplay::DEFAULT_TRACK`. A forward layout and its reverse share that
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
            track: Some(name.clone()),
            class: "VENOM".to_string(),
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
    race.tick(&PlayerInputs::single(held.snapshot()));
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
        .sim
        .world
        .ships
        .iter()
        .map(|ship| ship.physics.body.position)
        .collect();
    let heading: Vec<_> = race
        .sim
        .world
        .ships
        .iter()
        .map(|ship| ship.physics.body.forward())
        .collect();

    // Ten seconds, the same window `a_ship_stays_on_the_track_for_ten_seconds`
    // uses, and with the player released so that what moves is the AI.
    for _ in 0..600 {
        race.tick(&PlayerInputs::none());
    }

    for slot in 1..8 {
        let ship = &race.sim.world.ships[slot];
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

/// Every craft on the disc's own track burns, trails and boosts on its own.
///
/// The real-data half of the per-craft exhaust; `race::tests`' synthetic half
/// runs on a hand-built straight with no ship model, and so with no nozzle and no
/// pads at all. What only the disc can say is that the effect survives the real
/// article: a real `Engine Flare` locator on a real hull, and the track's own
/// authored speed pads under a field that actually drives over them.
///
/// **Three separate claims**, and the third is the one that used to be false:
///
/// 1. every craft's engine is lit and its ribbon ring is full,
/// 2. no two craft share a ribbon - each one's newest sample is at its own
///    nozzle,
/// 3. an opponent that crosses a speed pad gets a *plume*, not just the force.
///
/// Before 2026-08-11 there was one `Exhaust` and it belonged to slot 0, so the
/// seven opponents drove the whole circuit dark.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_craft_burns_trails_and_boosts_on_its_own() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(race.ship_count(), 8);
    assert!(
        race.nozzle().is_some(),
        "the disc's hull authors an Engine Flare node, so every craft has a nozzle"
    );
    assert!(
        !race.speedup_pads().is_empty(),
        "the third claim needs a track with authored speed pads"
    );

    // Twenty seconds. Ten is enough for the field to be driving and trailing;
    // reaching a speed pad takes longer, and this is the assertion that has to
    // observe one being crossed rather than assume it.
    //
    // **Watched every tick rather than sampled at the end**, and that is not
    // caution: the AI's throttle is bang-bang against a cornering-speed target,
    // so a craft braking into a corner on the final tick has its engine
    // legitimately off. Whether a craft's flare ever lit is the claim; what it
    // happens to be doing at tick 1,200 is the track's business.
    const TICKS: u32 = 1200;
    let mut lit = [0u32; 8];
    let mut boosted = [false; 8];
    for _ in 0..TICKS {
        race.tick(&PlayerInputs::none());
        for slot in 0..8 {
            lit[slot] += u32::from(race.exhaust_of(slot).engine_on());
            boosted[slot] |= race.exhaust_of(slot).boost_timer() > 0.0;
        }
    }

    let nozzles: Vec<Vec3> = (0..8)
        .map(|slot| race.nozzle_of(slot).expect("every craft has a nozzle"))
        .collect();
    for (slot, lit) in lit.iter().enumerate().skip(1) {
        let exhaust = race.exhaust_of(slot);
        // A quarter of the run, which separates "this craft is racing" from
        // both "its engine never came on" and "one stray tick". The AI holds
        // throttle down every straight and lifts for every corner, so the real
        // figure is far above this on any circuit - and the *final* tick's state
        // is deliberately not asserted at all: a craft braking into a corner at
        // tick 1,200 has its engine legitimately off and its intensity already
        // decayed to zero, which is what this counter replaced.
        assert!(
            *lit > TICKS / 4,
            "opponent {slot}'s engine was on for {lit} of {TICKS} ticks"
        );
        assert!(
            exhaust.trail_ready(),
            "opponent {slot} laid no ribbon in twenty seconds"
        );

        // The head of the ribbon sits on the rim of a cross centred on the
        // craft's own nozzle, so the nearest nozzle to it must be that craft's.
        // A shared `Exhaust` puts every head on whichever craft pushed last.
        let head = Vec3::from_array(exhaust.trail_vertices(Vec3::X, Vec3::Y)[0].position);
        let nearest = (0..8)
            .min_by(|a, b| {
                head.distance(nozzles[*a])
                    .total_cmp(&head.distance(nozzles[*b]))
            })
            .expect("eight craft");
        assert_eq!(
            nearest, slot,
            "opponent {slot}'s ribbon starts nearest slot {nearest}'s craft"
        );
    }

    // At least one *opponent*, which is the claim. Asserted over the field
    // rather than per slot: which craft finds a pad in twenty seconds is a
    // property of the racing line and the track, not something this test gets to
    // decide, and pinning a particular slot would make it a test of the AI's
    // route. The player is excluded outright - it is parked on the grid.
    assert!(
        boosted[1..].iter().any(|seen| *seen),
        "no opponent's plume was ever armed, so a pad's force is still reaching \
         craft its visual does not"
    );
}

/// The field spreads across the track's own AI corridor rather than queueing up
/// on one line.
///
/// **This is the real-data half of the personality work**; the synthetic half is
/// `oag-ai`'s `a_field_of_seeded_drivers_does_not_drive_one_line`, on an oval
/// with an invented corridor. What only real data can say is that the corridor
/// *off the disc* is wide enough to be worth spending and that the bounds were
/// rebased onto the racing line the right way round - a sign error there would
/// put the whole field on one side of the line, which is the same queue moved
/// sideways.
///
/// Signed offsets, in the sample's own lateral axis, measured off each craft's
/// own place on the line. The bound is the corridor's, so a track whose artists
/// left less room asserts less.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_field_spreads_across_the_ai_corridor() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(race.ship_count(), 8);

    // Ten seconds, the same window the driving test uses: long enough that each
    // craft has left the grid's own lateral stagger behind and settled onto the
    // line it chose.
    for _ in 0..600 {
        race.tick(&PlayerInputs::none());
    }

    let mut offsets = Vec::new();
    let mut room = f32::INFINITY;
    for slot in 1..8 {
        let ship = &race.sim.world.ships[slot];
        let sample = race
            .spline()
            .sample(ship.driver.index as usize)
            .expect("the driver stands on a sample");
        let lateral = Vec3::from_array(sample.lateral).normalize_or_zero();
        let line = Vec3::from_array(sample.pos)
            - oag_vex::track::HOVER_LIFT * Vec3::from_array(sample.down)
            + sample.racing_line * lateral;
        offsets.push((ship.physics.body.position - line).dot(lateral));
        room = room
            .min(sample.ai_bound_right - sample.racing_line)
            .min(sample.racing_line - sample.ai_bound_left);
    }

    let low = offsets.iter().copied().fold(f32::INFINITY, f32::min);
    let high = offsets.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        high - low > room,
        "the field spread {:.2} units across a corridor with {room:.2} to spare either side of \
         the line: {offsets:?}",
        high - low
    );
    // Both sides of it, or the rebasing has a sign error.
    assert!(
        low < 0.0 && high > 0.0,
        "the whole field sits on one side of the line: {offsets:?}"
    );
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
        first.tick(&PlayerInputs::none());
        second.tick(&PlayerInputs::none());
    }

    for slot in 0..8 {
        assert_eq!(
            first.sim.world.ships[slot].physics.body.position,
            second.sim.world.ships[slot].physics.body.position,
            "slot {slot} diverged between two runs of the same race"
        );
        assert_eq!(
            first.sim.world.ships[slot].driver, second.sim.world.ships[slot].driver,
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
    let ships: Vec<_> = race.sim.world.ships.iter().filter(|s| s.active).collect();
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

    // Two staggered columns, `GRID_COLUMN_OFFSET / 2` either side of the corridor
    // midpoint, the even slots on the node's side (`grid.md`). Two units: the
    // midpoint is read at the nearest resampled point, not the walked one.
    let off_midpoint = |p: Vec3| {
        let (_, s, _) = race.spline().nearest(p).expect("has samples");
        let lateral =
            (p - Vec3::from_array(s.pos)).dot(Vec3::from_array(s.lateral).normalize_or_zero());
        lateral - 0.5 * (s.ai_bound_left + s.ai_bound_right)
    };
    let side = off_midpoint(player.position).signum();
    for (index, ship) in ships.iter().enumerate() {
        let slot = if index == 0 { 8 } else { index as u8 };
        let want = if slot.is_multiple_of(2) { side } else { -side }
            * 0.5
            * oag_gameplay::GRID_COLUMN_OFFSET;
        let got = off_midpoint(ship.physics.body.position);
        assert!(
            (got - want).abs() < 2.0,
            "slot {slot} sits {got:.2} from the corridor midpoint, expected {want:.1}"
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
/// **One unit of tolerance, chosen, not measured**: the worst slot measures 0.62.
/// It was three, two of which were the raw node standing in for slot 8: the
/// original lays every slot `10` either side of the AI corridor's midpoint
/// (`grid.md`), and doing the same took the worst slot from 1.81 to 0.62.
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
        .sim
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
            error < 1.0,
            "slot {slot} is {error:.3} from where the original puts it"
        );
    }
    println!("worst slot: {worst:.3} units");
}

/// The weapon pads are drawn where they trigger.
///
/// Two independent decodes of the same nodes have to agree: `mesh::build_weapon_pads`
/// reads them as geometry through the mesh path, and `oag_vex::pads::volumes`
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
/// table.** The unit tests in `oag_raceplay` run it against a synthetic pad
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
            class: "VENOM".to_string(),
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
        oag_weapons::pickup::IMPLEMENTED.contains(&granted),
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
        race.ship_pickup() == Some(oag_tables::weapons::Weapon::Turbo)
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
    const CROSS: u32 = Button::Cross.bit();
    const SQUARE: u32 = Button::Square.bit();

    let snapshot = |buttons: &mut Input, mask: u32| {
        buttons.begin_frame(mask);
        oag_gameplay::InputSnapshot {
            buttons: *buttons,
            ..Default::default()
        }
    };

    // Both hold thrust; one of them also presses fire on the tick after the
    // pickup lands.
    control.tick(&PlayerInputs::single(snapshot(&mut control_buttons, CROSS)));
    fired.tick(&PlayerInputs::single(snapshot(&mut fired_buttons, CROSS)));
    control.tick(&PlayerInputs::single(snapshot(&mut control_buttons, CROSS)));
    fired.tick(&PlayerInputs::single(snapshot(
        &mut fired_buttons,
        CROSS | SQUARE,
    )));
    assert_eq!(fired.ship_pickup(), None, "firing must spend the pickup");
    assert!(
        fired.ship().physics.turbo_timer > 0.0,
        "firing must arm the turbo"
    );

    for _ in 0..60 {
        control.tick(&PlayerInputs::single(snapshot(&mut control_buttons, CROSS)));
        fired.tick(&PlayerInputs::single(snapshot(&mut fired_buttons, CROSS)));
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
    // any number of ticks. A time trial and a speed lap are handed a free
    // turbo at the countdown's release edge (`Race::tick`), cleared here first.
    for mode in [
        oag_race::Mode::TimeTrial,
        oag_race::Mode::SpeedLap,
        oag_race::Mode::Zone,
    ] {
        let mut race = race_at_pad(mode);
        race.sim.world.ships[0].pickup.weapon = None;
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
/// `oag_weapons::projectile`'s own tests fly rockets down a synthetic corridor:
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
            class: "VENOM".to_string(),
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
            race.ship_pickup() == Some(oag_tables::weapons::Weapon::Rocket)
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
    const SQUARE: u32 = Button::Square.bit();

    race.tick(&PlayerInputs::single(snapshot(&mut buttons, 0)));
    assert_eq!(
        race.ship_pickup(),
        Some(oag_tables::weapons::Weapon::Rocket)
    );

    let muzzle = race.ship().physics.body.position;
    race.tick(&PlayerInputs::single(snapshot(&mut buttons, SQUARE)));
    assert_eq!(race.ship_pickup(), None, "firing must spend the pickup");
    assert_eq!(
        race.sim.world.projectiles.live(),
        oag_weapons::projectile::ROCKET_SHOTS,
        "firing a rocket on a real track did not put a full volley in the air"
    );

    // **On the disc's own `spread`.** It authors `0.05` radians - under three
    // degrees - so the three fly very nearly parallel and diverge by only a few
    // units over the distance to a wall. That is the whole reason this is
    // asserted as a *fan* rather than eyeballed: at this angle a volley and
    // three copies of one shot look identical in a screenshot.
    let right = race.ship().physics.body.right();
    let lateral: Vec<f32> = (0..oag_weapons::projectile::ROCKET_SHOTS)
        .map(|slot| race.sim.world.projectiles.slots[slot].velocity.dot(right))
        .collect();
    println!("lateral velocity components off the disc's own spread: {lateral:?}");
    assert!(
        lateral.iter().any(|&l| l > 0.5) && lateral.iter().any(|&l| l < -0.5),
        "the volley did not fan on the disc's own numbers: {lateral:?}"
    );

    // It has to *travel*, which is what says the class speed was read at all,
    // and it has to stop, which is what says it met the real collision soup
    // rather than flying through it to the lifetime cap.
    //
    // **Measured per shot, not for the volley.** This used to run the clock
    // until the *last* rocket died and then assert that time was under the
    // reap, while only ever measuring slot 0 - two different rockets in one
    // assertion. On this circuit and at this firing point the disc's own
    // `spread` fans the third shot off the side, and it then falls for the
    // full five seconds because there is nothing under it to hit. That is
    // `Rocket_Update`'s own no-hit branch (`velocity.y -= dt * 50.0`, reaped
    // at `ROCKET_LIFETIME_SECONDS`), so the reap is the *correct* end for it -
    // and the failure it produced said "the rocket aged out instead of
    // hitting the track", which sent two passes looking for a speed bug that
    // was never there. The speed is exact: 222.22 units a second, the
    // authored `venomspeed` 800 alone over [`KMH_PER_UNIT_PER_SECOND`]
    // (277.78 with `launchSpeed` 200 added until 2026-10-01).
    let mut ticks = 1;
    let mut last_alive = [0usize; oag_weapons::projectile::ROCKET_SHOTS];
    let mut furthest = 0.0f32;
    let mut travelled = 0.0f32;
    let mut previous = race.sim.world.projectiles.slots[0].position;
    while race.sim.world.projectiles.live() > 0 {
        // Slot 0 is the middle rocket, the one that flies straight - the outer
        // two may detonate a tick either side of it.
        let flying = race.sim.world.projectiles.slots[0].position;
        if race.sim.world.projectiles.slots[0].kind.is_some() {
            furthest = furthest.max((flying - muzzle).length());
            travelled += (flying - previous).length();
            previous = flying;
        }
        for (slot, alive) in last_alive.iter_mut().enumerate() {
            if race.sim.world.projectiles.slots[slot].kind.is_some() {
                *alive = ticks;
            }
        }
        race.tick(&PlayerInputs::single(snapshot(&mut buttons, 0)));
        ticks += 1;
        assert!(ticks < 1200, "a rocket never stopped");
    }
    println!(
        "the middle rocket flew {travelled:.1} units of path, reaching \
         {furthest:.1} from the muzzle; last tick alive per shot: \
         {last_alive:?} of {ticks}"
    );
    assert!(
        furthest > 10.0,
        "the rocket travelled {furthest:.1} units, which is not flight"
    );

    // The reap in ticks, so a shot that ended on geometry is the one that
    // ended sooner than it.
    let reap = (oag_weapons::projectile::MAX_FLIGHT_SECONDS / race.dt()) as usize;
    let stopped = last_alive.iter().filter(|&&t| t < reap).count();

    // The middle shot is aimed down the craft's own forward axis, so it is
    // the one that must meet the circuit. If *this* ages out, either the
    // speed is wrong or the sweep is missing the soup - the two faults this
    // test exists to catch.
    assert!(
        last_alive[0] < reap,
        "the middle rocket aged out after {} tick(s) instead of hitting the \
         track; reap is {reap}",
        last_alive[0]
    );
    // And it must not be alone, or a volley that mostly flies through walls
    // would still pass on its one lucky shot.
    assert!(
        stopped >= 2,
        "only {stopped} of {} rockets met the collision soup: {last_alive:?}",
        oag_weapons::projectile::ROCKET_SHOTS
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
        race.tick(&PlayerInputs::none());
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
            race.sim.world.ships[a]
                .standing
                .distance(course)
                .total_cmp(&race.sim.world.ships[b].standing.distance(course))
        })
        .expect("eight craft");
    assert_eq!(
        leader, furthest,
        "slot {leader} is placed first but slot {furthest} is further round"
    );
}

/// The place reaches the HUD, which is the only place a player can read it.
///
/// `Race::places` had no reader outside the tests until 2026-08-11 - the table was
/// computed every tick and shown to nobody - so this asserts the *wiring* rather
/// than the ordering: the readout's place is the player's place, the field size is
/// the grid, and both halves of the widget group have a value at the same time.
/// `the_field_is_placed_by_how_far_round_it_is` above is what checks the ordering.
///
/// A single race is the mode that has a field at all, and `Arcade_HUD.xml` is the
/// only shipped layout with the place widgets - see
/// `oag_hud::place_owns_the_anchor`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_players_place_reaches_the_hud() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(
        race.ship_count(),
        8,
        "a single race with no field cannot test a place"
    );

    for _ in 0..900 {
        race.tick(&PlayerInputs::none());
    }

    let readout = race.readout();
    let course = race.course().expect("16_Track closes");
    let places = race.places();
    for (slot, place) in places.iter().enumerate() {
        let standing = race.sim.world.ships[slot].standing;
        println!(
            "slot {slot}: place {place}, lap {}, progress {:?}, distance {:.1}",
            standing.lap,
            standing.progress.map(|p| p.round()),
            standing.distance(course)
        );
    }
    println!(
        "the player is {} of {} after {} tick(s)",
        readout.place, readout.ships, 900
    );
    assert_eq!(readout.place, u32::from(race.player_place()));
    assert!(
        (1..=8).contains(&readout.place),
        "the HUD would draw a place of {}, which is not a position in a field of eight",
        readout.place
    );
    assert_eq!(readout.ships, 8);

    // Both halves together or neither: a lone `8` beside the `POS` caption was
    // what this HUD drew before the place was wired up.
    let context = loaded
        .hud
        .context()
        .expect("Arcade_HUD.xml parses; the widget-count test pins it");
    let frame = oag_hud::draw_list(&context, &readout);
    let drawn: Vec<&str> = frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    println!("the HUD's own font pass draws {drawn:?}");
    assert!(
        drawn.contains(&readout.place.to_string().as_str()),
        "the place is not on screen: {drawn:?}"
    );
    assert!(
        drawn.contains(&"8"),
        "the field size is not on screen: {drawn:?}"
    );
    assert!(
        drawn.contains(&"/"),
        "the separator is not on screen: {drawn:?}"
    );
}

/// An opponent actually fires, on a real circuit.
///
/// **The gate this is really about is the curvature one.** `wants_to_fire`
/// refuses a shot where the road between the firer and its target bends more
/// than `WEAPON_CURVATURE`, and a threshold set too tight would be invisible in
/// every synthetic test - a straight fixture always passes it - while making
/// the whole feature dead on real geometry. Only a real track can say whether
/// the number is a filter or a wall. The synthetic half is `oag-ai`'s
/// `a_driver_fires_at_a_craft_ahead_and_inside_its_cone`.
///
/// It hands the field the trigger rather than waiting for the pickup draw:
/// what is measured is whether the *aiming* gates ever open. Pinned to
/// `FireLaw::Ours`; the law's own check is `opponent_fire_ground_truth.rs`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_opponent_fires_at_a_craft_ahead_on_a_real_circuit() {
    let Some(loaded) = load_single_race() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    race.set_fire_law(race::FireLaw::Ours);
    assert_eq!(race.ship_count(), 8);

    let mut fired = 0usize;
    let mut owners = std::collections::BTreeSet::new();
    // Two minutes, and a rocket handed back every tick a craft is empty, so the
    // question is only ever whether the driver wants to use it.
    for _ in 0..7_200 {
        for slot in 1..8 {
            if race.sim.world.ships[slot].pickup.weapon.is_none() {
                race.sim.world.ships[slot].pickup.weapon =
                    Some(oag_tables::weapons::Weapon::Rocket);
            }
        }
        let before = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .filter(|p| p.kind.is_some())
            .count();
        race.tick(&PlayerInputs::none());
        for projectile in race.sim.world.projectiles.slots.iter() {
            if projectile.kind.is_some() && projectile.owner != 0 {
                owners.insert(projectile.owner);
            }
        }
        let after = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .filter(|p| p.kind.is_some())
            .count();
        if after > before {
            fired += after - before;
        }
    }

    assert!(
        fired > 0,
        "no opponent fired a rocket in two minutes of a real race, so the \
         aiming gates never open on real geometry"
    );
    assert!(
        !owners.is_empty(),
        "rockets were fired but none was owned by an opponent"
    );
    println!("opponents fired {fired} rockets; owners: {owners:?}");
}

/// One craft, alone, lapping.
///
/// **The benchmark the pace tuning should be judged against**, and the reason
/// it exists apart from the field tests: with seven craft on the circuit every
/// number is a mixture of how well a craft drives and how much the traffic cost
/// it, and the two move in opposite directions when aggression changes. A solo
/// lap is the driving alone. Tune here first, then check the field.
///
/// Returns the best lap the craft managed, in ticks.
fn solo_lap_ticks(level: oag_ai::Difficulty) -> Option<u64> {
    solo_lap_on(level, oag_pulse::race::DEFAULT_TRACK).best
}

/// What a lone craft managed on one circuit.
#[derive(Debug, Default, Clone)]
struct Solo {
    /// The quickest **clean** lap, in ticks.
    ///
    /// A lap the craft had to be recovered during is not counted, and that is
    /// the point of carrying [`Self::respawns`] alongside it. Respawning is a
    /// safety net for a race with seven other craft shoving; on an empty
    /// circuit a competent driver should never need it, so a benchmark that let
    /// recovery paper over a craft flying into the scenery would report the
    /// driving as fixed when only the symptom was.
    best: Option<u64>,
    /// How many times the craft had to be put back on the track.
    respawns: u32,
    /// How far round it got, in laps.
    laps: u32,
    /// The driver index the craft was on when each rescue fired, and the line's
    /// length, so a reader can tell whether the losses cluster anywhere.
    lost_at: Vec<u32>,
    line_len: u32,
}

/// The same, on a named circuit.
fn solo_lap_on(level: oag_ai::Difficulty, track: &str) -> Solo {
    solo_lap_tuned(level, track, None)
}

/// The same, with the drivers' tuning overridden - what a sweep calls.
fn solo_lap_tuned(level: oag_ai::Difficulty, track: &str, tuning: Option<oag_ai::Tuning>) -> Solo {
    let Some(image) = image() else {
        return Solo::default();
    };
    let Ok(loaded) = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        track: Some(track.to_string()),
        ..race::Options::default()
    }) else {
        return Solo::default();
    };
    let mut race = race::Race::start(loaded.setup);
    if let Some(tuning) = tuning {
        race.set_ai_tuning(tuning);
    }
    // Everyone but one opponent off the track, so nothing it does is about
    // anybody else.
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;

    let mut best: Option<u64> = None;
    let mut lap = race.sim.world.ships[1].standing.lap;
    let mut started = 0u64;
    let mut recovered_this_lap = false;
    let mut lost_at = Vec::new();
    for tick in 0..18_000u64 {
        let before = race.respawns_of(1);
        let was_at = race.sim.world.ships[1].driver.index;
        race.tick(&PlayerInputs::none());
        if race.respawns_of(1) != before {
            recovered_this_lap = true;
            lost_at.push(was_at);
        }
        let now = race.sim.world.ships[1].standing.lap;
        if now != lap {
            // The first lap is the standing start, and a lap the craft had to
            // be recovered during is not a lap it drove.
            if lap > 1 && !recovered_this_lap {
                let taken = tick - started;
                best = Some(best.map_or(taken, |held: u64| held.min(taken)));
            }
            started = tick;
            lap = now;
            recovered_this_lap = false;
        }
    }
    Solo {
        best,
        respawns: race.respawns_of(1),
        laps: lap,
        lost_at,
        line_len: race.racing_line().len() as u32,
    }
}

/// A solo craft laps, and laps faster at a harder setting.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_solo_craft_laps_faster_at_a_harder_setting() {
    let Some(novice) = solo_lap_ticks(oag_ai::Difficulty::Novice) else {
        return;
    };
    let ace = solo_lap_ticks(oag_ai::Difficulty::Ace).expect("the image is there");
    println!(
        "solo best lap: novice {novice} ticks ({:.1}s), ace {ace} ticks ({:.1}s)",
        novice as f32 / 60.0,
        ace as f32 / 60.0
    );
    assert!(
        ace < novice,
        "an ace should lap quicker alone: {ace} against {novice}"
    );
}

/// Every circuit on the disc, one craft, best lap.
///
/// **The benchmark that stops a tuning being fitted to one track.** Talon's
/// Junction is the default and therefore the one every other measurement here
/// happens to use; a knob that helps there and hurts everywhere else would look
/// like an improvement right up until someone played a different circuit.
fn solo_laps_everywhere(level: oag_ai::Difficulty) -> Vec<(String, Solo)> {
    solo_laps_tuned(level, None)
}

/// The same, with the drivers' tuning overridden.
fn solo_laps_tuned(
    level: oag_ai::Difficulty,
    tuning: Option<oag_ai::Tuning>,
) -> Vec<(String, Solo)> {
    let Some(image) = image() else {
        return Vec::new();
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");

    catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
        .map(|track| {
            let entry = track.entry_name();
            (track.id.clone(), solo_lap_tuned(level, &entry, tuning))
        })
        .collect()
}

/// What a grip figure is worth across the whole disc rather than on one
/// circuit.
///
/// A scratch sweep, kept `#[ignore]`d and printing rather than asserting: the
/// number it produces goes into `Tuning::lateral_accel` by hand, with the table
/// written into `docs/gameplay/ai.md` so the choice has its measurement next to
/// it. It exists because a solo lap on Talon's Junction said 260 was quicker
/// than 180, and Talon's Junction is one of the five circuits a craft gets
/// round cleanly - exactly the subset that cannot see the failure.
/// Twelve circuits times five grip figures times five minutes of simulation is
/// minutes of wall clock in a debug build, so it is off unless asked for:
///
/// ```sh
/// OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 \
///   cargo nextest run --release -p oag-game --run-ignored all sweep_grip --no-capture
/// ```
///
/// `#[ignore]` alone would not do it - `just test-data` runs ignored tests, and
/// a five-minute entry in that suite is how a suite stops being run.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table, choose"]
fn sweep_grip() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let mut report = String::from("\ngrip   clean  round  respawns  mean clean lap\n");
    for grip in [120.0f32, 180.0, 260.0, 340.0, 440.0, 560.0, 700.0] {
        let tuning = oag_ai::Tuning {
            lateral_accel: grip,
            ..oag_ai::Tuning::default()
        };
        let laps = solo_laps_tuned(oag_ai::Difficulty::Ace, Some(tuning));
        if laps.is_empty() {
            return;
        }
        let clean: Vec<u64> = laps.iter().filter_map(|(_, solo)| solo.best).collect();
        let mean = if clean.is_empty() {
            f32::NAN
        } else {
            clean.iter().sum::<u64>() as f32 / clean.len() as f32 / 60.0
        };
        report.push_str(&format!(
            "{grip:<6} {:<6} {:<6} {:<9} {mean:.1}s\n",
            clean.len(),
            laps.iter().filter(|(_, solo)| solo.laps >= 2).count(),
            laps.iter().map(|(_, solo)| solo.respawns).sum::<u32>(),
        ));
    }
    println!("{report}");
}

/// Every craft starts on its own racing line, on **every** circuit.
///
/// **The test that was missing, and the bug it would have caught was severe.**
/// `Driver::drive` locates a craft with a 48-sample *window* around its last
/// index - deliberately, so a circuit that passes over itself cannot make a
/// craft latch onto a stacked section. `Driver::index` therefore has to start
/// somewhere near the truth, and it did not: it started at zero, so a grid
/// sitting at sample 2,500 never found itself and steered at whatever piece of
/// track it believed it was on.
///
/// It went unnoticed because every other test here runs the default circuit,
/// and the default circuit's start line happens to sit near sample zero. On the
/// disc's twelve circuits the field began between 280 and 10,862 units from its
/// own racing line on eight of them, and on six it never got going at all -
/// 300 units of travel in a minute, and on one the craft ground itself to
/// destruction.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_craft_starts_on_its_line_on_every_circuit() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");

    let mut checked = 0;
    for track in catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
    {
        let Ok(loaded) = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            track: Some(track.entry_name()),
            ..race::Options::default()
        }) else {
            continue;
        };
        let race = race::Race::start(loaded.setup);
        checked += 1;

        for slot in 1..race.ship_count() as usize {
            let ship = &race.sim.world.ships[slot];
            let sample = race
                .ai_sample_for(slot, ship.driver.index as usize)
                .expect("the driver stands on a sample");
            let lateral = Vec3::from_array(sample.lateral).normalize_or_zero();
            let line = Vec3::from_array(sample.pos)
                - oag_vex::track::HOVER_LIFT * Vec3::from_array(sample.down)
                + sample.racing_line * lateral;
            let off = (ship.physics.body.position - line).length();
            // A grid is staggered across and along the track, so this is
            // "somewhere on the start line" rather than "on the racing line".
            assert!(
                off < 120.0,
                "{}: slot {slot} starts {off:.0} units from the sample its \
                 driver thinks it is on",
                track.id
            );
        }
    }
    assert!(checked >= 10, "only {checked} circuits were loadable");
}

/// How many of the disc's circuits a lone craft can actually get round.
///
/// **A record of a known-incomplete state, pinned so it cannot quietly get
/// worse**, and it holds two separate ratchets because there are two separate
/// failures behind the one symptom.
///
/// # What the numbers were, and what moved them
///
/// Two of twelve until `Driver::index` was seeded from the spawn (see
/// [`every_craft_starts_on_its_line_on_every_circuit`]), then five. Adding a
/// rescue for a craft that leaves the circuit ([`oag_race::recovery::RESCUE_HALF_WIDTHS`])
/// took *laps completed* to nine of twelve without moving *clean* laps at all,
/// and that gap is the finding: the craft were not failing to drive round, they
/// were driving off and never coming back, because a `Reset` volume cannot
/// catch something receding into open space.
///
/// # The two ratchets, and why a clean lap is the one that matters
///
/// A lap the craft had to be recovered during is not a lap it drove. On an
/// empty circuit at the top difficulty a competent driver should never need
/// recovering, so the respawn count *is* the driving-quality metric here and
/// the rescue must not be allowed to launder a craft that flies into the
/// scenery. Four circuits complete every lap and never manage a clean one.
///
/// and that gap is the finding. Dropping the paths a lap never drives out of
/// the AI line ([`race::Race::ai_sample`]) then took *round* to eleven and
/// *clean* to six, and the nine circuits it did not touch reproduced their lap
/// times, respawn counts and loss indices exactly - which is what said the
/// mapping was right.
///
/// # What is left
///
/// **Nothing, as of 2026-08-12**: all twelve circuits complete every lap and
/// all twelve manage a clean one, eleven of them without being recovered once.
/// 01 keeps two recoveries, one of them on the standing-start lap.
///
/// It took four fixes and none of them were the AI. Dropping the paths a lap
/// never drives out of the racing line ([`race::Race::ai_sample`]) took *round*
/// from nine to eleven. Gating the landing response on `rebound_jump_time` took
/// 01 from eight recoveries to three. Deriving the rear hover probe above
/// `oag_physics::hover::FAST_PROBE_SPEED` instead of casting it took clean laps
/// from seven to nine and 05 from never lapping to lapping without a scratch.
/// And a swept contact test (`oag_physics::hover::sweep`) - the one mechanism in
/// that crate that is ours rather than recovered - took the last three, because
/// a downward-only probe cannot find a surface the hull has already crossed.
///
/// Both bounds are "no worse than today" rather than targets: a number that
/// asserted twelve would be a test failing for a reason already written down,
/// which is noise rather than signal.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round() {
    let laps = solo_laps_everywhere(oag_ai::Difficulty::Ace);
    if laps.is_empty() {
        return;
    }
    let lapped: Vec<&String> = laps
        .iter()
        .filter(|(_, solo)| solo.best.is_some())
        .map(|(id, _)| id)
        .collect();
    let missed: Vec<&String> = laps
        .iter()
        .filter(|(_, solo)| solo.best.is_none())
        .map(|(id, _)| id)
        .collect();
    for (id, solo) in &laps {
        println!(
            "{id:<12} clean lap {:>6}  laps {:<3} respawns {:<3} of {} lost at {:?}",
            solo.best.map_or("none".to_string(), |ticks| format!(
                "{:.1}s",
                ticks as f32 / 60.0
            )),
            solo.laps,
            solo.respawns,
            solo.line_len,
            solo.lost_at
        );
    }
    println!("clean laps: {lapped:?}\nno clean lap: {missed:?}");
    // The full set, so this is now a plain regression bound rather than a
    // record of an incomplete state: any circuit that stops managing a clean
    // lap is a regression, and the message names it.
    assert_eq!(
        lapped.len(),
        laps.len(),
        "only {} of {} circuits saw a clean lap, and all of them used to: \
         {missed:?}",
        lapped.len(),
        laps.len()
    );

    // The second ratchet: got round at all, recovered or not. It is the one the
    // rescue moved, and keeping it separate is what stops a driving regression
    // hiding behind a recovery that still gets the craft home.
    let round: Vec<&String> = laps
        .iter()
        .filter(|(_, solo)| solo.laps >= 2)
        .map(|(id, _)| id)
        .collect();
    assert!(
        round.len() >= 12,
        "only {} of {} circuits were completed at all, and all twelve have been",
        round.len(),
        laps.len()
    );
}

/// Whether there is anything under the line an opponent is told to drive.
///
/// **The measurement that turned "the AI flies off at one corner" into
/// something that is not an AI question at all.** Craft were leaving six
/// circuits at a repeatable place every lap, falling five to eight hundred
/// units while only a few units laterally off their own line - straight down,
/// not wide. So the question stopped being how the driver steers and became
/// whether the surface it is steering along is there.
///
/// Casting down the surface normal at **every** sample of the racing line, from
/// one probe reach above to one below, which is exactly what a craft's own
/// antigravity probes reach:
///
/// | circuit | line samples with nothing under them | where |
/// | --- | --- | --- |
/// | 16, 03, 04 | **0** | - |
/// | 06 | 5 | 1196-1200 |
/// | 01 | 12 | 31-42 |
/// | 07 | 13 | 2364-2376 |
/// | 13 | 31 | 19-49 |
/// | 09 | 55 | 967-1021 |
/// | 02 | 70 | 1167-1236 |
/// | 10 | 72 | 135-206 |
/// | 14 | 82 | 684-765 |
/// | 05 | 134 | 161-211, 811-893 |
///
/// Each is a single contiguous run, and **the three circuits with none are
/// three of the circuits that never lose a craft**. A second, deeper cast
/// separates two different things wearing the same result: on 13 and most of 10
/// there is nothing within sixty reaches, which is an authored *jump* - 13 flies
/// it, lands, and laps cleanly. On 09, 02, 05, 14, 01 and 07 there is a surface,
/// one to sixty ride-heights **below** the line. That is not a gap in the track;
/// it is a racing line running above the track.
///
/// Which of those two this is per circuit is not settled, and neither is whether
/// the deeper cases are the disc's data or this repository's collision loading.
/// See `docs/gameplay/ai.md`.
///
/// The assertion is only that the three clean circuits stay clean, because that
/// is the part that is understood. The table prints so a reader gets the whole
/// picture rather than the one bound.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_racing_line_has_track_under_it_where_it_is_known_to() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");

    let mut measured: Vec<(String, usize)> = Vec::new();
    for track in catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
    {
        let Ok(loaded) = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            track: Some(track.entry_name()),
            ..race::Options::default()
        }) else {
            continue;
        };
        let reach = loaded.setup.handling.antigrav.ride_height;
        let collision = loaded.setup.collision.clone();
        let race = race::Race::start(loaded.setup);
        let line = race.racing_line();

        // Two casts per unsupported sample: one probe reach, which is what a
        // craft holds onto, then sixty, which says whether there is a surface
        // down there at all.
        let (mut gaps, mut below) = (Vec::new(), 0u32);
        for index in 0..line.len() {
            let Some(sample) = race.ai_sample(index) else {
                continue;
            };
            let up = (-Vec3::from_array(sample.down)).normalize_or_zero();
            let from = line.point(index) + up * reach;
            let cast = |length: f32| {
                collision
                    .raycast(oag_physics::Ray::new(from, -up, length), None, false)
                    .is_some()
            };
            if cast(reach * 2.0) {
                continue;
            }
            gaps.push(index);
            if cast(reach * 60.0) {
                below += 1;
            }
        }
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for index in &gaps {
            match runs.last_mut() {
                Some(last) if *index == last.1 + 1 => last.1 = *index,
                _ => runs.push((*index, *index)),
            }
        }
        println!(
            "{:<12} {:>4} of {} line samples unsupported ({below} have a surface further \
             below) in runs {:?}",
            track.id,
            gaps.len(),
            line.len(),
            runs.iter()
                .filter(|(from, to)| to - from >= 4)
                .map(|(from, to)| format!("{from}..{to}"))
                .collect::<Vec<_>>(),
        );
        measured.push((track.id.clone(), gaps.len()));
    }
    if measured.is_empty() {
        return;
    }

    for id in ["16_Track", "03_Track", "04_Track"] {
        let Some((_, gaps)) = measured.iter().find(|(name, _)| name == id) else {
            continue;
        };
        assert_eq!(
            *gaps, 0,
            "{id} used to have track under every sample of its racing line and now has \
             {gaps} without"
        );
    }
}

/// The Rocket's model is longest along the axis `Race::rocket_model_matrices`
/// aims at its velocity.
///
/// **This is the assertion that catches a rocket drawn flying sideways.**
/// `rocket_model_matrices` builds `Mat4::from_cols(side, up, forward, position)`,
/// which maps the model's own **+Z** onto the direction of travel. Nothing in the
/// unit tests can check that +Z is where the dart's nose actually points: they
/// assert the matrix contains what was written into it, which is true whatever
/// the mesh looks like. Only the real file can settle it.
///
/// It also bears on something left unresolved in
/// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`. `Rocket_Update`
/// builds the same forward/normal/cross basis and then rotates it by a further
/// `-pi/2` about an axis this project has not identified - and a quarter turn is
/// exactly the shape of a model-space axis convention. If this test ever fails
/// because the long axis is X, that deferred rotation is the reason, and the fix
/// is a fixed pre-rotation here rather than a new reading of the original.
#[test]
#[ignore = "needs a real disc image under data/images/"]
fn the_rocket_model_is_longest_along_the_axis_it_is_flown_down() {
    let Some(loaded) = load() else { return };
    let model = loaded
        .rocket_model
        .expect("the disc carries Data\\Weapons\\Rocket.vex");

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for vertex in &model.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex.position[axis]);
            max[axis] = max[axis].max(vertex.position[axis]);
        }
    }
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    println!(
        "Rocket.vex spans x {:.3}, y {:.3}, z {:.3}",
        span[0], span[1], span[2]
    );

    let longest = (0..3).max_by(|a, b| span[*a].total_cmp(&span[*b])).unwrap();
    assert_eq!(
        longest,
        2,
        "the dart is longest along {} but `rocket_model_matrices` aims +Z down the \
         velocity, so the model would be drawn broadside; spans are {span:?}",
        ["x", "y", "z"][longest]
    );
    // A dart, not a disc: the long axis should dominate, or "longest" is noise.
    let second = (0..3)
        .filter(|axis| *axis != longest)
        .map(|axis| span[axis])
        .fold(0.0f32, f32::max);
    assert!(
        span[longest] > second * 1.5,
        "expected one clearly dominant axis, got {span:?}"
    );
}

#[test]
#[ignore = "needs a real disc image under data/images/"]
fn pulses_default_track_reports_its_own_trackstartup_xml() {
    let Some(loaded) = load() else { return };
    let report = loaded.report.join("\n");
    let want = report.contains("8 billboard slot(s), 7 naming a model and 1 a colour")
        && report.contains("sound bank TALONS_JUNCTION_ENV.bnk");
    assert!(want, "no matching trackstartup.xml line: {report}");
}
