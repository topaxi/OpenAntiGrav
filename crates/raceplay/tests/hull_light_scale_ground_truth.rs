//! A craft's hull-light scale is the spline's blend at the craft's own place, not
//! the nearest ring point's value.
//!
//! Read live on PPSSPP (`docs/ghidra/functions/psp-pulse-usa/scene-light.md`, "De
//! Konstruct"): the Time Trial grid on De Konstruct White (`05_Track`, reversed
//! layout) holds the player at `(20.22, -17.40, -67.57)`, between a `255` control
//! point and a `127` one (0.59 of the way), and the model's list there carries
//! scale bytes `(179, 179, 250, 250)`. The nearest ring point holds `187`.
//!
//! **`#[ignore]`d and never run in CI**: it needs the Pulse PSP disc. Run with
//! `just test-data`.

use oag_core::math::Vec3;
use oag_race::Course;
use oag_vex::{track, vex};

const GRID: Vec3 = Vec3::new(20.22, -17.40, -67.57);

fn course() -> Option<Course> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let file = archives
        .read_name(r"Data\Environments\05_Track\track_reversed.vex")
        .expect("De Konstruct's reversed layout");
    let nodes = vex::nodes(&file).expect("nodes");
    let node = track::find_node(&file, &nodes).expect("the track node");
    let ai = track::parse(&file[node.payload()]).expect("the track");
    Course::from_track(&ai, None)
}

#[test]
#[ignore = "needs the Pulse PSP disc in data/images/"]
fn the_grid_craft_reads_the_blend_the_original_holds_not_the_nearest_ring_point() {
    let Some(course) = course() else {
        return;
    };
    let nearest = (0..course.len())
        .min_by(|&a, &b| {
            let d = |i: usize| {
                course
                    .position(i)
                    .map_or(f32::MAX, |p| p.distance_squared(GRID))
            };
            d(a).total_cmp(&d(b))
        })
        .expect("a ring");
    let near = course.light_scale(nearest).expect("a ring point");
    let blended = course.light_scale_at(nearest, GRID).expect("a ring point");
    assert!(
        near[0].abs_diff(187) <= 2,
        "the nearest ring point holds {near:?}, which is what ours drew"
    );
    assert!(
        blended[0].abs_diff(179) <= 2 && blended[1].abs_diff(179) <= 2,
        "the live list holds (179, 179, 250, 250); the blend reads {blended:?}"
    );
    assert_eq!(
        blended[2..],
        [250, 250],
        "the point and spot bytes of the live list"
    );
}
