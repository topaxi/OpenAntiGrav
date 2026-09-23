use super::*;

/// A craft that finished, and one still going round. Both carry a best lap of
/// their own, which is the column the race times.
fn finisher(slot: u8, place: u8, tick: u64) -> Craft {
    Craft {
        slot,
        place,
        lap: 4,
        finish_tick: Some(tick),
        best_lap_ticks: Some(3_000 + u32::from(slot)),
    }
}

fn racer(slot: u8, place: u8, lap: u32) -> Craft {
    Craft {
        slot,
        place,
        lap,
        finish_tick: None,
        // A craft still on lap 1 has completed none, so it has no best.
        best_lap_ticks: (lap > 1).then_some(3_100 + u32::from(slot)),
    }
}

fn board() -> Board {
    build(
        &[
            racer(0, 2, 4),
            finisher(1, 1, 9_000),
            racer(2, 3, 3),
            racer(3, 4, 2),
        ],
        Some(3),
        9_400,
    )
}

#[test]
fn rows_come_out_in_finishing_order() {
    let board = build(
        &[racer(0, 3, 3), finisher(1, 1, 100), finisher(2, 2, 200)],
        Some(3),
        300,
    );
    assert_eq!(
        board.rows.iter().map(|row| row.slot).collect::<Vec<_>>(),
        [1, 2, 0]
    );
}

#[test]
fn slot_zero_is_the_player_and_nobody_else_is() {
    let board = board();
    assert_eq!(board.player().map(|row| row.slot), Some(0));
    assert_eq!(board.rows.iter().filter(|row| row.player).count(), 1);
}

/// A finished craft's lap counter is one past the target, and reporting that
/// raw would credit it with a lap it never drove.
#[test]
fn a_finisher_is_credited_with_the_target_and_no_more() {
    let board = build(&[finisher(0, 1, 500)], Some(3), 500);
    assert_eq!(board.rows[0].laps_completed, 3);
    assert!(board.rows[0].finished());
}

#[test]
fn a_craft_still_racing_is_credited_with_the_laps_it_drove() {
    let board = build(&[racer(0, 1, 3)], Some(3), 500);
    assert_eq!(board.rows[0].laps_completed, 2);
    assert!(!board.rows[0].finished());
}

/// A mode with no lap target - a speed lap - still tables what everyone drove.
#[test]
fn a_mode_that_never_ends_still_counts_laps() {
    let board = build(&[racer(0, 1, 7)], None, 500);
    assert_eq!(board.rows[0].laps_completed, 6);
    assert_eq!(board.laps_target, None);
}

/// The place is what `oag_race::places` assigned; this must not re-derive it,
/// or the table and the HUD's position readout could disagree.
#[test]
fn the_place_column_is_the_one_it_was_given() {
    let board = build(&[racer(0, 4, 9), racer(1, 1, 1)], Some(3), 10);
    assert_eq!(
        board
            .rows
            .iter()
            .map(|row| (row.place, row.slot))
            .collect::<Vec<_>>(),
        [(1, 1), (4, 0)]
    );
}

fn texts(list: &[Draw]) -> Vec<String> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_list_draws_a_panel_a_heading_and_one_line_per_craft() {
    let board = board();
    let list = draw_list(&board, 12.0, None);
    assert!(matches!(list.first(), Some(Draw::Fill { .. })));
    let texts = texts(&list);
    assert!(texts.contains(&TITLE.to_string()));
    assert!(texts.contains(&"YOU".to_string()));
    assert!(texts.contains(&"SLOT 2".to_string()));
    assert!(texts.contains(&"SLOT 4".to_string()));
    // Five columns per craft, on top of the heading, five captions, the footer
    // and the hint.
    assert_eq!(texts.len(), 1 + 5 + board.rows.len() * 5 + 1 + 1);
}

#[test]
fn a_craft_that_did_not_finish_has_no_time() {
    let list = draw_list(&build(&[racer(0, 1, 2)], Some(3), 600), 12.0, None);
    assert!(texts(&list).contains(&"-".to_string()));
}

#[test]
fn a_finisher_shows_its_crossing_as_a_race_time() {
    let list = draw_list(&build(&[finisher(0, 1, 3_600)], Some(3), 3_600), 12.0, None);
    // 3,600 ticks at 60 Hz is one minute exactly, written the way the original
    // writes a time.
    assert!(texts(&list).contains(&"1.00.00".to_string()));
}

#[test]
fn the_footer_reports_the_race_time() {
    let texts = texts(&draw_list(&board(), 12.0, None));
    assert!(texts.iter().any(|text| text.starts_with("RACE TIME 2.36.")));
}

