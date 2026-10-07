//! Wipeout HD's magstrip wave, from the material record to the vertex.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run with `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_mag_wave_ground_truth)'
//! ```
//!
//! The law is on `docs/formats/rcsmaterial.md`, "The magstrip floor scrolls a
//! wave texture off the engine clock". `hd_mag_wave_lit_path.rs` proves the
//! shader moves it with `scene.time`; this proves a real circuit reaches that
//! shader: the offscreen capture helper has no scene clock, so the build is
//! what a disc-backed test can pin.

mod archive_cache;

use std::collections::BTreeSet;

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

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn talons_junctions_strip_binds_its_wave_and_its_floor_loop() {
    let Some((model, report)) = build(
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
    ) else {
        return;
    };
    assert!(
        report.mag_wave_bound > 0 && report.mag_wave_unread == 0,
        "{} bound, {} unread",
        report.mag_wave_bound,
        report.mag_wave_unread
    );
    let words: BTreeSet<u32> = model
        .vertices
        .iter()
        .map(|v| v.slots)
        .filter(|w| w & slots::MAG_WAVE != 0)
        .collect();
    assert!(!words.is_empty(), "the wave bit reaches no vertex");
    for word in &words {
        assert_eq!(
            word & slots::ADD_SECOND,
            0,
            "a wave material's second texture is not an additive glow"
        );
        assert!(
            slots::material_index(*word) > 0,
            "a wave material carries its glow-table entry"
        );
    }
    assert!(
        words.iter().any(|w| w & slots::MAG_LOOP != 0),
        "the floor loop (`mag_effect_loop_opaque`) is not classified"
    );
    assert!(
        model.wave_maps.iter().any(Option::is_some) && model.pad_masks.iter().any(Option::is_some),
        "the emissive picture and the wave are bound beside each other"
    );
    // Every entry the wave materials index carries the authored rate 1.
    for word in &words {
        let entry = model.emissive[slots::material_index(*word) as usize - 1];
        assert_eq!(entry.rate, 1.0);
        assert!(entry.scale > 0.0);
    }
}
