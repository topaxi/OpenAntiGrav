//! A replay arms the speed pad the original crossed, on the tick it crossed it.
//!
//! Until 2026-09-30 `oag_trace::replay` never set
//! [`oag_physics::Environment::pad_hit`], so step 15 of the force law never ran
//! in any comparison against a capture. On `talons-junction-clean-lap.csv` the
//! original crosses eight pads, the first at tick 161, where its forward
//! acceleration jumps from about 32 to about 273 units/s^2; a replay without the
//! pad came out of that crossing 40 units/s slow, and that one missing boost was
//! most of the 44 units of position error the single-seeded comparison read at
//! tick 240 (15 with it).
//!
//! Two measurements, both on the recording's own data:
//!
//! 1. **The pose walk.** The swept containment test, run on the recorded
//!    positions with no integration, fires on exactly the ticks the recording's
//!    own velocity says a boost began - no earlier, no later.
//! 2. **The crossing.** Replayed from a few ticks before each crossing, our
//!    speed gain across it matches the recording's, and without the pads it
//!    does not come close. A reseed landing mid-boost keeps the boost, because
//!    the pad timer is restored from the recorded positions rather than reset.

use oag_gameplay::{collision_world, handling_for};
use oag_physics::{Environment, Handling};
use oag_pulse as pulse;
use oag_tables::handling;
use oag_trace::Trace;
use oag_trace::replay::{Basis, DeltaSource, Inputs, Options, replay};
use oag_trace::trace::AngularReading;
use oag_vex::pads::PadVolume;
use oag_vex::{collision, pads, vex};

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
const TRACK: &str = r"Data\Environments\16_Track\track.vex";
const CAPTURE: &str = "data/traces/talons-junction-clean-lap.csv";
const TEAM: &str = "Assegai";
const CLASS: &str = "VENOM";

/// The ticks the pose walk finds a pad under the recorded hull on its first
/// tick inside, read off this capture on 2026-09-30.
const CROSSINGS: [u64; 8] = [161, 680, 989, 1284, 1708, 2398, 2409, 2793];

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn capture() -> Trace {
    let path = workspace(CAPTURE);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is tracked in git: {e}", path.display()));
    Trace::parse(&text).expect("parsing the capture")
}

fn load() -> Option<(Handling, oag_physics::CollisionWorld, Vec<PadVolume>)> {
    let image = workspace(IMAGE);
    if !image.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "{} is missing and OAG_REQUIRE_GAME_DATA is set",
            image.display()
        );
        return None;
    }
    let mut archives = pulse::open(image.to_str().expect("utf-8 path")).expect("the image");
    let blob = archives.read_name(TRACK).expect("the track");
    let collision = collision_world(&collision::from_vex(&blob).expect("collision"));
    let nodes = vex::nodes(&blob).expect("the .vex decodes");
    let volumes = pads::volumes(&blob, &nodes, vex::CLASS_SPEEDUP_PAD);

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
    .expect("the global file authors a <Global>");
    let (speedup, _, _) = global.class_named(CLASS).expect("VENOM's <GlobalClass>");
    let handling =
        handling_for(&stats, CLASS, speedup, global.special).expect("the team authors VENOM");
    Some((handling, collision, volumes))
}

