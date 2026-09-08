//! What [`super`] is asserted to do: the merge rule, the tolerant parse and
//! the shape a fresh record takes - all through the pure functions, never
//! [`super::load`]/[`super::save`], which touch a real config directory and
//! would make the suite pass on one machine only. See `settings/tests.rs`,
//! which keeps the same split for the same reason.

use super::*;

fn key(track: &str) -> Key {
    Key::new("pulse", Some(track), "single_race", "venom")
}

/// The disc's own rule, replicated: a finisher's laps read as the target, not
/// one past it.
#[test]
fn a_finisher_completes_exactly_the_target() {
    assert_eq!(laps_completed(4, true, Some(3)), 3);
}

/// An abandoned race reports what it actually got through.
#[test]
fn an_unfinished_race_completes_one_less_than_its_lap_counter() {
    assert_eq!(laps_completed(2, false, Some(3)), 1);
    assert_eq!(laps_completed(1, false, Some(3)), 0);
}

/// Speed Lap and Zone never finish - `laps_target` is `None` - so this is the
/// only formula either mode ever reaches.
#[test]
fn a_mode_with_no_target_always_takes_the_lap_counter_branch() {
    assert_eq!(laps_completed(5, false, None), 4);
    // `finished` cannot really be `true` with no target in practice, but the
    // arithmetic still has to not panic on the lap-zero edge either way.
    assert_eq!(laps_completed(0, false, None), 0);
}

/// The case the whole key exists to prevent: two spellings of one class must
/// not open two rows.
#[test]
fn a_key_lower_cases_every_part() {
    let a = Key::new(
        "Pulse",
        Some(r"Data\Environments\32_Track\track.vex"),
        "single_race",
        "VENOM",
    );
    let b = Key::new(
        "pulse",
        Some(r"data\environments\32_track\track.vex"),
        "single_race",
        "venom",
    );
    assert_eq!(a, b);
}

#[test]
fn a_missing_circuit_falls_back_to_the_named_constant() {
    let k = Key::new("pulse", None, "time_trial", "venom");
    assert_eq!(k.track, NO_CIRCUIT);
}

/// The first race on a key creates its row, seeded from that one
/// observation - nothing is "the best of one and something invented".
#[test]
fn the_first_race_on_a_key_seeds_every_field_from_itself() {
    let mut store = Store::default();
    store.record(
        key("16_track"),
        Observation {
            finished: true,
            place: Some(1),
            laps_completed: 3,
            tick: 6000,
            best_lap_ticks: Some(1900),
            campaign_medal: None,
        },
    );
    let row = store.get(&key("16_track")).expect("just recorded");
    assert_eq!(row.best_lap_ticks, Some(1900));
    assert_eq!(row.best_total_ticks, Some(6000));
    assert!(row.last_finished);
    assert_eq!(row.last_place, Some(1));
    assert_eq!(row.last_laps_completed, 3);
    assert_eq!(row.last_tick, 6000);
    assert_eq!(row.last_best_lap_ticks, Some(1900));
}

/// The bar this whole module exists to clear: a slower second race must not
/// erase a faster first one, even though it is now "the last result".
#[test]
fn a_slower_race_does_not_erase_the_standing_best() {
    let mut store = Store::default();
    store.record(
        key("16_track"),
        Observation {
            finished: true,
            place: Some(1),
            laps_completed: 3,
            tick: 6000,
            best_lap_ticks: Some(1900),
            campaign_medal: None,
        },
    );
    store.record(
        key("16_track"),
        Observation {
            finished: true,
            place: Some(3),
            laps_completed: 3,
            tick: 6500,
            best_lap_ticks: Some(2100),
            campaign_medal: None,
        },
    );
    let row = store.get(&key("16_track")).expect("still there");
    assert_eq!(row.best_lap_ticks, Some(1900), "the faster lap survives");
    assert_eq!(
        row.best_total_ticks,
        Some(6000),
        "the faster total survives"
    );
    // But the "last race" fields always take the newest race, worse or not -
    // that is what "last" promises.
    assert_eq!(row.last_place, Some(3));
    assert_eq!(row.last_tick, 6500);
    assert_eq!(row.last_best_lap_ticks, Some(2100));
}

/// The opposite direction: a faster race does improve both bests.
#[test]
fn a_faster_race_improves_both_bests() {
    let mut store = Store::default();
    store.record(
        key("16_track"),
        Observation {
            finished: true,
            place: Some(2),
            laps_completed: 3,
            tick: 6500,
            best_lap_ticks: Some(2100),
            campaign_medal: None,
        },
    );
    store.record(
        key("16_track"),
        Observation {
            finished: true,
            place: Some(1),
            laps_completed: 3,
            tick: 6000,
            best_lap_ticks: Some(1900),
            campaign_medal: None,
        },
    );
    let row = store.get(&key("16_track")).expect("still there");
    assert_eq!(row.best_lap_ticks, Some(1900));
    assert_eq!(row.best_total_ticks, Some(6000));
}

