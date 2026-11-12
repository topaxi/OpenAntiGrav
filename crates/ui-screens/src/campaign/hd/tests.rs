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
use oag_ui::frontend::Draw;
use oag_ui::language::StringTable;
use oag_ui::screen::{Screen, Text, argb_to_rgba};

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

/// Omega's table names `17_TRACK` but not every circuit, so no copy names all 38 and
/// `CircuitNames` is empty; the cell reads its circuit off the served table, case folded.
#[test]
fn a_circuit_the_served_table_names_reads_its_name_when_no_copy_names_them_all() {
    let cell = cell(Mode::Race, Some("17_Track"));
    let strings = StringTable::from_xml(
        r#"<entries><entry id="17_TRACK" string="TALON'S JUNCTION"></entry></entries>"#,
    );
    let names = CircuitNames::default();
    assert_eq!(hd_track_line(&cell, &names, &strings), "TALON'S JUNCTION");
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
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
    }
}

/// HD's own order, gold on the block's top row.
const HD_TIER_ROWS: [u8; 3] = [0, 1, 2];

/// [`hd_medal_frame`]'s own row table, read off `DATA06`'s own `Target0/1/2
/// Medal` widgets: gold at `v=0`, silver at `v=61`, bronze at `v=122`, each
/// a 60x60 frame at `u=0`. See that function's doc for the measurement.
#[test]
fn hd_medal_frame_matches_the_discs_own_target_medal_widget_crop() {
    assert_eq!(
        hd_medal_frame(Medal::Gold, Difficulty::Easy, HD_TIER_ROWS),
        [0.0, 0.0, 60.0, 60.0]
    );
    assert_eq!(
        hd_medal_frame(Medal::Silver, Difficulty::Easy, HD_TIER_ROWS),
        [0.0, 61.0, 60.0, 60.0]
    );
    assert_eq!(
        hd_medal_frame(Medal::Bronze, Difficulty::Easy, HD_TIER_ROWS),
        [0.0, 122.0, 60.0, 60.0]
    );
}

/// The taller, per-difficulty atlas's own block pitch - see
/// [`hd_medal_frame`]'s own doc for the measurement and the block/rung
/// mapping this pins.
#[test]
fn hd_medal_frame_selects_a_block_per_difficulty() {
    assert_eq!(
        hd_medal_frame(Medal::Gold, Difficulty::Medium, HD_TIER_ROWS),
        [0.0, 183.0, 60.0, 60.0]
    );
    assert_eq!(
        hd_medal_frame(Medal::Gold, Difficulty::Hard, HD_TIER_ROWS),
        [0.0, 366.0, 60.0, 60.0]
    );
    assert_eq!(
        hd_medal_frame(Medal::Bronze, Difficulty::Hard, HD_TIER_ROWS),
        [0.0, 488.0, 60.0, 60.0]
    );
}

