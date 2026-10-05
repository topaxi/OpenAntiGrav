//! HD's light cone, from the material record to the vertex.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run with `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_light_cone_ground_truth)'
//! ```
//!
//! `hd_light_cone_lit_path.rs` proves the shader's combine; this proves the
//! two circuits that author `dc_lightcone` reach it with the values their own
//! records carry (`crates/render/examples/hd_light_cone_census.rs`).

use oag_mesh::mesh::{self, slots};

fn build(archive: &str, path: &str) -> Option<(mesh::Model, mesh::rcs::Report)> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let data = mesh::read_blob(&spec, path).ok()?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    Some(
        mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
            mesh::read_blob(&spec, name).ok()
        })
        .expect("the scene builds"),
    )
}

/// The one glow-table entry the cone vertices of `model` index, with how many
/// vertices carry the bit.
fn cone_entry(model: &mesh::Model) -> (mesh::Emissive, usize) {
    let words: Vec<u32> = model
        .vertices
        .iter()
        .map(|v| v.slots)
        .filter(|w| w & slots::LIGHT_CONE != 0)
        .collect();
    assert!(!words.is_empty(), "the cone bit reaches no vertex");
    let index = slots::material_index(words[0]) as usize;
    assert!(index > 0, "a cone material carries its glow-table entry");
    for w in &words {
        assert_eq!(slots::material_index(*w) as usize, index);
        assert_eq!(w & slots::ADD_SECOND, 0, "its second texture is the noise");
    }
    (model.emissive[index - 1], words.len())
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn talons_junctions_cone_carries_k_100_and_a_gated_ramp() {
    let Some((model, report)) = build(
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
    ) else {
        return;
    };
    assert_eq!(report.light_cone_bound, 1, "{}", report.describe());
    let (entry, vertices) = cone_entry(&model);
    assert_eq!(entry.tint, [100.0, 100.0, 100.0]);
    assert_eq!(entry.scale, 1.0);
    assert_eq!(
        entry.rate, 1.0,
        "the ramp tap is predicated on this variant"
    );
    assert!(vertices > 0);
    // The cone's own vertex normal is the constant (0, 0, 1) the predicate
    // keys on: the gate is only meaningful while that holds.
    assert!(
        model
            .vertices
            .iter()
            .filter(|v| v.slots & slots::LIGHT_CONE != 0)
            .all(|v| v.normal[1] == 0.0)
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn amphiseums_cone_has_no_k_and_an_ungated_ramp() {
    let Some((model, report)) = build("DATA00.PSARC", "/data/environments/amphiseum/track.vex")
    else {
        return;
    };
    assert_eq!(report.light_cone_bound, 1, "{}", report.describe());
    let (entry, _) = cone_entry(&model);
    assert_eq!(entry.tint, [1.0, 1.0, 1.0], "no K multiply, so one");
    assert!((entry.scale - 0.49596).abs() < 1e-4, "{}", entry.scale);
    assert_eq!(entry.rate, 0.0, "the ramp tap is unconditional here");
}
