use super::*;

/// A craft that finished, and one still going round.
fn finisher(slot: u8, place: u8, tick: u64) -> Craft {
    Craft {
        slot,
        place,
        lap: 4,
        finish_tick: Some(tick),
    }
}

fn racer(slot: u8, place: u8, lap: u32) -> Craft {
    Craft {
        slot,
        place,
        lap,
        finish_tick: None,
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
        Some(3_050),
    )
}

#[test]
fn rows_come_out_in_finishing_order() {
    let board = build(
        &[racer(0, 3, 3), finisher(1, 1, 100), finisher(2, 2, 200)],
        Some(3),
        300,
        None,
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
    let board = build(&[finisher(0, 1, 500)], Some(3), 500, None);
    assert_eq!(board.rows[0].laps_completed, 3);
    assert!(board.rows[0].finished());
}

#[test]
fn a_craft_still_racing_is_credited_with_the_laps_it_drove() {
    let board = build(&[racer(0, 1, 3)], Some(3), 500, None);
    assert_eq!(board.rows[0].laps_completed, 2);
    assert!(!board.rows[0].finished());
}

/// A mode with no lap target - a speed lap - still tables what everyone drove.
#[test]
fn a_mode_that_never_ends_still_counts_laps() {
    let board = build(&[racer(0, 1, 7)], None, 500, None);
    assert_eq!(board.rows[0].laps_completed, 6);
    assert_eq!(board.laps_target, None);
}

/// The place is what `oag_race::places` assigned; this must not re-derive it,
/// or the table and the HUD's position readout could disagree.
#[test]
fn the_place_column_is_the_one_it_was_given() {
    let board = build(&[racer(0, 4, 9), racer(1, 1, 1)], Some(3), 10, None);
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
    let list = draw_list(&board, 12.0);
    assert!(matches!(list.first(), Some(Draw::Fill { .. })));
    let texts = texts(&list);
    assert!(texts.contains(&TITLE.to_string()));
    assert!(texts.contains(&"YOU".to_string()));
    assert!(texts.contains(&"SLOT 2".to_string()));
    assert!(texts.contains(&"SLOT 4".to_string()));
    // Four columns per craft, on top of the heading, four captions, two footers
    // and the hint.
    assert_eq!(texts.len(), 1 + 4 + board.rows.len() * 4 + 2 + 1);
}

#[test]
fn a_craft_that_did_not_finish_has_no_time() {
    let list = draw_list(&build(&[racer(0, 1, 2)], Some(3), 600, None), 12.0);
    assert!(texts(&list).contains(&"-".to_string()));
}

#[test]
fn a_finisher_shows_its_crossing_as_a_race_time() {
    let list = draw_list(&build(&[finisher(0, 1, 3_600)], Some(3), 3_600, None), 12.0);
    // 3,600 ticks at 60 Hz is one minute exactly, written the way the original
    // writes a time.
    assert!(texts(&list).contains(&"1.00.00".to_string()));
}

#[test]
fn the_footer_reports_the_best_lap_and_the_race_time() {
    let list = draw_list(&board(), 12.0);
    let texts = texts(&list);
    assert!(texts.iter().any(|text| text.starts_with("BEST LAP 0.50.")));
    assert!(texts.iter().any(|text| text.starts_with("RACE TIME 2.36.")));
}

#[test]
fn an_unset_best_lap_is_shown_as_none_rather_than_as_zero() {
    let list = draw_list(&build(&[racer(0, 1, 1)], Some(3), 10, None), 12.0);
    assert!(texts(&list).contains(&"BEST LAP -".to_string()));
}

/// A time trial, a speed lap and a Zone run grid the player alone, and `1 of 1`
/// is arithmetic rather than a standing - which is the rule `hud::Readout`
/// already applies to the position widget. The two must not disagree.
#[test]
fn a_field_of_one_shows_no_place_at_all() {
    let board = build(&[finisher(0, 1, 3_000)], Some(3), 3_000, Some(1_000));
    assert!(!board.shows_place());
    let texts = texts(&draw_list(&board, 12.0));
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
    let texts = texts(&draw_list(&board, 12.0));
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
    let board = build(&crafts, Some(3), 12_000, Some(3_000));
    let list = draw_list(&board, 12.0);
    let Some(Draw::Fill { rect, .. }) = list.first() else {
        panic!("the panel is the first draw");
    };
    assert!(rect[0] >= 0.0 && rect[1] >= 0.0);
    assert!(rect[0] + rect[2] <= crate::frontend::SCREEN.0);
    assert!(
        rect[1] + rect[3] <= crate::frontend::SCREEN.1,
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
