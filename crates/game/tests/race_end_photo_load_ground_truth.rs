//! `oag_game::endrace::load` hands a Pulse race its `Race End Photo` screen,
//! with both legend lines resolved to the disc's own text.
//!
//! This is the test that fails if the load is dropped: `crates/ui`'s
//! `race_end_photo_ground_truth` proves the screen parses, and this proves the
//! game reads it and resolves the strings - a race whose `photo` is `None`, or
//! whose texts are unresolved, shows the player no legend at all.
//!
//! `#[ignore]`d: needs `data/images/pulse-psp-usa.chd`. `just test-data`.

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulses_race_end_photo_loads_with_both_lines_resolved() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut opened =
        oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("Pulse's own disc opens");
    let plugins = opened
        .title
        .front_end
        .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
    let mut report = Vec::new();
    let languages =
        oag_ui::language::load::load_languages(&mut opened.archives, plugins, None, &mut report);
    let strings = oag_ui::language::load::load_strings(
        &mut opened.archives,
        &languages,
        Some("English"),
        &mut report,
    );
    let space = oag_display::space::Space::PSP;
    let screens = oag_game::endrace::load(
        &mut opened.archives,
        &strings,
        oag_ui_screens::picker::FaceScales::default(),
        [space.size.0, space.size.1],
        &oag_hud::sprite::Sheet::default(),
        &[],
        oag_pulse::TITLE,
    )
    .expect("Pulse's EndRace screens read off the real disc");

    let photo = screens
        .photo
        .expect("Race End Photo is read next to the three panels");
    let lines: Vec<(Option<&str>, f32, &str)> = photo
        .screen
        .texts
        .iter()
        .map(|text| {
            (
                text.idstring.as_deref(),
                text.y,
                text.string.as_deref().unwrap_or(""),
            )
        })
        .collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    for (id, _, string) in &lines {
        assert!(!string.is_empty(), "{id:?} did not resolve to text");
    }
    // The English text, read from the disc's own table rather than typed here
    // beyond the two words that tell a button line from a promise of photo mode.
    let proceed = lines.iter().find(|line| line.0 == Some("FE_PRESS_TO_CONT"));
    let photo_line = lines
        .iter()
        .find(|line| line.0 == Some("ER_PRESS_SELECT_PHOTO"));
    assert!(
        proceed.is_some_and(|line| line.2.to_lowercase().contains("continue")),
        "{lines:?}"
    );
    assert!(
        photo_line.is_some_and(|line| line.2.to_lowercase().contains("photo")),
        "{lines:?}"
    );

    // And it draws once the state is entered.
    let list = oag_ui_screens::endrace::photo::photo_draw_list(
        &photo,
        oag_ui_screens::endrace::photo::ENTER_TICKS + 1,
    );
    assert_eq!(list.len(), 2);
}
