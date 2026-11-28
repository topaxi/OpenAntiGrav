//! The cards over candidates built by hand. The disc art itself is
//! `oag-source`'s ground truth; the wired chooser is
//! `crates/game/tests/launcher_cards_ground_truth.rs`.

use oag_disc::Platform;
use oag_hud::sprite::{DecodedImage, Sheet};

use super::*;
use crate::launcher::{draw, draw_list, wants_cards};
use crate::settings::launcher::LauncherStyle;

fn row(name: &str, title: &'static oag_title::Title, serial: Option<&str>) -> Candidate {
    Candidate {
        source: format!("data/images/{name}"),
        name: name.to_string(),
        platform: Platform::Psp,
        serial: serial.map(str::to_string),
        state: State::Playable(title),
    }
}

fn row_on(
    name: &str,
    title: &'static oag_title::Title,
    platform: Platform,
    serial: Option<&str>,
) -> Candidate {
    Candidate {
        platform,
        ..row(name, title, serial)
    }
}

fn locked(name: &str, serial: &str) -> Candidate {
    Candidate {
        source: format!("data/images/{name}"),
        name: name.to_string(),
        platform: Platform::Ps3,
        serial: Some(serial.to_string()),
        state: State::NeedsKey,
    }
}

/// Pulse twice, Pure once.
fn library() -> Vec<Candidate> {
    vec![
        row("eu.chd", oag_pulse::TITLE, Some("UCES-00465")),
        row("usa.chd", oag_pulse::TITLE, Some("UCUS-98712")),
        row("pure.chd", oag_pure::TITLE, Some("UCES-00001")),
    ]
}

fn chooser(rows: Vec<Candidate>, remembered: &[(&str, &str)]) -> Launcher {
    let remembered = remembered
        .iter()
        .map(|(card, variant)| (card.to_string(), variant.to_string()))
        .collect();
    Launcher::new(rows).with_cards(
        Words::from_table(&oag_ui::strings::project_table(None)),
        remembered,
    )
}

fn read_art(launcher: &mut Launcher) {
    for source in ["eu.chd", "usa.chd", "pure.chd"] {
        launcher.set_picture(&format!("data/images/{source}"), "pic1");
    }
}

fn press(launcher: &mut Launcher, button: Button) -> Option<String> {
    let mut input = Input::new();
    input.begin_frame(button.bit());
    launcher.update(&mut input, Button::Circle)
}

fn click(launcher: &mut Launcher, at: (f32, f32)) -> Option<String> {
    launcher.pointer(&Pointer {
        at: Some(at),
        clicked: true,
        ..Pointer::default()
    })
}

fn hover(launcher: &mut Launcher, at: (f32, f32)) -> Option<String> {
    launcher.pointer(&Pointer {
        at: Some(at),
        moved: true,
        ..Pointer::default()
    })
}

fn centre(rect: [f32; 4]) -> (f32, f32) {
    (rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0)
}

