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
