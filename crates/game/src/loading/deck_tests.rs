//! Which feature a race's loading screen may show, by mode.
//!
//! The law is `oag_hd::loading::DECK`, read out of `LoadingScreen_Construct`;
//! these pin that the screen *uses* it, so a draw that ignored the mode and
//! went back to all five would fail here.

use super::*;
use oag_title::loading::Deck;

/// Five features laid out as Wipeout HD's table is, slot for slot.
fn five(deck: Option<Deck>) -> Assets {
    let features = (0..5)
        .map(|slot| Feature {
            slot,
            title: Some(format!("T{slot}")),
            heading: Some(format!("H{slot}")),
            description: format!("D{slot}"),
        })
        .collect();
    Assets {
        features,
        deck,
        wave: false,
        ..Assets::default()
    }
}

fn shown(assets: &Assets, mode: Option<u32>, seeds: u64) -> std::collections::BTreeSet<String> {
    (0..seeds)
        .filter_map(|draw| {
            Screen::for_mode(assets, AUTHORED_LINE_HEIGHT, draw, None, mode)
                .feature_title()
                .map(str::to_string)
        })
        .collect()
}

fn titles(slots: &[u8]) -> std::collections::BTreeSet<String> {
    slots.iter().map(|slot| format!("T{slot}")).collect()
}

/// A single race on the Fury disc draws from four, and never shows Flip.
///
/// `SPArcade` is mode 3, which takes the constructor's default branch, and the
/// default is `((b - 1) >> 31) + 4` wide for the Fury-content byte `b`. The
/// byte read `1` live on the EU Fury disc.
#[test]
fn a_single_race_draws_from_four_and_never_flip() {
    let assets = five(Some(oag_hd::loading::DECK));
    let seen = shown(&assets, Some(3), 200);
    assert_eq!(seen, titles(&[0, 1, 2, 3]), "{seen:?}");
}

/// Without the Fury content the default deck is three wide.
#[test]
fn without_the_fury_content_a_single_race_draws_from_three() {
    let deck = Deck {
        fury_content: false,
        ..oag_hd::loading::DECK
    };
    let seen = shown(&five(Some(deck)), Some(3), 200);
    assert_eq!(seen, titles(&[0, 1, 2]), "{seen:?}");
}

/// Eliminator is the one mode with all five.
#[test]
fn an_eliminator_race_draws_from_all_five() {
    let seen = shown(&five(Some(oag_hd::loading::DECK)), Some(8), 300);
    assert_eq!(seen, titles(&[0, 1, 2, 3, 4]), "{seen:?}");
}

/// The unnamed modes keep the decks the constructor gives them, including the
/// one that is not a prefix: `0xd` skips Pilot Assist for Absorb.
#[test]
fn the_unnamed_modes_keep_their_own_decks() {
    let assets = five(Some(oag_hd::loading::DECK));
    assert_eq!(shown(&assets, Some(0xd), 200), titles(&[0, 1, 3]));
    assert_eq!(shown(&assets, Some(0xe), 50), titles(&[1]));
    assert_eq!(shown(&assets, Some(6), 100), titles(&[0, 1]));
}

/// A screen covering no race, and a title with no deck, draw from everything.
#[test]
fn no_deck_means_every_feature() {
    let all = titles(&[0, 1, 2, 3, 4]);
    assert_eq!(shown(&five(None), Some(3), 300), all);
    assert_eq!(
        shown(&five(Some(oag_hd::loading::DECK)), None, 300),
        titles(&[0, 1, 2, 3])
    );
}

/// A deck whose features did not resolve falls back rather than drawing none.
#[test]
fn a_deck_with_nothing_resolved_falls_back_to_what_is_there() {
    let mut assets = five(Some(oag_hd::loading::DECK));
    assets.features.retain(|feature| feature.slot == 4);
    let seen = shown(&assets, Some(3), 20);
    assert_eq!(seen, titles(&[4]));
}

/// The slot, not the position, is what a deck names: dropping the first
/// feature must not shift the others into the wrong decks.
#[test]
fn a_dropped_feature_does_not_shift_the_decks() {
    let mut assets = five(Some(oag_hd::loading::DECK));
    assets.features.remove(0);
    let seen = shown(&assets, Some(3), 200);
    assert_eq!(seen, titles(&[1, 2, 3]), "{seen:?}");
}

/// The executable's own ids for the modes it names, and nothing invented for
/// the ones it does not.
#[test]
fn only_the_named_modes_have_an_executable_id() {
    use oag_race::Mode;
    assert_eq!(executable_mode(Mode::SingleRace), Some(3));
    assert_eq!(executable_mode(Mode::Eliminator), Some(8));
    assert_eq!(executable_mode(Mode::TimeTrial), Some(5));
    assert_eq!(executable_mode(Mode::Tournament), Some(4));
    assert_eq!(executable_mode(Mode::Zone), None);
    assert_eq!(executable_mode(Mode::SpeedLap), None);
}

/// The name sits over the picture and the heading over the prose, in one row.
#[test]
fn the_name_is_over_the_picture_and_the_heading_over_the_prose() {
    let assets = Assets {
        art: Some(Art {
            sheet: oag_hud::sprite::Sheet::default(),
            illustrations: vec![[0.0, 0.0, 64.0, 32.0]; 5],
            title_arrow: None,
            subtitle_arrow: None,
            rule: None,
            corner: None,
            dot: None,
            square: None,
        }),
        ..five(Some(oag_hd::loading::DECK))
    };
    let screen = Screen::for_mode(&assets, AUTHORED_LINE_HEIGHT, 0, None, Some(3));
    let list = screen.draw_list(Phase::Race, &Progress::default(), &Atlas::build());
    let at = |wanted: &str| {
        list.iter().find_map(|draw| match draw {
            Draw::Text { x, y, text, .. } if text.starts_with(wanted) => Some((*x, *y)),
            _ => None,
        })
    };
    let name = list
        .iter()
        .find_map(|draw| match draw {
            Draw::Text { text, x, y, .. } if text.starts_with('T') && text.len() == 2 => {
                Some((*x, *y, text.clone()))
            }
            _ => None,
        })
        .expect("a name is drawn");
    let heading = at("H").expect("a heading is drawn");
    assert!(
        name.0 < heading.0,
        "name left of heading: {name:?} {heading:?}"
    );
    assert!((name.1 - heading.1).abs() < 0.01, "one row");
}