/// Each craft's own quickest lap, not the player's eight times.
#[test]
fn every_row_carries_its_own_best_lap() {
    let texts = texts(&draw_list(&board(), 12.0, None));
    // The finisher's 3,001 ticks and the three racers' 3,100, 3,102 and 3,103.
    for expected in ["0.50.01", "0.51.66", "0.51.70", "0.51.71"] {
        assert!(
            texts.contains(&expected.to_string()),
            "no best lap {expected}"
        );
    }
}

/// A craft the flag caught on lap 1 has completed none, so it has no best.
#[test]
fn a_craft_with_no_completed_lap_has_no_best_one() {
    let board = build(&[racer(0, 1, 1)], Some(3), 10);
    assert_eq!(board.rows[0].best_lap_ticks, None);
    assert!(texts(&draw_list(&board, 12.0, None)).contains(&"-".to_string()));
}

/// A time trial, a speed lap and a Zone run grid the player alone, and `1 of 1`
/// is arithmetic rather than a standing - which is the rule `hud::Readout`
/// already applies to the position widget. The two must not disagree.
#[test]
fn a_field_of_one_shows_no_place_at_all() {
    let board = build(&[finisher(0, 1, 3_000)], Some(3), 3_000);
    assert!(!board.shows_place());
    let texts = texts(&draw_list(&board, 12.0, None));
    assert!(!texts.contains(&"POS".to_string()));
    assert!(!texts.contains(&"1".to_string()));
    // Everything else it has to say is still there.
    assert!(texts.contains(&"YOU".to_string()));
    assert!(texts.contains(&"3/3".to_string()));
}

#[test]
fn a_field_of_more_than_one_shows_every_place() {
    let board = board();
    assert!(board.shows_place());
    let texts = texts(&draw_list(&board, 12.0, None));
    assert!(texts.contains(&"POS".to_string()));
    for place in 1..=4 {
        assert!(texts.contains(&format!("{place}")), "no place {place}");
    }
}

/// The whole panel has to fit the 480x272 grid, or a row is drawn off the
/// bottom of a screen nobody can scroll.
#[test]
fn a_full_grid_fits_the_screen() {
    let crafts: Vec<Craft> = (0..8)
        .map(|slot| racer(slot, slot + 1, 3))
        .collect::<Vec<_>>();
    let board = build(&crafts, Some(3), 12_000);
    let list = draw_list(&board, 12.0, None);
    let Some(Draw::Fill { rect, .. }) = list.first() else {
        panic!("the panel is the first draw");
    };
    assert!(rect[0] >= 0.0 && rect[1] >= 0.0);
    assert!(rect[0] + rect[2] <= oag_display::space::SCREEN.0);
    assert!(
        rect[1] + rect[3] <= oag_display::space::SCREEN.1,
        "panel bottom {} is off the 272-line screen",
        rect[1] + rect[3]
    );
    for draw in &list {
        if let Draw::Text { y, .. } = draw {
            assert!(
                *y >= rect[1] && *y <= rect[1] + rect[3],
                "row at {y} escapes"
            );
        }
    }
}

/// `None` draws exactly what every test above already asserted - no personal
/// best means no extra line, not a blank or placeholder one. Named
/// separately from `the_list_draws_a_panel_a_heading_and_one_line_per_craft`
/// so a future change to that count is not the one place this guarantee is
/// pinned.
#[test]
fn no_personal_best_draws_no_extra_line() {
    let with_none = texts(&draw_list(&board(), 12.0, None));
    assert!(!with_none.iter().any(|t| t.contains("PERSONAL BEST")));
    assert!(!with_none.iter().any(|t| t.contains("BEST MEDAL")));
}

/// A stored lap this race did not beat draws in the ordinary caption colour
/// and carries no "new" - `lap_improved: false` is the whole reason
/// `PersonalBest::compare` has two fields for this rather than one.
#[test]
fn a_personal_best_lap_that_was_not_beaten_draws_plainly() {
    let pb = records::PersonalBest {
        best_lap_ticks: Some(3_000),
        lap_improved: false,
        ..records::PersonalBest::default()
    };
    let texts = texts(&draw_list(&board(), 12.0, Some(&pb)));
    assert!(texts.contains(&"PERSONAL BEST LAP 0.50.00".to_string()));
    assert!(!texts.iter().any(|t| t.contains("NEW")));
}

/// The race that sets the personal best says so, on the same line.
#[test]
fn a_personal_best_lap_just_beaten_says_so() {
    let pb = records::PersonalBest {
        best_lap_ticks: Some(2_900),
        lap_improved: true,
        ..records::PersonalBest::default()
    };
    let texts = texts(&draw_list(&board(), 12.0, Some(&pb)));
    assert!(texts.contains(&"PERSONAL BEST LAP 0.48.33 - NEW!".to_string()));
}

