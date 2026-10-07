//! The cursor and the draw list, with candidates built by hand.
//!
//! [`super::survey`] itself needs real disc images and is exercised by
//! `crates/game/tests/launcher_ground_truth.rs`; everything here is the logic
//! over whatever that returns.

use super::*;

fn playable(name: &str) -> Candidate {
    Candidate {
        source: format!("data/images/{name}"),
        name: name.to_string(),
        platform: Platform::Psp,
        serial: Some("UCUS-98712".to_string()),
        state: State::Playable(oag_pulse::TITLE),
    }
}

fn broken(name: &str) -> Candidate {
    Candidate {
        source: format!("data/images/{name}"),
        name: name.to_string(),
        platform: Platform::Ps3,
        serial: Some("BCES-00664".to_string()),
        state: State::Unavailable(why_not(Platform::Ps3)),
    }
}

fn press(input: &mut Input, button: Button) {
    input.begin_frame(button.bit());
}

#[test]
fn the_cursor_starts_on_the_first_playable_row() {
    let launcher = Launcher::new(vec![broken("enc.iso"), playable("a.chd")]);
    assert_eq!(launcher.cursor(), 1);
    assert_eq!(launcher.pick().unwrap(), "data/images/a.chd");
}

#[test]
fn a_row_that_will_not_open_is_stepped_over() {
    let mut launcher = Launcher::new(vec![
        playable("a.chd"),
        broken("enc.iso"),
        playable("b.chd"),
    ]);
    assert_eq!(launcher.cursor(), 0);

    let mut input = Input::new();
    press(&mut input, Button::Down);
    assert!(launcher.update(&mut input).is_none());
    assert_eq!(launcher.cursor(), 2, "the broken row is not landed on");
}

#[test]
fn the_cursor_wraps_in_both_directions() {
    let mut launcher = Launcher::new(vec![playable("a.chd"), playable("b.chd")]);
    let mut input = Input::new();

    press(&mut input, Button::Up);
    launcher.update(&mut input);
    assert_eq!(launcher.cursor(), 1);

    press(&mut input, Button::Down);
    launcher.update(&mut input);
    assert_eq!(launcher.cursor(), 0);
}

/// A folder of nothing but encrypted images is a real state, and it must not
/// spin: `step` has nowhere to go and `pick` has nothing to hand back.
#[test]
fn a_list_with_nothing_playable_neither_moves_nor_picks() {
    let mut launcher = Launcher::new(vec![broken("one.iso"), broken("two.iso")]);
    let mut input = Input::new();

    press(&mut input, Button::Down);
    assert!(launcher.update(&mut input).is_none());
    assert_eq!(launcher.cursor(), 0);
    assert!(!launcher.has_playable());
    assert!(launcher.pick().is_none());
}

#[test]
fn confirming_returns_the_source_under_the_cursor() {
    let mut launcher = Launcher::new(vec![playable("a.chd"), playable("b.chd")]);
    let mut input = Input::new();

    press(&mut input, Button::Down);
    launcher.update(&mut input);
    press(&mut input, Button::Cross);
    assert_eq!(launcher.update(&mut input).unwrap(), "data/images/b.chd");
}

/// START confirms as well as CROSS, the way it does everywhere else in the
/// front end.
#[test]
fn start_confirms_too() {
    let mut launcher = Launcher::new(vec![playable("a.chd")]);
    let mut input = Input::new();

    press(&mut input, Button::Start);
    assert_eq!(launcher.update(&mut input).unwrap(), "data/images/a.chd");
}

/// A held key must not walk the list: the press is consumed on the frame it
/// arrives and the next frame sees no edge.
#[test]
fn a_held_button_moves_once() {
    let mut launcher = Launcher::new(vec![
        playable("a.chd"),
        playable("b.chd"),
        playable("c.chd"),
    ]);
    let mut input = Input::new();

    press(&mut input, Button::Down);
    launcher.update(&mut input);
    assert_eq!(launcher.cursor(), 1);

    // Same mask again: held, not pressed.
    input.begin_frame(Button::Down.bit());
    launcher.update(&mut input);
    assert_eq!(launcher.cursor(), 1);
}

