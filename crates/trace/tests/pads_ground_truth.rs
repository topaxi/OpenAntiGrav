//! The `oag-trace pads` recipe against the real disc: pads decode, they sit on
//! the course ring, and "N units before" lands N units before.
//!
//! `crates/formats/tests/pads_ground_truth.rs` already pins the *decode* across
//! all 762 pad nodes on the disc. What it cannot pin is the projection this
//! crate adds on top - pad centre onto the arc-length ring, then an approach
//! point walked upstream - because `Course` lives above `oag-formats`. That
//! composition is what `scripts/psp-drive.py place` trusts, so it is measured
//! here rather than assumed.

use oag_core::math::Vec3;
use oag_formats::{pads, track, vex};
use oag_race::Course;

const IMAGE: &str = "data/images/pulse-psp-usa.chd";
/// Talon's Junction, the environment every capture under `data/traces/` was
/// taken on and the default track everywhere else in this crate.
const TRACK: &str = r"Data\Environments\16_Track\track.vex";

fn workspace(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// The track blob and its course, or `None` when this checkout has no image.
fn load() -> Option<(Vec<u8>, Course)> {
    let image = workspace(IMAGE);
    if !image.exists() {
        println!("skipping: {} is missing", image.display());
        return None;
    }
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
    let course =
        Course::from_track(&ai, Some(Vec3::from(slot.position))).expect("the primary chain closes");
    Some((blob, course))
}

/// Every speedup pad on Talon's Junction sits on the ring, and walking the
/// ring `length - 50` forward from a pad puts you 50 units of progress behind
/// it. The tolerance is the ring's own sampling: at four samples per segment
/// the points are a couple of units apart, so the walk can stop up to one
/// step short of the exact distance.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_approach_point_is_upstream_of_its_pad_by_the_asked_distance() {
    let Some((blob, course)) = load() else {
        return;
    };
    let nodes = vex::nodes(&blob).expect("the .vex decodes");
    let volumes = pads::volumes(&blob, &nodes, vex::CLASS_SPEEDUP_PAD);
    assert!(
        !volumes.is_empty(),
        "Talon's Junction authors no speedup pads?"
    );

    const BEFORE: f32 = 50.0;
    for (index, pad) in volumes.iter().enumerate() {
        let centre = Vec3::from_array(pad.centre());
        let located = course.locate(centre, None).expect("the pad is somewhere");
        println!(
            "speedup pad {index}: progress {:.1}, offset {:.1}",
            located.progress, located.offset
        );
        // A pad the ring does not cover (a shortcut branch) is allowed to
        // exist, but on this circuit none do, and asserting so is what makes
        // `offset` trustworthy as the "do not trust progress" indicator.
        assert!(
            located.offset < course.max_half_width() * 2.0,
            "pad {index} is {:.1} units off the ring",
            located.offset
        );

        let upstream = course.advance(located.index, course.length() - BEFORE);
        let progress = course.progress_at(upstream).expect("a ring index");
        let walked = (located.progress - progress).rem_euclid(course.length());
        assert!(
            (walked - BEFORE).abs() < 3.0,
            "pad {index}: asked for {BEFORE} units upstream, got {walked:.1}"
        );
    }
}
