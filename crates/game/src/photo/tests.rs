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
    let say = |id: &str| id.to_string();
    let button = |b: Button| b.to_string().to_uppercase();
    let mut photo = Photo::new(0, true, None);
    let grid = (483.0, 272.0);
    let shown = texts(&draw_list(&photo, grid, &words(&say, &button), None));
    assert!(
        shown.contains(&"OAG_PHOTO_CAMERA: OAG_PHOTO_INTERNAL".to_string()),
        "{shown:?}"
    );
    assert!(shown.contains(&"OAG_PHOTO_HUD: OAG_PHOTO_OFF".to_string()));
    photo.step(
        Pad {
            help: true,
            ..Pad::default()
        },
        &[0],
    );
    assert!(!photo.help());
    assert!(draw_list(&photo, grid, &words(&say, &button), None).is_empty());
    let saved = draw_list(&photo, grid, &words(&say, &button), Some("SAVED"));
    assert_eq!(texts(&saved), ["SAVED"]);
}
