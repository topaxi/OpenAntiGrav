//! The original's section-box view test, against the sections of every Pulse
//! circuit that ships.
//!
//! **`#[ignore]`d and never run in CI**: it needs a disc image in `data/images/`.
//! `just test-data` runs it.
//!
//! The test itself is `oag_render::pvs::sections_in_view`; what only the disc can
//! say is whether the authored boxes behave under it. Two things must hold on
//! every circuit, from a camera placed the way a chase camera is on the racing
//! line:
//!
//! 1. **The craft's own section is never rejected.** The craft sits in front of
//!    the camera, in its section, so a box that contains it cannot have every
//!    corner outside one clip plane. A section the test hid under the craft would
//!    be a bug in the test or a misauthored box, either way a hole in the frame.
//! 2. **The test does narrow.** If it rejected nothing it would be dead code
//!    that passed every other check; across a lap it must reject a real share of
//!    the declared sections.
//!
//! `docs/rendering/frame-audit.md` section 3 holds the comparison against the
//! original's own frames, which is the evidence that the test is the original's.

use oag_core::math::{Vec3, camera};
use oag_render::pvs::sections_in_view;
use oag_vex::pvs::TrackPvs;
use oag_vex::{track, vex};

const TRACK_IDS: &[&str] = &[
    "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "08_Track",
    "09_Track", "10_Track", "11_Track", "12_Track", "13_Track", "14_Track", "15_Track", "16_Track",
    "17_Track", "18_Track", "19_Track", "20_Track",
];
const VARIANTS: &[&str] = &["track.vex", "track_reversed.vex"];

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_craft_keeps_its_own_section_and_the_test_does_narrow() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    let projection = camera::perspective(60f32.to_radians(), 480.0 / 272.0, 1.0, 2500.0);
    let mut files = 0;
    let (mut samples, mut rejected, mut declared, mut outside) = (0usize, 0usize, 0usize, 0usize);
    for id in TRACK_IDS {
        for variant in VARIANTS {
            let entry = format!(r"Data\Environments\{id}\{variant}");
            let Ok(blob) = archives.read_name(&entry) else {
                continue;
            };
            files += 1;
            let pvs = TrackPvs::parse(&blob).expect("parsing sections");
            let nodes = vex::nodes(&blob).expect("walking the tree");
            let ai_node = track::find_node(&blob, &nodes).expect("a WO Track node");
            let ai = track::parse(&blob[ai_node.payload()]).expect("parsing the spline");
            let ids: Vec<u8> = pvs.ids().collect();
            for point in ai.paths.iter().flat_map(|p| p.points.iter()).step_by(8) {
                let pos = Vec3::from_array(point.pos);
                let forward = Vec3::from_array(point.tangent).normalize_or_zero();
                let Some(bound) = pvs.bounds_of(point.section_id) else {
                    continue;
                };
                // Only where the authored box really holds the craft: a spline
                // control point is not always inside its own section's box, and
                // then there is nothing for the test to keep.
                if forward == Vec3::ZERO || !bound.contains(point.pos) {
                    outside += 1;
                    continue;
                }
                // A chase camera: behind and above the craft, looking along the line.
                let eye = pos - forward * 10.0 + Vec3::Y * 3.0;
                let view = camera::look_at(eye, pos + forward * 20.0, Vec3::Y);
                let mask = sections_in_view(&pvs, &(projection * view));
                samples += 1;
                declared += ids.len();
                rejected += ids.iter().filter(|&&s| mask & (1u64 << s) == 0).count();
                assert!(
                    mask & (1u64 << point.section_id) != 0,
                    "{entry}: the section under the craft (id {}) is rejected from a chase \
                     camera at {:?}",
                    point.section_id,
                    point.pos
                );
            }
        }
    }
    println!(
        "{files} track file(s), {samples} camera(s) ({outside} control point(s) outside their \
         own section's box, skipped): {rejected} of {declared} section tests reject"
    );
    assert!(files >= 12, "only {files} track files found");
    assert!(
        rejected * 5 > declared,
        "the test rejected {rejected} of {declared} section tests: it is not narrowing"
    );
}
