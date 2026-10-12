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
//! every shape of the material reaches it with its three textures, its three
//! numbers (drift, signed scroll, scale) and the two coordinate pairs the
//! program reads: Amphiseum's inline-literal slots, the parameter-driven
//! `wes_` and Modesto Heights' `nr_`, and the DLC copy's fence-predicated one.
//! `scanlinebillboard` is another program and stays refused.
//! `crates/render/examples/hd_holo_billboard_census.rs` lists every slot.

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

/// The material parameters of a slot, read from the model's own record rather
/// than from the loader: `(drift, scroll, scale)`.
fn parameters(archive: &str, path: &str, slot: usize) -> Option<[f32; 3]> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let data = archive_cache::read(&spec, path)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    let source = oag_rcs::rcsmodel::Model::parse(&geometry).ok()?;
    let find = |hash: u32| {
        source.materials[slot]
            .parameters
            .iter()
            .find(|p| p.hash == hash)
            .map(|p| p.value[0])
    };
    Some([find(0x4f2a_ef81)?, find(0x088b_2c6a)?, find(0xef6e_4697)?])
}

/// The slots of `model` carrying the role bit, with their glow-table tint.
fn bound(model: &mesh::Model) -> Vec<(usize, [f32; 3])> {
    (0..model.material_slots.len())
        .filter(|&slot| model.material_slots[slot] & slots::HOLO_BILLBOARD != 0)
        .map(|slot| {
            let word = model.material_slots[slot];
            assert_eq!(word & slots::ADD_SECOND, 0, "slot {slot}: not a plain glow");
            for (name, texture) in [
                ("advert", model.textures.get(slot)),
                ("scanlines", model.lightmaps.get(slot)),
                ("layer", model.pad_masks.get(slot)),
            ] {
                assert!(
                    texture.is_some_and(Option::is_some),
                    "slot {slot}: the {name} texture is not bound"
                );
            }
            (
                slot,
                model.emissive[slots::material_index(word) as usize - 1].tint,
            )
        })
        .collect()
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn amphiseums_billboards_bind_inline_literals_and_parameters() {
    let path = "/data/environments/amphiseum/track.vex";
    let Some((model, report)) = build("DATA00.PSARC", path) else {
        return;
    };
    assert_eq!(
        report.holo_billboard_bound,
        BOUND.len() + WES.len(),
        "{}",
        report.describe()
    );
    assert_eq!(
        report.holo_billboard_other_shape,
        0,
        "{}",
        report.describe()
    );
    assert_eq!(report.holo_billboard_unread, 0, "{}", report.describe());

    let bound = bound(&model);
    let slots_bound: Vec<usize> = bound.iter().map(|b| b.0).collect();
    let mut want: Vec<usize> = BOUND.iter().chain(&WES).copied().collect();
    want.sort_unstable();
    assert_eq!(slots_bound, want);
    for (slot, tint) in bound {
        if WES.contains(&slot) {
            let read = parameters("DATA00.PSARC", path, slot).expect("the wes_ parameters");
            assert_eq!(tint, read, "slot {slot}: the material's own parameters");
        } else {
            assert_eq!(
                tint,
                [0.25, -5.625, 1.0],
                "slot {slot}: the program's own literals, scroll negated"
            );
        }
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
fn modesto_heights_parameter_driven_billboards_read_their_own_parameters() {
    let path = "/data/environments/modesto_heights/track.vex";
    let Some((model, report)) = build("DATA00.PSARC", path) else {
        return;
    };
    // 21 `nr_` slots and the 2 `wes_` ones.
    assert_eq!(report.holo_billboard_bound, 23, "{}", report.describe());
    assert_eq!(
        report.holo_billboard_other_shape,
        0,
        "{}",
        report.describe()
    );
    let bound = bound(&model);
    assert_eq!(bound.len(), 23);
    for (slot, tint) in &bound {
        let read = parameters("DATA00.PSARC", path, *slot).expect("the parameters");
        assert_eq!(*tint, read, "slot {slot}");
    }
    // Not one shared value: the slots author different rates and scales.
    assert!(bound.iter().any(|b| b.1 != bound[0].1));
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_dlc_copys_fence_predicated_shape_binds_with_its_inline_literals() {
    let Some((model, report)) = build("DATA02.PSARC", "/data/environments/02_track/track.vex")
    else {
        return;
    };
    assert_eq!(report.holo_billboard_bound, 5, "{}", report.describe());
    assert_eq!(
        report.holo_billboard_other_shape,
        0,
        "{}",
        report.describe()
    );
    let bound = bound(&model);
    assert_eq!(bound.len(), 5);
    for (slot, tint) in bound {
        assert_eq!(tint, [0.25, -5.625, 1.0], "slot {slot}");
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn scanlinebillboard_is_another_program_and_stays_refused() {
    let Some((model, report)) = build(
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
    ) else {
        return;
    };
    // The one `wes_` slot binds; the eight `scanlinebillboard` slots do not.
    assert_eq!(report.holo_billboard_bound, 1, "{}", report.describe());
    assert_eq!(
        report.holo_billboard_other_shape,
        8,
        "{}",
        report.describe()
    );
    assert_eq!(bound(&model).len(), 1);
}
