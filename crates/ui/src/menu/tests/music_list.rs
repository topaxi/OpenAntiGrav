//! The pause screen's music list (`super::super::music_list`), as Pulse reads
//! (`docs/ghidra/functions/psp-pulse-usa/music-list.md`).

use super::*;

use crate::menu::music_list::{self, Art, MusicList};
use crate::menu::nav::{Dir, Nav};
use crate::menu::pointer::{self, Part};
use crate::pointer::Pointer;

const TITLES: [&str; 3] = [
    "Fenix Funk 5 (Wipeout Edit)",
    "Steady Rush",
    "Suspicious Thoughts",
];

/// Pulse's pause page on its Time Trial screen, the race playing song `playing`.
fn paused(playing: usize) -> Menu {
    let mut definition = built_in();
    definition.open_on_pause_screen(
        oag_pulse::pause_menu::PAUSE_MENU,
        "time_trial",
        &crate::language::StringTable::from_xml(
            r#"<StringTable>
                 <Entry ID="IG_PAUSE_GPAUSED" String="GAME PAUSED"></Entry>
                 <Entry ID="IG_PAUSE_CONT" String="CONTINUE"></Entry>
                 <Entry ID="OPT_GAME_OPT" String="GAME OPTIONS"></Entry>
                 <Entry ID="EXT_PHOTO" String="PHOTO MODE"></Entry>
                 <Entry ID="IG_PAUSE_REST" String="RESTART RACE"></Entry>
                 <Entry ID="IG_PAUSE_QUIT" String="QUIT RACE"></Entry>
               </StringTable>"#,
        ),
    );
    let mut menu = Menu::new(definition);
    menu.set_pause_screen(true);
    menu.offer(Action::PhotoMode, true);
    let spec = oag_pulse::pause_menu::MUSIC_LIST;
    menu.set_music_list(Some(MusicList::new(
        TITLES.map(str::to_string).to_vec(),
        playing,
        spec.wraps,
    )));
    menu
}

fn frame() -> Frame {
    Frame {
        music_list: Some(Art {
            spec: oag_pulse::pause_menu::MUSIC_LIST,
            arrows: Some([[0.0, 0.0, 0.1, 0.1], [0.1, 0.0, 0.1, 0.1]]),
        }),
        ..Frame::default()
    }
}

fn shown(menu: &Menu) -> Option<&str> {
    menu.music_list().and_then(MusicList::title)
}

/// The list starts on the song that is playing and draws its title under the rows, after the
/// two arrow halves: the back half at half alpha at the first song, the forward half whole.
#[test]
fn the_list_shows_the_playing_song_with_its_arrows() {
    let menu = paused(1);
    assert_eq!(shown(&menu), Some("Steady Rush"));
    assert_eq!(menu.music_list().and_then(MusicList::pick), None);

    let first = paused(0);
    let mut out = Vec::new();
    music_list::draw(
        &first,
        &skin(),
        frame().music_list.as_ref().unwrap(),
        &mut out,
    );
    let colors: Vec<[f32; 4]> = out
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { color, .. } => Some(*color),
            _ => None,
        })
        .collect();
    assert_eq!(colors.len(), 2, "two arrow halves");
    assert_eq!(
        colors[0][3],
        colors[1][3] * oag_pulse::pause_menu::MUSIC_LIST.arrow_dim,
        "the back half keeps half its alpha at the first song (`ListWidget_ShadeArrows`)"
    );
    assert_eq!(colors[0][..3], colors[1][..3], "in the same colour");
    assert!(out.iter().any(|draw| matches!(
        draw,
        Draw::FacedText { text, .. } | Draw::Text { text, .. } if text == TITLES[0]
    )));
}

