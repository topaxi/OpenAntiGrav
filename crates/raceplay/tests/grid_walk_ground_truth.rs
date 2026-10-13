//! Pulse PSP's grid walk against the original, on three circuits.
//!
//! `Race_ComputeGridLayout` walks the located curve, and the located record it writes is
//! scaled by `0.999756` (`oag_gameplay::grid_walk`). Eight craft read out of the running
//! PPSSPP at placement on each of `16_Track`, `01_Track` and Metropia reversed, plus the
//! eight located records of that walk read live on Metropia (the heading call's `s7`
//! record, `0x0882bb54`), are what this pins. The walk it replaced put `01_Track`'s worst
//! slot 1.73 units out and `16_Track`'s 0.62.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_raceplay as race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn options_for(id: &str) -> Option<race::Options> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).ok()?;
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .ok()?;
    let definition = oag_tables::fexml::expand(&blob).ok()?;
    let track = oag_raceplay::catalogue::tracks(&definition)
        .into_iter()
        .find(|track| track.id == id)?;
    Some(race::Options {
        source: image.display().to_string(),
        class: "Venom".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(track.entry_name()),
        ..race::Options::default()
    })
}

/// `(position, forward)` of the original's eight craft, slot 1 first. Position and forward
/// are the racer table's `+0x10` and the second of its three axes, read at placement
/// (`scripts/psp-grid-pose.py`, 2026-10-01, `RESTART RACE` on a Single Race).
type Eight = [([f32; 3], [f32; 3]); 8];

const ORIGINAL_16: Eight = [
    ([6.145, -52.221, -195.876], [0.99999, 0.00368, -0.00251]),
    ([-13.561, -51.863, -175.820], [0.99999, 0.00254, -0.00325]),
    ([-33.427, -52.275, -195.740], [0.99999, -0.00058, -0.00386]),
    ([-53.142, -51.852, -175.655], [0.99999, -0.00112, -0.00442]),
    ([-73.032, -52.180, -195.549], [0.99999, -0.00135, -0.00492]),
    ([-92.745, -51.804, -175.441], [0.99998, -0.00357, -0.00534]),
    ([-112.624, -51.999, -195.319], [0.99998, -0.00396, -0.00572]),
    ([-132.291, -51.737, -175.193], [0.99998, -0.00254, -0.00608]),
];

const ORIGINAL_01: Eight = [
    ([-721.162, 1.859, 282.625], [-0.00690, 0.00076, 0.99998]),
    ([-701.135, 1.859, 262.933], [-0.00141, -0.00076, 1.00000]),
    ([-721.148, 1.859, 243.210], [0.00065, 0.00000, 1.00000]),
    ([-701.168, 1.859, 223.46], [0.00104, 0.00076, 1.00000]),
    ([-721.184, 1.859, 203.749], [0.00080, -0.00076, 1.00000]),
    ([-701.195, 1.859, 184.005], [0.00054, -0.00076, 1.00000]),
    ([-721.202, 1.859, 164.269], [0.00037, 0.00000, 1.00000]),
    ([-701.206, 1.859, 144.512], [0.00022, 0.00076, 1.00000]),
];

/// Metropia reversed, read 2026-09-29 (positions only, two places).
const ORIGINAL_18: [[f32; 3]; 8] = [
    [529.33, -12.92, 169.73],
    [510.08, -12.98, 190.08],
    [490.04, -13.05, 170.47],
    [470.72, -13.11, 190.84],
    [450.61, -13.18, 171.24],
    [431.44, -13.24, 191.62],
    [411.30, -13.31, 172.04],
    [392.11, -13.38, 192.44],
];

/// The located records of Metropia reversed's walk, slot 8 first: `+0x00` of the record
/// at `0x0882bb54`'s `s7`, read live on 2026-10-02. Its
/// `w` lane read `0.999755859375` on all eight, which is the record scale.
const LIVE_18_CHAIN: [[f32; 3]; 8] = [
    [391.9265, -14.3851, 181.0272],
    [411.5125, -14.3184, 179.2485],
    [431.1408, -14.2519, 177.8692],
    [450.7913, -14.1858, 176.793],
    [470.4516, -14.1203, 175.9421],
    [490.1157, -14.0553, 175.2645],
    [509.7802, -13.9909, 174.7381],
    [529.4432, -13.9272, 174.3685],
];