/// The reason `best_total_ticks` is gated on `finished` and `best_lap_ticks`
/// is not: an abandoned Speed Lap run has a real fast lap and no real total.
#[test]
fn an_unfinished_race_can_still_set_a_best_lap_but_never_a_best_total() {
    let mut store = Store::default();
    store.record(
        key("06_track"),
        Observation {
            finished: false,
            place: Some(1),
            laps_completed: 4,
            tick: 9000,
            best_lap_ticks: Some(1800),
            campaign_medal: None,
        },
    );
    let row = store.get(&key("06_track")).expect("just recorded");
    assert_eq!(row.best_lap_ticks, Some(1800), "a lap was completed");
    assert_eq!(row.best_total_ticks, None, "the race was never finished");
    assert!(!row.last_finished);
    assert_eq!(row.last_tick, 9000, "the tick it was left on, not a finish");
}

/// Two different tracks are two different rows, never merged.
#[test]
fn different_tracks_do_not_share_a_row() {
    let mut store = Store::default();
    store.record(
        key("16_track"),
        Observation {
            finished: true,
            place: Some(1),
            laps_completed: 3,
            tick: 6000,
            best_lap_ticks: Some(1900),
            campaign_medal: None,
        },
    );
    store.record(
        key("06_track"),
        Observation {
            finished: true,
            place: Some(1),
            laps_completed: 3,
            tick: 5000,
            best_lap_ticks: Some(1600),
            campaign_medal: None,
        },
    );
    assert_eq!(store.rows().len(), 2);
    assert_eq!(
        store.get(&key("16_track")).unwrap().best_total_ticks,
        Some(6000)
    );
    assert_eq!(
        store.get(&key("06_track")).unwrap().best_total_ticks,
        Some(5000)
    );
}

/// A row order that must not depend on insertion order, for the reason
/// [`Store`]'s own doc gives: a stable rewrite.
#[test]
fn rows_stay_sorted_regardless_of_insertion_order() {
    let mut store = Store::default();
    for track in ["09_track", "01_track", "16_track"] {
        store.record(
            key(track),
            Observation {
                finished: true,
                place: Some(1),
                laps_completed: 3,
                tick: 6000,
                best_lap_ticks: Some(1900),
                campaign_medal: None,
            },
        );
    }
    let tracks: Vec<&str> = store.rows().iter().map(|row| row.track.as_str()).collect();
    assert_eq!(tracks, ["01_track", "09_track", "16_track"]);
}

/// A round trip through the exact bytes [`super::save`] would write, without
/// touching a real file - `toml::to_string_pretty` plus [`parse`], the same
/// pairing `save`/`load` use on disk.
#[test]
fn a_store_round_trips_through_its_own_written_text() {
    let mut store = Store::default();
    store.record(
        key("16_track"),
        Observation {
            finished: true,
            place: Some(2),
            laps_completed: 3,
            tick: 6042,
            best_lap_ticks: Some(1987),
            campaign_medal: None,
        },
    );
    let text = toml::to_string_pretty(&store).expect("serialises");
    let (parsed, notes) = parse(&text).expect("parses back");
    assert!(
        notes.is_empty(),
        "a clean write should need no notes: {notes:?}"
    );
    assert_eq!(parsed.rows(), store.rows());
}

/// The bar the whole "deliberately not `Settings`'s own rule" section
/// promises: one bad row must not cost every other row in the file.
#[test]
fn one_malformed_row_is_dropped_and_every_other_row_survives() {
    let text = r#"
[[records]]
title = "pulse"
track = "16_track"
mode = "single_race"
class = "venom"
best_lap_ticks = 1900

[[records]]
title = "pulse"
track = "06_track"
mode = "single_race"
best_lap_ticks = "not a number"
"#;
    let (store, notes) = parse(text).expect("the document itself is valid TOML");
    assert_eq!(store.rows().len(), 1, "only the good row survives");
    assert_eq!(store.rows()[0].track, "16_track");
    assert_eq!(notes.len(), 1, "the bad row is named once: {notes:?}");
}

/// A key field missing entirely is the same kind of drop, not a panic and
/// not a default-filled row with an empty circuit name.
#[test]
fn a_row_missing_a_key_field_is_dropped_not_defaulted() {
    let text = r#"
[[records]]
title = "pulse"
track = "16_track"
mode = "single_race"
class = "venom"

[[records]]
title = "pulse"
mode = "single_race"
class = "venom"
"#;
    let (store, notes) = parse(text).expect("valid TOML");
    assert_eq!(store.rows().len(), 1);
    assert_eq!(notes.len(), 1);
}

/// A whole file that is not TOML at all is the one case [`parse`] refuses
/// outright - [`super::load`] is what decides to move it aside rather than
/// overwrite it, which this test does not reach.
#[test]
fn a_document_that_is_not_toml_at_all_is_an_error() {
    assert!(parse("this is not { valid toml at all").is_err());
}

