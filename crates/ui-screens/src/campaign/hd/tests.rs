//! HD-specific unit tests: the pure functions and pointer helpers that do
//! not need a full `CellMode_Definition.xml` fixture, on the same "under
//! 200 lines or split out" rule `draw/tests.rs` and `campaign/tests.rs`
//! already follow. A full draw-list smoke test is left to the real
//! `--menu-page grid-select --source data/images/hdfury-ps3-eu-dec.iso`
//! capture this thread's own report links, the same way Pulse's own
//! draw-list functions are proven against a real disc rather than a
//! hand-typed fixture that could drift from the file.

use super::*;
use oag_tables::race_campaign::{DifficultyTargets, MedalTargets, Mode};
use oag_ui::language::StringTable;
use oag_ui::screen::{Screen, Text};

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
    assert_eq!(model.difficulty(), Difficulty::Medium);
    model.cycle_difficulty();
    assert_eq!(model.difficulty(), Difficulty::Hard);
    model.cycle_difficulty();
    assert_eq!(model.difficulty(), Difficulty::Easy);
    model.cycle_difficulty();
    assert_eq!(model.difficulty(), Difficulty::Medium);
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
    assert_eq!(model.difficulty(), Difficulty::Hard);
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
        reveal: Vec::new(),
        transition: 0.0,
        start_enabled: true,
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
    assert_eq!(
        hd_medal_frame(Medal::Gold, Difficulty::Easy),
        [0.0, 0.0, 60.0, 60.0]
    );
    assert_eq!(
        hd_medal_frame(Medal::Silver, Difficulty::Easy),
        [0.0, 61.0, 60.0, 60.0]
    );
    assert_eq!(
        hd_medal_frame(Medal::Bronze, Difficulty::Easy),
        [0.0, 122.0, 60.0, 60.0]
    );
}

