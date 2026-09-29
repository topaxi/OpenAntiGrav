//! How high does the original fly over a crest, against how high do we?
//!
//! The maintainer's report was that craft fall off the track more easily than
//! the original's, with "flying too high?" as one guess. `ride_height_ground_
//! truth.rs` answers the resting half. This is the airborne half: every airborne
//! window (a run of ticks with `grounded == 0`) in each Talon's Junction capture
//! is replayed through our force law **seeded once, two ticks before the
//! original's liftoff, and never reseeded**, so what is compared is the free
//! flight and the landing from an identical start. Per window: the original's
//! and our airtime in ticks, and the apex height above the surface below (a ray
//! along the craft's own down axis) for each.
//!
//! Prints rather than asserts. `#[ignore]`d and never run in CI.

use oag_gameplay::{collision_world, handling_for};
use oag_physics::{Environment, Handling, Ray, Raycaster};
use oag_pulse as pulse;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, DeltaSource, Inputs, Options, replay};
use oag_trace::trace::AngularReading;
use oag_vex::collision;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const TEAM: &str = "Assegai";
const CLASS: &str = "VENOM";

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn load() -> Option<(Handling, oag_physics::CollisionWorld)> {
    let image = workspace(IMAGE);
    if !image.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "{} is missing and OAG_REQUIRE_GAME_DATA is set",
            image.display()
        );
        return None;
    }
    let mut archives = pulse::open(image.to_str().expect("utf-8 path")).ok()?;
    let track_blob = archives.read_name(TRACK).expect("reading the track");
    let nodes = collision::from_vex(&track_blob).expect("decoding the collision nodes");
    let collision = collision_world(&nodes);
    let stats_blob = archives
        .read_name(&handling::entry_name(TEAM))
        .expect("reading the handling stats");
    let stats = handling::from_blob(&stats_blob).expect("parsing the handling stats");
    let handling = handling_for(
        &stats,
        CLASS,
        handling::SpeedupPads::default(),
        handling::Special::default(),
    )
    .expect("this team's file authors the class under test");
    Some((handling, collision))
}

fn trace(name: &str) -> Trace {
    let text =
        std::fs::read_to_string(workspace(&format!("data/traces/{name}"))).expect("reading it");
    Trace::parse(&text).expect("parsing it")
}

fn height_above(collision: &oag_physics::CollisionWorld, frame: &oag_trace::Frame) -> f32 {
    let down = -frame.up;
    collision
        .raycast(Ray::new(frame.position, down, 400.0), None, false)
        .map_or(f32::NAN, |hit| hit.distance)
}

/// Airborne windows: first and last tick with `grounded < 1.0`, holding at
/// least three ticks with `grounded == 0`.
fn windows(trace: &Trace) -> Vec<(u64, u64)> {
    const MIN: usize = 3;
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut full = 0usize;
    for (i, f) in trace.frames.iter().enumerate() {
        if f.grounded < 1.0 {
            if start.is_none() {
                start = Some(i);
                full = 0;
            }
            if f.grounded == 0.0 {
                full += 1;
            }
        } else if let Some(s) = start.take()
            && full >= MIN
        {
            out.push((trace.frames[s].tick, trace.frames[i - 1].tick));
        }
    }
    out
}

fn run(name: &str) {
    let Some((handling, collision)) = load() else {
        return;
    };
    let recorded = trace(name);
    println!("\n== {name}");
    println!(
        "{:>6} {:>6} | {:>5} {:>7} {:>7} | {:>5} {:>7} {:>7} | apex ratio  speed",
        "lift", "land", "o_air", "o_apex", "o_vy", "u_air", "u_apex", "u_vy"
    );
    for (lift, land) in windows(&recorded) {
        let from = lift.saturating_sub(2);
        let to = land + 60;
        let slice = Trace {
            frames: recorded
                .frames
                .iter()
                .filter(|f| f.tick >= from && f.tick <= to)
                .cloned()
                .collect(),
        };
        if slice.frames.len() < 6 {
            continue;
        }
        let options = Options {
            inputs: Inputs::FromTrace,
            dt: DeltaSource::Trace,
            basis: Basis::LeftUpForward,
            angular: AngularReading::NegatedLocal,
            scheme: Default::default(),
            reseed: None,
        };
        let ours = replay(
            &slice,
            &handling,
            &Environment::default(),
            &collision,
            &options,
            None,
        );
        let stats = |t: &Trace| {
            let mut air = 0u64;
            let mut apex = 0.0f32;
            let mut vy_at_lift = f32::NAN;
            let mut in_air = false;
            for f in &t.frames {
                if f.tick < lift || f.tick > land + 5 {
                    continue;
                }
                if f.grounded < 1.0 {
                    if !in_air {
                        in_air = true;
                        vy_at_lift = f.velocity.dot(f.up);
                    }
                    air += 1;
                    let h = height_above(&collision, f);
                    if h.is_finite() {
                        apex = apex.max(h);
                    }
                }
            }
            (air, apex, vy_at_lift)
        };
        let (oa, oh, ov) = stats(&slice);
        let (ua, uh, uv) = stats(&ours);
        let speed = slice
            .frames
            .iter()
            .find(|f| f.tick == lift)
            .map_or(0.0, |f| f.speed);
        println!(
            "{lift:>6} {land:>6} | {oa:>5} {oh:>7.2} {ov:>7.2} | {ua:>5} {uh:>7.2} {uv:>7.2} | {:>10.2} {speed:>6.0}",
            uh / oh
        );
    }
}

#[test]
#[ignore = "needs data/images/"]
fn airborne_windows_against_the_original() {
    for name in [
        "talons-junction-clean-lap.csv",
        "talons-junction-autopilot.csv",
        "talons-junction-time-trial-lap.csv",
    ] {
        run(name);
    }
}
