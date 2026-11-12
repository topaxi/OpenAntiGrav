//! Does our craft clear de Konstruct Black's first jump exactly when the original's does?
//!
//! `05_Track` forward's first jump takes off over the convex top of a ramp whose left two thirds
//! are `Mag Floor`, onto an upper road whose lip a short craft hits at about y `65`. Each capture
//! here is the original under player input (PPSSPP, Pulse USA, Time Trial, VENOM, Assegai,
//! throttle held, no steer, never the Autopilot), its rigid body written to one pose
//! (`psp-trace.py --place ... --place-repeat 2`) and recorded for 180 frames. Ours is seeded
//! from the first row and driven by the same held input over the same frame times.
//!
//! On the authored line the original falls short from every placement up to 145 units/s and
//! clears from 146; six units right, off the strip, it clears from 130. The body damping decides
//! the band in between: without it ours clears from 143 (`docs/physics/body-damping.md`, "De
//! Konstruct Black's first jump, under player input").

use oag_gameplay::{collision_world, handling_for};
use oag_physics::Environment;
use oag_pulse as pulse;
use oag_race::Course;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, DeltaSource, Held, Inputs, Options, replay};
use oag_trace::trace::AngularReading;
use oag_vex::{collision, pads, track, vex};

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\05_Track\track.vex";
const TEAM: &str = "Assegai";
const CLASS: &str = "VENOM";

/// Each capture and whether the original cleared the jump in it.
const CAPTURES: [(&str, bool); 7] = [
    ("line-130", false),
    ("line-143", false),
    ("line-144", false),
    ("line-145", false),
    ("line-146", true),
    ("right6-130", true),
    // The pose `oag_ai::original::Autopilot` flew on a lap it fell from, 30 ticks before the
    // takeoff: 139.4 units/s, five units further onto the strip than the line.
    ("port-2652", false),
];

/// Past this x the craft is over the upper road, beyond the lip a short craft stops at.
const BEYOND_LIP_X: f32 = -690.0;
/// The upper road there is above this height; the pit under it is far below.
const UPPER_ROAD_Y: f32 = 55.0;

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn cleared(trace: &Trace) -> bool {
    trace
        .frames
        .iter()
        .any(|f| f.position.x < BEYOND_LIP_X && f.position.y > UPPER_ROAD_Y)
}

/// The highest point past the ramp's top, which for a short craft is where it meets the lip.
fn apex(trace: &Trace) -> f32 {
    trace
        .frames
        .iter()
        .filter(|f| f.position.x < -540.0)
        .map(|f| f.position.y)
        .fold(f32::MIN, f32::max)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_first_jump_is_cleared_from_the_same_placements_as_the_original() {
    let image = workspace(IMAGE);
    if !image.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "{} is missing and OAG_REQUIRE_GAME_DATA is set",
            image.display()
        );
        return;
    }
    let mut archives = pulse::open(image.to_str().expect("utf-8 path")).expect("the image");
    let blob = archives.read_name(TRACK).expect("the track");
    let world = collision_world(&collision::from_vex(&blob).expect("collision"));
    let nodes = vex::nodes(&blob).expect("the .vex decodes");
    // The magstrip locator's samples, resampled as `oag-trace run` resamples them.
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
    let handling = handling_for(
        &stats,
        CLASS,
        handling::SpeedupPads::default(),
        handling::Special::default(),
        oag_gameplay::handling::DEFAULT_SKILL_LEVEL,
    )
    .expect("the team authors VENOM");
    let options = Options {
        inputs: Inputs::Held(Held {
            buttons: oag_gameplay::input::button_from_name("cross")
                .expect("cross is a button")
                .bit(),
            ..Held::default()
        }),
        dt: DeltaSource::Trace,
        basis: Basis::LeftUpForward,
        angular: AngularReading::NegatedLocal,
        scheme: Default::default(),
        reseed: None,
        pads: pads::volumes(&blob, &nodes, vex::CLASS_SPEEDUP_PAD),
        cage: oag_trace::replay::cage_of(&blob),
    };

    let mut wrong = Vec::new();
    for (name, original_cleared) in CAPTURES {
        let path = workspace(&format!("data/traces/dekonstruct-takeoff-{name}.csv"));
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} is tracked in git: {e}", path.display()));
        let recorded = Trace::parse(&text).expect("parsing the capture");
        assert_eq!(
            cleared(&recorded),
            original_cleared,
            "{name}: the capture no longer reads as this table says"
        );
        let ours = replay(
            &recorded,
            &handling,
            &Environment::default(),
            &world,
            &options,
            Some(&samples),
        );
        println!(
            "{name}: original {} apex {:.1}, ours {} apex {:.1}",
            if original_cleared { "clears" } else { "short" },
            apex(&recorded),
            if cleared(&ours) { "clears" } else { "short" },
            apex(&ours),
        );
        if cleared(&ours) != original_cleared {
            wrong.push(name);
        }
    }
    assert!(
        wrong.is_empty(),
        "ours and the original part ways at the first jump from {wrong:?}"
    );
}
