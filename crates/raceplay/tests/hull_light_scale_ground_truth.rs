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

/// De Konstruct's reversed layout, the track node's path 0.
fn dekonstruct_ai() -> Option<track::AiTrack> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let file = archives
        .read_name(r"Data\Environments\05_Track\track_reversed.vex")
        .expect("De Konstruct's reversed layout");
    let nodes = vex::nodes(&file).expect("nodes");
    let node = track::find_node(&file, &nodes).expect("the track node");
    Some(track::parse(&file[node.payload()]).expect("the track"))
}

/// The law: the integer blend of segment 36's four control points at `t` 0.578 is the live list's bytes exactly.
/// `t` 0.5774 to 0.5799 reproduces all four; the nearest ring point (`187`) and the nearest-point `t` (0.587, `176`)
/// do not, so the pose-to-`t` step below is where the remaining error lives.
#[test]
#[ignore = "needs the Pulse PSP disc in data/images/"]
fn the_integer_blend_at_the_crafts_parameter_is_the_live_list_exactly() {
    let Some(ai) = dekonstruct_ai() else {
        return;
    };
    let controls = ai.paths[0].light_controls(36).expect("segment 36");
    let blended = track::blend_light_bytes(&track::basis(0.578), controls);
    assert_eq!(blended, [179, 179, 250, 250], "the live list's scale bytes");
}

/// The step from a pose to `t`: the Time Trial grid on De Konstruct White (`05_Track`, reversed layout) holds the
/// player at `(20.22, -17.40, -67.57)` (two decimals), and the spline's nearest point reads `t` 0.587 where the live
/// list implies 0.578: the original tracks its own parameter, which is not the nearest point to a pose. The ramp
/// climbs about 35 bytes per unit of road, so that 0.024 units is three bytes; the bound is that distance, stated.
#[test]
#[ignore = "needs the Pulse PSP disc in data/images/"]
fn the_grid_craft_reads_within_three_bytes_of_the_live_list() {
    let Some(ai) = dekonstruct_ai() else {
        return;
    };
    let course = Course::from_track(&ai, None).expect("a ring");
    let nearest = course.locate(GRID, None).expect("a ring point").index;
    let near = course.light_scale(nearest).expect("a ring point");
    let blended = course.light_scale_at(nearest, GRID).expect("a ring point");
    assert!(
        near[0].abs_diff(187) <= 2,
        "the nearest ring point holds {near:?}, which is what ours drew"
    );
    assert!(
        blended[0].abs_diff(179) <= 3 && blended[2].abs_diff(250) <= 2,
        "the live list holds (179, 179, 250, 250); the blend reads {blended:?}"
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
/// craft stands. The original reads `250` on all four; ours reads `250` on three and `249` on Chenghou, because the
/// pose-to-`t` step (the spline's nearest point) is not the original's own tracked parameter (see
/// [`the_grid_craft_reads_within_three_bytes_of_the_live_list`]). One byte is the bound the law's own range allows.
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
        assert!(
            scale.iter().all(|&byte| byte.abs_diff(250) <= 1),
            "{circuit}: the original's grid scale is 250, the blend reads {scale:?}"
        );
    }
}

/// Modesto Heights' grid hull takes the second light the original's GE list carries: `(208, 47, 1)` along
/// `(-19.24, 7.19, 1.42)` (`docs/ghidra/functions/psp-pure-usa/hull-shine.md`, "Light scale"). It is `wopointShape2`
/// (colour `(0.84, 0.192, 0.011)`, full strength inside 22.398 units, none beyond 33.779), 20.59 units from the grid
/// craft, so the builder hands it to the GE as a type-0 light at `light - craft`; its twin `wopointShape3` is 47.64
/// units away and reaches nothing, which leaves the second point slot empty. The colour byte is
/// `trunc(0.84 * 255) = 214`, scaled by the grid's `250`: `214 * 250 >> 8 = 208`.
///
/// Goes through Pure's own load, so it fails when the loader stops giving the hull the circuit's point lights.
#[test]
#[ignore = "needs the Pure PSP disc in data/images/"]
fn modestos_grid_hull_takes_the_second_light_the_ge_list_carries() {
    let Some(image) = oag_testdata::image("data/images/pure-psp-usa.chd") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        track: Some(r"Data\Environments\03_Modesto_Heights\track.vex".to_string()),
        class: "VENOM".to_string(),
        ..oag_raceplay::Options::default()
    })
    .expect("loading Modesto Heights");
    let points = &loaded.view.hull_points;
    assert_eq!(points.len(), 130, "Modesto authors 130 `wopoint`s");

    let grid = PURE_GRIDS[3].1;
    let slots = points.at(grid, 0.0, [250, 250]);
    let packed = slots[0][3] as u32;
    assert_eq!(
        [packed & 0xff, (packed >> 8) & 0xff, packed >> 16],
        [208, 47, 1],
        "slot 0 carries the original's colour bytes"
    );
    let want = Vec3::new(-19.24, 7.19, 1.42).normalize();
    let got = Vec3::new(slots[0][0], slots[0][1], slots[0][2]);
    assert!(
        (got - want).length() < 1e-3,
        "slot 0 points along the original's vector: {got:?} against {want:?}"
    );
    assert_eq!(slots[1], [0.0; 4], "the twin is out of range, slot 1 is empty");
}
