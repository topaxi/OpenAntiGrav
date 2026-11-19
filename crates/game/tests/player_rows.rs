//! The PLAYER NAME and PLAYER TAG rows follow `Title::profile`: a title that keeps no tag shows
//! no tag row, and one that keeps no name shows neither.

use oag_ui::menu::{Action, Definition, Entry};

fn rows(definition: &Definition) -> (bool, bool) {
    let has = |wanted: Action| {
        definition.pages.iter().any(|page| {
            page.entries
                .iter()
                .any(|entry| matches!(entry, Entry::Run { action, .. } if *action == wanted))
        })
    };
    (has(Action::EditPlayerName), has(Action::EditPlayerTag))
}

fn trimmed(title: &oag_title::Title) -> (bool, bool) {
    let mut definition = Definition::built_in(&oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse");
    definition.drop_unavailable_player_rows(title);
    rows(&definition)
}

#[test]
fn the_built_in_menu_offers_both_rows_before_a_title_trims_them() {
    let definition = Definition::built_in(&oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse");
    assert_eq!(rows(&definition), (true, true));
}

#[test]
fn pulse_and_pure_show_a_name_and_a_tag() {
    assert_eq!(trimmed(oag_pulse::TITLE), (true, true));
    assert_eq!(trimmed(oag_pure::TITLE), (true, true));
}

#[test]
fn hd_shows_a_name_and_no_tag() {
    assert_eq!(trimmed(oag_hd::TITLE), (true, false));
}

#[test]
fn omega_and_2048_show_neither() {
    assert_eq!(trimmed(oag_omega::TITLE), (false, false));
    assert_eq!(trimmed(oag_2048::TITLE), (false, false));
}

fn ffmpeg_row_follows_spatial_audio(definition: &Definition) -> Option<bool> {
    let page = definition.page("audio")?;
    let at = page.entries.iter().position(
        |entry| matches!(entry, Entry::Run { action, .. } if *action == Action::FetchFfmpeg),
    )?;
    Some(
        page.entries[..at]
            .iter()
            .any(|entry| entry.setting() == Some("audio.spatial_audio")),
    )
}

#[test]
fn the_download_ffmpeg_row_is_on_the_audio_page_only_while_there_is_something_to_fetch() {
    let mut offered = Definition::built_in(&oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse");
    offered.drop_unavailable_fetch_ffmpeg(true);
    assert_eq!(ffmpeg_row_follows_spatial_audio(&offered), Some(true));
    let options = offered.page("options").expect("options page");
    assert!(!options.entries.iter().any(|e| matches!(
        e,
        Entry::Run {
            action: Action::FetchFfmpeg,
            ..
        }
    )));

    let mut none = Definition::built_in(&oag_ui::language::StringTable::default()).unwrap();
    none.drop_unavailable_fetch_ffmpeg(false);
    assert_eq!(ffmpeg_row_follows_spatial_audio(&none), None);
}
