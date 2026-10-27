//! What the circuits' road and glass read off the disc, from the material record to the vertex
//! (`hd-track-surface`, 2026-10-21).
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not ship.
//! Run with `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_track_surface_ground_truth::/)'
//! ```
//!
//! The laws are on `docs/formats/rcsmaterial.md`, "The track's road and the lightmapped glass".

use crate::archive_cache;

use oag_mesh::mesh::{self, slots};

fn build(archive: &str, path: &str) -> Option<mesh::Model> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let data = archive_cache::read(&spec, path)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    Some(
        mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
            archive_cache::read(&spec, name)
        })
        .expect("the scene builds")
        .0,
    )
}

/// Tech de Ra's glass has two live fragment programs (`0x76fac0`, the vertex-colour light, and
/// `0x7704c0`, the baked atlas's): the second declares `lightmap` beside the grid, the ramp and the
/// probe, and used to draw as an ordinary lightmapped surface with the ramp as its picture (a blown
/// white stripe). It is the same combine, with the grid bound third.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn tech_de_ras_lightmapped_glass_is_the_sheen_with_its_grid_bound_third() {
    let Some(model) = build("DATA00.PSARC", "/data/environments/tech_de_ra/track.vex") else {
        return;
    };
    let lightmapped: Vec<usize> = model
        .material_slots
        .iter()
        .enumerate()
        .filter(|(_, w)| {
            **w & slots::FACING_RAMP_SHEEN != 0 && **w & slots::SECOND_IS_LIGHTMAP != 0
        })
        .map(|(slot, _)| slot)
        .collect();
    assert!(
        !lightmapped.is_empty(),
        "no glass material is the lightmapped variant of the sheen"
    );
    for slot in &lightmapped {
        assert!(
            model.pad_masks.get(*slot).is_some_and(Option::is_some),
            "slot {slot}: the sheen's grid is not bound third"
        );
    }
    let unlit_by_atlas = model
        .material_slots
        .iter()
        .filter(|w| **w & slots::FACING_RAMP_SHEEN != 0 && **w & slots::SECOND_IS_LIGHTMAP == 0)
        .count();
    assert!(
        unlit_by_atlas > 0,
        "the variant with no atlas (slots 436, 449, 463) stopped being a sheen"
    );
}

/// Every glass-sheen slot carries the block's own `c` (`ADD H4.xyz, H2, {0x512f8e65}`, 0.26562 on
/// `etched_glass_tech`) as its glow entry's `offset`. Tech de Ra's road (slot 449, with 436 and 463)
/// ran with `c = 0` until 2026-10-21: `emissive` claimed the slot as an additive glow (the program's
/// unit 1 accumulates, but that unit is the reflection probe inside the sheen's own combine) and
/// `glass_sheen::bias` leaves a slot that already has an entry alone. Checked on every circuit the
/// census names (`hd_glass_sheen_census`): Talon's Junction, Tech de Ra, Modesto Heights, The Amphiseum.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_glass_sheen_slot_carries_its_c() {
    // Each material's own `0x512f8e65`, read off its record (`hd_glass_sheen_census --params`):
    // Modesto Heights' `etched_glass` authors 0, The Amphiseum's `etched_glass_amph` 0.22916.
    for (track, c) in [
        ("/data/environments/talons_junction/track.vex", 0.26562),
        ("/data/environments/tech_de_ra/track.vex", 0.26562),
        ("/data/environments/modesto_heights/track.vex", 0.0),
        ("/data/environments/amphiseum/track.vex", 0.22916),
    ] {
        let Some(model) = build("DATA00.PSARC", track) else {
            return;
        };
        let sheen: Vec<usize> = model
            .material_slots
            .iter()
            .enumerate()
            .filter(|(_, w)| **w & slots::FACING_RAMP_SHEEN != 0)
            .map(|(slot, _)| slot)
            .collect();
        assert!(!sheen.is_empty(), "{track}: no glass-sheen slot");
        for slot in sheen {
            let word = model.material_slots[slot];
            let index = slots::material_index(word) as usize;
            assert!(
                index > 0,
                "{track} slot {slot}: no glow entry carries its c"
            );
            let entry = &model.emissive[index - 1];
            assert!(
                (entry.offset - c).abs() < 1e-4 && entry.tint == [0.0; 3],
                "{track} slot {slot}: entry {entry:?} is not the sheen's c ({c}, tint 0)"
            );
            assert!(
                word & slots::ADD_SECOND == 0,
                "{track} slot {slot}: the sheen is still claimed as an additive glow"
            );
        }
    }
}

/// A circuit's road and walls read a tangent-space normal map on unit 1 (`ds_floor_n_rh`,
/// `ds_wall_n`) whose frame the vertex program derives from the chunk's `tangent` attribute. They
/// are world-pass chunks, which passed no tangent to the vertex until this lane.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn modesto_heights_road_carries_its_authored_tangent() {
    let Some(model) = build(
        "DATA00.PSARC",
        "/data/environments/modesto_heights/track.vex",
    ) else {
        return;
    };
    assert!(
        model.material_tangent.iter().any(|marked| marked.carried()),
        "no circuit material reads a normal map"
    );
    let with_tangent = model.vertices.iter().filter(|v| v.tangent != 0).count();
    assert!(
        with_tangent > 10_000,
        "only {with_tangent} vertices carry a tangent: the road's chunks are not read for one"
    );
}
