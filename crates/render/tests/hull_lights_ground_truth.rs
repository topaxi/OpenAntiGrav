//! `HullLights::from_track` against the light list PPSSPP's GE was handed for
//! the player's hull on Talon's Junction - see
//! `docs/ghidra/functions/psp-pulse-usa/scene-light.md`, "Read live".
//!
//! The live list, at the grid's scale of `250`:
//!
//! ```text
//! AMBIENT=(41,47,50)
//! L0 (-0.639, 0.529, -0.559) diffuse (249,232,153)
//! L1 (-0.093, 0.328,  0.940) diffuse (78,95,117)
//! L2 ( 0.965, 0.260,  0.024) diffuse (47,58,71)
//! ```

use oag_mesh::mesh_render::HullLights;
use oag_vex::vex;

fn bytes(rgb: [f32; 4]) -> [u32; 3] {
    [0, 1, 2].map(|i| (rgb[i] * 255.0).round() as u32)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn talons_junction_builds_the_list_the_ge_was_handed_for_the_players_hull() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
    let blob = archives
        .read_name(r"Data\Environments\16_Track\track.vex")
        .expect("track.vex");
    let nodes = vex::nodes(&blob).expect("nodes");
    let lights = HullLights::from_track(&blob, &nodes, 250).expect("16_Track authors lights");

    assert!(lights.enabled());
    assert_eq!(bytes(lights.ambient), [41, 47, 50]);
    let diffuse: Vec<_> = lights.diffuse.iter().map(|d| bytes(*d)).collect();
    assert_eq!(
        diffuse,
        [[249, 232, 153], [78, 95, 117], [47, 58, 71], [0, 0, 0]]
    );
    let live = [
        [-0.639, 0.529, -0.559],
        [-0.093, 0.328, 0.940],
        [0.965, 0.260, 0.024],
    ];
    for (slot, want) in live.iter().enumerate() {
        let got = lights.direction[slot];
        assert_eq!(got[3], 1.0, "light {slot} enabled");
        for k in 0..3 {
            assert!(
                (got[k] - want[k]).abs() < 1e-3,
                "light {slot}: {got:?} against the live {want:?}"
            );
        }
    }
    assert_eq!(lights.direction[3][3], 0.0, "LIGHT3 is off live");
}

/// The `hull_lights` rule carries the PSP's law to the PS2 because the PS2
/// circuits author the same light nodes: every circuit builds a list, and
/// Talon's Junction builds the PSP's own.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and pulse-psp-usa.chd"]
fn pulse_ps2_circuits_author_the_psps_hull_lights() {
    let (Some(ps2), Some(psp)) = (
        oag_testdata::image("pulse-ps2-eu.chd"),
        oag_testdata::image("pulse-psp-usa.chd"),
    ) else {
        return;
    };
    let lights = |image: &std::path::Path, circuit: &str| {
        let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
        let blob = archives
            .read_name(&format!(r"Data\Environments\{circuit}\track.vex"))
            .ok()?;
        let nodes = vex::nodes(&blob).expect("nodes");
        HullLights::from_track(&blob, &nodes, 250)
    };
    let ps2_talons = lights(&ps2, "16_Track").expect("PS2 16_Track authors lights");
    let psp_talons = lights(&psp, "16_Track").expect("PSP 16_Track authors lights");
    assert_eq!(bytes(ps2_talons.ambient), bytes(psp_talons.ambient));
    assert_eq!(ps2_talons.diffuse, psp_talons.diffuse);
    assert_eq!(ps2_talons.direction, psp_talons.direction);
}

fn unpack(word: f32) -> [u32; 3] {
    let w = word as u32;
    [w & 255, w >> 8 & 255, w >> 16 & 255]
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    v.map(|c| c / l)
}

fn point_slots(
    track: &str,
    directionals: usize,
    craft: [f32; 3],
    scale: u8,
) -> [oag_mesh::mesh_render::HullPoint; 2] {
    let image = oag_testdata::image("pulse-psp-usa.chd").expect("image");
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
    let blob = archives
        .read_name(&format!(r"Data\Environments\{track}.vex"))
        .expect("track vex");
    let nodes = vex::nodes(&blob).expect("nodes");
    oag_mesh::mesh_render::HullPointLights::from_track(&blob, &nodes, directionals)
        .at(craft, [scale, scale])
}

/// Tech de Ra's grid, read live from PPSSPP (2026-10-20): three directionals, so one
/// slot, and the fourth light the GE was handed was a `wopoint` 10.25 units away - LDC3
/// `(61, 142, 157)`, vector `(-0.003, 4.043, 9.421)`, every scale byte `249`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn tech_de_ras_fourth_hull_light_is_the_wopoint_the_ge_was_handed() {
    if oag_testdata::image("pulse-psp-usa.chd").is_none() {
        return;
    }
    let got = point_slots("04_Track\\track", 3, [-58.02138, -9.513782, 79.93865], 249);
    assert_eq!(unpack(got[0][3]), [61, 142, 157]);
    let want = unit([-0.0030, 4.0426, 9.4211]);
    for k in 0..3 {
        assert!(
            (got[0][k] - want[k]).abs() < 2e-3,
            "{got:?} against {want:?}"
        );
    }
    assert_eq!(got[1], [0.0; 4], "three directionals leave one slot");
}

/// Fort Gale's grid (the reversed layout), read live: two directionals, two free slots,
/// `LDC2 (5, 19, 31)` at `(-34.6, 4.97, -8.64)` and `LDC3 (245, 245, 219)` at
/// `(-0.147, 25.89, 9.27)`, every scale byte `251`. Both come from the circuit's 105
/// `wopoint`s and 111 `wospot`s, so this is the cone law's one live check.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn fort_gales_two_free_hull_slots_take_the_lights_the_ge_was_handed() {
    if oag_testdata::image("pulse-psp-usa.chd").is_none() {
        return;
    }
    let got = point_slots(
        "14_Track\\track_reversed",
        2,
        [102.8246, 111.6629, 560.3614],
        251,
    );
    let live = [
        ([5, 19, 31], unit([-34.6045, 4.9731, -8.6423])),
        ([245, 245, 219], unit([-0.1466, 25.8931, 9.2671])),
    ];
    for (slot, (bytes, dir)) in live.iter().enumerate() {
        assert_eq!(unpack(got[slot][3]), *bytes, "slot {slot}: {:?}", got[slot]);
        for k in 0..3 {
            assert!(
                (got[slot][k] - dir[k]).abs() < 2e-3,
                "slot {slot}: {:?}",
                got[slot]
            );
        }
    }
}

/// Talon's Junction's grid took no point light live (`LIGHT3` off): none of its 21
/// `wopoint`s reaches the hull there, and ours agrees.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn talons_junctions_grid_takes_no_point_light() {
    if oag_testdata::image("pulse-psp-usa.chd").is_none() {
        return;
    }
    let image = oag_testdata::image("pulse-psp-usa.chd").expect("image");
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
    let blob = archives
        .read_name(r"Data\Environments\16_Track\track.vex")
        .expect("vex");
    let nodes = vex::nodes(&blob).expect("nodes");
    // The craft's grid position is the spline's first sample region; the live list had
    // LIGHT3=0 with the craft on the grid at (104.8, -48.4, -203.7), the probe's own
    // reference point.
    let lights = oag_mesh::mesh_render::HullPointLights::from_track(&blob, &nodes, 3);
    assert_eq!(lights.at([104.8, -48.4, -203.7], [250, 250]), [[0.0; 4]; 2]);
}
