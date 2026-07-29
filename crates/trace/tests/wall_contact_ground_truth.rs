//! Does our hull start scraping the wall where the original's does?
//!
//! The standing start crosses Talon's Junction diagonally under full thrust and
//! meets the outer wall, and the capture dates that moment precisely: the
//! original's `speed` column climbs on every one of ticks 0-186 and then falls by
//! 4.38 units/s on 187. Replaying the same scenario through our own physics, ours
//! takes its hit on tick **179** - eight ticks and about sixteen units of track
//! early - having tracked the original's position to 1.06 units and its heading
//! to 0.29 degrees the whole way there.
//!
//! That leaves two candidate explanations and no way to tell them apart from a
//! replay, because a replay's pose is our own: either our craft really is sitting
//! far enough toward the wall to touch it, or our hull answers "am I in contact"
//! differently from `Collision_BoxAgainstMesh` (`0x08815cd4`) at the same pose.
//!
//! So this test takes our simulation out of the question entirely. It walks the
//! **recorded** poses - the original's own position and basis, tick by tick - and
//! asks [`oag_physics::wall::resolve`] whether our hull finds a wall there. No
//! integration, no force law, no accumulated error: just our contact geometry
//! against the original's trajectory.
//!
//! **It splits the eight ticks in two.** On the original's own line our hull
//! first reports a respondable contact at tick **181**, six ticks before the
//! original responds at 187. So six of the eight are our contact geometry
//! answering earlier than `Collision_BoxAgainstMesh` does, and the remaining two
//! are our craft sitting fractionally further toward the wall by the time it gets
//! there. Neither half was decidable from the replay alone.
//!
//! **It is a hull probe finding it, not the swept query.** Both paths run inside
//! [`oag_physics::wall::resolve`], and the swept one is the only thing the
//! `previous_position` argument here could confound. Probed directly: the corner
//! probe reports the wall at 181 at a depth of `0.182` with no swept contact
//! anywhere near it, which is the same tick `resolve` reports. So the reading is
//! about the sample points and their extents.
//!
//! **A geometry change that moves this has a cliff under it.** By the time the
//! original responds, that corner is `1.82` units past the wall plane, against
//! [`oag_physics::wall::MAX_CONTACT_DEPTH`] of `2.0` - and that gate *drops* a
//! contact rather than clamping it, reproducing the original. So a fix that
//! delays our response by letting the hull penetrate further has about two tenths
//! of a unit of headroom before contacts start being discarded, and the symptom
//! then is tunnelling rather than a late response.
//!
//! **What a failure means.** This is not a tolerance on a fitted number. It pins
//! a measured gap that is currently an open question, so a change here is a
//! finding either way: shrinking is progress on the contact geometry and wants
//! the new figure written down, and growing is a regression in it.
//!
//! `#[ignore]`d and never run in CI: it needs `data/traces/` and a disc image
//! under `data/images/`, both gitignored. Run it with `just test-data`.

use oag_assets::pulse;
use oag_formats::{collision, handling};
use oag_gameplay::{collision_world, handling_for};
use oag_physics::{Environment, Handling, wall};
use oag_trace::Trace;
use oag_trace::replay::{Basis, initial_state};
use oag_trace::trace::AngularReading;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const CAPTURE: &str = "data/traces/talons-junction-standing-start.csv";
const LAP_CAPTURE: &str = "data/traces/talons-junction-time-trial-lap.csv";

/// Same team and class as every other capture-backed test here; see
/// `yaw_authority_ground_truth.rs` for why this is written down rather than
/// inferred from the filename.
const TEAM: &str = "Assegai";
const CLASS: oag_physics::SpeedClass = oag_physics::SpeedClass::Venom;

/// Paths in `data/` are workspace-relative, but a test's working directory is its
/// own package root.
fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// The tick the **original** first loses speed, and so first meets the wall.
///
/// Read off the capture rather than asserted: `speed` rises on every one of ticks
/// 0-186 and falls by 4.38 on 187.
/// [`the_capture_still_dates_its_own_wall_the_way_this_test_assumes`] re-derives
/// it from the file, so this constant cannot quietly go stale against a
/// recapture.
const RECORDED_WALL_TICK: usize = 187;

/// How many ticks early our hull may find the wall on the original's own line.
///
/// **Measured, not chosen**: 6 on the day this test was written, so the bound is
/// the measurement plus one. It is a bound on a known gap rather than a target -
/// see the module docs.
const ALLOWED_EARLY_TICKS: usize = 7;

fn load() -> (Handling, oag_physics::CollisionWorld, Trace) {
    let mut archives = pulse::Archives::open(
        workspace(IMAGE)
            .to_str()
            .expect("the image path is not valid UTF-8"),
    )
    .expect("opening the disc image");

    let track_blob = archives.read_name(TRACK).expect("reading the track");
    let nodes = collision::from_vex(&track_blob).expect("decoding the collision nodes");
    let collision = collision_world(&nodes);

    let stats_blob = archives
        .read_name(&handling::entry_name(TEAM))
        .expect("reading the handling stats");
    let stats = handling::from_blob(&stats_blob).expect("parsing the handling stats");
    let handling = handling_for(&stats, CLASS);

    let text = std::fs::read_to_string(workspace(CAPTURE)).expect("reading the capture");
    let trace = Trace::parse(&text).expect("parsing the capture");

    (handling, collision, trace)
}

