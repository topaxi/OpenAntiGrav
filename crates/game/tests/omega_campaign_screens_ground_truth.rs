//! Why Omega's campaign screens draw through HD's list, and why one widget is
//! left out of it - against the disc.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^omega_campaign_screens_ground_truth::/)'
//! ```

#[test]
fn omega_and_hd_draw_hd_campaign_screens_and_nothing_else_does() {
    for (title, expected) in [
        (oag_hd::TITLE, true),
        (oag_omega::TITLE, true),
        (oag_pulse::TITLE, false),
        (oag_pure::TITLE, false),
        (oag_2048::TITLE, false),
    ] {
        assert_eq!(
            oag_game::campaign::draws_hd_campaign(title),
            expected,
            "{}",
            title.name
        );
    }
}

/// Omega's `Cell Selection` authors `RecordsButton` at `DifficultyButton`'s own
/// position, so only one can be on screen - the premise of dropping the
/// Records one - and both carry the raw placeholder texts a Pulse-shaped draw
/// list would print.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_cell_selection_authors_records_and_difficulty_buttons_in_one_place() {
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let xml = String::from_utf8(
        archives
            .read_name(oag_omega::campaign::SCREEN_ENTRY)
            .expect("CellMode_Definition.xml"),
    )
    .expect("utf-8");
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let screen = screens
        .screens
        .iter()
        .find(|s| s.name == "Cell Selection")
        .expect("Cell Selection");
    let at = |name: &str| {
        screen
            .texts
            .iter()
            .find(|t| t.name.as_deref() == Some(name))
            .map(|t| (t.x, t.y))
    };
    assert!(at("DifficultyButton").is_some());
    assert_eq!(at("RecordsButton"), at("DifficultyButton"));
}

/// Omega's campaign carries Fury's `Grid Selection Fury` and decodes every flyer
/// card, front and back, through the PS4 scene path: dropping the layout read,
/// the card loader or the `psp2` dispatch in `flyer` fails this. Its
/// `Campaign Selection` is deliberately not kept (the chooser lists three
/// campaigns, 2048 among them).
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omegas_campaign_carries_fury_s_layout_and_every_card() {
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let base = oag_hud::sprite::Sheet::build(&[], &mut Vec::new());
    let campaign = oag_game::campaign::load(
        &mut archives,
        &oag_ui::language::StringTable::default(),
        oag_ui_screens::picker::FaceScales::default(),
        [1920.0, 1080.0],
        &base,
        &[],
        oag_omega::TITLE,
        &[],
    )
    .expect("the campaign loads");
    assert!(campaign.grid_layout_fury.is_some());
    assert!(campaign.selection_layout.is_none());
    let flyers = campaign
        .flyers
        .expect("Omega's screen authors the flyer widget");
    let broken: Vec<&String> = flyers
        .report
        .iter()
        .filter(|line| !line.contains("00_flyer"))
        .collect();
    assert!(
        broken.is_empty(),
        "cards that would not decode: {broken:#?}"
    );
    let fury = &campaign.grids[oag_hd::campaign::FURY_GRID_RANGE];
    assert_eq!(fury.len(), 8);
    assert_eq!(fury[0].flyer_name.as_deref(), Some("09_Blitzed"));
    // The card stands where the shadPS4 frame has it (x 816-1628, y 151-702 of 1920 by 1080,
    // measured off `omega/menu-grid-select`), not at HD's `GRID_POSE` (x 747-1688).
    let show = flyers.grid_show("09_Blitzed");
    assert_eq!(
        show.pose.offset,
        oag_omega::TITLE.campaign.grid_card_pose.unwrap().offset
    );
    let [x, y, width, height] = flyers
        .card_rect(&show, oag_display::space::Space::HD)
        .expect("the card is loaded");
    for (got, want) in [
        (x, 816.0),
        (y, 151.0),
        (x + width, 1628.0),
        (y + height, 702.0),
    ] {
        assert!(
            (got - want).abs() < 25.0,
            "card edge {got} against the frame's {want}"
        );
    }
}

