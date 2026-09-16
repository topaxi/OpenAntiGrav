//! What `talons-junction-time-trial-lap.csv` actually contains, and where the
//! start line is.
//!
//! Two questions, one capture, and the answers turned out to point in opposite
//! directions.
//!
//! # The start line is where the original puts it, and the craft is behind it
//!
//! `Course::START_LINE_ADVANCE` is the original's own rule (154 units along the
//! slot's tangent, re-projected; `RaceManager_Construct`,
//! `docs/ghidra/functions/psp-pulse-usa/race-progress.md`). A captured time
//! trial begins **17 units short of that line**: the original spawns its craft
//! behind the line and lets the first crossing start the race, which is the
//! same shape as this crate's lap gate. Before the rule was recovered this file
//! asserted the capture began *on* the line, because the line was a constant
//! fitted to this very capture.
//!
//! # The capture is not a lap
//!
//! `HANDOVER.md` doubted it: *"either the capture is not a lap, the path-length
//! measure is not what it is read as, or the original spent most of the capture
//! barely moving. Check it before any exit-criterion argument leans on it."*
//!
//! Measured here, off the capture's own recorded positions and speeds: the ship
//! drives about **430 units of a 5,094-unit circuit** - 8 % - in the first ten
//! seconds, then sits between 0.8 and 3.4 units/s for roughly a thousand ticks,
//! then **drives back the way it came**, past the start line, and carries on away
//! from it for the rest of the recording.
//!
//! So the third reading is the right one, with a reversal on the end. This file
//! exists to stop the capture being used as lap ground truth again: it cannot
//! confirm a lap counter, because it contains no lap.
//!
//! **This does not implicate the locator.** The reversal is in the recorded
//! `position` column, not in anything computed here - the assertions below read
//! the capture's own coordinates and its own `speed`.

use oag_race::{Course, Mode, RaceState, SpeedClass};
use oag_trace::Trace;
use oag_vex::{track, vex};

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
/// Talon's Junction. The environment every capture under `data/traces/` was taken
/// on - `wall_contact_ground_truth.rs` pins the pairing.
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const CAPTURE: &str = "data/traces/talons-junction-time-trial-lap.csv";

/// The fixed timestep, from ADR-0007.
const DT: f32 = 1.0 / 60.0;

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// A capture under `data/traces/`, tracked in git per
/// [ADR-0046](../../../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md).
///
/// Its absence is never a legitimate reason to skip: a tracked file missing
/// from the working tree means the checkout is broken, not that this test
/// does not apply. Panics unconditionally, independent of
/// `OAG_REQUIRE_GAME_DATA` - that variable escalates *optional* inputs (a
/// disc image under `data/images/`), and a tracked trace is not optional.
/// This used to return `None` instead.
fn require_capture(relative: &str) -> std::path::PathBuf {
    let path = workspace(relative);
    assert!(
        path.exists(),
        "{} is missing, but it is tracked in git per ADR-0046 - the checkout \
         is broken, not merely missing optional data.",
        path.display()
    );
    path
}

/// The course and the capture, or `None` when this checkout has no disc image.
///
/// The disc image is an optional input - `OAG_REQUIRE_GAME_DATA=1` escalates
/// its absence to a hard error, otherwise it is a quiet skip. The capture
/// itself is not optional: `CAPTURE` is tracked in git per
/// [ADR-0046](../../../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md),
/// so its absence means the checkout is broken and panics unconditionally -
/// see `require_capture`.
fn load() -> Option<(Course, Trace)> {
    let image = workspace(IMAGE);
    if !image.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            image.display()
        );
        println!("skipping: {} is missing", image.display());
        return None;
    }
    let capture = require_capture(CAPTURE);

    let mut archives = oag_pulse::open(image.to_str().expect("utf-8 path")).expect("the image");
    let blob = archives.read_name(TRACK).expect("the track");

    let nodes = vex::nodes(&blob).expect("the .vex decodes");
    let ai_node = track::find_node(&blob, &nodes).expect("a WO Track node");
    let ai = track::parse(blob.get(ai_node.payload()).expect("the payload")).expect("the spline");

    let slot = nodes
        .iter()
        .find(|node| node.class_id == vex::CLASS_START_POSITION)
        .and_then(|node| track::start_position(blob.get(node.payload())?, vex::byte_order(&blob)))
        .expect("this track has an authored slot");

    let course = Course::from_track(&ai, Some(oag_core::math::Vec3::from(slot.position)))
        .expect("the primary chain closes");
    let text = std::fs::read_to_string(&capture).expect("the capture");
    Some((course, Trace::parse(&text).expect("the capture parses")))
}

