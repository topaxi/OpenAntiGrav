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
        blended[0].abs_diff(179) <= 3 && blended[1].abs_diff(179) <= 3,
        "the live list holds (179, 179, 250, 250); the blend reads {blended:?}"
    );
    // The pose is read to two decimals and the ramp climbs about 35 per unit of road, so three bytes is a tenth of a unit.
    assert!(
        blended[2].abs_diff(250) <= 2 && blended[3].abs_diff(250) <= 2,
        "the point and spot bytes of the live list are 250; the blend reads {blended:?}"
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
    let mut race = oag_raceplay::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.core.sim.world.ships[0].physics.body.position = TUNNEL;
    assert_eq!(
        race.core.sim.world.ships[0].standing.course_index, None,
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

/// Where the four Pure compare pairs put the player, with the grid scale the original's GE list holds there: `250`
/// (`249/255` on every light byte, `docs/ghidra/functions/psp-pure-usa/hull-shine.md`, "Light scale").
const PURE_GRIDS: [(&str, [f32; 3]); 4] = [
    ("01_Vineta_K", [38.59997, 37.47333, -96.69416]),
    ("04_Chenghou_Project", [-301.54987, 28.148235, 454.7532]),
    ("07_Blue_Ridge", [-203.15292, 23.825203, -84.73067]),
    ("03_Modesto_Heights", [-309.90353, 197.99696, -429.9906]),
];

/// The grid's scale is the spline's double-truncated blend of `255` points, `249..=251` by where in a segment the
/// craft stands, and `250` where the original read it. Blue Ridge's pose is 21 units off the ring, so its chord
/// projection is the loosest estimate of the four: it may read one byte off.
#[test]
#[ignore = "needs the Pure PSP disc in data/images/"]
fn a_pure_grid_craft_reads_the_original_s_250_not_a_full_255() {
    assert_eq!(
        (255 * 250u32) >> 8,
        249,
        "scale 250 is the GE lists' 249 on a 255 light byte"
    );
    let Some(image) = oag_testdata::image("data/images/pure-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pure::open(&image.display().to_string()).expect("mounting the disc");
    for (circuit, pose) in PURE_GRIDS {
        let file = archives
            .read_name(&format!(r"Data\Environments\{circuit}\track.vex"))
            .expect("the circuit's track file");
        let nodes = vex::nodes(&file).expect("nodes");
        let node = track::find_node(&file, &nodes).expect("the track node");
        let ai = track::parse(&file[node.payload()]).expect("the track");
        let course = Course::from_track(&ai, None).expect("a ring");
        let at = Vec3::new(pose[0], pose[1], pose[2]);
        let index = course.locate(at, None).expect("a ring point").index;
        let scale = course.light_scale_at(index, at).expect("a blend");
        let slack = u8::from(circuit == "07_Blue_Ridge");
        assert!(
            scale.iter().all(|&byte| byte.abs_diff(250) <= slack),
            "{circuit}: the original's grid scale is 250, the blend reads {scale:?}"
        );
    }
}