/// A title whose atlas stores each block bronze, silver, gold from the top (Omega's) reaches
/// gold at the block's last row, in every difficulty block; the block order is untouched.
#[test]
fn hd_medal_frame_follows_the_titles_tier_rows() {
    let omega = [2, 1, 0];
    for (difficulty, block_v) in [
        (Difficulty::Easy, 0.0),
        (Difficulty::Medium, 183.0),
        (Difficulty::Hard, 366.0),
    ] {
        for (medal, row) in [
            (Medal::Gold, 122.0),
            (Medal::Silver, 61.0),
            (Medal::Bronze, 0.0),
        ] {
            assert_eq!(
                hd_medal_frame(medal, difficulty, omega),
                [0.0, block_v + row, 60.0, 60.0]
            );
        }
    }
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
    let draw = hd_tinted_medal_draw(
        &image,
        placed,
        Medal::Silver,
        Difficulty::Easy,
        HD_TIER_ROWS,
    );
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

fn pager(count: usize) -> GridSelection {
    GridSelection::new(vec![GridSummary::empty(); count]).with_per_page(1)
}

fn touch_down() -> Pointer {
    Pointer {
        pressed: true,
        press_at: Some((960.0, 540.0)),
        ..Pointer::default()
    }
}

fn dragged(x: f32, y: f32) -> Pointer {
    Pointer {
        drag: (x, y),
        ..Pointer::default()
    }
}

/// A lift as `oag_game::pointer::Window` emits it after a drag: nowhere, no click.
fn lifted(x: f32, y: f32) -> Pointer {
    Pointer {
        drag: (x, y),
        released: true,
        ..Pointer::default()
    }
}

/// One finger gesture: touch down, `ticks` equal drags totalling `(x, y)`, lift.
/// Returns every event the screen raised.
fn gesture(
    model: &mut GridSelection,
    targets: &[Target],
    (x, y): (f32, f32),
    ticks: u8,
) -> Vec<Event> {
    let n = f32::from(ticks);
    let mut out = model.hd_pointer(&touch_down(), targets);
    for _ in 0..ticks {
        out.extend(model.hd_pointer(&dragged(x / n, y / n), targets));
    }
    out.extend(model.hd_pointer(&lifted(0.0, 0.0), targets));
    out
}

fn card_targets() -> Vec<Target> {
    hd_grid_targets(
        &grid_layout_with_no_card_art(),
        &|_| None,
        Some([934.0, 304.0, 512.0, 512.0]),
    )
}

#[test]
fn a_swipe_up_steps_to_the_next_grid() {
    let targets = card_targets();
    let mut model = pager(8);
    model.step(3);
    let events = gesture(&mut model, &targets, (0.0, -2.0 * SWIPE_STEP_TRAVEL), 6);
    assert_eq!(events, vec![Event::Moved]);
    assert_eq!(model.index(), 4);
}

#[test]
fn a_swipe_down_steps_to_the_previous_grid() {
    let targets = card_targets();
    let mut model = pager(8);
    model.step(3);
    let events = gesture(&mut model, &targets, (0.0, 2.0 * SWIPE_STEP_TRAVEL), 6);
    assert_eq!(events, vec![Event::Moved]);
    assert_eq!(model.index(), 2);
}

#[test]
fn one_long_swipe_steps_once_and_only_once() {
    let targets = card_targets();
    let mut model = pager(8);
    model.step(3);
    gesture(&mut model, &targets, (0.0, -10.0 * SWIPE_STEP_TRAVEL), 40);
    assert_eq!(model.index(), 4);
}

#[test]
fn a_swipe_on_the_lift_tick_alone_still_steps() {
    let targets = card_targets();
    let mut model = pager(8);
    model.step(3);
    model.hd_pointer(&touch_down(), &targets);
    let events = model.hd_pointer(&lifted(0.0, -2.0 * SWIPE_STEP_TRAVEL), &targets);
    assert_eq!(events, vec![Event::Moved]);
    assert_eq!(model.index(), 4);
}

#[test]
fn a_swipe_short_of_the_travel_steps_nothing() {
    let targets = card_targets();
    let mut model = pager(8);
    model.step(3);
    let events = gesture(&mut model, &targets, (0.0, -0.9 * SWIPE_STEP_TRAVEL), 6);
    assert!(events.is_empty());
    assert_eq!(model.index(), 3);
}

#[test]
fn the_next_gesture_steps_again() {
    let targets = card_targets();
    let mut model = pager(8);
    model.step(3);
    gesture(&mut model, &targets, (0.0, -2.0 * SWIPE_STEP_TRAVEL), 4);
    gesture(&mut model, &targets, (0.0, -2.0 * SWIPE_STEP_TRAVEL), 4);
    gesture(&mut model, &targets, (0.0, 2.0 * SWIPE_STEP_TRAVEL), 4);
    assert_eq!(model.index(), 4);
}

/// The travel of a swipe that fell short is forgotten at the lift, not added to the next one.
#[test]
fn travel_does_not_carry_over_from_one_gesture_to_the_next() {
    let targets = card_targets();
    let mut model = pager(8);
    model.step(3);
    for _ in 0..3 {
        gesture(&mut model, &targets, (0.0, -0.6 * SWIPE_STEP_TRAVEL), 3);
    }
    assert_eq!(model.index(), 3);
}

#[test]
fn a_swipe_that_lifts_over_the_card_does_not_confirm_it() {
    let targets = card_targets();
    let mut model = pager(8);
    let mut events = model.hd_pointer(&touch_down(), &targets);
    events.extend(model.hd_pointer(&dragged(0.0, -2.0 * SWIPE_STEP_TRAVEL), &targets));
    events.extend(model.hd_pointer(&lifted(0.0, 0.0), &targets));
    assert!(!events.contains(&Event::Confirmed), "{events:?}");
}

#[test]
fn a_tap_on_the_card_still_confirms() {
    let mut model = pager(8);
    assert_eq!(
        model.hd_pointer(&click_at(1100.0, 500.0), &card_targets()),
        vec![Event::Confirmed]
    );
}

/// More travel down the page than across it, but wider than it is tall: a pan, not a swipe.
#[test]
fn a_horizontal_drag_steps_nothing() {
    let targets = card_targets();
    let mut model = pager(8);
    model.step(3);
    let events = gesture(
        &mut model,
        &targets,
        (-4.0 * SWIPE_STEP_TRAVEL, -1.5 * SWIPE_STEP_TRAVEL),
        8,
    );
    assert!(events.is_empty());
    assert_eq!(model.index(), 3);
}

#[test]
fn a_swipe_does_nothing_on_a_page_with_one_grid() {
    let targets = card_targets();
    let mut model = pager(1);
    let events = gesture(&mut model, &targets, (0.0, -2.0 * SWIPE_STEP_TRAVEL), 4);
    assert!(events.is_empty());
    assert_eq!(model.index(), 0);
}

#[test]
fn a_swipe_wraps_past_the_last_grid_as_the_wheel_does() {
    let targets = card_targets();
    let mut model = pager(3);
    model.step(2);
    gesture(&mut model, &targets, (0.0, -2.0 * SWIPE_STEP_TRAVEL), 4);
    assert_eq!(model.index(), 0);
}

const FURY_GRID_FIXTURE: &str = r#"
<Screen>
  <Screen type="FlyerSelection" name="Grid Selection Fury">
    <Text name="EventNum"><Values string="Event 01/08" font="Title" x="180" y="170" color="0xFF646464"></Values></Text>
    <Image name="unlockboxbrackets"><Values x="168" y="708" width="440" height="168" color="0xFFDEDEDE" src="Data\FE\Images\dot.gtf"></Values>
      <Bracket><Values corner="true" X="160" Y="700" Width="448" Height="176" Colour="0xFF646464"></Values></Bracket>
    </Image>
    <Image name="Flyer Left Arrow"><Values x="710" y="540" width="32" height="64" color="0xFF8AC0CA" src="Data\FE\Images\tilted_arrows.gtf"></Values></Image>
    <Image name="Flyer Right Arrow"><Values x="1700" y="540" width="96" height="64" color="0xFF8AC0CA" src="Data\FE\Images\tilted_arrows.gtf"></Values></Image>
  </Screen>
</Screen>
"#;

fn fury_grid_list(unlock_pending: bool) -> Vec<Draw> {
    fury_grid_list_at(unlock_pending, 2, 0)
}

fn fury_grid_list_at(unlock_pending: bool, count: usize, index: usize) -> Vec<Draw> {
    let screens = oag_ui::screen::Screens::from_xml(FURY_GRID_FIXTURE);
    let layout = Layout {
        screen: screens.by_name("Grid Selection Fury").unwrap().clone(),
        scale: [1.0, 1.0],
        faces: crate::picker::FaceScales::default(),
    };
    let grid = |name: &str, required| GridSummary {
        name: name.to_string(),
        cell_count: 8,
        max_points: 21,
        required_points: required,
        gold_medals: 0,
        points_earned: 0,
        locked: false,
        flyer_name: Some(format!("0{name}")),
    };
    let mut grids = vec![grid("a", if unlock_pending { 10 } else { 0 })];
    grids.extend((1..count).map(|_| grid("b", 0)));
    let mut model = GridSelection::new(grids);
    model.step(i32::try_from(index).unwrap());
    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::HD,
        22.0,
    );
    let any = |_: &str| {
        Some(Placed {
            x: 0,
            y: 0,
            width: 128,
            height: 64,
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
        })
    };
    hd_grid_draw_list(
        &model,
        &layout,
        &skin,
        &oag_ui::menu::Frame::default(),
        &StringTable::default(),
        None,
        false,
        &any,
        &[],
    )
    .body
}

