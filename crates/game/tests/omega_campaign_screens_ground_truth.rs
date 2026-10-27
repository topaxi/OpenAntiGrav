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
