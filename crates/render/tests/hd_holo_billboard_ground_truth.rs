//! HD's holographic billboard, from the material record to the vertex.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run with `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_holo_billboard_ground_truth)'
//! ```
//!
//! `hd_holo_billboard_lit_path.rs` proves the shader's combine; this proves
//! Amphiseum's eight inline-literal slots reach it with their three textures,
//! their two clock rates and the two coordinate pairs the program reads, and
//! that the parameter-driven shape (`wes_`, Modesto Heights' `nr_`) stays on
//! the generic reading. `crates/render/examples/hd_holo_billboard_census.rs`
//! lists every slot.

mod archive_cache;

use oag_mesh::mesh::{self, slots};

fn build(archive: &str, path: &str) -> Option<(mesh::Model, mesh::rcs::Report)> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let data = archive_cache::read(&spec, path)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    Some(
        mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
            archive_cache::read(&spec, name)
        })
        .expect("the scene builds"),
    )
}

/// Amphiseum's nr_ billboards and the two `wes_` ones, by material slot.
const BOUND: [usize; 8] = [590, 613, 614, 616, 621, 622, 625, 630];
const WES: [usize; 2] = [618, 632];

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn amphiseums_inline_literal_billboards_bind_three_textures_and_both_rates() {
    let Some((model, report)) = build("DATA00.PSARC", "/data/environments/amphiseum/track.vex")
    else {
        return;
    };
    assert_eq!(
        report.holo_billboard_bound,
        BOUND.len(),
        "{}",
        report.describe()
    );
    assert_eq!(
        report.holo_billboard_other_shape,
        WES.len(),
        "{}",
        report.describe()
    );
    assert_eq!(report.holo_billboard_unread, 0, "{}", report.describe());

    let mut indices = Vec::new();
    for slot in 0..model.material_slots.len() {
        let word = model.material_slots[slot];
        if word & slots::HOLO_BILLBOARD == 0 {
            continue;
        }
        indices.push(slot);
        assert_eq!(word & slots::ADD_SECOND, 0, "slot {slot}: not a plain glow");
        let entry = model.emissive[slots::material_index(word) as usize - 1];
        assert_eq!(
            entry.tint,
            [0.25, 5.625, 0.0],
            "slot {slot}: the program's own literals"
        );
        for (name, bound) in [
            ("advert", model.textures.get(slot)),
            ("scanlines", model.lightmaps.get(slot)),
            ("layer", model.pad_masks.get(slot)),
        ] {
            assert!(
                bound.is_some_and(Option::is_some),
                "slot {slot}: the {name} texture is not bound"
            );
        }
    }
    assert_eq!(indices, BOUND);
    for slot in WES {
        let word = model.material_slots[slot];
        assert_eq!(
            word & slots::HOLO_BILLBOARD,
            0,
            "slot {slot}: parameter-driven, refused"
        );
        assert_ne!(
            word & slots::ADD_SECOND,
            0,
            "slot {slot}: keeps the generic glow"
        );
    }

    // The two extra coordinate pairs reach the vertex, and differ from the
    // advert's own: three distinct pairs, not one pair three times.
    let flagged: Vec<&mesh::GpuVertex> = model
        .vertices
        .iter()
        .filter(|v| v.slots & slots::HOLO_BILLBOARD != 0)
        .collect();
    assert!(!flagged.is_empty());
    let carried = |pair: fn(&mesh::GpuVertex) -> [f32; 2]| {
        flagged.iter().filter(|v| pair(v) != [0.0; 2]).count() * 10 >= flagged.len() * 9
    };
    assert!(carried(|v| v.lightmap_texcoord), "the second layer's pair");
    assert!(carried(|v| v.texcoord2), "the scanlines' pair");
    assert!(flagged.iter().any(|v| v.texcoord2 != v.texcoord));
    assert!(flagged.iter().any(|v| v.lightmap_texcoord != v.texcoord));
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn modesto_heights_parameter_driven_billboards_stay_on_the_generic_reading() {
    let Some((model, report)) = build(
        "DATA00.PSARC",
        "/data/environments/modesto_heights/track.vex",
    ) else {
        return;
    };
    assert_eq!(report.holo_billboard_bound, 0, "{}", report.describe());
    assert!(
        report.holo_billboard_other_shape >= 17,
        "{}",
        report.describe()
    );
    assert!(
        model
            .vertices
            .iter()
            .all(|v| v.slots & slots::HOLO_BILLBOARD == 0)
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_dlc_copys_fence_predicated_shape_stays_on_the_generic_reading() {
    let Some((model, report)) = build("DATA02.PSARC", "/data/environments/02_track/track.vex")
    else {
        return;
    };
    assert_eq!(report.holo_billboard_bound, 0, "{}", report.describe());
    assert_eq!(
        report.holo_billboard_other_shape,
        5,
        "{}",
        report.describe()
    );
    assert!(
        model
            .vertices
            .iter()
            .all(|v| v.slots & slots::HOLO_BILLBOARD == 0)
    );
}
