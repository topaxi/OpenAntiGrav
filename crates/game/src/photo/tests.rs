//! Photo mode's screen and files, with no GPU and no disc.

use super::*;
use oag_raceplay::photo::Pad;

fn words<'a>(say: &'a dyn Fn(&str) -> String, button: &'a dyn Fn(Button) -> String) -> Words<'a> {
    Words { say, button }
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
fn the_mode_is_not_on_offer_in_a_split() {
    assert!(offered(1));
    assert!(!offered(2));
    assert!(!offered(4));
}

#[test]
fn the_race_does_not_tick_while_the_mode_is_open() {
    assert!(ticks(false));
    assert!(!ticks(true));
}

#[test]
fn a_second_shot_in_the_same_second_gets_a_name_of_its_own() {
    let dir = std::env::temp_dir().join(format!("oag-photo-name-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let first = shot_path(&dir, "Wipeout Pulse", 1_700_000_000);
    assert_eq!(
        first.file_name().unwrap().to_str().unwrap(),
        "oag-wipeout-pulse-1700000000-1.png"
    );
    std::fs::write(&first, b"x").unwrap();
    let second = shot_path(&dir, "Wipeout Pulse", 1_700_000_000);
    assert_ne!(first, second);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn every_chip_has_a_rect_inside_the_grid_and_a_click_finds_it() {
    let grid = (483.0, 272.0);
    for chip in CHIPS {
        let [x, y, w, h] = chip_rect(chip, grid);
        assert!(x >= 0.0 && x + w <= grid.0 && y >= 0.0 && y + h <= grid.1);
        assert_eq!(chip_at(grid, (x + w / 2.0, y + h / 2.0)), Some(chip));
    }
    assert_eq!(chip_at(grid, (240.0, 150.0)), None);
}

#[test]
fn the_help_names_the_lens_and_the_hud_and_hiding_it_leaves_only_the_message() {
    let say = |id: &str| id.trim_start_matches("OAG_PHOTO_").to_string();
    let button = |b: Button| b.to_string().to_uppercase();
    let mut photo = Photo::new(0, true, None);
    let grid = (483.0, 272.0);
    let shown = texts(&draw_list(&photo, grid, &words(&say, &button), None));
    assert!(shown.contains(&"CAMERA: INTERNAL".to_string()), "{shown:?}");
    assert!(shown.contains(&"HUD: OFF".to_string()));
    let start = Pad {
        help: true,
        ..Pad::default()
    };
    // START shows the effects panel after the help, and hides both after that.
    photo.step(start, &[0]);
    photo.step(start, &[0]);
    assert!(!photo.help());
    assert!(draw_list(&photo, grid, &words(&say, &button), None).is_empty());
    let saved = draw_list(&photo, grid, &words(&say, &button), Some("SAVED"));
    assert_eq!(texts(&saved), ["SAVED"]);
}

#[test]
fn a_row_the_disc_words_takes_the_discs_text_and_one_it_does_not_keeps_ours() {
    let table = oag_ui::language::StringTable::from_xml(
        r#"<StringTable><Entry ID="MSC_PHOTO_PAN" String="Pan Camera"></Entry></StringTable>"#,
    );
    assert_eq!(
        disc_word(strings::PAN, &oag_title::photo::Strings::HD, &table).as_deref(),
        Some("PAN CAMERA")
    );
    assert_eq!(
        disc_word(strings::PAN, &oag_title::photo::Strings::NONE, &table),
        None,
        "a title with no word of its own"
    );
    assert_eq!(
        disc_word(strings::SHIP, &oag_title::photo::Strings::HD, &table),
        None,
        "the disc has the id but this table does not"
    );
    assert_eq!(
        disc_word(strings::TAKE, &oag_title::photo::Strings::HD, &table),
        None,
        "a chip with no disc row"
    );
}

#[test]
fn the_effects_panel_lists_four_rows_and_two_more_with_depth_of_field_on() {
    let say = |id: &str| id.to_string();
    let button = |b: Button| b.to_string().to_uppercase();
    let grid = (483.0, 272.0);
    let mut photo = Photo::new(0, false, None);
    let help_only = texts(&draw_list(&photo, grid, &words(&say, &button), None));
    assert!(
        !help_only.contains(&"OAG_PHOTO_EXPOSURE".to_string()),
        "the panel is not up yet"
    );
    photo.toggle_panel();
    let shown = texts(&draw_list(&photo, grid, &words(&say, &button), None));
    for id in [
        "OAG_PHOTO_EXPOSURE",
        "OAG_PHOTO_SATURATION",
        "OAG_PHOTO_SPEED_BLUR",
        "OAG_PHOTO_DEPTH",
    ] {
        assert!(shown.contains(&id.to_string()), "{id} in {shown:?}");
    }
    assert!(
        shown.contains(&"33%".to_string()) && shown.contains(&"20%".to_string()),
        "{shown:?}"
    );
    assert!(!shown.contains(&"OAG_PHOTO_APERTURE".to_string()));
    photo
        .effects_mut()
        .select(oag_raceplay::photo::effects::Row::DepthOfField);
    photo.effects_mut().adjust(1);
    let shown = texts(&draw_list(&photo, grid, &words(&say, &button), None));
    assert!(
        shown.contains(&"OAG_PHOTO_APERTURE".to_string())
            && shown.contains(&"OAG_PHOTO_FOCUS".to_string())
    );
}

#[test]
fn a_click_on_the_panel_finds_its_row_and_its_arrows() {
    use oag_raceplay::photo::effects::Row;
    let grid = (483.0, 272.0);
    let mut photo = Photo::new(0, false, None);
    assert_eq!(
        row_at(&photo, grid, (400.0, 40.0)),
        None,
        "no panel, no rows"
    );
    photo.toggle_panel();
    for (index, &row) in photo.effects().rows().iter().enumerate() {
        let [x, y, w, h] = row_rect(index, grid);
        assert!(x >= 0.0 && x + w <= grid.0 && y + h <= grid.1);
        let mid = y + h / 2.0;
        assert_eq!(
            row_at(&photo, grid, (x + 8.0, mid)),
            Some(RowHit { row, step: 0 })
        );
        assert_eq!(
            row_at(&photo, grid, (x + w - 3.0, mid)),
            Some(RowHit { row, step: 1 })
        );
        assert_eq!(
            row_at(&photo, grid, (x + w - ARROW - VALUE - 3.0, mid)),
            Some(RowHit { row, step: -1 })
        );
    }
    assert_eq!(row_at(&photo, grid, (20.0, 150.0)), None);
    let _ = Row::Exposure;
}

#[test]
fn a_chip_whose_row_word_is_too_long_for_it_says_its_value_alone() {
    let say = |id: &str| match id {
        strings::CAMERA => "PHOTO CAMERA MODE".to_string(),
        strings::INTERNAL => "INTERNAL".to_string(),
        other => other.to_string(),
    };
    let button = |b: Button| b.to_string().to_uppercase();
    let photo = Photo::new(0, false, None);
    let shown = texts(&draw_list(
        &photo,
        (483.0, 272.0),
        &words(&say, &button),
        None,
    ));
    assert!(shown.contains(&"INTERNAL".to_string()), "{shown:?}");
    assert!(
        !shown.iter().any(|t| t.starts_with("PHOTO CAMERA MODE:")),
        "{shown:?}"
    );
}
