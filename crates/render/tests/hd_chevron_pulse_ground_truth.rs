//! Wipeout HD's chevron pulse on Anulpha Pass, from the material record to the vertex.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run with `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_chevron_pulse_ground_truth::/)'
//! ```
//!
//! The program is on `docs/rendering/hd-track-tone.md`, "The chevron panel draws black":
//! `cf_chevron_pulse` samples five units, so its emask and its wave ride the fifth and sixth
//! material bindings while the normal map keeps the third, and the wave's offset is the
//! material's own curve, `fract(t / 3)`.

use crate::archive_cache;

use std::collections::BTreeSet;

use oag_mesh::mesh::{self, AnimTrack, PROGRAM_CHEVRON, slots};

const TRACK: &str = "/data/environments/15_anulpha_pass/track.vex";

fn build() -> Option<(mesh::Model, mesh::rcs::Report)> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let data = archive_cache::read(&spec, TRACK)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, TRACK, &data)?;
    Some(
        mesh::rcs::build_scene(TRACK, &data, &geometry, &mut |name| {
            archive_cache::read(&spec, name)
        })
        .expect("the scene builds"),
    )
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn anulphas_chevron_binds_its_emask_wave_normal_and_offset_curve() {
    let Some((model, report)) = build() else {
        return;
    };
    assert!(
        report.chevron_pulse_bound > 0 && report.chevron_pulse_unread == 0,
        "{} bound, {} unread",
        report.chevron_pulse_bound,
        report.chevron_pulse_unread
    );
    let label = |slots: &mesh::TextureSlots, slot: usize| {
        slots
            .get(slot)
            .and_then(Option::as_ref)
            .map(|t| t.label.to_ascii_lowercase())
    };
    let chevron: BTreeSet<usize> = model
        .vertices
        .iter()
        .map(|v| v.slots)
        .filter(|&w| w & slots::LIT_ALPHA != 0 && slots::material_index(w) > 0)
        .filter_map(|w| {
            let entry = model.emissive[slots::material_index(w) as usize - 1];
            (entry.program[0] == PROGRAM_CHEVRON).then_some(entry)
        })
        .map(|entry| {
            assert_eq!(entry.tint, [0.0, 1.0, 0.0], "the record's `Colour`");
            assert!(
                (entry.scale - 0.2).abs() < 1e-6,
                "the record's `k`, {}",
                entry.scale
            );
            let row = entry.offset as usize;
            let AnimTrack::Param(track) = &model.anim_tracks[row - 1] else {
                panic!("row {row} is not the offset curve");
            };
            for (t, want) in [(0.0, 0.0), (1.5, 0.5), (2.9, 0.9667), (3.1, 0.0333)] {
                let got = track.sample(t);
                assert!(
                    (got - want).abs() < 2e-3,
                    "offset({t}) = {got}, the curve is fract(t / 3)"
                );
            }
            row
        })
        .collect();
    assert!(!chevron.is_empty(), "no vertex reaches the chevron program");
    let slots_drawn: BTreeSet<usize> = model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
        .filter_map(|d| d.texture)
        .filter(|&slot| label(&model.fifth_maps, slot).is_some())
        .collect();
    // 441, 442, 474, 475, 480, 481, 483, 484: the floor's and the wall's chevrons, each
    // copy, every one through the same block.
    assert_eq!(slots_drawn.len(), 8, "drawn chevron slots {slots_drawn:?}");
    for slot in slots_drawn {
        let emask = label(&model.fifth_maps, slot).unwrap();
        assert!(emask.contains("chevron_emask"), "fifth: {emask}");
        let wave = label(&model.sixth_maps, slot).expect("the wave beside the emask");
        assert!(wave.contains("ds_wave_c"), "sixth: {wave}");
        let normal = label(&model.pad_masks, slot).expect("the normal map stays bound third");
        assert!(normal.contains("chevron_n"), "third: {normal}");
    }
}