/// The recorded forward acceleration entering tick `t`, units/s^2.
fn forward_acceleration(trace: &Trace, t: usize) -> f32 {
    let (now, next) = (&trace.frames[t], &trace.frames[t + 1]);
    ((next.velocity - now.velocity) / next.dt).dot(now.forward)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_pose_walk_finds_a_pad_exactly_where_the_recording_boosts() {
    let Some((_, _, volumes)) = load() else {
        return;
    };
    let trace = capture();

    // The same swept test the replay runs, on the recorded positions alone.
    let mut previous = None;
    let mut inside_before = false;
    let mut entries = Vec::new();
    for frame in &trace.frames {
        let position = frame.position;
        let from: Option<oag_core::math::Vec3> = previous.replace(position);
        let sweep: Vec<_> = match from {
            Some(from) if from.distance(position) > 0.0 && from.distance(position) < 25.0 => (1
                ..=4)
                .map(|s| from.lerp(position, s as f32 / 4.0))
                .collect(),
            _ => vec![position],
        };
        let inside = volumes
            .iter()
            .any(|pad| sweep.iter().any(|p| pad.contains(p.to_array())));
        if inside && !inside_before {
            entries.push(frame.tick);
        }
        inside_before = inside;
    }
    assert_eq!(entries, CROSSINGS, "the pad entries on the recorded poses");

    // And every entry is the tick the recording's own forward acceleration
    // steps up to near the boost's first-tick force, `amount * 10 * time` = 253
    // for VENOM less the drag. An unboosted tick on this lap stays under about
    // 60, so 200 is diagnostic. The step is required too, because 2409 is
    // entered eleven ticks after 2398 with that boost still decaying (136): the
    // re-arm is what lifts it back to 218.
    for tick in CROSSINGS {
        let t = tick as usize;
        let before = forward_acceleration(&trace, t - 1);
        let at = forward_acceleration(&trace, t);
        println!("tick {tick}: forward acceleration {before:.1} -> {at:.1}");
        assert!(
            at > 200.0 && at - before > 50.0,
            "tick {tick}: the recording does not boost here ({before:.1} -> {at:.1})"
        );
    }
}

/// Our speed gain from `from` to `to`, replayed from a slice of the capture,
/// with and without the track's pads.
fn gain(
    trace: &Trace,
    handling: &Handling,
    collision: &oag_physics::CollisionWorld,
    pads: &[PadVolume],
    from: u64,
    to: u64,
    reseed: Option<usize>,
) -> f32 {
    let slice = Trace {
        frames: trace
            .frames
            .iter()
            .filter(|f| (from..=to).contains(&f.tick))
            .cloned()
            .collect(),
    };
    let options = Options {
        inputs: Inputs::FromTrace,
        dt: DeltaSource::Trace,
        basis: Basis::LeftUpForward,
        angular: AngularReading::NegatedLocal,
        scheme: Default::default(),
        reseed: reseed.and_then(std::num::NonZeroUsize::new),
        pads: pads.to_vec(),
    };
    let ours = replay(
        &slice,
        handling,
        &Environment::default(),
        collision,
        &options,
        None,
    );
    ours.frames.last().expect("a row").speed - ours.frames.first().expect("a row").speed
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_replay_gains_the_speed_the_original_gains_crossing_a_pad() {
    let Some((handling, collision, volumes)) = load() else {
        return;
    };
    let trace = capture();
    // 2409 is a second pad eleven ticks after 2398, entered with that boost
    // still running and on a stretch where the recording itself is losing
    // speed, so a gain across it measures the stretch rather than the pad. The
    // pose walk above still pins that it fires.
    for tick in CROSSINGS.into_iter().filter(|&tick| tick != 2409) {
        let (from, to) = (tick - 3, tick + 14);
        let recorded = trace.frames[to as usize].speed - trace.frames[from as usize].speed;
        let with = gain(&trace, &handling, &collision, &volumes, from, to, None);
        let without = gain(&trace, &handling, &collision, &[], from, to, None);
        // Reseeded every five ticks, so two window boundaries land inside the
        // boost: the restored timer is what keeps them from cutting it off.
        let reseeded = gain(&trace, &handling, &collision, &volumes, from, to, Some(5));
        println!(
            "tick {tick}: recorded +{recorded:.2}, ours +{with:.2}, reseeded +{reseeded:.2}, \
             without pads +{without:.2}"
        );
        // Single-seeded drifts for seventeen ticks and lands within 13 %.
        // Reseeded stays within about 2 %; with the timer reset instead of
        // restored it falls about 10 % short, so 5 % is what tells them apart.
        for (what, ours, within) in [("single-seeded", with, 0.15), ("reseeded", reseeded, 0.05)] {
            assert!(
                (ours - recorded).abs() < within * recorded,
                "tick {tick} {what}: ours +{ours:.2} against the recording's +{recorded:.2}"
            );
        }
        assert!(
            without < 0.5 * recorded,
            "tick {tick}: without the pads the gain should fall well short, +{without:.2}"
        );
    }
}
