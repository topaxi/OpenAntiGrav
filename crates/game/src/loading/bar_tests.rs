//! The bar's dots and the five labels, off a hand-built screen.
//!
//! The disc-backed half, with the real `dot.gtf` and string table, is
//! `tests/loading_screen_ground_truth.rs`.

use super::tests::feature_screen_with;
use super::*;

fn tiled(list: &[Draw]) -> Vec<([f32; 4], [f32; 2])> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::TiledSprite { rect, repeat, .. } => Some((*rect, *repeat)),
            _ => None,
        })
        .collect()
}

/// The lit columns are whole columns of the fraction, and the two runs tile the
/// bar exactly: a fill that drifted off the grid would cut a dot in half.
#[test]
fn the_fill_lights_whole_columns_and_the_runs_tile_the_bar() {
    for (done, lit) in [(1, 83.0), (1, 83.0), (0, 0.0)] {
        let list = feature_screen_with(None).draw_list(
            Phase::Prefetch,
            &Progress {
                total: 2,
                done,
                ..Progress::default()
            },
            &Atlas::build(),
        );
        let runs = tiled(&list);
        let columns: f32 = runs.iter().map(|(_, repeat)| repeat[0]).sum();
        assert_eq!(columns, bar::COLUMNS, "{runs:?}");
        let lit_columns = runs.first().map_or(0.0, |(_, repeat)| repeat[0]);
        assert_eq!(if done == 0 { 0.0 } else { lit_columns }, lit, "{runs:?}");
    }
}

/// Dropping the dot falls back to flat rectangles rather than drawing no bar.
#[test]
fn no_dot_texture_means_a_flat_bar_not_a_missing_one() {
    let mut screen = feature_screen_with(None);
    screen.dot = None;
    let list = screen.draw_list(Phase::Race, &Progress::default(), &Atlas::build());
    assert!(tiled(&list).is_empty());
    assert!(list.iter().any(
        |draw| matches!(draw, Draw::Fill { rect, .. } if (rect[1] - BAR_BOX.1).abs() < f32::EPSILON)
    ));
}

/// Every label the title names is drawn, each behind a bullet; one the string
/// table did not carry is left out rather than drawn as its id.
#[test]
fn the_labels_are_drawn_behind_their_bullets_and_a_missing_one_is_skipped() {
    let labels = Labels {
        brand: "BRAND".to_string(),
        feature_image: Some("IMAGE".to_string()),
        feature_description: Some("DESC".to_string()),
        progression_bar: None,
        mode_icon: Some("ICON".to_string()),
    };
    let mut screen = feature_screen_with(Some(labels));
    screen.square = Some([0.0, 0.0, 8.0, 8.0]);
    let list = screen.draw_list(Phase::Race, &Progress::default(), &Atlas::build());
    let texts: Vec<&str> = list
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    for wanted in ["BRAND", "IMAGE", "DESC", "ICON"] {
        assert!(texts.contains(&wanted), "{wanted} in {texts:?}");
    }
    assert!(!texts.contains(&"PROGRESSION BAR"), "{texts:?}");
    let bullets = list
        .iter()
        .filter(
            |draw| matches!(draw, Draw::Sprite { rect, .. } if rect[2] == 4.0 && rect[3] == 4.0),
        )
        .count();
    assert_eq!(bullets, 4, "one bullet per label drawn");
}

/// Lit columns of a race load's bar, `0` while no column is lit.
fn lit_in_race(screen: &Screen) -> f32 {
    let list = screen.draw_list(Phase::Race, &Progress::default(), &Atlas::build());
    list.iter()
        .find_map(|draw| match draw {
            Draw::TiledSprite { rect, color, .. } if color[..3] == BAR_FILL[..3] => {
                Some((rect[2] / (BAR_BOX.2 / bar::COLUMNS)).round())
            }
            _ => None,
        })
        .unwrap_or(0.0)
}

/// HD's bar follows the load's own stages: a first target of a fifth that it
/// eases up to and holds, and each stage the load reports moves the target on.
/// Dropping the stage report leaves the bar parked at 33 columns.
#[test]
fn a_stage_driven_bar_holds_at_its_target_until_the_load_reports_a_stage() {
    let mut screen = feature_screen_with(None);
    screen.fill = Some(fill::Fill::new(oag_hd::loading::PROGRESSION));
    for _ in 0..100 {
        screen.advance(false);
    }
    assert!(
        (lit_in_race(&screen) - 10.0).abs() < f32::EPSILON,
        "0.1 a frame"
    );
    for _ in 0..1000 {
        screen.advance(false);
    }
    assert!(
        (lit_in_race(&screen) - 33.0).abs() < f32::EPSILON,
        "held at 20 %"
    );

    for stage in 1..=4 {
        screen.set_load_stage(stage);
        for _ in 0..2000 {
            screen.advance(false);
        }
        let expected =
            (oag_hd::loading::PROGRESSION.milestones[usize::from(stage) - 1] * 166.0).floor();
        assert!(
            (lit_in_race(&screen) - expected).abs() < f32::EPSILON,
            "stage {stage}: {} lit against {expected}",
            lit_in_race(&screen)
        );
    }

    screen.advance(true);
    assert!(
        (lit_in_race(&screen) - 166.0).abs() < f32::EPSILON,
        "full once ready"
    );
}

/// A title that names no progression keeps the time estimate, so Pulse's and
/// Pure's screens do not change.
#[test]
fn a_title_with_no_progression_keeps_the_time_estimate() {
    let mut screen = feature_screen_with(None);
    assert!(screen.fill.is_none());
    screen.set_load_stage(4);
    for _ in 0..240 {
        screen.advance(false);
    }
    let expected = ((1.0 - (-1.0f32).exp()) * 166.0).floor();
    assert!((lit_in_race(&screen) - expected).abs() <= 1.0);
}
