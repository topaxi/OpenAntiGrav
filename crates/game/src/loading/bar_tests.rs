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
