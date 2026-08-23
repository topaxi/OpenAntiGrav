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
    input.begin_frame(1 << button.index());
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
    input.begin_frame(1 << Button::Down.index());
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
    let atlas = crate::font::Atlas::build();

    for draw in draw_list(&launcher) {
        let Draw::Text { x, text, .. } = draw else {
            continue;
        };
        let right = x + crate::font::measure(&atlas, &text);
        assert!(
            right <= SCREEN.0 - MARGIN,
            "{text:?} ends at {right}, past {}",
            SCREEN.0 - MARGIN
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
