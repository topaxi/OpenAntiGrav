use oag_game::hud::{Precision, format_lap_time};
use oag_ui::language::StringTable;

use super::*;

/// A minimal two-row page - MODE and TRACK, exactly what the RECORDS page's
/// own definition carries - so `held_text` can be exercised without a real
/// window, GPU or disc. The same fixture shape `pilots::tests::fixture_menu_on_axis`
/// uses for the AI PILOTS page.
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

#[test]
fn held_text_reads_the_row_a_setting_names() {
    let model = fixture_menu("records");
    assert_eq!(
        held_text(&model, "race.mode").as_deref(),
        Some("time_trial")
    );
    assert_eq!(held_text(&model, "race.track").as_deref(), Some("16_Track"));
}

/// A setting no row on this page edits - the guard `table_for` relies on to
/// draw nothing rather than a stale key built from a row that is not there.
#[test]
fn held_text_is_none_for_a_setting_no_row_edits() {
    let model = fixture_menu("records");
    assert_eq!(held_text(&model, "race.class"), None);
}

#[test]
fn show_total_is_false_only_for_speed_lap_and_zone() {
    for mode in oag_race::Mode::ALL {
        let expected = !matches!(mode, oag_race::Mode::SpeedLap | oag_race::Mode::Zone);
        assert_eq!(show_total(mode), expected, "{mode:?}");
    }
}

const TRACK: &str = r"data\environments\16_track\track.vex";

fn observation(finished: bool, tick: u64, best_lap_ticks: Option<u32>) -> records::Observation {
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
fn build_table_reads_the_total_for_a_finished_mode() {
    let mut store = Store::default();
    store.record(
        records::Key::new("wipeout pulse", Some(TRACK), "time_trial", "venom"),
        observation(true, 6042, Some(1987)),
    );
    let classes = [
        menu::Choice::labelled("venom", "VENOM"),
        menu::Choice::labelled("flash", "FLASH"),
    ];
    let table = build_table(
        "wipeout pulse",
        TRACK,
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

/// Speed Lap never finishes - `oag_game::records`'s own module doc names it
/// by name - so its column is the best *lap*, not the (permanently absent)
/// total.
#[test]
fn build_table_reads_the_lap_for_speed_lap() {
    let mut store = Store::default();
    store.record(
        records::Key::new("wipeout pulse", Some(TRACK), "speed_lap", "venom"),
        observation(false, 5000, Some(1987)),
    );
    let classes = [menu::Choice::labelled("venom", "VENOM")];
    let table = build_table(
        "wipeout pulse",
        TRACK,
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
/// convention `crate::scoreboard::draw_list` already uses for a craft that
/// never crossed - never a formatted zero.
#[test]
fn build_table_draws_a_dash_for_a_class_never_raced() {
    let store = Store::default();
    let classes = [menu::Choice::labelled("phantom", "PHANTOM")];
    let table = build_table(
        "wipeout pulse",
        TRACK,
        oag_race::Mode::TimeTrial,
        &classes,
        &store,
    );
    assert_eq!(table, vec![("PHANTOM".to_string(), "-".to_string())]);
}