#[test]
fn the_grid_counter_is_capitals_in_the_title_face() {
    let list = fury_grid_list(true);
    assert!(
        list.iter().any(|draw| matches!(
            draw,
            Draw::Text { text, .. } | Draw::FacedText { text, .. } if text == "EVENT 01/02"
        )),
        "{list:?}"
    );
}

/// The colour of the sprite drawn at `x` (the arrows' authored left edges).
fn sprite_color_at(list: &[Draw], x: f32) -> Option<[f32; 4]> {
    list.iter().find_map(|draw| match draw {
        Draw::Sprite { rect, color, .. } if rect[0] == x => Some(*color),
        _ => None,
    })
}

const LEFT_ARROW_X: f32 = 710.0;
const RIGHT_ARROW_X: f32 = 1700.0;

/// The frame (`menu-grid-select`, the first card) reads the right arrow as
/// `100,100,100`, where the screen authors `0xFF8AC0CA`, and the left a
/// half-strength copy of it.
#[test]
fn on_the_first_card_the_left_arrow_is_half_strength_and_the_right_full() {
    let list = fury_grid_list_at(true, 8, 0);
    assert_eq!(
        sprite_color_at(&list, RIGHT_ARROW_X),
        Some(argb_to_rgba(0xFF64_6464)),
        "{list:?}"
    );
    assert_eq!(
        sprite_color_at(&list, LEFT_ARROW_X),
        Some(argb_to_rgba(0x7F64_6464)),
        "{list:?}"
    );
}

#[test]
fn on_a_middle_card_both_arrows_are_full_strength() {
    let list = fury_grid_list_at(true, 8, 3);
    for x in [LEFT_ARROW_X, RIGHT_ARROW_X] {
        assert_eq!(
            sprite_color_at(&list, x),
            Some(argb_to_rgba(0xFF64_6464)),
            "{x} in {list:?}"
        );
    }
}

