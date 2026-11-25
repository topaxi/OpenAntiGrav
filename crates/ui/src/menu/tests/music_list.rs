//! The pause screen's music list (`super::super::music_list`), as Pulse reads
//! (`docs/ghidra/functions/psp-pulse-usa/music-list.md`).

use super::*;

use crate::menu::music_list::{self, Art, MusicList};
use crate::menu::nav::{Dir, Nav};
use crate::menu::pointer::{self, Part};
use crate::pointer::Pointer;

use oag_display::space::Space;
use oag_title::{Origin, PauseAction, PauseMenu, PauseRow, PauseScreen, pause_menu::row};

/// Where Pulse's two rows' faces stand their capitals, as `RowInk::baseline` reads them off the
/// PSP's and the PS2's menu face.
const PSP_BASELINE: f32 = 19.0;
const PS2_BASELINE: f32 = 21.0;

/// One pressing: its menu skin with the face's metrics, and the list it authors.
struct Pressing {
    skin: Skin,
    spec: oag_title::PauseMusicList,
    baseline: f32,
}

fn psp() -> Pressing {
    let mut skin = skin();
    skin.set_row_baseline(Some(PSP_BASELINE));
    Pressing {
        skin,
        spec: oag_pulse::pause_menu::MUSIC_LIST,
        baseline: PSP_BASELINE,
    }
}

fn ps2() -> Pressing {
    let mut skin = Skin::new(oag_pulse::frontend::PS2_MENU_SKIN, Space::PS2, 24.0);
    skin.set_row_baseline(Some(PS2_BASELINE));
    Pressing {
        skin,
        spec: oag_pulse::pause_menu::MUSIC_LIST_PS2,
        baseline: PS2_BASELINE,
    }
}

/// A pause page of `rows` rows with the list under it, `visible` of them on screen at once.
fn paused_with(rows: usize, visible: usize, spec: oag_title::PauseMusicList) -> Menu {
    static ROWS: [PauseRow; 9] = [row("IG_PAUSE_CONT", Some(PauseAction::Resume)); 9];
    let screens: &'static [PauseScreen] = Box::leak(Box::new([PauseScreen {
        name: "Long",
        rows: &ROWS[..rows],
    }]));
    let table = PauseMenu {
        screens,
        default_screen: "Long",
        by_mode: &[],
        offers_photo: false,
        origin: Origin::Chosen,
        ..*oag_pulse::pause_menu::PAUSE_MENU
    };
    let mut definition = built_in();
    definition.open_on_pause_screen(
        &table,
        "time_trial",
        &crate::language::StringTable::from_xml(
            r#"<StringTable><Entry ID="IG_PAUSE_CONT" String="CONTINUE"></Entry></StringTable>"#,
        ),
    );
    let mut menu = Menu::new(definition);
    menu.set_pause_screen(true);
    menu.set_visible_rows(visible);
    menu.set_music_list(Some(MusicList::new(
        TITLES.map(str::to_string).to_vec(),
        0,
        spec.wraps,
    )));
    menu
}

fn frame_of(spec: oag_title::PauseMusicList) -> Frame {
    let mut frame = frame();
    frame.music_list.as_mut().expect("a list").spec = spec;
    frame
}

/// The lines the page draws: each row's pen `y`, and the list's title's.
fn lines(menu: &Menu, skin: &Skin, frame: &Frame) -> (Vec<f32>, f32) {
    let layers = draw_list(
        menu,
        skin,
        &|_| Vec::new(),
        &|text| text.len() as f32,
        None,
        frame,
        true,
    );
    let mut rows = Vec::new();
    let mut title = None;
    for draw in &layers.body {
        if let Draw::Text { y, text, .. } | Draw::FacedText { y, text, .. } = draw {
            if text == "CONTINUE" {
                rows.push(*y);
            } else if text == TITLES[0] {
                title = Some(*y);
            }
        }
    }
    (rows, title.expect("the list's title is drawn"))
}

/// The title's line never starts above the baseline of a row on screen, whatever the page's
/// length, and moves no row; the disc's own row counts leave it at the disc's `y`.
#[test]
fn the_list_stays_clear_of_every_row_and_where_the_disc_put_it_when_it_can() {
    // The longest of the PSP's authored pause screens has seven rows (Speed Lap), the PS2
    // table drawn on its own grid fits five before the sixth reaches the disc's `y`.
    for (name, pressing, kept) in [("psp", psp(), 7), ("ps2", ps2(), 5)] {
        let Pressing {
            skin,
            spec,
            baseline,
        } = pressing;
        let frame = frame_of(spec);
        for rows in 1..=9 {
            let menu = paused_with(rows, 9, spec);
            let (row_ys, title_y) = lines(&menu, &skin, &frame);
            assert_eq!(row_ys.len(), rows, "{name}: every row on screen");
            for row_y in &row_ys {
                assert!(
                    title_y >= row_y + baseline,
                    "{name}, {rows} rows: the list at {title_y} is over the row at {row_y}"
                );
            }
            if rows <= kept {
                assert_eq!(title_y, spec.y, "{name}, {rows} rows: the disc's own place");
            } else {
                let last = row_ys[rows - 1];
                assert!(title_y > spec.y, "{name}, {rows} rows: moved down");
                assert_eq!(
                    title_y,
                    last + skin.row_pitch() + skin.message_gap(),
                    "{name}, {rows} rows: under the last row"
                );
            }
        }
    }
}

/// The arrows and their pointer targets go where the title goes; the rows' targets are the
/// same with and without a list.
#[test]
fn the_lists_pointer_targets_move_with_it_and_the_rows_do_not_move() {
    let Pressing { skin, spec, .. } = ps2();
    let measure = |text: &str| text.len() as f32 * 8.0;
    for (rows, moved) in [(5, false), (7, true)] {
        let menu = paused_with(rows, 9, spec);
        let frame = frame_of(spec);
        let regions = pointer::regions(&menu, &skin, &frame, &measure);
        let (_, title_y) = lines(&menu, &skin, &frame);
        assert_eq!(title_y > spec.y, moved);
        let back = regions
            .iter()
            .find(|region| region.part == Part::MusicBack)
            .expect("the back arrow");
        let title = regions
            .iter()
            .find(|region| region.part == Part::MusicRow)
            .expect("the title");
        assert_eq!(back.rect[1], title.rect[1]);
        assert_eq!(
            back.rect[1],
            title_y + spec.arrow_offset[0].1 * skin.theirs_scale().1
        );

        let without = Frame::default();
        let rows_of = |regions: &[pointer::Region]| -> Vec<[f32; 4]> {
            regions
                .iter()
                .filter(|region| region.part == Part::Row)
                .map(|region| region.rect)
                .collect()
        };
        assert_eq!(
            rows_of(&regions),
            rows_of(&pointer::regions(&menu, &skin, &without, &measure))
        );
    }
}

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
        &frame(),
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
