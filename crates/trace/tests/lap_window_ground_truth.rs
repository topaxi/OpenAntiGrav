//! How long a single-seeded replay of the reference lap tracks the original,
//! inside the capture's own wall-free window.
//!
//! `talons-junction-clean-lap.csv` first touches a wall at tick 256, so ticks
//! 0-255 are the stretch where nothing but the force law and the pads decides
//! where the craft goes. Seeded once at tick 0 and driven by the same script
//! the capture was flown with (`--script-lead 2`), our craft used to be 44
//! units from the original by tick 240. Two causes, both fixed on 2026-09-30
//! and both pinned here, because either one alone puts the error back above
//! the bound:
//!
//! - **The replay never armed the speed pads.** The original crosses pad 0 at
//!   tick 161 and gains about 42 units/s over the next fifteen ticks; without
//!   it, 43.9 units at tick 240. See `pad_crossing_ground_truth.rs`.
//! - **The airbrake's forward `drag` term ran 100x weak.** `steerX` is read
//!   raw off the input snapshot, which holds the axis on `+/-100`, and
//!   `oag_physics::airbrake::evaluate` multiplied by the normalised `-1..=1`
//!   axis instead. With the pads fixed and not this, 14.9 units at tick 240.
//!
//! With both: 4.6 units at tick 240, and under 5 units until tick 258.
//!
//! This is a single-seeded number on a variable-`dt` original, which
//! `docs/tools/oag-trace.md` warns grows chaotically over a long run - two runs
//! of the original itself are 100 units apart by tick 495. Inside the first
//! 256 ticks it is still a statement about the law, which is why the bound is
//! set here and nowhere later.

use oag_core::math::Vec3;
use oag_gameplay::{collision_world, handling_for};
use oag_physics::Environment;
use oag_pulse as pulse;
use oag_race::Course;
use oag_tables::handling;
use oag_trace::replay::{Basis, DeltaSource, Inputs, Options, replay};
use oag_trace::trace::AngularReading;
use oag_trace::{Script, Trace};
use oag_vex::{collision, pads, track, vex};

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const CAPTURE: &str = "data/traces/talons-junction-clean-lap.csv";
const SCRIPT: &str = "verification/scenarios/talons-junction-clean-lap.inputs";
const TEAM: &str = "Assegai";
const CLASS: &str = "VENOM";

/// The capture's first wall contact, from `oag-trace show`'s cleanliness line.
const FIRST_WALL: usize = 256;

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn read(relative: &str) -> String {
    let path = workspace(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is tracked in git: {e}", path.display()))
}

/// Position error per tick over the wall-free window, or `None` without a disc.
fn position_error() -> Option<Vec<f32>> {
    let image = workspace(IMAGE);
    if !image.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "{} is missing and OAG_REQUIRE_GAME_DATA is set",
            image.display()
        );
        return None;
    }
    let recorded = Trace::parse(&read(CAPTURE)).expect("parsing the capture");
    let script = Script::parse(&read(SCRIPT)).expect("parsing the script");

    let mut archives = pulse::open(image.to_str().expect("utf-8 path")).expect("the image");
    let blob = archives.read_name(TRACK).expect("the track");
    let collision = collision_world(&collision::from_vex(&blob).expect("collision"));
    let nodes = vex::nodes(&blob).expect("the .vex decodes");
    let speedup = pads::volumes(&blob, &nodes, vex::CLASS_SPEEDUP_PAD);

    // The magstrip locator's samples, resampled exactly as `oag-trace run`
    // resamples them.
    let ai_node = track::find_node(&blob, &nodes).expect("a WO Track node");
    let ai = track::parse(blob.get(ai_node.payload()).expect("the payload")).expect("the spline");
    let steps = Course::STEPS_PER_SEGMENT;
    let samples: Vec<track::Sample> = ai
        .paths
        .iter()
        .flat_map(|path| {
            (0..path.points.len()).flat_map(move |segment| {
                (0..steps).filter_map(move |step| path.sample(segment, step as f32 / steps as f32))
            })
        })
        .collect();

    let stats = handling::from_blob(
        &archives
            .read_name(&handling::entry_name(TEAM))
            .expect("the team's stats"),
    )
    .expect("parsing the stats");
    let global = handling::global_from_blob(
        &archives
            .read_name(handling::GLOBAL_ENTRY)
            .expect("the global stats"),
    )
    .expect("parsing the global stats")
    .expect("a <Global>");
    let (speedup_pads, _, _) = global.class_named(CLASS).expect("VENOM's <GlobalClass>");
    let handling =
        handling_for(&stats, CLASS, speedup_pads, global.special).expect("the team authors VENOM");

    let options = Options {
        inputs: Inputs::Scripted(script.with_capture_lead(2).states),
        dt: DeltaSource::Trace,
        basis: Basis::LeftUpForward,
        angular: AngularReading::NegatedLocal,
        scheme: Default::default(),
        reseed: None,
        pads: speedup,
    };
    let ours = replay(
        &recorded,
        &handling,
        &Environment::default(),
        &collision,
        &options,
        Some(&samples),
    );
    Some(
        recorded.frames[..FIRST_WALL]
            .iter()
            .zip(&ours.frames)
            .map(|(original, ours): (_, &oag_trace::Frame)| {
                Vec3::distance(original.position, ours.position)
            })
            .collect(),
    )
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_single_seeded_replay_tracks_the_reference_lap_through_its_wall_free_window() {
    let Some(error) = position_error() else {
        return;
    };
    let first_over_five = error.iter().position(|&e| e > 5.0);
    println!(
        "position error: tick 120 {:.2}, tick 180 {:.2}, tick 240 {:.2}, max {:.2}, first over 5 units {:?}",
        error[120],
        error[180],
        error[240],
        error.iter().copied().fold(0.0f32, f32::max),
        first_over_five
    );
    // 4.6 measured. Pads missing: 43.9. Airbrake term 100x weak: 14.9.
    assert!(
        error[240] < 8.0,
        "tick 240 is {:.2} units from the original",
        error[240]
    );
    // Measured: never over 5 before the wall at 256. Either regression crosses
    // 5 by tick 136.
    assert!(
        first_over_five.is_none(),
        "over 5 units at tick {first_over_five:?}, inside the wall-free window"
    );
}
