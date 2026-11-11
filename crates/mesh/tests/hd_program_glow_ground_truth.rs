//! Wipeout HD's glow mask is what each material's fragment program writes, read
//! off the disc's own microcode.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh --run-ignored all \
//!     -E 'test(/^hd_program_glow_ground_truth::/)'
//! ```
//!
//! The reference is the original running: a live RPCS3 frame of Metropia's
//! start (2026-10-21) has its scene target's alpha at **255 on the neon sign's
//! pixels, 122 on the sky and 0 on the lit road and walls**, and the programs
//! behind those pixels are `cf_constantcolourglow` (`MOV H0.w, 1`),
//! the sky draw (`COL0.w`) and `lambertzeroalpha`/`diffuse_specular`
//! (`MOV H0.w, 0`). The loader carries each material's value per slot in
//! `Model::material_glow`; dropping the reading leaves every slot at 0.

use std::path::{Path, PathBuf};

use oag_mesh::mesh;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// Metropia's scene, its loader report and the `.rcsmodel`'s own material table.
fn build(image: &Path) -> Option<(mesh::Model, mesh::rcs::Report, oag_rcs::rcsmodel::Model)> {
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let path = "/data/environments/02_track/track.vex";
    let data = mesh::read_blob(&spec, path).ok()?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    let (model, report) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        mesh::read_blob(&spec, name).ok()
    })
    .expect("the scene builds");
    let source = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the model parses");
    Some((model, report, source))
}

/// The neon rings of the stadium behind the start line: `dn_diffuse_specular_emiss`
/// slots painting `mar_col_rim3.gtf`, whose alpha is the neon strip. Their
/// programs add `picture.a * 0x8f3d0540` after the light
/// (`docs/formats/rcsmaterial.md`, "The picture's alpha times a colour"), and
/// the model authors `(0.821, 0.856, 0.993)` for it. The original's rings read
/// near white; without the term ours drew the picture's own dim cyan.
fn the_neon_rings_glow_by_their_pictures_alpha(
    model: &mesh::Model,
    report: &mesh::rcs::Report,
    source: &oag_rcs::rcsmodel::Model,
) {
    use mesh::slots;
    let rings: Vec<usize> = source
        .materials
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            m.name.ends_with("/dn_diffuse_specular_emiss.rcsmaterial")
                && m.samplers.iter().any(|(_, p)| {
                    p.as_deref()
                        .is_some_and(|p| p.ends_with("/mar_col_rim3.gtf"))
                })
        })
        .map(|(slot, _)| slot)
        .collect();
    assert_eq!(rings.len(), 9, "the ring slots: {rings:?}");
    for &slot in &rings {
        let word = model.material_slots[slot];
        let index = slots::material_index(word) as usize;
        let entry = index
            .checked_sub(1)
            .and_then(|i| model.emissive.get(i))
            .unwrap_or_else(|| panic!("ring slot {slot} names no glow entry"));
        assert_eq!(
            entry.program,
            [mesh::PROGRAM_PICTURE_ALPHA_EMISSIVE, 0.0],
            "ring slot {slot} is not read as a picture-alpha emissive"
        );
        for (got, want) in entry.tint.iter().zip([0.821_211, 0.856_112_8, 0.993]) {
            assert!(
                (got - want).abs() < 1e-5,
                "ring slot {slot} carries {:?}, not the authored colour",
                entry.tint
            );
        }
        // The program's unit-1 accumulate is its specular times the specular
        // map, not a glow: the generic additive reading must not survive.
        assert_eq!(
            word & slots::ADD_SECOND,
            0,
            "ring slot {slot} still adds its specular map as a glow"
        );
    }
    // 17 `dn_diffuse_specular_emiss` and 13 `diffuse_emissive_alpha_glow` slots.
    assert_eq!(report.alpha_emissive_bound, 30);
    assert_eq!(report.alpha_emissive_unread, 0);
}

