//! `Race End Photo`'s timing and draw list, off a miniature of the screen.
//! The disc-backed placement check is `tests/race_end_photo_ground_truth.rs`.

use super::*;
use crate::language::StringTable;
use crate::screen::Screens;

/// `InGame_Definition.xml`'s `Race End Photo`, as the expanded file spells it.
const XML: &str = r#"
<Screen name="Race End Photo">
<Text name="ProceedMessage" disabletransition="0">
<Values font="Stats" scale="1.0" align="left" y="242" x="20" idstring="FE_PRESS_TO_CONT" color="0xffffffff"></Values>
</Text>
<Text name="PhotoMessage" disabletransition="0" GSDisable="1">
<Values font="Stats" scale="1.0" align="left" y="212" x="20" idstring="ER_PRESS_SELECT_PHOTO" color="0xffffffff"></Values>
</Text>
</Screen>
"#;

fn layout(strings: &StringTable) -> Layout {
    Layout::read(
        &Screens::from_xml(XML),
        SCREEN,
        strings,
        crate::picker::FaceScales::default(),
        [480.0, 272.0],
    )
    .unwrap()
}

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Strings>
<Entry ID="FE_PRESS_TO_CONT" String="Press x to continue"></Entry>
<Entry ID="ER_PRESS_SELECT_PHOTO" String="Press SELECT button for Photo Mode"></Entry>
</Strings>"#,
    )
}

fn alphas(list: &[Draw]) -> Vec<f32> {
    list.iter()
        .map(|draw| match draw {
            Draw::Text { color, .. } => color[3],
            other => panic!("not text: {other:?}"),
        })
        .collect()
}

#[test]
fn the_view_is_clean_until_the_state_is_entered() {
    let layout = layout(&strings());
    assert!(photo_draw_list(&layout, 0).is_empty());
    assert!(photo_draw_list(&layout, ENTER_TICKS).is_empty());
    assert!(!entered(ENTER_TICKS - 1));
    assert!(entered(ENTER_TICKS));
    assert_eq!(photo_draw_list(&layout, ENTER_TICKS + 1).len(), 2);
}

#[test]
fn the_legend_ramps_straight_to_full_ink_and_stays() {
    assert_eq!(legend_alpha(ENTER_TICKS), 0.0);
    let half = legend_alpha(ENTER_TICKS + FADE_TICKS / 2);
    assert!((half - 0.5).abs() < 1e-6, "{half}");
    assert_eq!(legend_alpha(ENTER_TICKS + FADE_TICKS), 1.0);
    assert_eq!(legend_alpha(ENTER_TICKS + FADE_TICKS + 600), 1.0);
}

#[test]
fn both_lines_sit_where_the_screen_puts_them_in_the_order_it_lists_them() {
    let layout = layout(&strings());
    let list = photo_draw_list(&layout, ENTER_TICKS + FADE_TICKS);
    let places: Vec<(f32, f32, String)> = list
        .iter()
        .map(|draw| match draw {
            Draw::Text { x, y, text, .. } => (*x, *y, text.clone()),
            other => panic!("not text: {other:?}"),
        })
        .collect();
    assert_eq!(
        places,
        [
            (20.0, 242.0, "Press x to continue".to_string()),
            (
                20.0,
                212.0,
                "Press SELECT button for Photo Mode".to_string()
            ),
        ]
    );
    assert_eq!(alphas(&list), [1.0, 1.0]);
}

#[test]
fn a_string_that_does_not_resolve_draws_nothing_rather_than_its_id() {
    let only_one = StringTable::from_xml(
        r#"<Strings>
<Entry ID="FE_PRESS_TO_CONT" String="Press x to continue"></Entry>
</Strings>"#,
    );
    let list = photo_draw_list(&layout(&only_one), ENTER_TICKS + FADE_TICKS);
    assert_eq!(list.len(), 1);
}