/// A `records` key that exists but is not an array is treated as absent
/// rather than refused - a hand edit or a future shape, not a crash.
#[test]
fn a_non_array_records_key_starts_empty_with_a_note() {
    let (store, notes) = parse("records = \"oops\"").expect("still valid TOML");
    assert!(store.rows().is_empty());
    assert_eq!(notes.len(), 1);
}

/// An empty document is not an error - the ordinary shape of a file nobody
/// has raced under yet.
#[test]
fn an_empty_document_parses_to_an_empty_store_with_no_notes() {
    let (store, notes) = parse("").expect("empty is valid TOML");
    assert!(store.rows().is_empty());
    assert!(notes.is_empty());
}

/// A record from an older build that has not gained a later field yet still
/// parses, with that field defaulting rather than the whole row refusing -
/// the same promise `settings.rs` makes for its own tables.
#[test]
fn a_row_missing_every_optional_field_still_parses() {
    let text = r#"
[[records]]
title = "pulse"
track = "16_track"
mode = "time_trial"
class = "venom"
"#;
    let (store, notes) = parse(text).expect("valid TOML");
    assert!(notes.is_empty());
    let row = &store.rows()[0];
    assert_eq!(row.best_lap_ticks, None);
    assert_eq!(row.best_total_ticks, None);
    assert!(!row.last_finished);
}

fn observation_with_medal(medal: Option<Medal>) -> Observation {
    Observation {
        finished: true,
        place: Some(1),
        laps_completed: 3,
        tick: 6000,
        best_lap_ticks: Some(1900),
        campaign_medal: medal,
    }
}

/// The bar `Medal::better` exists to clear: a later, worse race must not
/// erase a standing gold, even though "last" still moves to it - the exact
/// mirror of `a_slower_race_does_not_erase_the_standing_best` above, for the
/// medal fields instead of the lap/total ones.
#[test]
fn a_gold_medal_is_never_downgraded_by_a_later_bronze() {
    let mut store = Store::default();
    store.record(key("16_track"), observation_with_medal(Some(Medal::Gold)));
    store.record(key("16_track"), observation_with_medal(Some(Medal::Bronze)));
    let row = store.get(&key("16_track")).expect("still there");
    assert_eq!(
        row.best_medal,
        Some(Medal::Gold),
        "the better medal survives"
    );
    assert_eq!(
        row.best_points,
        Some(3),
        "points stay in sync with the medal"
    );
    assert_eq!(
        row.last_medal,
        Some(Medal::Bronze),
        "but the last race is still reported as it actually went"
    );
}

/// The opposite direction: a better medal does replace a worse standing one.
#[test]
fn a_better_medal_upgrades_the_standing_best() {
    let mut store = Store::default();
    store.record(key("16_track"), observation_with_medal(Some(Medal::Silver)));
    store.record(key("16_track"), observation_with_medal(Some(Medal::Gold)));
    let row = store.get(&key("16_track")).expect("still there");
    assert_eq!(row.best_medal, Some(Medal::Gold));
    assert_eq!(row.best_points, Some(3));
}

/// A race with no campaign cell in play - `campaign_medal: None`, the only
/// value any real call site produces today - must not read as "no medal was
/// ever earned here" and erase a standing one.
#[test]
fn no_campaign_cell_leaves_the_standing_medal_untouched() {
    let mut store = Store::default();
    store.record(key("16_track"), observation_with_medal(Some(Medal::Gold)));
    store.record(key("16_track"), observation_with_medal(None));
    let row = store.get(&key("16_track")).expect("still there");
    assert_eq!(
        row.best_medal,
        Some(Medal::Gold),
        "the standing medal survives"
    );
    assert_eq!(row.last_medal, None, "but the last race truly had none");
}

/// `Store::record` keeps `best_points` derived from `best_medal` rather than
/// letting the two drift - checked directly against the measured
/// gold/silver/bronze = 3/2/1 table.
#[test]
fn best_points_matches_the_medal_it_was_earned_by() {
    let mut store = Store::default();
    store.record(key("16_track"), observation_with_medal(Some(Medal::Silver)));
    assert_eq!(store.get(&key("16_track")).unwrap().best_points, Some(2));
}

/// The reason [`Medal`] carries its own `Serialize`/`Deserialize` rather
/// than deriving through its ordinal: a bare integer in the file would be
/// exactly the kind of value the in-race HUD's *opposite*-direction medal
/// ordinal could be misread against.
#[test]
fn a_medal_round_trips_as_a_lowercase_word_not_an_ordinal() {
    let mut store = Store::default();
    store.record(key("16_track"), observation_with_medal(Some(Medal::Gold)));
    let text = toml::to_string_pretty(&store).expect("serialises");
    assert!(
        text.contains("best_medal = \"gold\""),
        "expected a lowercase word in:\n{text}"
    );
    let (parsed, notes) = parse(&text).expect("parses back");
    assert!(
        notes.is_empty(),
        "a clean write should need no notes: {notes:?}"
    );
    assert_eq!(parsed.rows(), store.rows());
}