/// The tower behind the start line, the disc's one `pb_glass_2nduv_reflect_specular1_glow`
/// slot (`docs/formats/rcsmaterial.md`, "The window with a specular map"): its
/// picture is `wes_windows_d`, the strip at entry 0 is the glow mask, and the
/// specular is multiplied by `pb_wes_windows_d_s`. Read generically it painted
/// the white strip as the facade and added the specular map as a glow.
fn the_tower_binds_its_picture_its_glow_mask_and_its_specular_map(
    model: &mesh::Model,
    source: &oag_rcs::rcsmodel::Model,
) {
    use mesh::slots;
    let towers: Vec<usize> = source
        .materials
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            m.name
                .ends_with("/pb_glass_2nduv_reflect_specular1_glow.rcsmaterial")
        })
        .map(|(slot, _)| slot)
        .collect();
    assert_eq!(towers.len(), 1, "the tower's slot: {towers:?}");
    let slot = towers[0];
    let label = |table: &mesh::TextureSlots| {
        table[slot]
            .as_ref()
            .map(|t| t.label.clone())
            .unwrap_or_default()
    };
    assert!(
        label(&model.textures).ends_with("/wes_windows_d.gtf"),
        "the picture is {}",
        label(&model.textures)
    );
    assert!(
        label(&model.lightmaps).ends_with("/pb_wes_windows_d_s.gtf"),
        "the second binding is {}",
        label(&model.lightmaps)
    );
    assert!(
        label(&model.pad_masks).ends_with("/wes_windows_d_strip.gtf"),
        "the glow mask is {}",
        label(&model.pad_masks)
    );
    let word = model.material_slots[slot];
    assert_eq!(word & slots::ADD_SECOND, 0, "no generic additive glow");
    let index = slots::material_index(word) as usize;
    assert!(index > 0, "the tower names no glow entry");
    let (window, glow) = (&model.emissive[index - 1], &model.emissive[index]);
    assert_eq!(window.rate, mesh::WINDOW_REFLECTION_RATE);
    // Reflection through `1 - picture.a`, a glow pair, and the specular map bound second.
    assert_eq!(window.tint, [1.0, 0.0, 1.0]);
    assert_eq!(window.offset, 1.0);
    assert_eq!(window.program, [0.0, 1.0]);
    for (got, want) in glow.tint.iter().zip([0.918_078_6, 0.921, 0.849_162]) {
        assert!(
            (got - want).abs() < 1e-5,
            "the glow colour is {:?}",
            glow.tint
        );
    }
}

/// The start gantry's boards: Metropia's two `cf_vex_billboard` slots resolve to
/// the compile with no `k` (`@0x1360`), `picture * (glow.a + glow.rgb * light)`
/// all the same (`mesh::rcs::billboard_glass`). Unclassified, they drew the card
/// lit by the sun.
fn the_gantry_boards_multiply_their_card_by_the_glow(
    model: &mesh::Model,
    report: &mesh::rcs::Report,
    source: &oag_rcs::rcsmodel::Model,
) {
    use mesh::slots;
    let boards: Vec<usize> = source
        .materials
        .iter()
        .enumerate()
        .filter(|(_, m)| m.name.ends_with("/cf_vex_billboard.rcsmaterial"))
        .map(|(slot, _)| slot)
        .collect();
    assert_eq!(boards.len(), 2, "the board slots: {boards:?}");
    for &slot in &boards {
        let word = model.material_slots[slot];
        assert_ne!(
            word & slots::ADD_SECOND,
            0,
            "board slot {slot} does not fetch its glow"
        );
        let index = slots::material_index(word) as usize;
        let entry = index
            .checked_sub(1)
            .and_then(|i| model.emissive.get(i))
            .unwrap_or_else(|| panic!("board slot {slot} names no glow entry"));
        // `k` 1 on every lane, the glow tap's `v` and `u` scales as authored (1 and 1).
        assert_eq!(entry.tint, [1.0; 3], "slot {slot}");
        assert_eq!(entry.scale, 1.0, "slot {slot}");
        assert_eq!(entry.program, [mesh::PROGRAM_BILLBOARD, 1.0], "slot {slot}");
    }
    assert_eq!(report.billboard_multiplied, 2);
}

