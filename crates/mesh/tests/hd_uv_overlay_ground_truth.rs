//! HD's `uvdistortion_*plusoverlay*` banner on Modesto Heights, from the material record to
//! the vertex.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. Run with `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh --run-ignored all \
//!     -E 'test(/^hd_uv_overlay_ground_truth::/)'
//! ```
//!
//! The program is on `docs/formats/rcsmaterial.md`, "Modesto Heights: the banner band is drawn white":
//! the grille is the lit albedo and the Ossego logo enters only through an overlay of the teal
//! cloud term. Binding the logo as the albedo drew the band white where the original draws it
//! dark; this checks each shipped copy binds the grille and carries the record's own values.

use crate::archive_cache;

use oag_mesh::mesh::{self, PROGRAM_UV_OVERLAY, slots};

fn build(path: &str) -> Option<(mesh::Model, mesh::rcs::Report)> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let data = archive_cache::read(&spec, path)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    Some(
        mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
            archive_cache::read(&spec, name)
        })
        .expect("the scene builds"),
    )
}

/// Every drawn slot whose vertices name an entry of the mode: the slot, its three pictures'
/// labels (albedo, second, third) and the two entries.
type Bound = (usize, [String; 3], [mesh::Emissive; 2]);

fn bound(model: &mesh::Model) -> Vec<Bound> {
    let label = |slots: &mesh::TextureSlots, slot: usize| {
        slots
            .get(slot)
            .and_then(Option::as_ref)
            .map_or_else(String::new, |t| t.label.to_ascii_lowercase())
    };
    let mut out: Vec<Bound> = Vec::new();
    for draw in &model.draws {
        let Some(slot) = draw.texture else { continue };
        let range = draw.range.start as usize..draw.range.end as usize;
        for &i in &model.indices[range] {
            let v = &model.vertices[i as usize];
            let index = slots::material_index(v.slots) as usize;
            if index == 0
                || model.emissive[index - 1].program[0] != PROGRAM_UV_OVERLAY
                || out.iter().any(|(s, ..)| *s == slot)
            {
                continue;
            }
            assert_eq!(
                v.slots & slots::ADD_SECOND,
                0,
                "slot {slot}: the generic additive second texture is cleared"
            );
            assert_eq!(
                model.material_colour_factor.get(slot),
                Some(&true),
                "slot {slot}: VertexColour1 multiplies the light, as the program's @0x35 does"
            );
            out.push((
                slot,
                [
                    label(&model.textures, slot),
                    label(&model.lightmaps, slot),
                    label(&model.pad_masks, slot),
                ],
                [model.emissive[index - 1], model.emissive[index]],
            ));
        }
    }
    out.sort_by_key(|b| b.0);
    out
}

fn assert_banner(banner: &Bound, slot: usize) {
    let (found, [grille, clouds, logo], [first, second]) = banner;
    assert_eq!(*found, slot);
    assert!(grille.contains("and_tunnelscreens_small"), "{grille}");
    assert!(clouds.contains("and_line_rgb_clouds_alpha"), "{clouds}");
    assert!(logo.contains("and_ossego"), "{logo}");
    // `961662ae` and `d0989794`, then `c0594d0e`, `2bdce348`, `fe619466` and `96e30da5`.
    assert_eq!(first.tint, [0.1438845, 0.52450675, 0.80834]);
    assert_eq!(first.offset, 0.14348);
    assert_eq!(second.tint, [7.174, -0.05, 1.0]);
    assert_eq!(second.offset, -0.25);
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn modesto_reversed_slot_550_binds_the_grille_and_overlays_the_logo() {
    let Some((model, report)) = build("/data/environments/modesto_heights/track_reversed.vex")
    else {
        return;
    };
    assert_eq!(
        (
            report.uv_overlay_bound,
            report.uv_overlay_other_shape,
            report.uv_overlay_unread
        ),
        (2, 0, 0)
    );
    let banners = bound(&model);
    // Slot 96 is the `bloom` copy; the `uvdistortion_diff_vertexcol` slots (464, 467, 472, 486,
    // 527, 538) have no logo unit and keep the generic reading.
    let slots: Vec<usize> = banners.iter().map(|b| b.0).collect();
    assert_eq!(slots, [96, 550]);
    assert_banner(&banners[1], 550);
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn modesto_forward_slot_551_binds_the_same_banner() {
    let Some((model, _)) = build("/data/environments/modesto_heights/track.vex") else {
        return;
    };
    let banners = bound(&model);
    let banner = banners.iter().find(|b| b.0 == 551).expect("slot 551 binds");
    assert_banner(banner, 551);
}
