//! Proves the join this pass exists for, end to end: a campaign cell's own
//! law (`oag_tables::race_campaign::Cell::evaluate_medal`) evaluates a
//! race's result, the caller converts the answer into
//! `oag_game::records::Medal`, `Store::record` merges it into a row, and a
//! `save`/`parse` round trip through the exact bytes `records.toml` would
//! hold gets the medal and its points back unchanged.
//!
//! This is *not* a claim that any real race is wired to a campaign cell
//! yet - it is not, see `Observation::campaign_medal`'s own doc and the
//! `campaign` handover thread. It demonstrates that the two pieces
//! `crates/formats/src/race_campaign.rs` and `crates/game/src/records.rs`
//! land tonight actually agree with each other end to end, with no
//! disc, no GPU and no simulation step - an invented `Cell`, per
//! `docs/architecture/adr/0006-no-copyrighted-content.md`, the same rule
//! `race_campaign`'s own unit tests already follow.

use oag_game::records::{Key, Medal, Observation, Store};
use oag_tables::race_campaign::{Cell, Mode};

/// A `Zone` cell, chosen deliberately over a `Race` one: it is the mode the
/// direction flip applies to, so a join test that used a non-counting mode
/// would not actually exercise the part of `evaluate_medal` this pass spent
/// its guard tests on.
fn zone_cell() -> Cell {
    Cell {
        name: "gridX_4_2".to_string(),
        track: Some("77_Track".to_string()),
        mode: Mode::Zone,
        class: "Zone".to_string(),
        weapons: false,
        damage: true,
        locked: None,
        status: None,
        ai_count: None,
        skill: None,
        skill_easy: None,
        skill_hard: None,
        laps: Some(0),
        ship: Some("None".to_string()),
        ship_choice: Some(true),
        gold: 20,
        silver: 17,
        bronze: 15,
        tournament_tracks: Vec::new(),
        // Neither is Pulse's own shape - see `race_campaign`'s HD section.
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

/// A `Head2Head` cell - win or nothing, the flat law all 23 authored cells
/// carry (`docs/ghidra/functions/psp-pulse-usa/head2head.md`): gold target
/// `1`, silver and bronze both `0`.
fn head2head_cell() -> Cell {
    Cell {
        name: "gridX_5_2".to_string(),
        track: Some("18_Track".to_string()),
        mode: Mode::Head2Head,
        class: "Flash".to_string(),
        weapons: false,
        damage: true,
        locked: None,
        status: None,
        ai_count: Some(1),
        skill: None,
        skill_easy: None,
        skill_hard: None,
        laps: Some(4),
        ship: Some("None".to_string()),
        ship_choice: Some(true),
        gold: 1,
        silver: 0,
        bronze: 0,
        tournament_tracks: Vec::new(),
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

/// Converts `oag_tables::race_campaign::Medal` to `oag_game::records::Medal`,
/// a plain match rather than a shared type or a trait impl on purpose: see
/// `records.rs`'s own module doc for why it stays free of an `oag-formats`
/// import. This is exactly the conversion a real call site (once one
/// selects a campaign cell) would write.
fn to_records_medal(medal: oag_tables::race_campaign::Medal) -> Medal {
    match medal {
        oag_tables::race_campaign::Medal::Gold => Medal::Gold,
        oag_tables::race_campaign::Medal::Silver => Medal::Silver,
        oag_tables::race_campaign::Medal::Bronze => Medal::Bronze,
    }
}

#[test]
fn a_zone_cells_medal_evaluates_persists_and_round_trips() {
    let cell = zone_cell();

    // 16 zones: bronze, only because the comparison is flipped for Zone -
    // the same guard case `race_campaign`'s own tests cover, evaluated here
    // through the exact path a real capture would use.
    let medal = cell.evaluate_medal(16).expect("16 zones clears bronze");
    assert_eq!(medal, oag_tables::race_campaign::Medal::Bronze);

    let mut store = Store::default();
    let key = Key::new("pulse", cell.track.as_deref(), "zone", "zone");
    store.record(
        key.clone(),
        Observation {
            finished: false,
            place: None,
            laps_completed: 0,
            tick: 4000,
            best_lap_ticks: None,
            campaign_medal: Some(to_records_medal(medal)),
        },
    );

    let row = store.get(&key).expect("just recorded");
    assert_eq!(row.best_medal, Some(Medal::Bronze));
    assert_eq!(
        row.best_points,
        Some(medal.points()),
        "records::Medal::points must agree with race_campaign::Medal::points"
    );

    // The exact round trip `records::save`/`records::load` perform, without
    // touching a real file - `oag_game::records::parse` on
    // `toml::to_string_pretty`'s own output.
    let text = toml::to_string_pretty(&store).expect("serialises");
    let (parsed, notes) = oag_game::records::parse(&text).expect("parses back");
    assert!(
        notes.is_empty(),
        "a clean write should need no notes: {notes:?}"
    );
    let round_tripped = parsed.get(&key).expect("survives the round trip");
    assert_eq!(round_tripped.best_medal, Some(Medal::Bronze));
    assert_eq!(round_tripped.best_points, Some(1));
}

/// Win or nothing, exercised through the same `evaluate_medal` path every
/// other campaign mode uses: 1st earns gold, 2nd earns no medal at all -
/// silver and bronze targets of `0` can never match a finishing position,
/// which is always `>= 1`.
#[test]
fn a_head2head_cells_medal_is_win_or_nothing() {
    let cell = head2head_cell();

    let first = cell.evaluate_medal(1).expect("1st place earns a medal");
    assert_eq!(first, oag_tables::race_campaign::Medal::Gold);

    assert_eq!(
        cell.evaluate_medal(2),
        None,
        "2nd place in a two-craft race earns nothing"
    );
}

/// A gold Zone run must not read as bronze - the mirror of the guard test
/// above, joined through the same persistence path.
#[test]
fn a_gold_zone_run_persists_as_gold_not_bronze() {
    let cell = zone_cell();
    let medal = cell.evaluate_medal(20).expect("20 zones is gold");
    assert_eq!(medal, oag_tables::race_campaign::Medal::Gold);

    let mut store = Store::default();
    let key = Key::new("pulse", cell.track.as_deref(), "zone", "zone");
    store.record(
        key.clone(),
        Observation {
            finished: false,
            place: None,
            laps_completed: 0,
            tick: 4000,
            best_lap_ticks: None,
            campaign_medal: Some(to_records_medal(medal)),
        },
    );

    let row = store.get(&key).expect("just recorded");
    assert_eq!(row.best_medal, Some(Medal::Gold));
    assert_eq!(row.best_points, Some(3));
}