#[test]
fn a_row_reads_as_its_title_platform_and_serial() {
    let row = playable("pulse-psp-usa.chd");
    assert_eq!(row.title(), "Wipeout Pulse");
    assert_eq!(row.provenance(), "PSP UCUS-98712");

    // Nothing invented for a source that reports nothing.
    let quiet = Candidate {
        platform: Platform::Unknown,
        serial: None,
        ..playable("mine.chd")
    };
    assert_eq!(quiet.provenance(), "unknown");
}

/// The PS3 message names the fix, because that failure has one. Everything
/// else gets the short honest answer rather than a guessed diagnosis.
#[test]
fn only_the_ps3_reason_names_a_fix() {
    assert!(why_not(Platform::Ps3).contains(".dkey"));
    assert!(!why_not(Platform::Psp).contains(".dkey"));
}

#[test]
fn the_draw_list_has_a_line_for_every_row() {
    let launcher = Launcher::new(vec![playable("a.chd"), broken("enc.iso")]);
    let list = draw_list(&launcher);

    let texts: Vec<&str> = list
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();

    assert!(texts.contains(&"Wipeout Pulse"));
    assert!(texts.contains(&"a.chd"));
    assert!(texts.contains(&"enc.iso"));
    assert!(texts.contains(&UNAVAILABLE));
    assert!(
        texts.iter().any(|line| line.contains(".dkey")),
        "the reason a row will not open is on screen: {texts:?}"
    );
}

/// The keys are drawn whether or not anything is wrong.
#[test]
fn the_key_hint_is_always_drawn() {
    for rows in [vec![playable("a.chd")], vec![broken("enc.iso")]] {
        let launcher = Launcher::new(rows);
        let list = draw_list(&launcher);
        assert!(
            list.iter().any(|draw| matches!(
                draw,
                Draw::Text { text, .. } if text == HINT
            )),
            "no hint drawn"
        );
    }
}

/// **The check that caught a line running off the right edge.** Every string
/// this screen draws is measured in the face it is actually drawn in - the
/// engine's own 5x7 set, since there is no disc here to take one from - against
/// the grid it is drawn in. A sentence too long to fit tells a player less than
/// a shorter one that does.
#[test]
fn every_line_fits_on_screen() {
    let launcher = Launcher::new(vec![
        playable("pulse-psp-usa.chd"),
        broken("hdfury-ps3-eu.iso"),
        playable("hdfury-ps3-eu-dec.iso"),
    ]);
    let atlas = oag_ui::font::Atlas::build();

    for draw in draw_list(&launcher) {
        let Draw::Text { x, align, text, .. } = draw else {
            continue;
        };
        let width = oag_ui::font::measure(&atlas, &text);
        // `x` anchors the edge alignment names, not always the left one - the
        // commit hash at the bottom right is drawn `Align::Right`, so its `x`
        // is where the text *ends*, and the risk this test exists to catch
        // for that row is running off the left edge instead.
        let (left, right) = match align {
            Align::Left => (x, x + width),
            Align::Centre => (x - width / 2.0, x + width / 2.0),
            Align::Right => (x - width, x),
        };
        assert!(
            right <= SCREEN.0 - MARGIN,
            "{text:?} ends at {right}, past {}",
            SCREEN.0 - MARGIN
        );
        assert!(left >= 0.0, "{text:?} starts at {left}, off the left edge");
    }
}

