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
/// white boxes and dark blocks Omega's cards had.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_card_materials_read_their_coverage_channels() {
    use oag_mesh::mesh::slots;
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let channel = |slots: u32| (slots >> 3) & 3;
    let (front, _) = oag_game::preview::psp2_scene::model_named(
        &mut archives,
        "Data/FE/Flyers/09_Blitzed/flyer.vex",
    )
    .expect("the Fury front");
    assert!(
        front
            .vertices
            .iter()
            .all(|v| channel(v.slots) == 3 || v.slots & slots::ALPHA_FROM_SECOND != 0),
        "every front vertex reads the picture's alpha"
    );
    let (back, _) = oag_game::preview::psp2_scene::model_named(
        &mut archives,
        "Data/FE/Flyers/09_Blitzed/flyer_Back.vex",
    )
    .expect("the Fury back");
    let from_atlas: Vec<_> = back
        .vertices
        .iter()
        .filter(|v| v.slots & slots::ALPHA_FROM_SECOND != 0)
        .collect();
    assert!(
        !from_atlas.is_empty(),
        "the wordmark and the stripes read an atlas"
    );
    assert!(from_atlas.iter().all(|v| channel(v.slots) == 0));
    assert_eq!(
        back.lightmaps.iter().filter(|l| l.is_some()).count(),
        2,
        "scrollingalpha and basicalpha each bind their atlas"
    );
}
