//! What Talon's Junction stamps into the bloom's glow mask, against the
//! original's own EDRAM read - see `docs/rendering/glow-mask.md`.
//!
//! Live, on the grid, the mask read `4` almost everywhere and the `_GLOW`
//! textures' own bytes under their batches: `255` for the neon strips, `254`
//! for the `nuricom` banner, `92` for the lit panels overhead. Those batches
//! are cutouts drawn coplanar with the wall they light, on the `0x10`
//! reference, so they reach the frame only through the equal-depth test.

use std::collections::BTreeSet;

use oag_mesh::mesh;
use oag_mesh::mesh_render::cutout;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn talons_junction_stamps_the_values_the_original_s_mask_holds() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
    let name = r"Data\Environments\16_Track\track.vex";
    let blob = archives.read_name(name).expect("track.vex");
    let model = mesh::build(name, &blob).expect("build");

    let stamps = |draws: &[mesh::DrawCall]| -> BTreeSet<u8> {
        draws
            .iter()
            .flat_map(|draw| &model.indices[draw.range.start as usize..draw.range.end as usize])
            .map(|&i| (model.vertices[i as usize].glow * 255.0).round() as u8)
            .collect()
    };
    assert_eq!(
        stamps(&model.draws),
        BTreeSet::from([mesh::glow::BASE]),
        "every opaque batch stamps the base"
    );
    let cutouts = stamps(&model.alpha_tested_draws);
    for live in [255, 254, 92] {
        assert!(
            cutouts.contains(&live),
            "no cutout stamps {live}: {cutouts:?}"
        );
    }

    // Every glow cutout is on the decal reference, so it passes on equal
    // depth; without that it ties with its wall and never draws.
    let decals = model
        .alpha_tested_draws
        .iter()
        .filter(|draw| draw.alpha_test_ref == Some(cutout::DECAL_REFERENCE))
        .count();
    assert!(decals > 0, "no decal-reference cutouts on 16_Track");
}

/// Moa Therma's blended meshes come out in the order a recorded frame of the original
/// drew them: far to near by view depth, **not** in layer order.
///
/// The plane is the view of a recorded Moa frame (`psp-ge-dump.py`, 469 prims, GE
/// prims 413 to 425): `clip.w` is `-z_view`, with `z_view = m[2] x + m[5] y + m[8] z +
/// m[11]` over the frame's `{0.99893, -0.04363, -0.01516, ..., 405.703, -43.212,
/// 161.344}` matrix. The original drew the meshes of nodes 261, 267, 878, 258, 875, 293
/// and 294 in that order (`mesh-draw.md`, "The list-B queue key"), alternating layers
/// `0x45` and `0x4a`; a layer-only order puts every tree before the strip.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn moa_therma_blended_meshes_are_ordered_by_view_depth_as_the_original_drew_them() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
    let name = r"Data\Environments\03_Track\track.vex";
    let blob = archives.read_name(name).expect("track.vex");
    let model = mesh::build(name, &blob).expect("build");

    let plane = [0.015_162_944, -0.107_353_21, -0.994_094_85, -161.343_75];
    let order = model
        .blended_order(plane)
        .expect("a circuit's list-B meshes carry a depth point");
    let recorded = [261u32, 267, 878, 258, 875, 293, 294];
    let mut seen: Vec<u32> = Vec::new();
    for &index in &order {
        let Some(node) = model.transparent_draws[index as usize].node else {
            continue;
        };
        if recorded.contains(&node) && seen.last() != Some(&node) {
            seen.push(node);
        }
    }
    assert_eq!(
        seen, recorded,
        "the blended list is not in the original's view-depth order"
    );
    let moving = model
        .transparent_draws
        .iter()
        .filter(|d| d.depth_point.is_some() && d.moving)
        .count();
    let keyed = model
        .transparent_draws
        .iter()
        .filter(|d| d.depth_point.is_some())
        .count();
    println!("03_Track: {keyed} depth-keyed transparent draws, {moving} of them moving");
}