/// The medal line is independent of the lap line - a race can carry one
/// without the other, and each is judged on its own `_improved` flag.
#[test]
fn a_standing_medal_draws_with_its_own_label_and_no_lap_line() {
    let pb = records::PersonalBest {
        best_medal: Some(records::Medal::Silver),
        medal_improved: false,
        ..records::PersonalBest::default()
    };
    let texts = texts(&draw_list(&board(), 12.0, Some(&pb)));
    assert!(texts.contains(&"BEST MEDAL SILVER".to_string()));
    assert!(!texts.iter().any(|t| t.contains("PERSONAL BEST LAP")));
}

/// A medal this race itself earned is highlighted the same way a beaten lap
/// is.
#[test]
fn a_medal_just_earned_says_so() {
    let pb = records::PersonalBest {
        best_medal: Some(records::Medal::Gold),
        medal_improved: true,
        ..records::PersonalBest::default()
    };
    let texts = texts(&draw_list(&board(), 12.0, Some(&pb)));
    assert!(texts.contains(&"BEST MEDAL GOLD - NEW!".to_string()));
}

/// Both lines at once still fit the 480x272 grid - the panel height has to
/// grow by exactly the two extra rows, not stay pinned at the no-personal-best
/// size.
#[test]
fn both_lines_together_still_fit_the_screen() {
    let pb = records::PersonalBest {
        best_lap_ticks: Some(2_900),
        lap_improved: true,
        best_medal: Some(records::Medal::Bronze),
        medal_improved: false,
        ..records::PersonalBest::default()
    };
    let list = draw_list(&board(), 12.0, Some(&pb));
    let Some(Draw::Fill { rect, .. }) = list.first() else {
        panic!("the panel is the first draw");
    };
    assert!(
        rect[1] + rect[3] <= oag_display::space::SCREEN.1,
        "panel bottom {} is off the 272-line screen",
        rect[1] + rect[3]
    );
    let texts = texts(&list);
    assert!(texts.contains(&"PERSONAL BEST LAP 0.48.33 - NEW!".to_string()));
    assert!(texts.contains(&"BEST MEDAL BRONZE".to_string()));
}

#[test]
fn show_total_is_false_only_for_speed_lap_and_zone() {
    for mode in oag_race::Mode::ALL {
        let expected = !matches!(mode, oag_race::Mode::SpeedLap | oag_race::Mode::Zone);
        assert_eq!(show_total(mode), expected, "{mode:?}");
    }
}

const CLASS_TABLE_TRACK: &str = r"data\environments\16_track\track.vex";

fn class_table_observation(
    finished: bool,
    tick: u64,
    best_lap_ticks: Option<u32>,
) -> records::Observation {
    records::Observation {
        finished,
        place: Some(1),
        laps_completed: 3,
        tick,
        best_lap_ticks,
        campaign_medal: None,
    }
}

/// A finished mode (Time Trial here) reads `best_total_ticks`: recorded for
/// the class that raced, a dash for the class that never has.
#[test]
fn class_table_reads_the_total_for_a_finished_mode() {
    let mut store = records::Store::default();
    store.record(
        records::Key::new(
            "wipeout pulse",
            Some(CLASS_TABLE_TRACK),
            "time_trial",
            "venom",
        ),
        class_table_observation(true, 6042, Some(1987)),
    );
    let classes = [
        menu::Choice::labelled("venom", "VENOM"),
        menu::Choice::labelled("flash", "FLASH"),
    ];
    let table = class_table(
        "wipeout pulse",
        CLASS_TABLE_TRACK,
        oag_race::Mode::TimeTrial,
        &classes,
        &store,
    );
    assert_eq!(
        table,
        vec![
            (
                "VENOM".to_string(),
                format_lap_time(6042, Precision::Hundredths)
            ),
            ("FLASH".to_string(), "-".to_string()),
        ]
    );
}

/// Speed Lap never finishes - `records`'s own module doc names it by name -
/// so its column is the best *lap*, not the (permanently absent) total.
#[test]
fn class_table_reads_the_lap_for_speed_lap() {
    let mut store = records::Store::default();
    store.record(
        records::Key::new(
            "wipeout pulse",
            Some(CLASS_TABLE_TRACK),
            "speed_lap",
            "venom",
        ),
        class_table_observation(false, 5000, Some(1987)),
    );
    let classes = [menu::Choice::labelled("venom", "VENOM")];
    let table = class_table(
        "wipeout pulse",
        CLASS_TABLE_TRACK,
        oag_race::Mode::SpeedLap,
        &classes,
        &store,
    );
    assert_eq!(
        table,
        vec![(
            "VENOM".to_string(),
            format_lap_time(1987, Precision::Hundredths)
        )]
    );
}

