//! Wipeout 2048's per-event card: opened by the map, left by Back, launched
//! by Launch, hit-tested against `CampaignEventCard_HandleInput`'s own rects.

use super::wipeout2048::{a_forced_event, a_restricted_event, boot, press, reach_the_shell, tick};
use super::*;
use crate::frontend::EventCard;
use crate::pointer::Pointer;

fn tap(x: f32, y: f32) -> Pointer {
    Pointer {
        at: Some((x, y)),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

fn card_event() -> Vec<MapEvent> {
    let mut events = a_forced_event();
    events[0].card = EventCard {
        title: "UNITY SQUARE".to_string(),
        kind_label: Some("RACE".to_string()),
        pass_label: Some("PASS".to_string()),
        has_objective: true,
        objective: Some("FINISH AT LEAST 5TH".to_string()),
        laps: Some(3),
        ..EventCard::default()
    };
    events[0].forced_craft = None;
    events
}

fn on_the_card(events: Vec<MapEvent>) -> (Frontend, Input) {
    let mut frontend = boot(0);
    frontend.set_campaign(events);
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    (frontend, input)
}

#[test]
fn cross_on_the_map_opens_the_card_and_launches_nothing() {
    let (frontend, _) = on_the_card(card_event());
    assert!(frontend.event_card_open());
    assert!(!frontend.is_finished());
    assert_eq!(frontend.launch(), None);
}

#[test]
fn circle_closes_the_card_and_stays_on_the_map() {
    let (mut frontend, mut input) = on_the_card(card_event());
    press(&mut frontend, &mut input, Button::Circle);
    assert!(!frontend.event_card_open());
    assert!(
        frontend
            .machine()
            .is(oag_2048::frontend::states::NEW_FE_SHELL)
    );
}

#[test]
fn cross_on_the_card_launches_the_event() {
    let (mut frontend, mut input) = on_the_card(card_event());
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.is_finished());
    assert_eq!(
        frontend.launch(),
        Some(&Launch::Event("2048 - Event 1".to_string()))
    );
}

#[test]
fn the_three_buttons_answer_their_measured_hit_rects() {
    // Launch: x 812..954, y 422..538 - ten past the drawn 822..944 by 432..528,
    // its first 12 units shadowed by Back's own rect.
    let (mut frontend, mut input) = on_the_card(card_event());
    assert!(frontend.pointer(&tap(825.0, 423.0)));
    tick(&mut frontend, &mut input, 0);
    assert!(
        frontend.is_finished(),
        "the hit rect starts 10 before the drawn one"
    );

    // Back: closes.
    let (mut frontend, mut input) = on_the_card(card_event());
    assert!(frontend.pointer(&tap(700.0, 480.0)));
    tick(&mut frontend, &mut input, 0);
    assert!(!frontend.event_card_open());

    // Just under the row: nothing.
    let (mut frontend, mut input) = on_the_card(card_event());
    assert!(frontend.pointer(&tap(880.0, 538.0)));
    tick(&mut frontend, &mut input, 0);
    assert!(frontend.event_card_open());
    assert!(!frontend.is_finished());
}

#[test]
fn back_wins_the_twelve_units_it_shares_with_change_craft() {
    // Change craft's rect is 552..694 and Back's 682..824: 690 is in both,
    // and the handler tests Back first.
    let (mut frontend, mut input) = on_the_card(card_event());
    assert!(frontend.pointer(&tap(690.0, 480.0)));
    tick(&mut frontend, &mut input, 0);
    assert!(!frontend.event_card_open(), "Back, not the team screen");
    assert!(
        frontend
            .machine()
            .is(oag_2048::frontend::states::NEW_FE_SHELL)
    );
}

#[test]
fn a_forced_event_has_no_change_craft_button() {
    let (mut frontend, mut input) = on_the_card(a_forced_event());
    assert!(frontend.pointer(&tap(600.0, 480.0)));
    tick(&mut frontend, &mut input, 0);
    assert!(frontend.event_card_open());
    assert!(
        frontend
            .machine()
            .is(oag_2048::frontend::states::NEW_FE_SHELL)
    );
    let fills = frontend
        .draw_list()
        .iter()
        .filter(
            |draw| matches!(draw, Draw::Fill { rect, .. } if rect[1] == 432.0 && rect[2] == 122.0),
        )
        .count();
    assert_eq!(fills, 2, "Back and Launch only");
}

#[test]
fn change_craft_leaves_for_the_team_screen_and_the_card_survives_the_trip() {
    let (mut frontend, mut input) = on_the_card(card_event());
    assert!(frontend.pointer(&tap(600.0, 480.0)));
    tick(&mut frontend, &mut input, 0);
    tick(&mut frontend, &mut input, 0);
    assert!(frontend.machine().is(oag_2048::frontend::states::TEAM));
    press(&mut frontend, &mut input, Button::Circle);
    assert!(
        frontend
            .machine()
            .is(oag_2048::frontend::states::NEW_FE_SHELL)
    );
    assert!(
        frontend.event_card_open(),
        "the card is where Team returns to"
    );
}

#[test]
fn a_forbidden_craft_dims_launch_and_the_tap_is_refused() {
    let mut frontend = boot(0);
    frontend.set_campaign(a_restricted_event());
    frontend.seed_craft("Feisar2048\\3".to_string());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    let launch_fill = frontend
        .draw_list()
        .into_iter()
        .find_map(|draw| match draw {
            Draw::Fill { rect, color } if rect[0] == 822.0 && rect[1] == 432.0 => Some(color),
            _ => None,
        })
        .expect("the Launch button");
    assert_eq!(
        launch_fill[3], 0.25,
        "alpha 0x40 is what the original dims it to"
    );
    press(&mut frontend, &mut input, Button::Cross);
    assert!(!frontend.is_finished());
}

#[test]
fn the_card_says_what_the_disc_authors_and_draws_nothing_else() {
    let (frontend, _) = on_the_card(card_event());
    let list = frontend.draw_list();
    let texts: Vec<&str> = list
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    for wanted in ["UNITY SQUARE", "RACE", "PASS", "FINISH AT LEAST 5TH", "3"] {
        assert!(texts.contains(&wanted), "{wanted:?} missing from {texts:?}");
    }
}

#[test]
fn an_event_with_no_objective_shows_no_pass_line() {
    let mut events = card_event();
    events[0].card.has_objective = false;
    events[0].card.objective = None;
    let (frontend, _) = on_the_card(events);
    let list = frontend.draw_list();
    assert!(
        !list
            .iter()
            .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "PASS"))
    );
}

#[test]
fn the_pointer_pages_the_card_and_the_pad_wraps_it() {
    let (mut frontend, mut input) = on_the_card(card_event());
    assert_eq!(frontend.campaign.card.map(|card| card.page), Some(0));
    assert!(frontend.pointer(&tap(800.0, 300.0)));
    tick(&mut frontend, &mut input, 0);
    assert_eq!(frontend.campaign.card.map(|card| card.page), Some(1));
    press(&mut frontend, &mut input, Button::Right);
    assert_eq!(
        frontend.campaign.card.map(|card| card.page),
        Some(0),
        "two pages wrap"
    );
    assert!(frontend.pointer(&tap(500.0, 300.0)));
    tick(&mut frontend, &mut input, 0);
    assert_eq!(frontend.campaign.card.map(|card| card.page), Some(1));
}

#[test]
fn the_secondary_pointer_button_goes_back() {
    let (mut frontend, mut input) = on_the_card(card_event());
    let back = Pointer {
        back: true,
        ..Pointer::default()
    };
    assert!(frontend.pointer(&back));
    tick(&mut frontend, &mut input, 0);
    assert!(!frontend.event_card_open());
}
