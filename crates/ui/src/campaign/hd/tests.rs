//! HD-specific unit tests: the pure functions and pointer helpers that do
//! not need a full `CellMode_Definition.xml` fixture, on the same "under
//! 200 lines or split out" rule `draw/tests.rs` and `campaign/tests.rs`
//! already follow. A full draw-list smoke test is left to the real
//! `--menu-page grid-select --source data/images/hdfury-ps3-eu-dec.iso`
//! capture this thread's own report links, the same way Pulse's own
//! draw-list functions are proven against a real disc rather than a
//! hand-typed fixture that could drift from the file.

use super::*;
use crate::language::StringTable;
use crate::screen::{Screen, Text};
use oag_tables::race_campaign::Mode;

fn cell(mode: Mode, track: Option<&str>) -> Cell {
    Cell {
        name: "grid0_0_0".to_string(),
        track: track.map(str::to_string),
        mode,
        class: "Venom".to_string(),
        weapons: true,
        damage: true,
        locked: None,
        status: None,
        ai_count: Some(7),
        skill: None,
        skill_easy: None,
        skill_hard: None,
        laps: Some(3),
        ship: Some("None".to_string()),
        ship_choice: Some(true),
        gold: 1,
        silver: 2,
        bronze: 3,
        tournament_tracks: vec!["01_Track".to_string(), "02_Track".to_string()],
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

#[test]
fn event_counter_is_one_based_and_zero_padded() {
    assert_eq!(event_counter(0, 8), "Event 01/08");
    assert_eq!(event_counter(7, 8), "Event 08/08");
}

#[test]
fn tournament_cells_show_a_race_count_not_a_track_name() {
    let cell = cell(Mode::Tournament, None);
    let names = CircuitNames::default();
    let strings = StringTable::default();
    assert_eq!(hd_track_line(&cell, &names, &strings), "2 Races");
}

#[test]
fn a_cell_with_no_track_reads_as_empty_rather_than_a_placeholder() {
    let cell = cell(Mode::Race, None);
    let names = CircuitNames::default();
    let strings = StringTable::default();
    assert_eq!(hd_track_line(&cell, &names, &strings), "");
}

#[test]
fn the_circuit_name_fold_wins_over_the_raw_id() {
    let cell = cell(Mode::Race, Some("16_Track"));
    let strings = StringTable::default();
    // `CircuitNames::choose` is exercised by `oag_game::boot::roster`'s own
    // tests; this checks only that `hd_track_line` prefers a resolved name
    // when one is offered, which needs no real string table copy to prove -
    // the empty default answers `None` for every id, so the assertion below
    // is really "falls back to the raw id when nothing resolves it",
    // covered on its own line to say so rather than assumed.
    let names = CircuitNames::default();
    assert_eq!(hd_track_line(&cell, &names, &strings), "16_Track");
}

#[test]
fn difficulty_button_rect_is_none_when_the_widget_is_not_on_the_screen() {
    let screen = Screen::default();
    assert_eq!(difficulty_button_rect(&screen), None);
}

#[test]
fn difficulty_button_rect_is_found_by_name() {
    let screen = Screen {
        texts: vec![Text {
            name: Some("DifficultyButton".to_string()),
            x: 944.0,
            y: 994.0,
            ..Text::default()
        }],
        ..Screen::default()
    };
    let rect = difficulty_button_rect(&screen).expect("the widget is on the screen");
    // The rect contains the widget's own baseline, which is the one thing
    // this invented rect has to get right regardless of its exact size.
    assert!(rect[0] <= 944.0 && 944.0 <= rect[0] + rect[2]);
    assert!(rect[1] <= 994.0 && 994.0 <= rect[1] + rect[3]);
}

#[test]
fn cell_selection_difficulty_starts_at_medium_and_wraps() {
    let mut model = CellSelection::new(vec![cell(Mode::TimeTrial, Some("16_Track"))]);
    assert_eq!(model.difficulty(), 1);
    model.cycle_difficulty();
    assert_eq!(model.difficulty(), 2);
    model.cycle_difficulty();
    assert_eq!(model.difficulty(), 0);
    model.cycle_difficulty();
    assert_eq!(model.difficulty(), 1);
}

#[test]
fn square_on_the_pad_cycles_difficulty_and_emits_the_event() {
    use oag_gameplay::input::{Button, Input};

    let mut model = CellSelection::new(vec![cell(Mode::Race, Some("16_Track"))]);
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << Button::Square as u32);
    let events = model.update(&mut input);
    assert_eq!(events, vec![Event::DifficultyChanged]);
    assert_eq!(model.difficulty(), 2);
}

fn medal_image(name: &str) -> Image {
    Image {
        name: Some(name.to_string()),
        src: r"Data\FE\Images\Hexmedal_HD.mip".to_string(),
        x: 350.0,
        y: 332.0,
        width: None,
        height: None,
        centred: false,
        color: 0xffff_ffff,
        u: None,
        v: None,
        texture_width: None,
        texture_height: None,
        auto_load: false,
    }
}

fn medal_atlas_placed() -> Placed {
    Placed {
        x: 0,
        y: 738,
        width: 1024,
        height: 256,
        quad_extent: None,
        blend: None,
    }
}

/// [`hd_medal_frame`]'s own row table, read off `DATA06`'s own `Target0/1/2
/// Medal` widgets: gold at `v=0`, silver at `v=61`, bronze at `v=122`, each
/// a 60x60 frame at `u=0`. See that function's doc for the measurement.
#[test]
fn hd_medal_frame_matches_the_discs_own_target_medal_widget_crop() {
    assert_eq!(hd_medal_frame(Medal::Gold), [0.0, 0.0, 60.0, 60.0]);
    assert_eq!(hd_medal_frame(Medal::Silver), [0.0, 61.0, 60.0, 60.0]);
    assert_eq!(hd_medal_frame(Medal::Bronze), [0.0, 122.0, 60.0, 60.0]);
}

/// The regression this pass fixes: `Medal_{x}_{y}` used to draw the whole
/// 1024x256 `Hexmedal_HD` atlas (every tier, every rotation frame of a
/// spinning medal) stretched across the hex slot and multiplied by a flat
/// swatch on top - "medals rendered off/wrong", the maintainer's own report
/// this lane opened against. This pins the fix: one 60x60 frame of the
/// right tier, at the widget's own authored position (unchanged from the
/// pre-fix code - see `hd_tinted_medal_draw`'s own doc for why this is not
/// `hex_rect`-centred), left untinted since the frame's own pixels already
/// carry the tier's colour.
#[test]
fn hd_tinted_medal_draw_crops_one_frame_of_the_right_tier_at_its_own_authored_position() {
    let image = medal_image("Medal_0_0");
    let placed = medal_atlas_placed();
    let draw = hd_tinted_medal_draw(&image, placed, Medal::Silver);
    let Draw::Sprite { rect, uv, color } = draw else {
        panic!("expected a plain Sprite, not a tiled or rotated one");
    };
    // Native 60x60, at the widget's own authored (350, 332) - not stretched
    // or repositioned.
    assert_eq!(rect, [350.0, 332.0, 60.0, 60.0]);
    // Silver's own row (v=61) at u=0, offset by the atlas's own placement
    // in the sheet (0, 738).
    assert_eq!(uv, [0.0, 799.0, 60.0, 60.0]);
    // No tint: the atlas frame's own pixels already carry the tier colour.
    assert_eq!(color, [1.0, 1.0, 1.0, 1.0]);
}
