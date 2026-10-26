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

/// Where the pair `pulse-psp-race/talons-alongside` put the player: inside Talon's Junction's tunnel.
const TUNNEL: Vec3 = Vec3::new(-118.38782, -49.23908, -702.0541);

/// A craft no tick has located reads the scale of where it stands, not the default `255`.
///
/// The original's GE list at that pose (a recorded frame, PPSSPP v1.20.4) lights the hull's first directional with
/// `(122, 114, 75)` where the open road holds `(254, 237, 156)`: a scale of about `123`, the `127` the tunnel's points
/// author blended. A posed capture runs no tick, so `standing.course_index` was `None` and the hull drew at full light,
/// `1.9x` the original over the fins and the body.
#[test]
#[ignore = "needs the Pulse PSP disc in data/images/"]
fn a_craft_no_tick_has_located_is_lit_by_the_tunnel_it_stands_in() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        class: "VENOM".to_string(),
        ..oag_raceplay::Options::default()
    })
    .expect("loading Talon's Junction");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.sim.world.ships[0].physics.body.position = TUNNEL;
    assert_eq!(
        race.sim.world.ships[0].standing.course_index,
        None,
        "the premise: nothing has located the craft"
    );
    let packed = race.ship_hull_scale(0);
    assert!(packed != 0.0, "the default 255 is the bug");
    let directional = (packed as u16 >> 8) as u8;
    assert!(
        (118..=130).contains(&directional),
        "the original's list holds a directional scale near 123, the pack reads {directional}"
    );
}