/// The taller, per-difficulty atlas's own block pitch - see
/// [`hd_medal_frame`]'s own doc for the measurement and the block/rung
/// mapping this pins.
#[test]
fn hd_medal_frame_selects_a_block_per_difficulty() {
    assert_eq!(
        hd_medal_frame(Medal::Gold, Difficulty::Medium),
        [0.0, 183.0, 60.0, 60.0]
    );
    assert_eq!(
        hd_medal_frame(Medal::Gold, Difficulty::Hard),
        [0.0, 366.0, 60.0, 60.0]
    );
    assert_eq!(
        hd_medal_frame(Medal::Bronze, Difficulty::Hard),
        [0.0, 488.0, 60.0, 60.0]
    );
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
    let draw = hd_tinted_medal_draw(&image, placed, Medal::Silver, Difficulty::Easy);
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

/// `Target0/1/2 Medal` needs no `hd_medal_frame` help at all - unlike
/// `Medal_{x}_{y}`, `DATA06`'s own widget authors its own crop
/// (`width="60" height="60" u="0" v="61" TxtrWidth="60" TxtrHeight="60"` for
/// `Target1 Medal`), so the plain [`image_draw`] path already reads it
/// through `sprite_draw`. Pins the geometry this pass switched to drawing
/// for real (see the module doc's "The winning archive" section) against
/// the exact numbers the `DATA06` campaign-selection XML authors.
#[test]
fn a_target_medal_widget_crops_through_the_plain_image_path_with_no_extra_help() {
    let image = Image {
        name: Some("Target1 Medal".to_string()),
        src: r"Data\FE\Images\Hexmedal_HD.gtf".to_string(),
        x: 21.0,
        y: 2.0,
        width: Some(60.0),
        height: Some(60.0),
        centred: false,
        color: 0xffff_ffff,
        u: Some(0.0),
        v: Some(61.0),
        texture_width: Some(60.0),
        texture_height: Some(60.0),
        auto_load: false,
        reveal: Vec::new(),
        transition: 0.0,
        start_enabled: true,
    };
    let placed = medal_atlas_placed();
    let Draw::Sprite { rect, uv, color } = image_draw(&image, placed) else {
        panic!("expected a plain Sprite");
    };
    assert_eq!(rect, [21.0, 2.0, 60.0, 60.0]);
    assert_eq!(uv, [0.0, 799.0, 60.0, 60.0]);
    assert_eq!(color, [1.0, 1.0, 1.0, 1.0]);
}

fn placement_targets() -> MedalTargets {
    MedalTargets {
        gold: 1,
        silver: 2,
        bronze: 3,
    }
}

/// `hd_target_title`'s three measured shapes - one live RPCS3 frame each,
/// see that function's own doc for the capture paths and the reasoning
/// behind each part.
#[test]
fn target_title_reads_a_plain_target_with_the_difficulty_in_parens_for_race() {
    let mut cell = cell(Mode::Race, Some("17_Track"));
    cell.difficulty_targets = Some(DifficultyTargets {
        easy: placement_targets(),
        medium: placement_targets(),
        hard: placement_targets(),
    });
    cell.nitro_elimination_targets = Some((1, 1, 1));
    let strings = StringTable::default();
    assert_eq!(
        hd_target_title(&cell, &strings, Difficulty::Easy),
        "IG_HUD_TARGET (Easy)"
    );
}

#[test]
fn target_title_uses_its_own_idstring_for_speed_lap() {
    let mut cell = cell(Mode::SpeedLap, Some("19_Track"));
    cell.difficulty_targets = Some(DifficultyTargets {
        easy: placement_targets(),
        medium: placement_targets(),
        hard: placement_targets(),
    });
    cell.nitro_elimination_targets = Some((1, 1, 1));
    let strings = StringTable::default();
    assert_eq!(
        hd_target_title(&cell, &strings, Difficulty::Easy),
        "FE_TLTIME (Easy)"
    );
}

#[test]
fn target_title_inserts_the_nitro_target_only_for_elimination() {
    let mut cell = cell(Mode::Elimination, Some("19_Track"));
    cell.difficulty_targets = Some(DifficultyTargets {
        easy: placement_targets(),
        medium: placement_targets(),
        hard: placement_targets(),
    });
    cell.nitro_elimination_targets = Some((200, 200, 200));
    let strings = StringTable::default();
    assert_eq!(
        hd_target_title(&cell, &strings, Difficulty::Easy),
        "IG_HUD_TARGET 200 (Easy)"
    );
}

/// `Mode::Other("NitroBattle")` gets the same number-insertion `Elimination`
/// does - not from a capture of its own, but from
/// `campaign_grids_ground_truth.rs`'s own
/// `eliminationfamily_cells_carry_a_real_nitro_triple_and_a_dummy_flat_one`,
/// which ground-truths the pairing against the real disc. `"Detonator"`
/// keeps the plain default (that same ground-truth test's mirror case: a
/// real `difficulty_targets`, a dummy nitro triple).
#[test]
fn target_title_extends_the_nitro_target_to_nitrobattle_but_not_detonator() {
    let mut nitro_battle = cell(Mode::Other("NitroBattle".to_string()), Some("19_Track"));
    nitro_battle.difficulty_targets = Some(DifficultyTargets {
        easy: placement_targets(),
        medium: placement_targets(),
        hard: placement_targets(),
    });
    nitro_battle.nitro_elimination_targets = Some((12, 15, 20));
    let strings = StringTable::default();
    assert_eq!(
        hd_target_title(&nitro_battle, &strings, Difficulty::Easy),
        "IG_HUD_TARGET 12 (Easy)"
    );

    let mut detonator = cell(Mode::Other("Detonator".to_string()), Some("26_Track"));
    detonator.difficulty_targets = Some(DifficultyTargets {
        easy: MedalTargets {
            gold: 100_000,
            silver: 90_000,
            bronze: 80_000,
        },
        medium: placement_targets(),
        hard: placement_targets(),
    });
    detonator.nitro_elimination_targets = Some((1, 1, 1));
    assert_eq!(
        hd_target_title(&detonator, &strings, Difficulty::Easy),
        "IG_HUD_TARGET (Easy)"
    );
}

#[test]
fn target_title_has_no_difficulty_suffix_when_the_cell_authors_no_rung() {
    let cell = cell(Mode::Race, Some("17_Track"));
    let strings = StringTable::default();
    assert_eq!(
        hd_target_title(&cell, &strings, Difficulty::Medium),
        "IG_HUD_TARGET"
    );
}

/// `hd_target_value`'s three arms - the ordinal reading (measured for
/// `Race`/`Elimination`, chosen by documented equivalence for the rest of
/// its `_` arm), the excluded `Zone` case, and the HD-only time separator.
/// See that function's own doc for the capture evidence.
#[test]
fn target_value_reads_a_placement_as_an_ordinal() {
    let strings = StringTable::default();
    let race = cell(Mode::Race, Some("17_Track"));
    assert_eq!(hd_target_value(1, &race, &strings), "IG_HUD_1ST");
    assert_eq!(hd_target_value(2, &race, &strings), "IG_HUD_2ND");
    assert_eq!(hd_target_value(3, &race, &strings), "IG_HUD_3RD");
    let elimination = cell(Mode::Elimination, Some("19_Track"));
    assert_eq!(hd_target_value(1, &elimination, &strings), "IG_HUD_1ST");
}

#[test]
fn target_value_keeps_a_zone_count_as_a_plain_number() {
    let strings = StringTable::default();
    let zone = cell(Mode::Zone, Some("01_Track"));
    assert_eq!(hd_target_value(10, &zone, &strings), "10");
}

#[test]
fn target_value_formats_a_lap_time_with_periods_not_a_colon() {
    let strings = StringTable::default();
    let speed_lap = cell(Mode::SpeedLap, Some("19_Track"));
    assert_eq!(hd_target_value(4750, &speed_lap, &strings), "0.47.50");
    let time_trial = cell(Mode::TimeTrial, Some("08_Track"));
    assert_eq!(hd_target_value(6600, &time_trial, &strings), "1.06.00");
}

#[test]
fn target_value_reads_nitrobattle_as_an_ordinal_too_but_a_big_detonator_score_stays_a_number() {
    let strings = StringTable::default();
    let nitro_battle = cell(Mode::Other("NitroBattle".to_string()), Some("19_Track"));
    assert_eq!(hd_target_value(1, &nitro_battle, &strings), "IG_HUD_1ST");
    let detonator = cell(Mode::Other("Detonator".to_string()), Some("26_Track"));
    assert_eq!(hd_target_value(100_000, &detonator, &strings), "100000");
}

#[test]
fn difficulty_id_maps_the_three_rungs_in_order() {
    assert_eq!(hd_difficulty_id(Difficulty::Easy), "Easy");
    assert_eq!(hd_difficulty_id(Difficulty::Medium), "Medium");
    assert_eq!(hd_difficulty_id(Difficulty::Hard), "Hard");
}

#[test]
fn difficulty_button_line_reads_ai_difficulty_for_race_and_head2head() {
    let strings = StringTable::default();
    assert_eq!(
        hd_difficulty_button_line(&Mode::Race, Difficulty::Easy, &strings).as_deref(),
        Some("RB_AI_DIF (Easy)")
    );
    assert_eq!(
        hd_difficulty_button_line(&Mode::Head2Head, Difficulty::Hard, &strings).as_deref(),
        Some("RB_AI_DIF (Hard)")
    );
}

#[test]
fn difficulty_button_line_reads_bare_difficulty_for_target_threshold_modes() {
    let strings = StringTable::default();
    for mode in [
        Mode::TimeTrial,
        Mode::Zone,
        Mode::Elimination,
        Mode::SpeedLap,
        Mode::Other("NitroBattle".to_string()),
    ] {
        assert_eq!(
            hd_difficulty_button_line(&mode, Difficulty::Medium, &strings).as_deref(),
            Some("RB_DIF (Medium)"),
            "{mode:?}"
        );
    }
}

#[test]
fn difficulty_button_line_is_none_for_modes_the_button_is_not_measured_active_on() {
    let strings = StringTable::default();
    for mode in [Mode::Tournament, Mode::CustomGrid, Mode::AiRace] {
        assert_eq!(
            hd_difficulty_button_line(&mode, Difficulty::Medium, &strings),
            None,
            "{mode:?}"
        );
    }
}

fn grid_layout_with_no_card_art() -> Layout {
    Layout {
        screen: Screen::default(),
        scale: [1.0, 1.0],
        faces: crate::picker::FaceScales::default(),
    }
}

fn click_at(x: f32, y: f32) -> Pointer {
    Pointer {
        at: Some((x, y)),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

/// Omega draws no flyer card, so the confirm target cannot be the invisible
/// padlock rect it used to be: a click anywhere on the page confirms the one
/// tier on it.
#[test]
fn with_no_card_drawn_a_click_anywhere_on_the_page_confirms_the_tier() {
    let layout = grid_layout_with_no_card_art();
    let targets = hd_grid_targets(&layout, &|_| None, None);
    let mut model = GridSelection::new(vec![GridSummary::empty()]);
    for at in [(700.0, 533.0), (200.0, 300.0), (1700.0, 900.0)] {
        assert_eq!(
            model.hd_pointer(&click_at(at.0, at.1), &targets),
            vec![Event::Confirmed],
            "a click at {at:?}"
        );
    }
    assert!(
        model
            .hd_pointer(&click_at(700.0, 30.0), &targets)
            .is_empty(),
        "the header is not the page"
    );
}

/// With a card on screen the click region is the card, as before.
#[test]
fn with_a_card_drawn_only_the_card_confirms() {
    let layout = grid_layout_with_no_card_art();
    let card = [934.0, 304.0, 512.0, 512.0];
    let targets = hd_grid_targets(&layout, &|_| None, Some(card));
    let mut model = GridSelection::new(vec![GridSummary::empty()]);
    assert_eq!(
        model.hd_pointer(&click_at(1100.0, 500.0), &targets),
        vec![Event::Confirmed]
    );
    assert!(
        model
            .hd_pointer(&click_at(300.0, 500.0), &targets)
            .is_empty()
    );
}
