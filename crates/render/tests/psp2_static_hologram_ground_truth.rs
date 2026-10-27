//! Omega's Amphiseum hologram board draws HD's mode 7, minus HD's `/2`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! Omega's plain `dc_hologramwithstatic.rcsmaterial` carries HD's three pictures, rates and
//! hashes (`oag_mesh::mesh::rcs::psp2`, `static_hologram`). This builds Amphiseum's models
//! through the patch first (`data08` holds the live copy), and holds that every circuit model
//! that names the material reaches `PROGRAM_STATIC_HOLOGRAM` with the record's own rates and
//! the three pictures, unlit like HD's slot 637. Remove the builder's `static_hologram::plan`
//! call and the count reads zero.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{Model, PROGRAM_STATIC_HOLOGRAM, slots};

const ARCHIVES: [&str; 9] = [
    "ps4/omega-eu-patch/uroot/data08.psarc",
    "ps4/omega-eu-patch/uroot/data09.psarc",
    "ps4/omega-eu-patch/uroot/data07.psarc",
    "ps4/omega-eu-patch/uroot/data05.psarc",
    "ps4/omega-eu/uroot/data00.psarc",
    "ps4/omega-eu/uroot/data01.psarc",
    "ps4/omega-eu/uroot/data02.psarc",
    "ps4/omega-eu/uroot/data03.psarc",
    "ps4/omega-eu/uroot/data04.psarc",
];

fn label(slots: &oag_mesh::mesh::TextureSlots, i: usize) -> String {
    slots
        .get(i)
        .and_then(Option::as_ref)
        .map_or_else(String::new, |t| t.label.to_ascii_lowercase())
}

fn bound(model: &Model) -> Vec<(usize, [f32; 3], u32)> {
    let mut out = Vec::new();
    for draw in model
        .draws
        .iter()
        .chain(&model.transparent_draws)
        .chain(&model.alpha_tested_draws)
    {
        let Some(texture) = draw.texture else {
            continue;
        };
        let v = model.vertices[model.indices[draw.range.start as usize] as usize];
        let index = slots::material_index(v.slots) as usize;
        if index != 0 && model.emissive[index - 1].program[0] == PROGRAM_STATIC_HOLOGRAM {
            out.push((texture, model.emissive[index - 1].tint, v.slots));
        }
    }
    out.sort_by_key(|b| b.0);
    out.dedup_by_key(|b| b.0);
    out
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu and omega-eu-patch"]
fn amphiseums_board_binds_hds_mode_7_with_the_records_own_rates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    let mut archives: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    if archives.len() != ARCHIVES.len() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but Omega is not extracted"
        );
        println!("skipping: Omega not extracted");
        return;
    }
    let models: Vec<String> = archives
        .iter()
        .flat_map(|a| a.paths().to_vec())
        .filter(|p| {
            let l = p.to_ascii_lowercase();
            l.contains("amphiseum") && l.ends_with("track.final.rcsmodel")
        })
        .collect();
    assert!(!models.is_empty(), "no Amphiseum circuit model found");
    let mut drawn = 0;
    for path in models {
        let index = archives
            .iter()
            .position(|a| a.contains(&path))
            .expect("listed");
        let blob = archives[index].read_path(&path).expect("the model");
        let (model, report) = psp2::build(&path, &blob, None, &mut |p| {
            archives.iter_mut().find_map(|a| a.read_path(p).ok())
        })
        .expect("builds");
        assert_eq!(
            report.static_hologram_unread,
            0,
            "{path}: {}",
            report.describe()
        );
        let found = bound(&model);
        assert_eq!(
            found.len(),
            report.static_hologram_materials,
            "{path}: {}",
            report.describe()
        );
        for (texture, tint, word) in &found {
            // The record's `Speed`, `Speed2` and `Alpha`, read, not typed.
            assert_eq!(*tint, [1.0, 4.5384, 0.73802], "{path}");
            assert!(
                label(&model.textures, *texture).contains("dc_scanlines"),
                "{path}"
            );
            assert!(
                label(&model.lightmaps, *texture).contains("advert_a"),
                "{path}"
            );
            assert!(
                label(&model.pad_masks, *texture).contains("dc_gradient_horizontal"),
                "{path}"
            );
            assert_eq!(
                word & (slots::NO_AMBIENT | slots::NO_SUN),
                slots::NO_AMBIENT | slots::NO_SUN,
                "{path}"
            );
        }
        // HD's `SRC_ALPHA`/`ONE` pair, inherited by name (`lineage_blend`), on the blended list.
        for (texture, ..) in &found {
            let draw = model
                .transparent_draws
                .iter()
                .find(|d| d.texture == Some(*texture))
                .unwrap_or_else(|| panic!("{path}: the board is not on the blended list"));
            let state = draw.blend_state.expect("a blended draw states its blend");
            assert_eq!(
                (state.color.src_factor, state.color.dst_factor),
                (wgpu::BlendFactor::SrcAlpha, wgpu::BlendFactor::One),
                "{path}"
            );
        }
        drawn += found.len();
    }
    assert!(drawn > 0, "no Amphiseum model reaches the mode");
}