/// The cream and green towers: `glass_2uv_rgb_reflect_glow`, 13 slots, the
/// reflective window whose reflection is gated by the glow mask's red
/// (`mesh::rcs::window_glass`). The mask sits at entry 0, so the generic reading
/// painted it as the facade.
fn the_rgb_windows_bind_their_picture_and_gate_the_sky_by_their_mask(
    model: &mesh::Model,
    source: &oag_rcs::rcsmodel::Model,
) {
    use mesh::slots;
    let windows: Vec<(usize, &oag_rcs::rcsmodel::Material)> = source
        .materials
        .iter()
        .enumerate()
        .filter(|(_, m)| m.name.ends_with("/glass_2uv_rgb_reflect_glow.rcsmaterial"))
        .collect();
    assert_eq!(windows.len(), 13);
    for (slot, material) in windows {
        let path_of = |hash: u32| {
            material
                .samplers
                .iter()
                .find(|(h, p)| *h == hash && p.is_some())
                .and_then(|(_, p)| p.clone())
                .unwrap_or_default()
        };
        let label = |table: &mesh::TextureSlots| {
            table[slot]
                .as_ref()
                .map(|t| t.label.clone())
                .unwrap_or_default()
        };
        // `texture1_alpha_for_reflect` is the picture, `Texture2_alpha_for_glow` the mask.
        assert_eq!(label(&model.textures), path_of(0xc29b_64e4), "slot {slot}");
        assert_eq!(label(&model.pad_masks), path_of(0x290a_5adb), "slot {slot}");
        let index = slots::material_index(model.material_slots[slot]) as usize;
        assert!(index > 0, "window slot {slot} names no entry");
        let window = &model.emissive[index - 1];
        assert_eq!(window.rate, mesh::WINDOW_REFLECTION_RATE, "slot {slot}");
        assert_eq!(window.tint, [1.0, 0.0, 3.0], "slot {slot}");
        assert_eq!(window.offset, 1.0, "slot {slot}");
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn metropia_carries_its_programs_glow_masks() {
    let Some(image) = image() else {
        return;
    };
    let Some((model, report, source)) = build(&image) else {
        return;
    };
    the_neon_rings_glow_by_their_pictures_alpha(&model, &report, &source);
    the_tower_binds_its_picture_its_glow_mask_and_its_specular_map(&model, &source);
    the_gantry_boards_multiply_their_card_by_the_glow(&model, &report, &source);
    the_rgb_windows_bind_their_picture_and_gate_the_sky_by_their_mask(&model, &source);
    assert!(
        model.glow_from_program,
        "HD's scene writes its mask by program"
    );
    let glow = &model.material_glow;
    assert_eq!(
        glow.len(),
        model.textures.len(),
        "one value per material slot"
    );
    let constants = glow.iter().filter(|&&g| g == 1.0).count();
    let texels = glow.iter().filter(|&&g| g < 0.0).count();
    let dark = glow.iter().filter(|&&g| g == 0.0).count();
    assert!(
        constants >= 1,
        "`cf_constantcolourglow` (the neon sign) writes alpha 1"
    );
    assert!(
        texels >= 1,
        "the emissive pads and lights write a texture's alpha"
    );
    assert!(
        dark * 2 > glow.len(),
        "most of a circuit is lit surface, which writes no mask ({dark} of {})",
        glow.len()
    );
    // A constant is a value in [0, 1] and a texel code is below -1.
    assert!(glow.iter().all(|&g| g <= 1.0), "no constant above 1");
}