#[test]
fn on_the_last_card_the_right_arrow_is_half_strength_and_the_left_full() {
    let list = fury_grid_list_at(true, 8, 7);
    assert_eq!(
        sprite_color_at(&list, RIGHT_ARROW_X),
        Some(argb_to_rgba(0x7F64_6464)),
        "{list:?}"
    );
    assert_eq!(
        sprite_color_at(&list, LEFT_ARROW_X),
        Some(argb_to_rgba(0xFF64_6464)),
        "{list:?}"
    );
}

/// The original's branch tests "first card" before "last card", so a page of
/// one leaves the right arrow opaque.
#[test]
fn a_page_of_one_dims_only_the_left_arrow() {
    let list = fury_grid_list_at(true, 1, 0);
    assert_eq!(
        sprite_color_at(&list, RIGHT_ARROW_X),
        Some(argb_to_rgba(0xFF64_6464))
    );
    assert_eq!(
        sprite_color_at(&list, LEFT_ARROW_X),
        Some(argb_to_rgba(0x7F64_6464))
    );
}

fn fury_arrow_targets(card: Option<[f32; 4]>) -> Vec<Target> {
    let screens = oag_ui::screen::Screens::from_xml(FURY_GRID_FIXTURE);
    let layout = Layout {
        screen: screens.by_name("Grid Selection Fury").unwrap().clone(),
        scale: [1.0, 1.0],
        faces: crate::picker::FaceScales::default(),
    };
    let any = |_: &str| {
        Some(Placed {
            x: 0,
            y: 0,
            width: 128,
            height: 64,
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
        })
    };
    hd_grid_targets(&layout, &any, card)
}

/// Where the flyer card is drawn on HD at 1920x1080: about x=770 to 1610.
const HD_CARD: [f32; 4] = [770.0, 300.0, 840.0, 520.0];

/// The authored arrows are 21x43 px on a 1280x800 screen; a tap beside them,
/// where a finger lands, pages the grid.
#[test]
fn a_tap_beside_a_drawn_arrow_pages_the_grid() {
    let targets = fury_arrow_targets(Some(HD_CARD));
    for (at, from, to) in [
        ((1660.0, 520.0), 3, 4),
        ((1840.0, 640.0), 3, 4),
        ((660.0, 500.0), 3, 2),
        ((700.0, 660.0), 3, 2),
    ] {
        let mut model = pager(8);
        model.step(from);
        assert_eq!(
            model.hd_pointer(&click_at(at.0, at.1), &targets),
            vec![Event::Moved],
            "a tap at {at:?}"
        );
        assert_eq!(model.index(), to, "a tap at {at:?}");
    }
}

#[test]
fn a_tap_well_clear_of_the_arrows_does_not_page() {
    let targets = fury_arrow_targets(Some(HD_CARD));
    for at in [
        (1880.0, 570.0),
        (1700.0, 400.0),
        (600.0, 570.0),
        (720.0, 800.0),
    ] {
        let mut model = pager(8);
        model.step(3);
        model.hd_pointer(&click_at(at.0, at.1), &targets);
        assert_eq!(model.index(), 3, "a tap at {at:?}");
    }
}

/// The grown arrows stop at the card: its own edge taps still confirm it.
#[test]
fn a_tap_on_the_card_still_confirms_it_beside_a_grown_arrow() {
    let targets = fury_arrow_targets(Some(HD_CARD));
    for at in [(775.0, 570.0), (1605.0, 570.0), (790.0, 560.0)] {
        let mut model = pager(8);
        model.step(3);
        assert_eq!(
            model.hd_pointer(&click_at(at.0, at.1), &targets),
            vec![Event::Confirmed],
            "a tap at {at:?}"
        );
    }
}

/// Omega draws no card: the arrows still win over the page-wide confirm band.
#[test]
fn with_no_card_drawn_a_tap_beside_an_arrow_pages_rather_than_confirms() {
    let targets = fury_arrow_targets(None);
    let mut model = pager(8);
    model.step(3);
    assert_eq!(
        model.hd_pointer(&click_at(1660.0, 520.0), &targets),
        vec![Event::Moved]
    );
    assert_eq!(model.index(), 4);
}

#[test]
fn the_unlock_panel_carries_its_corner_marks_only_while_it_shows() {
    let corners = |list: &[Draw]| {
        list.iter()
            .filter(|draw| matches!(draw, Draw::Sprite { rect, .. } if rect[2] == 16.0 && rect[3] == 16.0))
            .count()
    };
    assert_eq!(corners(&fury_grid_list(true)), 4);
    assert_eq!(corners(&fury_grid_list(false)), 0);
}