fn texts(draws: &[Draw]) -> Vec<String> {
    draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn every_title_gets_one_card_however_many_images_it_has() {
    let mut rows = library();
    // HD twice with one serial (an encrypted copy that a key opens beside its
    // decrypted twin), a PS2 Pulse, and a locked copy of HD.
    rows.push(row_on(
        "dec.iso",
        oag_hd::TITLE,
        Platform::Ps3,
        Some("BCES-00664"),
    ));
    rows.push(row_on(
        "enc.iso",
        oag_hd::TITLE,
        Platform::Ps3,
        Some("BCES-00664"),
    ));
    rows.push(row_on(
        "ps2.chd",
        oag_pulse::TITLE,
        Platform::Ps2,
        Some("SCES-54748"),
    ));
    rows.push(locked("locked.iso", "BCES-00664"));
    let list = chooser(rows, &[]).card_list();

    let keys: Vec<&str> = list.iter().map(|card| card.key.as_str()).collect();
    assert_eq!(keys.len(), 4, "{keys:?}");
    let rows_of = |at: usize| -> Vec<usize> { list[at].variants.iter().map(|v| v.row).collect() };
    // Oldest release first: Pure (2005), Pulse PSP (2007), HD (2008), Pulse PS2 (2009).
    assert_eq!(rows_of(0), [2], "Pure");
    assert_eq!(rows_of(1), [0, 1], "both PSP pressings of Pulse");
    assert_eq!(rows_of(2), [3], "the same serial twice is one variant");
    assert_eq!(rows_of(3), [5], "Pulse on PS2 is a card of its own");
}

#[test]
fn a_copy_on_no_known_platform_joins_its_titles_card() {
    let list = chooser(
        vec![
            row_on("psn", oag_hd::TITLE, Platform::Unknown, Some("NPEA00057")),
            row_on("disc.iso", oag_hd::TITLE, Platform::Ps3, Some("BCES-00664")),
        ],
        &[],
    )
    .card_list();
    assert_eq!(
        list.len(),
        1,
        "{:?}",
        list.iter().map(|c| &c.key).collect::<Vec<_>>()
    );
    assert_eq!(list[0].variants.len(), 2);
}

#[test]
fn only_a_title_on_two_platforms_carries_a_platform_badge() {
    let mut rows = library();
    rows.push(row_on(
        "ps2.chd",
        oag_pulse::TITLE,
        Platform::Ps2,
        Some("SCES-54748"),
    ));
    let launcher = chooser(rows, &[]);
    let drawn = texts(&draw(&launcher, &Sheet::default()));
    assert_eq!(drawn.iter().filter(|t| *t == "PSP").count(), 1, "{drawn:?}");
    assert_eq!(drawn.iter().filter(|t| *t == "PS2").count(), 1, "{drawn:?}");

    let alone = texts(&draw(&chooser(library(), &[]), &Sheet::default()));
    assert!(!alone.iter().any(|t| t == "PSP"), "{alone:?}");
}

#[test]
fn a_locked_disc_with_no_playing_copy_has_a_card_that_asks_for_its_key() {
    let mut launcher = chooser(vec![locked("enc.iso", "BCES-00664")], &[]);
    assert_eq!(launcher.card_list().len(), 1);
    let drawn = texts(&draw(&launcher, &Sheet::default()));
    assert!(
        drawn.iter().any(|t| t.contains("enc.iso: ENCRYPTED")),
        "{drawn:?}"
    );
    assert_eq!(press(&mut launcher, Button::Cross), None);
    assert!(launcher.entry().is_some(), "the key prompt opens");
}

#[test]
fn a_locked_copy_listed_first_leaves_the_card_on_the_one_that_plays() {
    let mut launcher = chooser(
        vec![
            locked("enc.iso", "BCES-00664"),
            row("dec.iso", oag_hd::TITLE, Some("BCES-00664")),
        ],
        &[],
    );
    assert_eq!(launcher.card_list().len(), 1);
    assert_eq!(
        press(&mut launcher, Button::Cross).as_deref(),
        Some("data/images/dec.iso")
    );
    assert!(launcher.entry().is_none());
}

#[test]
fn art_is_asked_for_only_from_discs_that_open() {
    let launcher = chooser(
        vec![
            locked("enc.iso", "BCES-00664"),
            row("eu.chd", oag_pulse::TITLE, Some("UCES-00465")),
        ],
        &[],
    );
    // A locked disc is left out, so unlocking it is a source not yet asked for.
    assert_eq!(
        launcher.art_sources().collect::<Vec<_>>(),
        ["data/images/eu.chd"]
    );
}

#[test]
fn the_disc_steps_through_a_titles_images_and_reports_the_choice() {
    let mut launcher = chooser(library(), &[]);
    read_art(&mut launcher);
    // The cursor starts on the first row's card: Pulse, the second card.
    assert_eq!(launcher.focused_card(), 1);
    assert_eq!(launcher.pick().as_deref(), Some("data/images/eu.chd"));
    assert_eq!(launcher.take_choice(), None, "nothing was chosen yet");

    press(&mut launcher, Button::Square);
    assert_eq!(launcher.pick().as_deref(), Some("data/images/usa.chd"));
    let choice = launcher.take_choice().expect("a choice to save");
    assert_eq!(choice.1, "UCUS-98712:pic1");
    assert_eq!(choice.0, launcher.card_list()[1].key);
    assert_eq!(launcher.take_choice(), None, "reported once");

    press(&mut launcher, Button::Square);
    assert_eq!(
        launcher.pick().as_deref(),
        Some("data/images/eu.chd"),
        "wraps"
    );
}

#[test]
fn a_remembered_image_is_shown_again_even_before_its_art_is_read() {
    let key = chooser(library(), &[]).card_list()[1].key.clone();
    let mut launcher = chooser(library(), &[(&key, "UCUS-98712:pic1")]);
    assert_eq!(
        launcher.pick().as_deref(),
        Some("data/images/usa.chd"),
        "confirming before the art lands boots the remembered pressing"
    );
    read_art(&mut launcher);
    assert_eq!(launcher.pick().as_deref(), Some("data/images/usa.chd"));
    let card = launcher.card_list()[1].clone();
    assert_eq!(launcher.shown_variant(&card), 1);
    assert_eq!(
        launcher.take_choice(),
        None,
        "restoring is not a new choice"
    );
}

#[test]
fn a_click_on_the_disc_switches_and_a_click_on_the_card_boots() {
    let mut launcher = chooser(library(), &[]);
    read_art(&mut launcher);
    let rects = layout(2, 0);

    let disc = centre(disc_hit(rects[1]));
    assert_eq!(click(&mut launcher, disc), None, "the disc boots nothing");
    assert_eq!(launcher.pick().as_deref(), Some("data/images/usa.chd"));

    assert_eq!(
        click(&mut launcher, centre(rects[1])).as_deref(),
        Some("data/images/usa.chd")
    );
    // Pure has one image, so its corner is the card like any other part.
    assert_eq!(
        click(&mut launcher, centre(disc_hit(rects[0]))).as_deref(),
        Some("data/images/pure.chd")
    );
}

#[test]
fn pad_and_pointer_move_between_cards() {
    let mut rows = library();
    rows.push(row("hd.iso", oag_hd::TITLE, Some("BCES-00664")));
    rows.push(row("2048", oag_2048::TITLE, None));
    rows.push(row("omega", oag_omega::TITLE, None));
    let mut launcher = chooser(rows, &[]);
    assert_eq!(launcher.card_list().len(), 5);

    // The first row is Pulse, the second card by release.
    assert_eq!(launcher.focused_card(), 1);
    press(&mut launcher, Button::Right);
    assert_eq!(launcher.focused_card(), 2);
    press(&mut launcher, Button::Down);
    assert!(launcher.focused_card() >= 3, "down reaches the lower row");
    press(&mut launcher, Button::Up);
    assert!(launcher.focused_card() < 3);
    press(&mut launcher, Button::Left);
    press(&mut launcher, Button::Left);
    press(&mut launcher, Button::Left);

    let rects = layout(5, 0);
    assert_eq!(hover(&mut launcher, centre(rects[4])), None);
    assert_eq!(launcher.focused_card(), 4, "hover focuses");
    assert_eq!(launcher.pick().as_deref(), Some("data/images/omega"));
}

/// Every card is on screen, so nothing ever scrolls under a hovering pointer.
#[test]
fn cards_fit_the_screen_without_overlapping() {
    for count in 1..=12 {
        for notes in 0..=2 {
            let rects = layout(count, notes);
            assert_eq!(rects.len(), count);
            for (at, a) in rects.iter().enumerate() {
                assert!(a[2] >= 40.0, "{count} cards: {a:?}");
                assert!(a[0] >= 0.0 && a[0] + a[2] <= SCREEN.0, "{a:?}");
                assert!(a[1] >= 0.0 && a[1] + a[3] <= SCREEN.1 - 30.0, "{a:?}");
                for b in &rects[at + 1..] {
                    let apart = a[0] + a[2] <= b[0]
                        || b[0] + b[2] <= a[0]
                        || a[1] + a[3] <= b[1]
                        || b[1] + b[3] <= a[1];
                    assert!(apart, "{count} cards: {a:?} overlaps {b:?}");
                }
            }
        }
    }
}

#[test]
fn a_card_whose_art_will_not_read_says_its_name() {
    let mut launcher = chooser(library(), &[]);
    // Nothing read at all, then a read that found nothing for Pure and a
    // picture for Pulse.
    let draws = draw(&launcher, &Sheet::default());
    assert!(!draws.iter().any(|d| matches!(d, Draw::Sprite { .. })));
    let drawn = texts(&draws);
    assert!(drawn.contains(&"Wipeout Pulse".to_string()), "{drawn:?}");
    assert!(drawn.contains(&"Wipeout Pure".to_string()), "{drawn:?}");

    launcher.set_picture("data/images/eu.chd", "pic1");
    let mut report = Vec::new();
    let sheet = Sheet::build_with(
        &[],
        vec![DecodedImage {
            src: picture_key("data/images/eu.chd", "pic1"),
            width: 16,
            height: 9,
            rgba: vec![255; 16 * 9 * 4],
            quad_extent: None,
            quad_centre: None,
            blend: None,
        }],
        &mut report,
    );
    let draws = draw(&launcher, &sheet);
    assert_eq!(
        draws
            .iter()
            .filter(|d| matches!(d, Draw::Sprite { .. }))
            .count(),
        1,
        "Pulse's picture and no stand-in for Pure"
    );
    let drawn = texts(&draws);
    // Each card's caption names its title and image; Pure's name is also drawn
    // in its empty card.
    let count = |name: &str| drawn.iter().filter(|t| *t == name).count();
    assert_eq!(count("Wipeout Pulse  UCES-00465"), 1, "{drawn:?}");
    assert_eq!(count("Wipeout Pure  UCES-00001"), 1, "{drawn:?}");
    assert_eq!(count("Wipeout Pure"), 1, "{drawn:?}");
}

#[test]
fn a_caption_cuts_the_title_and_keeps_the_image() {
    let long = row("ps4", oag_omega::TITLE, None);
    let chars = |count: f32| count * CAPTION_CHAR;
    assert_eq!(
        caption(&long, chars(40.0)),
        "Wipeout: Omega Collection  ps4"
    );
    assert_eq!(caption(&long, chars(20.0)), "Wipeout: Ome...  ps4");
}

#[test]
fn the_list_is_what_it_was_and_is_asked_for_by_flag_or_setting() {
    assert!(wants_cards(false, LauncherStyle::Cards));
    assert!(!wants_cards(true, LauncherStyle::Cards));
    assert!(!wants_cards(false, LauncherStyle::List));

    let list = Launcher::new(library());
    assert!(!list.is_cards());
    assert_eq!(draw(&list, &Sheet::default()), draw_list(&list));
}

/// The footer offers the switch only where there is something to switch to.
#[test]
fn the_square_hint_follows_the_focused_card() {
    let mut launcher = chooser(library(), &[]);
    let presses = |launcher: &Launcher| -> Vec<Press> {
        crate::launcher::hints(launcher)
            .iter()
            .map(|hint| hint.press)
            .collect()
    };
    assert!(presses(&launcher).contains(&Press::Button(Button::Square)));
    press(&mut launcher, Button::Right);
    assert!(!presses(&launcher).contains(&Press::Button(Button::Square)));
}