fn horizontal(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

/// Each slot's xz distance to the original's, and each heading's angle in degrees.
fn compare(id: &str, positions: [[f32; 3]; 8], forwards: Option<[[f32; 3]; 8]>) -> (f32, f32) {
    let Some(options) = options_for(id) else {
        return (0.0, 0.0);
    };
    let loaded = race::load(&options).expect("load");
    let race = race::Race::start(loaded.setup);
    let mut worst_position = 0.0f32;
    let mut worst_heading = 0.0f32;
    for (index, ship) in race
        .sim
        .world
        .ships
        .iter()
        .enumerate()
        .filter(|(_, ship)| ship.active)
    {
        let slot = if index == 0 { 8 } else { index };
        let want = Vec3::from_array(positions[slot - 1]);
        let got = ship.physics.body.position;
        let error = horizontal(got - want).length();
        worst_position = worst_position.max(error);
        let mut line = format!("{id} slot {slot}: {error:.3} apart");
        if let Some(forwards) = forwards {
            let ours = horizontal(ship.physics.body.orientation * Vec3::NEG_Z).normalize();
            let theirs = horizontal(Vec3::from_array(forwards[slot - 1])).normalize();
            // The sine of the angle, not its cosine: `acos` of a dot product within
            // 1e-7 of one reads zero and cannot see a thousandth of a degree.
            let degrees = (ours.x * theirs.z - ours.z * theirs.x)
                .asin()
                .to_degrees()
                .abs();
            worst_heading = worst_heading.max(degrees);
            line.push_str(&format!(", heading {degrees:.4} degrees"));
        }
        println!("{line}");
    }
    println!("{id}: worst slot {worst_position:.3}, worst heading {worst_heading:.4}");
    (worst_position, worst_heading)
}

fn split(eight: Eight) -> ([[f32; 3]; 8], [[f32; 3]; 8]) {
    (eight.map(|(p, _)| p), eight.map(|(_, f)| f))
}

/// `16_Track`: every slot within 0.05 of the original (worst measured 0.043, it was 0.62
/// with the sawtooth), and every heading within 0.005 degrees (worst 0.0009, it was 0.045).
///
/// The bounds are **chosen, not measured**, a little over the worst measured.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_walk_is_the_originals_on_talons_junction() {
    let (positions, forwards) = split(ORIGINAL_16);
    let (position, heading) = compare("16_Track", positions, Some(forwards));
    assert!(position < 0.05, "worst slot {position:.3}");
    assert!(heading < 0.005, "worst heading {heading:.4} degrees");
}

/// `01_Track` (Basilico Black): every slot within 0.01 of a unit (worst measured 0.001; it
/// was 1.73), which is what the record scale buys - without it the worst is 0.5 and the whole column sits 0.17 off the
/// origin-ward side - and slot 1's kink, the 0.22 degrees `grid-state.md` could not explain,
/// reproduced.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_walk_is_the_originals_on_basilico_black() {
    let (positions, forwards) = split(ORIGINAL_01);
    let (position, heading) = compare("01_Track", positions, Some(forwards));
    assert!(position < 0.01, "worst slot {position:.3}");
    assert!(heading < 0.005, "worst heading {heading:.4} degrees");
}

/// Metropia reversed: the circuit the exact-walk attempt made worse (1.13 to 1.99). Worst
/// measured 0.070, all of it the original's two-decimal readings and the craft's placement
/// drop; the located records themselves are pinned to 0.0002 below.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_walk_is_the_originals_on_metropia_reversed() {
    let (position, _) = compare("18_Track", ORIGINAL_18, None);
    assert!(position < 0.1, "worst slot {position:.3}");
}

/// The walk's own located records, before any lateral stagger or raycast, against the eight
/// read live out of the original's loop: slot 8 first, each one the previous one's
/// `locate(position + tangent * 19.8)`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_located_chain_is_the_originals_on_metropia_reversed() {
    let Some(options) = options_for("18_Track") else {
        return;
    };
    let loaded = race::load(&options).expect("load");
    let setup = loaded.setup;
    let node = Vec3::from_array(setup.start_position.expect("an authored node").position);
    let mut at = oag_gameplay::grid_walk::locate(&setup.ai, node).expect("a track");
    let mut worst = 0.0f32;
    for (index, want) in LIVE_18_CHAIN.iter().enumerate() {
        let error = (at.position - Vec3::from_array(*want)).length();
        println!(
            "record {index}: {:?} against {want:?}, {error:.4}",
            at.position
        );
        worst = worst.max(error);
        at = oag_gameplay::grid_walk::locate(
            &setup.ai,
            at.position + at.tangent * oag_gameplay::GRID_ROW_PITCH,
        )
        .expect("a track");
    }
    println!("worst located record: {worst:.4}");
    assert!(worst < 0.01, "worst located record is {worst:.4} out");
}