/// A row this store has never seen at all draws a dash, the same "no time"
/// convention [`draw_list`] already uses for a craft that never crossed.
#[test]
fn class_table_draws_a_dash_for_a_class_never_raced() {
    let store = records::Store::default();
    let classes = [menu::Choice::labelled("phantom", "PHANTOM")];
    let table = class_table(
        "wipeout pulse",
        CLASS_TABLE_TRACK,
        oag_race::Mode::TimeTrial,
        &classes,
        &store,
    );
    assert_eq!(table, vec![("PHANTOM".to_string(), "-".to_string())]);
}

mod records_table_tests {
    use oag_ui::language::StringTable;

    use super::*;

    /// A minimal two-row page - MODE and TRACK, exactly what the RECORDS
    /// page's own definition carries - so [`records_table`] can be
    /// exercised with no real window, GPU or disc. The same fixture shape
    /// `pilots::tests::fixture_menu_on_axis` uses for the AI PILOTS page.
    fn fixture_menu(page_id: &str) -> menu::Menu {
        let text = format!(
            "version = 1\n\
             root = \"{page_id}\"\n\
             [[page]]\n\
             id = \"{page_id}\"\n\
             [[page.entry]]\n\
             kind = \"choice\"\n\
             label = \"MODE\"\n\
             setting = \"race.mode\"\n\
             values_from = \"race_modes\"\n\
             [[page.entry]]\n\
             kind = \"choice\"\n\
             label = \"TRACK\"\n\
             setting = \"race.track\"\n\
             values_from = \"tracks\"\n"
        );
        let strings = StringTable::default();
        let definition = menu::Definition::parse(&text, &strings).expect("a tiny valid definition");
        let mut model = menu::Menu::new(definition);
        model.supply(
            menu::ValueSource::RaceModes,
            &[
                menu::Choice::labelled("time_trial", "TIME TRIAL"),
                menu::Choice::labelled("speed_lap", "SPEED LAP"),
            ],
        );
        model.supply(
            menu::ValueSource::Tracks,
            &[menu::Choice::labelled("16_Track", "TALON'S JUNCTION")],
        );
        model
    }

    const TRACK: &str = r"data\environments\16_track\track.vex";

    #[test]
    fn held_text_reads_the_row_a_setting_names() {
        let model = fixture_menu("records");
        assert_eq!(
            held_text(&model, "race.mode").as_deref(),
            Some("time_trial")
        );
        assert_eq!(held_text(&model, "race.track").as_deref(), Some("16_Track"));
    }

    /// A setting no row on this page edits - the guard [`records_table`]
    /// relies on to draw nothing rather than a stale key built from a row
    /// that is not there.
    #[test]
    fn held_text_is_none_for_a_setting_no_row_edits() {
        let model = fixture_menu("records");
        assert_eq!(held_text(&model, "race.class"), None);
    }

    #[test]
    fn none_off_any_page_but_records() {
        let model = fixture_menu("race");
        let store = records::Store::default();
        assert!(
            records_table(&model, oag_pulse::TITLE, &store, |_, _| Some(
                TRACK.to_string()
            ))
            .is_none()
        );
    }

    /// Mirrors what a capture whose track lookup has no Zone list sees when
    /// `race.mode` is seeded to `zone`: no table, not one built from the
    /// wrong list - see [`records_table`]'s own doc.
    #[test]
    fn none_when_track_entry_cannot_resolve_the_held_track() {
        let model = fixture_menu("records");
        let store = records::Store::default();
        assert!(records_table(&model, oag_pulse::TITLE, &store, |_, _| None).is_none());
    }

    #[test]
    fn builds_one_row_per_class_off_the_held_mode_and_track() {
        let model = fixture_menu("records");
        let mut store = records::Store::default();
        store.record(
            records::Key::new("wipeout pulse", Some(TRACK), "time_trial", "venom"),
            class_table_observation(true, 6042, Some(1987)),
        );
        let table = records_table(&model, oag_pulse::TITLE, &store, |_, _| {
            Some(TRACK.to_string())
        })
        .expect("MODE and TRACK are both held on this fixture's records page");
        let (_, venom_time) = table
            .iter()
            .find(|(label, _)| label == "VENOM")
            .expect("Pulse's ladder ships Venom");
        assert_eq!(*venom_time, format_lap_time(6042, Precision::Hundredths));
        assert!(
            table.iter().any(|(_, value)| value == "-"),
            "a class this store never saw should still draw a dash, not be left out: {table:?}"
        );
    }
}
