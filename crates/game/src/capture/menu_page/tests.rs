use super::*;

/// Every other closed vocabulary in this crate is checked at load -
/// `menu::Action::parse` against `Action::all()`, `ValueSource::parse`,
/// `Definition::check`. This one is a runtime string match, so it gets the
/// equivalent here rather than being the one that can only be found by
/// running the flag and reading the error.
#[test]
fn every_prompt_the_flag_names_draws_something_and_an_unknown_one_errors() {
    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let strings = oag_ui::language::StringTable::default();
    for kind in [
        "rename",
        "rename-note",
        "delete",
        "delete-built-in",
        "binding",
    ] {
        let list = prompt_draws(kind, "winston", &strings, &skin, None)
            .unwrap_or_else(|e| panic!("{kind:?} is named by the flag's own help: {e:#}"));
        assert!(!list.is_empty(), "{kind:?} drew nothing");
        // `name` reaches every one of them, which is the whole reason the
        // substitution exists - a prompt that asked about `%s` would be
        // worse than one that asked about nothing. The test passes
        // `"winston"` for `binding` too, standing in for a button name
        // here the same way it stands in for a pilot's elsewhere: this
        // checks the substitution mechanism, not what `menu_page`'s own
        // caller derives it from.
        assert!(
            list.iter().any(
                |draw| matches!(draw, oag_ui::frontend::Draw::Text { text, .. }
                    if text.contains("winston"))
            ),
            "{kind:?} does not substitute name"
        );
        assert!(
            !list.iter().any(
                |draw| matches!(draw, oag_ui::frontend::Draw::Text { text, .. }
                    if text.contains("%s"))
            ),
            "{kind:?} left a %s unsubstituted"
        );
    }
    let error = prompt_draws("qwerty", "winston", &strings, &skin, None)
        .expect_err("an unknown prompt has to be an error, not an empty picture");
    assert!(format!("{error:#}").contains("qwerty"));
}

/// `tag-entry` is a real prompt name - unlike `"qwerty"` above - but it
/// reads Pulse's own `TagInput` screens live off a disc, which this test has
/// none of. It has to fail loudly rather than draw an empty or invented
/// picture; see `crates/ui-screens/tests/tag_entry_ground_truth.rs` for the version
/// of this that runs against the real thing.
#[test]
fn tag_entry_with_no_source_is_an_error_not_an_empty_picture() {
    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let strings = oag_ui::language::StringTable::default();
    let error = prompt_draws("tag-entry", "winston", &strings, &skin, None)
        .expect_err("no --race source means nothing was read");
    assert!(format!("{error:#}").contains("source"));
}

/// [`records_draws`] end to end, on the real built-in RECORDS page and a
/// `catalogue::Track` whose `entry_name()` needs the `Reversed="True"`
/// branch resolved - the exact closure `menu_page`'s own caller passes,
/// pinned here so deleting either the closure or the
/// `oag_ui_screens::prompt::record_row_draw` call fails a test rather than only
/// a screenshot nobody re-reads on every change.
#[test]
fn records_draws_builds_one_row_per_class_below_the_pages_own_rows() {
    let strings = oag_ui::language::StringTable::default();
    let mut definition = oag_ui::menu::Definition::parse(oag_ui::menu::BUILT_IN, &strings)
        .expect("the built-in menu definition parses");
    definition.drop_unavailable_race_variant(oag_pulse::TITLE);
    definition.drop_rows_picked_on_screen(oag_pulse::TITLE);
    let mut model = oag_ui::menu::Menu::new(definition);

    // Basilico White: a real pair from `Data\Plugins\PI001\Definition.xml`
    // where the reversed id (`17_Track`) shares its base id's own
    // directory (`01_Track`, Basilico Black - the two ids' own names come
    // from `entries.xml`, not from this file, and do not track which one
    // carries `Reversed`) - `Track::entry_name`'s `Reversed` branch, not
    // the identity case a track named after its own directory would pass
    // even with the branch broken.
    let track = oag_raceplay::catalogue::Track {
        id: "17_Track".to_string(),
        location: r"Data\Environments\01_Track".to_string(),
        reversed: true,
        available_in_zone: true,
        unlock_grid: None,
    };
    let tracks = [track.clone()];
    model.supply(
        oag_ui::menu::ValueSource::Tracks,
        &[oag_ui::menu::Choice::labelled(&track.id, "BASILICO WHITE")],
    );
    model.supply(
        oag_ui::menu::ValueSource::RaceModes,
        &oag_ui::menu::mode_choices(&strings),
    );
    model.seed(
        "race.mode",
        &oag_ui::menu::Value::Text("single_race".to_string()),
    );
    model.seed("race.track", &oag_ui::menu::Value::Text(track.id.clone()));
    model.open("records");
    model.settle();

    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let frame = oag_ui::menu::Frame::default();
    model.set_visible_rows(oag_ui::menu::visible_rows(&skin, &frame, false));

    // One class raced (a real time), one never touched (a dash) - both
    // outcomes `class_table` produces, so a regression that lost either
    // path fails here. The key is built from the **literal** reversed
    // path, not from `track.entry_name()` again - computing both sides
    // through the same call would still pass if `entry_name` ignored
    // `reversed` entirely, since the key and the lookup would agree by
    // construction either way. Spelling it out is what actually pins
    // the `Reversed="True"` branch this track exists to exercise.
    let mut store = crate::records::Store::default();
    store.record(
        crate::records::Key::new(
            oag_pulse::TITLE.name,
            Some(r"Data\Environments\01_Track\track_reversed.vex"),
            "single_race",
            "venom",
        ),
        crate::records::Observation {
            finished: true,
            place: Some(1),
            laps_completed: 3,
            tick: 6042,
            best_lap_ticks: Some(1987),
            campaign_medal: None,
            campaign_difficulty: None,
        },
    );

    let table_draws = records_draws(&model, &skin, oag_pulse::TITLE, &tracks, &store);
    let texts: Vec<&str> = table_draws
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        texts.contains(&oag_hud::format_lap_time(6042, oag_hud::Precision::Hundredths).as_str()),
        "no row shows Venom's real total time: {texts:?}"
    );
    assert!(
        texts.contains(&"-"),
        "no row shows a dash for a class this store never saw: {texts:?}"
    );

    // Below the page's own rows, not overlapping them: the first table
    // row's `y` has to be past the BACK row's, the same ordering
    // `MenuStage::render`'s own live chain draws in.
    let back_y = layers_text_y(&model, &skin, &frame, "BACK")
        .expect("the built-in RECORDS page carries a BACK row");
    let first_table_y = table_draws
        .iter()
        .filter_map(|draw| match draw {
            oag_ui::frontend::Draw::Text { y, .. } => Some(*y),
            _ => None,
        })
        .fold(f32::INFINITY, f32::min);
    assert!(
        first_table_y > back_y,
        "the table's first row ({first_table_y}) does not sit below BACK ({back_y})"
    );
}

/// `BACK`'s own drawn `y`, off the ordinary row draw list - the same one
/// `menu_page` builds, with no bindings and a trivial `measure` since
/// this test only reads a position, never a wrapped width.
fn layers_text_y(
    model: &oag_ui::menu::Menu,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    label: &str,
) -> Option<f32> {
    let layers = oag_ui::menu::draw_list(
        model,
        skin,
        &|_| Vec::new(),
        &|text: &str| text.len() as f32 * 8.0,
        None,
        frame,
        false,
    );
    layers.flatten().into_iter().find_map(|draw| match draw {
        oag_ui::frontend::Draw::Text { text, y, .. } if text == label => Some(y),
        _ => None,
    })
}
