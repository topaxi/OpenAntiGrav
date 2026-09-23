//! What Talon's Junction stamps into the bloom's glow mask, against the
//! original's own EDRAM read - see `docs/rendering/glow-mask.md`.
//!
//! Live, on the grid, the mask read `4` almost everywhere and the `_GLOW`
//! textures' own bytes under their batches: `255` for the neon strips, `254`
//! for the `nuricom` banner, `92` for the lit panels overhead. Those batches
//! are cutouts drawn coplanar with the wall they light, on the `0x10`
//! reference, so they reach the frame only through the equal-depth test.

use std::collections::BTreeSet;

use oag_render::mesh;
use oag_render::mesh_render::cutout;

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