/// The PS4 `00_flyer.rcsmodel` is the card's shell: its outline and the reflection's fade
/// come off the PS4 container (`flyer::shell`'s `load_psp2`), where the PS3 chunk reader
/// refused it and every card was a plain rectangle.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_cards_take_the_ps4_shell() {
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let shell = oag_game::flyer::Shell::load(&mut archives).expect("the PS4 shell reads");
    assert_eq!(
        shell.outline.len(),
        25,
        "twenty-five front triangles, as HD's"
    );
    assert!(
        (shell.half_size[0] - 51.2).abs() < 0.01,
        "{:?}",
        shell.half_size
    );
    assert!(
        (shell.half_size[1] - 33.3).abs() < 0.1,
        "{:?}",
        shell.half_size
    );
    assert!(
        (shell.fade.top_alpha - 76.0 / 255.0).abs() < 1e-6,
        "0x4c at the edge"
    );
}

/// The card materials read their coverage the way HD does: `simpletextureandtexturealpha`
/// from the picture's own alpha, `basicalpha` and `scrollingalpha` from the red channel of
/// the elements atlas (bound second) - read off the red channel or unbound they draw the
/// white boxes and dark blocks Omega's cards had. Walks every card of every grid, front and
/// back: the slot guard must never skip one.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_card_materials_read_their_coverage_channels() {
    use oag_mesh::mesh::slots;
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let base = oag_hud::sprite::Sheet::build(&[], &mut Vec::new());
    let campaign = oag_game::campaign::load(
        &mut archives,
        &oag_ui::language::StringTable::default(),
        oag_ui_screens::picker::FaceScales::default(),
        [1920.0, 1080.0],
        &base,
        &[],
        oag_omega::TITLE,
        &[],
    )
    .expect("the campaign loads");
    let channel = |slots: u32| (slots >> 3) & 3;
    let mut flyers: Vec<&str> = campaign
        .grids
        .iter()
        .filter_map(|grid| grid.flyer_name.as_deref())
        .collect();
    flyers.sort_unstable();
    flyers.dedup();
    assert!(flyers.len() >= 16, "{flyers:?}");
    let mut atlas_draws = 0;
    for flyer in flyers {
        let front = format!("Data/FE/Flyers/{flyer}/flyer.vex");
        let (model, _) = oag_game::preview::psp2_scene::model_named(&mut archives, &front)
            .unwrap_or_else(|e| panic!("{front}: {e:#}"));
        assert!(
            model
                .vertices
                .iter()
                .all(|v| channel(v.slots) == 3 || v.slots & slots::ALPHA_FROM_SECOND != 0),
            "{front}: every vertex reads the picture's alpha or an atlas"
        );
        let back = format!("Data/FE/Flyers/{flyer}/flyer_Back.vex");
        let (model, _) = oag_game::preview::psp2_scene::model_named(&mut archives, &back)
            .unwrap_or_else(|e| panic!("{back}: {e:#}"));
        let atlas_slots: std::collections::BTreeSet<usize> = model
            .draws
            .iter()
            .chain(&model.alpha_tested_draws)
            .chain(&model.transparent_draws)
            .filter(|draw| {
                let first = model.indices[draw.range.start as usize] as usize;
                model.vertices[first].slots & slots::ALPHA_FROM_SECOND != 0
            })
            .filter_map(|draw| draw.texture)
            .collect();
        let geometry = archives
            .read_name(&back.replace(".vex", ".rcsmodel"))
            .expect("the back's geometry");
        let parsed = oag_rcs::rcsmodel::psp2::parse(&geometry).expect("the back parses");
        let mut owners: Vec<usize> = parsed
            .submeshes
            .iter()
            .filter_map(|sub| sub.material)
            .collect();
        owners.sort_unstable();
        owners.dedup();
        let wanted = owners
            .iter()
            .filter(|&&m| {
                let name = parsed.materials[m].name.as_str();
                (name.ends_with("/basicalpha.rcsmaterial")
                    || name.ends_with("/scrollingalpha.rcsmaterial"))
                    && parsed.materials[m].sampler(0xa826_2b61).is_some()
            })
            .count();
        assert!(wanted >= 2, "{back}: {wanted}");
        assert_eq!(
            atlas_slots.len(),
            wanted,
            "{back}: every basicalpha and scrollingalpha material binds its atlas (the slot guard skipped one)"
        );
        for slot in atlas_slots {
            let atlas = model.lightmaps.get(slot).and_then(Option::as_ref);
            let picture = model.textures[slot].as_ref().expect("a picture");
            assert!(
                atlas.is_some_and(|t| t.label != picture.label),
                "{back}: slot {slot} binds an atlas second, apart from its picture {}",
                picture.label
            );
            atlas_draws += 1;
        }
    }
    assert!(atlas_draws >= 32);
}