/// LEFT and RIGHT step the list one song a press whichever row the cursor is on, with `UPDOWN`;
/// a press at an end plays `DECLINE` and changes nothing (no `Wrap`). The cursor stays put.
#[test]
fn left_and_right_step_the_list_without_wrapping() {
    let mut menu = paused(0);
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Down]);
    let row = menu.selected();
    let _ = menu.take_nav();

    assert_eq!(press(&mut menu, &[Button::Left]), []);
    assert_eq!(menu.take_nav(), [Nav::Decline]);
    assert_eq!(shown(&menu), Some(TITLES[0]));

    press(&mut menu, &[Button::Right]);
    assert_eq!(menu.take_nav(), [Nav::Moved(Some(Dir::Right))]);
    press(&mut menu, &[Button::Right]);
    assert_eq!(shown(&menu), Some(TITLES[2]));
    let _ = menu.take_nav();
    press(&mut menu, &[Button::Right]);
    assert_eq!(menu.take_nav(), [Nav::Decline]);
    assert_eq!(shown(&menu), Some(TITLES[2]));
    press(&mut menu, &[Button::Left]);
    assert_eq!(menu.take_nav(), [Nav::Moved(Some(Dir::Left))]);
    assert_eq!(shown(&menu), Some(TITLES[1]));
    assert_eq!(menu.selected(), row, "the cursor does not move");
}

/// Resuming keeps the pick for the race to start; the playing song picked again is no pick.
#[test]
fn resume_hands_over_the_pick_and_the_playing_song_is_none() {
    let mut menu = paused(0);
    press(&mut menu, &[Button::Right]);
    press(&mut menu, &[Button::Right]);
    assert_eq!(press(&mut menu, &[Button::Start]), [MenuEvent::Closed]);
    assert_eq!(menu.music_list().and_then(MusicList::pick), Some(2));

    let mut back = paused(1);
    press(&mut back, &[Button::Right]);
    press(&mut back, &[Button::Left]);
    assert_eq!(back.music_list().and_then(MusicList::pick), None);
}

/// GAME OPTIONS and back drops a pick: the list shows the playing song again (measured).
#[test]
fn another_page_drops_the_pick() {
    let mut menu = paused(0);
    press(&mut menu, &[Button::Right]);
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(menu.page().id, "options");
    assert!(menu.music_list().is_none(), "not drawn under another page");
    press(&mut menu, &[Button::Circle]);
    assert_eq!(menu.page().id, PAUSE_PAGE);
    press(&mut menu, &[]);
    assert_eq!(shown(&menu), Some(TITLES[0]));
    assert_eq!(menu.music_list().and_then(MusicList::pick), None);
}

/// A click on an arrow half steps the way it points, with the pad's sounds; a click on the
/// title steps forward. Neither moves the cursor.
#[test]
fn a_click_on_an_arrow_or_the_title_steps_the_list() {
    let click_on = |menu: &mut Menu, part: Part| {
        let regions = pointer::regions(menu, &skin(), &frame(), &measure);
        let region = regions
            .iter()
            .find(|region| region.part == part)
            .unwrap_or_else(|| panic!("{part:?} has a region"));
        let at = (
            region.rect[0] + region.rect[2] * 0.5,
            region.rect[1] + region.rect[3] * 0.5,
        );
        assert_eq!(pointer::hit(&regions, at).map(|hit| hit.part), Some(part));
        menu.pointer(
            &Pointer {
                at: Some(at),
                moved: true,
                clicked: true,
                ..Pointer::default()
            },
            &regions,
        )
    };
    let mut menu = paused(0);
    let row = menu.selected();
    let _ = menu.take_nav();
    assert_eq!(click_on(&mut menu, Part::MusicBack), []);
    assert_eq!(menu.take_nav(), [Nav::Decline]);
    click_on(&mut menu, Part::MusicForward);
    assert_eq!(menu.take_nav(), [Nav::Moved(Some(Dir::Right))]);
    assert_eq!(shown(&menu), Some(TITLES[1]));
    click_on(&mut menu, Part::MusicRow);
    assert_eq!(shown(&menu), Some(TITLES[2]));
    click_on(&mut menu, Part::MusicBack);
    assert_eq!(shown(&menu), Some(TITLES[1]));
    assert_eq!(menu.selected(), row, "the cursor does not move");
}

/// A page opened and left again before the menu next updates (ESCAPE is handled out of band)
/// still drops the pick.
#[test]
fn a_page_left_before_the_next_update_drops_the_pick() {
    let mut menu = paused(0);
    press(&mut menu, &[Button::Right]);
    press(&mut menu, &[Button::Down]);
    press(&mut menu, &[Button::Cross]);
    assert_eq!(menu.page().id, "options");
    let _ = menu.back();
    assert_eq!(menu.page().id, PAUSE_PAGE);
    assert_eq!(menu.music_list().and_then(MusicList::pick), None);
}
