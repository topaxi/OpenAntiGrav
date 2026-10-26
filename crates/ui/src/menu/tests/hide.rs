//! A choice row with nothing to choose is left out, and a row greyed by another
//! setting is not: `super::super::hide`.

use super::*;

use crate::menu::pointer::{self, Part, hit};

const MUSIC: &str = "audio.music_source";
const VARIANT: &str = "race.variant";

fn rows_of(menu: &Menu, setting: &str) -> usize {
    menu.page()
        .entries
        .iter()
        .filter(|entry| entry.setting() == Some(setting))
        .count()
}

fn index_of(menu: &Menu, setting: &str) -> Option<usize> {
    menu.page()
        .entries
        .iter()
        .position(|entry| entry.setting() == Some(setting))
}

fn two() -> Vec<Choice> {
    vec![Choice::plain("a"), Choice::plain("b")]
}

fn labels(menu: &Menu) -> Vec<String> {
    list(menu, &no_bindings, None)
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_supply_of_nothing_or_one_hides_the_row_and_two_shows_it() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("audio"));
    let before = menu.page().entries.len();

    for offered in [vec![], vec![Choice::plain("only")]] {
        menu.supply(ValueSource::MusicSources, &offered);
        assert_eq!(rows_of(&menu, MUSIC), 0, "{} value(s)", offered.len());
        assert_eq!(menu.page().entries.len(), before - 1);
        assert_eq!(menu.page().subtitles.len(), before - 1);
        assert_eq!(menu.page().meta.len(), before - 1);
        assert!(!labels(&menu).iter().any(|text| text == "MUSIC SOURCE"));
    }

    menu.supply(ValueSource::MusicSources, &two());
    assert_eq!(rows_of(&menu, MUSIC), 1);
    assert_eq!(menu.page().entries.len(), before);
    assert!(labels(&menu).iter().any(|text| text == "MUSIC SOURCE"));
}

#[test]
fn a_row_returns_to_the_place_it_left() {
    let mut shown = Menu::new(built_in());
    shown.supply(ValueSource::MusicSources, &two());
    let at = index_of_on(&mut shown, "audio", MUSIC);

    let mut menu = Menu::new(built_in());
    menu.supply(ValueSource::MusicSources, &[]);
    menu.supply(ValueSource::MusicSources, &two());
    assert_eq!(index_of_on(&mut menu, "audio", MUSIC), at);
    assert_eq!(menu.page().entries, shown.page().entries);
}

fn index_of_on(menu: &mut Menu, page: &str, setting: &str) -> usize {
    assert!(menu.open(page));
    index_of(menu, setting).expect("the row is shown")
}

#[test]
fn the_pad_walks_past_a_hidden_row_and_wraps_without_landing_on_it() {
    let mut menu = Menu::new(built_in());
    menu.supply(ValueSource::MusicSources, &[]);
    assert!(menu.open("audio"));
    let rows = menu.page().entries.len();
    for button in [Button::Down, Button::Up] {
        for _ in 0..=rows * 2 {
            press(&mut menu, &[button]);
            let entry = &menu.page().entries[menu.selected()];
            assert_ne!(entry.setting(), Some(MUSIC));
            assert!(menu.selected() < rows);
        }
    }
}

#[test]
fn a_cursor_on_a_row_that_becomes_hidden_moves_to_a_neighbour() {
    let mut menu = Menu::new(built_in());
    menu.supply(ValueSource::MusicSources, &two());
    let row = index_of_on(&mut menu, "audio", MUSIC);
    for _ in 0..row {
        press(&mut menu, &[Button::Down]);
    }
    assert_eq!(menu.selected(), row);

    menu.supply(ValueSource::MusicSources, &[]);
    assert!(menu.selected() < menu.page().entries.len());
    assert_ne!(menu.page().entries[menu.selected()].setting(), Some(MUSIC));

    menu.supply(ValueSource::MusicSources, &two());
    assert!(menu.selected() < menu.page().entries.len());
}

#[test]
fn the_pointer_has_no_region_for_a_hidden_row() {
    let mut menu = Menu::new(built_in());
    menu.supply(ValueSource::MusicSources, &[]);
    assert!(menu.open("audio"));
    let regions = pointer::regions(&menu, &skin(), &Frame::default(), &measure);
    let rows = menu.page().entries.len();
    assert!(regions.iter().all(|region| region.row < rows));
    assert_eq!(
        regions.iter().filter(|r| r.part == Part::Row).count(),
        rows.min(menu.visible_rows())
    );
    for region in regions.iter().filter(|r| r.part == Part::Row) {
        let at = (region.rect[0] + 1.0, region.rect[1] + 1.0);
        let landed = hit(&regions, at).expect("inside a region");
        assert_ne!(menu.page().entries[landed.row].setting(), Some(MUSIC));
    }
}

#[test]
fn the_variant_row_follows_the_team_it_is_supplied_for() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("race"));
    menu.supply(ValueSource::RaceVariant, &two());
    assert_eq!(rows_of(&menu, VARIANT), 1);
    menu.supply(ValueSource::RaceVariant, &[]);
    assert_eq!(rows_of(&menu, VARIANT), 0);
    assert_eq!(rows_of(&menu, "race.team"), 1, "its neighbours stay");
    menu.supply(ValueSource::RaceVariant, &two());
    assert_eq!(rows_of(&menu, VARIANT), 1);
}

/// The greyed half of the distinction: a source that is empty but not one of
/// the two left out keeps its row, and a `disabled_by` row stays drawn.
#[test]
fn only_the_choiceless_rows_are_hidden_and_a_disabled_row_stays() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("race"));
    let rows = menu.page().entries.len();
    menu.supply(ValueSource::Tracks, &[]);
    assert_eq!(
        menu.page().entries.len(),
        rows,
        "an empty tracks list keeps its row"
    );

    let mut menu = Menu::new(built_in());
    assert!(menu.open("display"));
    menu.seed("display.vsync", &Value::Text("on".to_string()));
    let limit = index_of(&menu, "display.frame_limit").expect("frame limit is still listed");
    assert!(menu.is_disabled(&menu.page().entries[limit]));
}