/// The captured run begins a little behind the line the lap counter uses.
///
/// `START_LINE_ADVANCE` places the line off the slot and the spline; the
/// capture's first frame is where the original puts a craft to start a time
/// trial. The craft resolves to a ring point **before** the line, by a distance
/// a standing start covers in a second or two - measured here at 17.3 units,
/// eleven ring points - and never on or past it. That is the original's own
/// design (its `started` flag is set by the first crossing, and no lap is
/// recorded for it), and it is what `RaceState`'s spawn-to-line arm exists for.
///
/// The bounds are wide on purpose: the claim is "behind, and not far", not the
/// exact eleven points, which would pin the ring's sampling resolution rather
/// than anything about the game.
#[test]
#[ignore = "needs a disc image in data/images/ and a capture in data/traces/"]
fn the_captured_run_begins_just_behind_our_start_line() {
    let Some((course, trace)) = load() else {
        return;
    };
    let first = course
        .locate(trace.frames[0].position, None)
        .expect("the captured start is on the ring");
    let behind = course.length() - first.progress;
    println!(
        "capture starts at ring point {} (progress {:.1} of {:.1}); start line is point {}; \
         the craft spawns {behind:.1} units behind the line",
        first.index,
        first.progress,
        course.length(),
        course.start_index()
    );
    assert!(
        (5.0..=30.0).contains(&behind),
        "the captured run should start a few units behind the line, not {behind:.1} units \
         (progress {:.1} on a {:.1}-unit ring)",
        first.progress,
        course.length()
    );
}

/// The capture stalls and reverses, so it holds no lap.
///
/// Pinned rather than merely written down, because the temptation to reach for
/// "the lap capture" as lap ground truth will come back. If someone recaptures a
/// real lap under the same filename this fails, which is the right outcome: the
/// file's meaning changed and everything citing it needs rereading.
///
/// Asserted on the capture's **own** columns - `position` and `speed` - so no
/// part of this project's simulation can move the result.
#[test]
#[ignore = "needs a disc image in data/images/ and a capture in data/traces/"]
fn the_capture_stalls_and_reverses_rather_than_completing_a_lap() {
    let Some((course, trace)) = load() else {
        return;
    };

    // Speed lap: no lap target, so a capture that did contain laps reports all of
    // them instead of stopping at three. The speed class is immaterial here for
    // exactly that reason - `Mode::SpeedLap.laps_target` is `None` whatever it
    // is - so Venom is the engine's own default rather than a choice this test
    // depends on.
    let mut state = RaceState::new(Mode::SpeedLap, SpeedClass::Venom);
    let mut laps = Vec::new();
    let mut furthest_forward = 0.0f32;
    let mut reversed_past_the_line = false;
    // Forward travel is measured from the capture's own first frame, which sits
    // a few units *behind* the start line (see the test above): measured as raw
    // progress it would read as "all the way round" from tick 0, which is
    // exactly the trap that made an earlier version of this test report 100%.
    let origin = course
        .locate(trace.frames[0].position, None)
        .expect("the captured start is on the ring")
        .progress;

    for (tick, frame) in trace.frames.iter().enumerate() {
        let before = state.progress;
        let outcome = state.update(&course, frame.position, tick as u64, DT, false);
        if outcome.lap_completed {
            laps.push(tick);
        }
        if let (Some(before), Some(after)) = (before, state.progress) {
            // A jump of more than half the circuit is the line being crossed
            // backwards, not travel.
            if after - before > course.length() * 0.5 {
                reversed_past_the_line = true;
            }
            // How far the run got *going forwards*. Stops accumulating once the
            // run has reversed over the line, because after that a high progress
            // means "just short of the line from the wrong side" rather than
            // "nearly all the way round".
            if !reversed_past_the_line {
                let forward = (after - origin).rem_euclid(course.length());
                furthest_forward = furthest_forward.max(forward);
            }
        }
    }

    let slowest_stretch = trace.frames[700..1700]
        .iter()
        .map(|frame| frame.speed)
        .fold(f32::INFINITY, f32::min);
    let fraction = furthest_forward / course.length();
    println!(
        "{} ticks: reached {furthest_forward:.0} of {:.0} units ({:.1}%), \
         slowest speed over ticks 700-1700 is {slowest_stretch:.1}, laps {laps:?}",
        trace.frames.len(),
        course.length(),
        fraction * 100.0
    );

    assert!(
        laps.is_empty(),
        "the capture was read as containing {} lap(s) at {laps:?}; if it has been \
         recaptured as a real lap, this test and everything citing the file need \
         rereading",
        laps.len()
    );
    assert!(
        fraction < 0.2,
        "the capture now reaches {:.1}% of the circuit; it used to reach 8%, so it \
         is no longer the run this test describes",
        fraction * 100.0
    );
    assert!(
        reversed_past_the_line,
        "the capture no longer drives back over the start line, so its shape has \
         changed"
    );
}
