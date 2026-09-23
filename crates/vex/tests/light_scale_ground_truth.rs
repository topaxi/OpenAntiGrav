//! The spline points' hull light scales (`+0x62..+0x65`) against what a live
//! race carries - see `docs/ghidra/functions/psp-pulse-usa/scene-light.md`.
//!
//! Live on PPSSPP (Talon's Junction, the player on the grid), the player's
//! hull model carried `250` on all four scales, and an AI craft inside a
//! tunnel `133`. The disc side of that: the grid's points author `255` and
//! the tunnels `127`, and the original's blend truncates a run of `255` to
//! just under it.

use oag_vex::{track, vex};

#[test]
#[ignore = "needs a disc image in data/images/"]
fn talons_junction_authors_full_light_on_the_grid_and_half_in_its_tunnels() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
    let blob = archives
        .read_name(r"Data\Environments\16_Track\track.vex")
        .expect("track.vex");
    let nodes = vex::nodes(&blob).expect("nodes");
    let node = track::find_node(&blob, &nodes).expect("a WO Track node");
    let ai = track::parse(&blob[node.payload()]).expect("parse");
    assert!(
        ai.version >= track::LIGHT_SCALE_VERSION,
        "{:#x}",
        ai.version
    );

    let points: Vec<_> = ai.paths.iter().flat_map(|p| &p.points).collect();
    let full = points.iter().filter(|p| p.light_scale == [255; 4]).count();
    let half = points
        .iter()
        .filter(|p| p.light_scale == [127, 127, 255, 255])
        .count();
    assert_eq!((points.len(), full, half), (862, 455, 231));

    // The grid slot the live read was taken at: the nearest point authors
    // full light, and the original's blend of four such points lands on the
    // 250/251 the race carried, not on 255.
    let grid = [104.8f32, -48.4, -203.7];
    let nearest = points
        .iter()
        .min_by(|a, b| {
            let d =
                |p: &track::SplinePoint| (0..3).map(|k| (p.pos[k] - grid[k]).powi(2)).sum::<f32>();
            d(a).total_cmp(&d(b))
        })
        .expect("points");
    assert_eq!(nearest.light_scale, [255; 4]);
    let blended = track::blend_light_scale(&track::basis(0.5), &[nearest; 4]);
    assert!(
        blended.iter().all(|&b| (249..=252).contains(&b)),
        "{blended:?}"
    );
}