/// The first recorded tick at which our hull reports a respondable contact.
///
/// `previous_position` is the previous *recorded* position, which is the honest
/// value here: the original really was there one tick earlier, so the swept query
/// sees the segment the original actually travelled.
fn first_contact_on_the_recorded_line(
    handling: &Handling,
    collision: &oag_physics::CollisionWorld,
    trace: &Trace,
) -> Option<usize> {
    let environment = Environment::default();
    trace
        .frames
        .windows(2)
        .position(|pair| {
            let mut state = initial_state(
                &pair[1],
                handling,
                Basis::LeftUpForward,
                AngularReading::NegatedLocal,
            );
            let response = wall::resolve(
                &mut state,
                handling,
                &environment,
                collision,
                pair[0].position,
            );
            assert!(
                !response.hull_degenerate,
                "the hull has no usable extent, so this test measures nothing"
            );
            response.contacts > 0
        })
        // `windows(2).position` counts from the *second* frame, so index 0 is tick 1.
        .map(|index| index + 1)
}

/// Guards [`RECORDED_WALL_TICK`] against a recapture that moves it.
#[test]
#[ignore = "needs data/traces/ and a disc image; run with `just test-data`"]
fn the_capture_still_dates_its_own_wall_the_way_this_test_assumes() {
    let (_, _, trace) = load();
    // A real deceleration, not the sub-tick jitter a craft at rest shows: the
    // capture's `speed` reads 0.021 at tick 0 and 0.015 at tick 1, which is a
    // stationary ship and not a wall. The impact itself is 4.38 units/s in one
    // tick, so any threshold between those two dates the same event.
    let first_loss = trace
        .frames
        .windows(2)
        .position(|pair| pair[1].speed < pair[0].speed - 1.0)
        .map(|index| index + 1)
        .expect("the capture never loses speed, so it never meets the wall");

    assert_eq!(
        first_loss, RECORDED_WALL_TICK,
        "the capture's own wall moved to tick {first_loss}; re-derive the gap \
         below rather than editing the bound to fit"
    );
}

/// The measurement this file exists for.
#[test]
#[ignore = "needs data/traces/ and a disc image; run with `just test-data`"]
fn our_hull_finds_the_wall_within_a_few_ticks_of_where_the_original_does() {
    let (handling, collision, trace) = load();
    let ours = first_contact_on_the_recorded_line(&handling, &collision, &trace)
        .expect("our hull never finds the wall on a line that ends up scraping one");
    eprintln!(
        "our hull first finds the wall on the recorded line at tick {ours}; the \
         original's own response begins at {RECORDED_WALL_TICK}"
    );

    assert!(
        ours <= RECORDED_WALL_TICK,
        "our hull finds the wall at tick {ours}, *after* the original's own \
         response begins at {RECORDED_WALL_TICK} - which is the opposite of the \
         known gap and wants investigating rather than accommodating"
    );
    let early = RECORDED_WALL_TICK - ours;
    assert!(
        early <= ALLOWED_EARLY_TICKS,
        "our hull finds the wall at tick {ours}, {early} ticks before the \
         original's own response at {RECORDED_WALL_TICK}; the measured gap when \
         this was written was 6"
    );
}

/// The lap capture, which is where the gap stops being one wall's property.
///
/// It cannot date its wall by `speed` the way the standing start does - the craft
/// is cornering under power and loses speed for reasons that are not walls - so
/// the cleanliness test dates it instead: `speed / |velocity|` reads exactly
/// `1.0000` in free flight, and the first tick it does not is the first tick the
/// craft is touching something. That puts the original's first wall at tick 171
/// and ours at 168.
///
/// **Both captures fire the same probe**, the lower *front* corner, on opposite
/// sides of the craft and on different walls - which is what makes this a
/// property of the hull rather than of one triangle. What the pair does **not**
/// do is separate the width term from the length term. The forward-to-wall angle
/// is 4.05 degrees on the standing start against 7.33 here, so the two do sample
/// the trade-off; but turning a tick count into a distance needs the wall's own
/// divergence along the approach, and the spline's half-widths are the AI track's
/// rather than the collision mesh's. Fitting the two extents to two tick counts
/// through that proxy gives a length correction larger than the length. Read
/// `Collider_BoxSamplePoints` for both axes.
#[test]
#[ignore = "needs data/traces/ and a disc image; run with `just test-data`"]
fn the_same_gap_shows_on_the_lap_capture_and_the_same_probe_finds_it() {
    let (handling, collision, _) = load();
    let text = std::fs::read_to_string(workspace(LAP_CAPTURE)).expect("reading the lap capture");
    let trace = Trace::parse(&text).expect("parsing the lap capture");

    // The original's first contact, by the cleanliness test. Ticks 0-5 are the
    // craft sitting on the grid, where `|velocity|` is small enough for the ratio
    // to be noise rather than a reading.
    let theirs = trace
        .frames
        .iter()
        .enumerate()
        .skip(6)
        .find(|(_, frame)| {
            let magnitude = frame.velocity.length();
            magnitude > 1e-6 && (frame.speed / magnitude - 1.0).abs() >= 1e-4
        })
        .map(|(tick, _)| tick)
        .expect("the lap capture never touches anything");

    let ours = first_contact_on_the_recorded_line(&handling, &collision, &trace)
        .expect("our hull never finds a wall on a line that spends a third of itself in one");
    eprintln!("lap: ours {ours}, the original's {theirs}");

    assert!(
        ours <= theirs,
        "our hull finds the lap's wall at tick {ours}, after the original's \
         {theirs} - the opposite of the known gap"
    );
    assert!(
        theirs - ours <= ALLOWED_EARLY_TICKS,
        "our hull finds the lap's wall {} ticks before the original's {theirs}; \
         the measured gap when this was written was 3",
        theirs - ours
    );
}
