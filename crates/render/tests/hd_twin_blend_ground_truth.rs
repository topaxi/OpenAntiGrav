//! HD's `nr_twinblend`, from the material record to the vertex.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run with `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_twin_blend_ground_truth::/)'
//! ```
//!
//! The program is on `docs/formats/rcsmaterial.md`, "`nr_twinblend`: two pictures, a
//! scrolling wipe and bars": the first and second pictures cross-faded by `(1 - wipe)^4`,
//! the wipe scrolled by the vertex block's literal times the clock, the product striped by
//! the bars. `hd_twin_blend_lit_path.rs` checks the shader's combine; this checks every
//! shipped copy reaches it with its four pictures, its rate and its three coordinate pairs.

use crate::archive_cache;

use oag_mesh::mesh::{self, PROGRAM_TWIN_BLEND, slots};

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

/// The slots bound to the mode, each with its four pictures' labels and whether the bars'
/// pair differs from the pictures' on some vertex (Chenghou authors the two identical, Tech de
/// Ra does not).
fn bound(model: &mesh::Model) -> Vec<(usize, [String; 4], bool)> {
    let label = |slots: &mesh::TextureSlots, slot: usize| {
        slots
            .get(slot)
            .and_then(Option::as_ref)
            .map_or_else(String::new, |t| t.label.to_ascii_lowercase())
    };
    let mut out: Vec<(usize, [String; 4], bool)> = Vec::new();
    for draw in &model.draws {
        let Some(slot) = draw.texture else { continue };
        let range = draw.range.start as usize..draw.range.end as usize;
        let vertices: Vec<&mesh::GpuVertex> = model.indices[range]
            .iter()
            .map(|&i| &model.vertices[i as usize])
            .collect();
        let twin = vertices.iter().any(|v| {
            let index = slots::material_index(v.slots) as usize;
            index != 0 && model.emissive[index - 1].program[0] == PROGRAM_TWIN_BLEND
        });
        if !twin || out.iter().any(|(s, ..)| *s == slot) {
            continue;
        }
        assert_eq!(
            model.emissive[slots::material_index(vertices[0].slots) as usize - 1].tint[0],
            1.875,
            "slot {slot}: the wipe's rate, the vertex block's literal"
        );
        // 2048 and Omega inherit this number rather than read it: it must stay HD's.
        assert_eq!(
            mesh::rcs::psp2::TWIN_BLEND_RATE,
            model.emissive[slots::material_index(vertices[0].slots) as usize - 1].tint[0],
            "slot {slot}: the psp2 builder's inherited rate"
        );
        assert!(
            vertices.iter().all(|v| v.slots & slots::ADD_SECOND == 0),
            "slot {slot}: the generic additive second texture is cleared"
        );
        // The wipe's pair is its own: `route` falls back silently.
        assert!(vertices.iter().any(|v| v.lightmap_texcoord != v.texcoord));
        assert!(vertices.iter().any(|v| v.texcoord2 != v.lightmap_texcoord));
        out.push((
            slot,
            [
                label(&model.textures, slot),
                label(&model.lightmaps, slot),
                label(&model.pad_masks, slot),
                label(&model.wave_maps, slot),
            ],
            vertices.iter().any(|v| v.texcoord2 != v.texcoord),
        ));
    }
    out.sort();
    out
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn tech_de_ras_banner_behind_the_gantry_cross_fades_the_two_ag_squares() {
    let Some((model, report)) = build("DATA00.PSARC", "/data/environments/tech_de_ra/track.vex")
    else {
        return;
    };
    assert_eq!(
        (
            report.twin_blend_bound,
            report.twin_blend_other_shape,
            report.twin_blend_unread
        ),
        (1, 0, 0)
    );
    let slots = bound(&model);
    assert_eq!(slots.len(), 1, "{slots:?}");
    let (slot, [first, second, wipe, bars], bars_pair) = &slots[0];
    assert_eq!(*slot, 133);
    assert!(
        *bars_pair,
        "the bars' pair, v[8].xy, is not the pictures' v[2]"
    );
    assert!(first.contains("ag_square01cl_glow"), "{first}");
    assert!(second.contains("ag_square02cl_glow"), "{second}");
    assert!(wipe.contains("animscaledballs"), "{wipe}");
    assert!(bars.contains("glowbars_noisy"), "{bars}");
}

/// The `DATA02` circuits' copy runs the same law with its registers renumbered (the `/2` on
/// the fog's `MAD` before `MOV H0.w`, the scroll into `o[TC0].w`); Chenghou carries two slots.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn chenghous_two_slots_bind_through_the_renumbered_copy() {
    let Some((model, report)) = build(
        "DATA02.PSARC",
        "/data/environments/04_chenghou_project/track.vex",
    ) else {
        return;
    };
    assert_eq!(
        (
            report.twin_blend_bound,
            report.twin_blend_other_shape,
            report.twin_blend_unread
        ),
        (2, 0, 0)
    );
    let slots: Vec<usize> = bound(&model).into_iter().map(|(s, ..)| s).collect();
    assert_eq!(slots, [485, 489]);
}