/// **The check that caught Omega's title column overlapping the platform
/// column next to it.** `every_line_fits_on_screen` only catches a line
/// running off the *screen's* right edge; a row's title is free to run into
/// its own neighbour column and that test would not see it, which is
/// exactly what `oag_omega::TITLE.name` did against a `TITLE_COLUMN` sized
/// for `oag_hd::TITLE.name`'s ten characters. This measures every row
/// against the columns [`draw_list`] actually places them in.
#[test]
fn title_and_provenance_columns_do_not_overlap_their_neighbour() {
    let launcher = Launcher::new(vec![
        Candidate {
            source: "data/extracted/ps4".to_string(),
            name: "ps4".to_string(),
            platform: Platform::Unknown,
            serial: None,
            state: State::Playable(oag_omega::TITLE),
        },
        playable("hdfury-ps3-eu-dec.iso"),
    ]);
    let atlas = oag_ui::font::Atlas::build();

    for row in launcher.rows() {
        let title_width = oag_ui::font::measure(&atlas, row.title());
        assert!(
            MARGIN + title_width <= TITLE_COLUMN,
            "{:?} ends at {}, past the provenance column at {TITLE_COLUMN}",
            row.title(),
            MARGIN + title_width
        );

        let provenance = row.provenance();
        let provenance_width = oag_ui::font::measure(&atlas, &provenance);
        assert!(
            TITLE_COLUMN + provenance_width <= NAME_COLUMN,
            "{provenance:?} ends at {}, past the name column at {NAME_COLUMN}",
            TITLE_COLUMN + provenance_width
        );
    }
}

/// Rows and their notes must not overlap, whatever the list holds.
#[test]
fn the_notes_sit_below_the_last_row() {
    let launcher = Launcher::new(vec![
        playable("a.chd"),
        broken("one.iso"),
        broken("two.iso"),
    ]);
    let last_row = FIRST_ROW + (launcher.rows().len() as f32 - 1.0) * ROW;

    let notes: Vec<f32> = draw_list(&launcher)
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { y, text, .. } if text.contains(".dkey") => Some(*y),
            _ => None,
        })
        .collect();

    assert_eq!(notes.len(), 2, "one note per unavailable row");
    for y in notes {
        assert!(y > last_row, "a note at {y} overlaps the row at {last_row}");
    }
}

/// Where each row's file name is drawn, in row order - the name, because
/// two images of one title share a title.
fn drawn_row_pens(launcher: &Launcher) -> Vec<(f32, f32)> {
    let list = draw_list(launcher);
    launcher
        .rows()
        .iter()
        .map(|row| {
            list.iter()
                .find_map(|draw| match draw {
                    Draw::Text { x, y, text, .. } if *text == row.name && *x == NAME_COLUMN => {
                        Some((*x, *y))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{} is drawn", row.name))
        })
        .collect()
}

fn click_at(at: (f32, f32)) -> oag_ui::pointer::Pointer {
    oag_ui::pointer::Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Default::default()
    }
}

/// The drift guard: every drawn row is under its own pen, and a click on
/// a playable one picks it while a click on a broken one is nothing.
#[test]
fn a_click_on_a_drawn_row_picks_it_unless_it_will_not_open() {
    let mut launcher = Launcher::new(vec![
        playable("a.chd"),
        broken("enc.iso"),
        playable("b.chd"),
    ]);
    let pens = drawn_row_pens(&launcher);
    for (index, pen) in pens.iter().enumerate() {
        assert_eq!(row_at((pen.0 + 2.0, pen.1 + 2.0), 3), Some(index));
    }
    assert_eq!(
        launcher.pointer(&click_at((pens[2].0 + 2.0, pens[2].1 + 2.0))),
        Some("data/images/b.chd".to_string())
    );
    assert_eq!(launcher.cursor(), 2);
    assert_eq!(
        launcher.pointer(&click_at((pens[1].0 + 2.0, pens[1].1 + 2.0))),
        None
    );
    assert_eq!(launcher.cursor(), 2, "a broken row is not a target");
    assert_eq!(launcher.pointer(&click_at((pens[0].0, 10.0))), None);
}

#[test]
fn hovering_a_row_moves_the_cursor_and_the_wheel_steps_it() {
    let mut launcher = Launcher::new(vec![playable("a.chd"), playable("b.chd")]);
    let pens = drawn_row_pens(&launcher);
    let hover = oag_ui::pointer::Pointer {
        at: Some((pens[1].0 + 2.0, pens[1].1 + 2.0)),
        moved: true,
        ..Default::default()
    };
    assert_eq!(launcher.pointer(&hover), None);
    assert_eq!(launcher.cursor(), 1);
    let wheel = oag_ui::pointer::Pointer {
        scroll: 1,
        ..Default::default()
    };
    assert_eq!(launcher.pointer(&wheel), None);
    assert_eq!(
        launcher.cursor(),
        0,
        "the wheel steps like the d-pad, wrapping"
    );
}

const ANDROID_IMAGES: &str = "/sdcard/Android/data/org.openantigrav.game/files/data/images";

/// The not-found screen carries the exact push command and every line of it
/// fits the 480-unit screen. It draws no rows and picks nothing.
#[test]
fn the_not_found_screen_names_the_adb_push_and_fits() {
    let launcher = Launcher::not_found(not_found_notice(ANDROID_IMAGES));
    assert!(launcher.pick().is_none() && !launcher.has_playable());
    let atlas = oag_ui::font::Atlas::build();
    let texts: Vec<String> = draw_list(&launcher)
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text { x, text, .. } => {
                let width = oag_ui::font::measure(&atlas, &text);
                assert!(x + width <= SCREEN.0, "{text:?} runs off the screen");
                Some(text)
            }
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|t| t == "NO DISC IMAGE FOUND"),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.contains(&format!("{ANDROID_IMAGES}/"))),
        "the destination is spelled out: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == "SELECT A DISC IMAGE"),
        "there is nothing to select"
    );
}

