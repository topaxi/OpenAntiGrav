//! Wipeout HD's snow-capped rock on Sebenco Climb, from the material record to the vertex.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run with `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_snow_rock_ground_truth::/)'
//! ```
//!
//! The program is on `docs/formats/rcsmaterial.md`, "Sebenco's snow-capped rocks":
//! `sebenco_snow` samples six units, the snow as the albedo, the mask third (at the second
//! coordinate pair), the snow's normal fourth, the rock fifth and the rock's normal sixth,
//! and its constants ride three consecutive glow-table entries.

use crate::archive_cache;

use std::collections::BTreeSet;

use oag_mesh::mesh::{self, PROGRAM_SNOW, slots};

const TRACK: &str = "/data/environments/10_sebenco_climb/track.vex";

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
fn sebencos_rock_binds_its_snow_mask_rock_and_both_normals() {
    let Some((model, report)) = build() else {
        return;
    };
    assert!(
        report.snow_rock_bound > 0 && report.snow_rock_unread == 0,
        "{} bound, {} unread",
        report.snow_rock_bound,
        report.snow_rock_unread
    );
    let label = |slots: &mesh::TextureSlots, slot: usize| {
        slots
            .get(slot)
            .and_then(Option::as_ref)
            .map(|t| t.label.to_ascii_lowercase())
    };
    // Every vertex a snow slot draws carries its tangent and the entry; the mask's pair
    // reaches them.
    let mut entries = BTreeSet::new();
    let mut second_pair = false;
    let mut drawn = BTreeSet::new();
    for draw in &model.draws {
        let Some(slot) = draw.texture else { continue };
        for &i in &model.indices[draw.range.start as usize..draw.range.end as usize] {
            let v = &model.vertices[i as usize];
            let index = slots::material_index(v.slots) as usize;
            if index == 0 || model.emissive[index - 1].program[0] != PROGRAM_SNOW {
                continue;
            }
            drawn.insert(slot);
            entries.insert(index - 1);
            assert_ne!(v.tangent, 0, "slot {slot}: a snow vertex with no tangent");
            second_pair |= v.texcoord2 != [0.0, 0.0];
        }
    }
    // 157 and 290 take the vertex-lit variant (block #5), the other ten the lightmapped one.
    assert_eq!(drawn.len(), 12, "drawn snow slots {drawn:?}");
    assert!(
        second_pair,
        "the mask's coordinate pair reaches no snow vertex"
    );
    for first in entries {
        let [lead, snow, rock] = [0, 1, 2].map(|i| model.emissive[first + i]);
        assert!(
            (lead.tint[0] - 0.663).abs() < 1e-3 && (lead.tint[2] - 1.0).abs() < 1e-3,
            "pale {:?}",
            lead.tint
        );
        assert!(
            (lead.offset - 59.63).abs() < 0.01,
            "snow exponent {}",
            lead.offset
        );
        assert!(
            lead.program[1] > 0.0 && lead.program[1] < 0.03,
            "rock exponent {}",
            lead.program[1]
        );
        assert_eq!(lead.scale, 1.0, "the rock's coordinate scale");
        assert!(
            (snow.tint[0] - 0.152).abs() < 1e-3,
            "snow specular {:?}",
            snow.tint
        );
        assert!(
            (rock.tint[0] - 0.127).abs() < 1e-3,
            "rock specular {:?}",
            rock.tint
        );
    }
    for slot in drawn {
        let albedo = label(&model.textures, slot).expect("the snow");
        assert!(albedo.contains("and_snow3alpha"), "albedo: {albedo}");
        for (what, slots) in [
            ("mask", &model.pad_masks),
            ("snow normal", &model.wave_maps),
            ("rock", &model.fifth_maps),
            ("rock normal", &model.sixth_maps),
        ] {
            assert!(label(slots, slot).is_some(), "slot {slot}: no {what}");
        }
    }
}