fn locked(name: &str) -> Candidate {
    Candidate {
        state: State::NeedsKey,
        ..broken(name)
    }
}

/// A locked image is a row the cursor lands on - it is the only way into the
/// prompt - but it is never offered to play.
#[test]
fn a_locked_row_is_selectable_and_never_picked() {
    let l = Launcher::new(vec![locked("enc.iso")]);
    assert_eq!(l.cursor(), 0);
    assert!(l.rows()[0].is_selectable());
    assert!(!l.rows()[0].is_playable());
    assert_eq!(l.pick(), None);
    assert_eq!(l.rows()[0].title(), NEEDS_KEY);
}

#[test]
fn confirming_a_locked_row_opens_the_prompt_and_circle_closes_it() {
    let mut l = Launcher::new(vec![locked("enc.iso")]);
    let mut input = Input::new();
    press(&mut input, Button::Cross);
    assert_eq!(
        l.update(&mut input),
        None,
        "opening the prompt picks nothing"
    );
    assert!(l.typing_is_open());
    // The prompt owns the d-pad now: it does not move the list cursor.
    press(&mut input, Button::Circle);
    assert_eq!(l.update(&mut input), None);
    assert!(!l.typing_is_open());
}

#[test]
fn a_click_on_a_locked_row_opens_the_prompt() {
    let mut l = Launcher::new(vec![playable("a.chd"), locked("enc.iso")]);
    let at = (SCREEN.0 / 2.0, FIRST_ROW + ROW + 4.0);
    let click = oag_ui::pointer::Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Default::default()
    };
    assert_eq!(l.pointer(&click), None);
    assert!(l.typing_is_open());
}

#[test]
fn the_prompt_replaces_the_list_and_a_desk_keyboard_types_into_it() {
    let mut l = Launcher::new(vec![locked("enc.iso")]);
    let mut input = Input::new();
    press(&mut input, Button::Cross);
    l.update(&mut input);
    l.type_digit('a');
    l.type_digit('Z');
    l.type_digit('0');
    assert_eq!(l.entry().unwrap().len(), 2);
    let texts: Vec<String> = draw_list(&l)
        .into_iter()
        .filter_map(|d| match d {
            Draw::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t == "DISC KEY"));
    assert!(!texts.iter().any(|t| t == "SELECT A DISC IMAGE"));
}

#[test]
fn a_paste_request_is_serviced_once_and_only_where_a_clipboard_exists() {
    let mut l = Launcher::new(vec![locked("enc.iso")]).with_paste(true);
    let mut input = Input::new();
    press(&mut input, Button::Cross);
    l.update(&mut input);
    l.request_paste();
    assert!(l.take_paste_request());
    assert!(!l.take_paste_request());
    l.paste("00112233445566778899aabbccddeeff");
    assert_eq!(l.entry().unwrap().len(), 32);

    let mut none = Launcher::new(vec![locked("enc.iso")]);
    press(&mut input, Button::Cross);
    none.update(&mut input);
    none.request_paste();
    assert!(!none.take_paste_request());
}
