//! The 2048 campaign map opened by a title's own menus (Omega's chooser), on the
//! shared fixture: it opens on the map, Back hands the window back, and an event
//! launched from it leaves with the launch request. The disc-backed twin is
//! `crates/game/tests/omega_2048_map_ground_truth.rs`.

use super::wipeout2048::{XML, a_forced_event, press, tick};
use super::*;
use oag_2048::frontend::states as w2048;

/// The fixture's `newFEshell` stands in for Omega's `Cell Selection 2048`, which
/// carries the canvas under another name.
const SCREEN: &str = "newFEshell";

fn hosted() -> (Frontend, Input) {
    hosted_over(XML)
}

#[test]
fn a_hosted_map_opens_on_the_map_not_on_a_boot_screen() {
    let (mut frontend, mut input) = hosted();
    tick(&mut frontend, &mut input, 0);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
    assert!(!frontend.left_map());
    assert!(!frontend.is_finished());
}

#[test]
fn back_leaves_a_hosted_map_instead_of_firing_home_or_the_mode_grid() {
    for button in [Button::Circle, Button::Triangle] {
        let (mut frontend, mut input) = hosted();
        tick(&mut frontend, &mut input, 0);
        press(&mut frontend, &mut input, button);
        assert!(frontend.left_map(), "{button:?}");
        assert!(
            frontend.machine().is(w2048::NEW_FE_SHELL),
            "{button:?} stays on the map for the host to take over"
        );
        assert!(!frontend.is_finished());
    }
}

#[test]
fn an_event_launched_from_a_hosted_map_finishes_with_its_launch() {
    let (mut frontend, mut input) = hosted();
    tick(&mut frontend, &mut input, 0);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.event_card_open());
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.is_finished());
    assert_eq!(
        frontend.launch(),
        Some(&Launch::Event("2048 - Event 1".to_string()))
    );
    assert!(!frontend.left_map());
}

fn hosted_over(xml: &str) -> (Frontend, Input) {
    let mut frontend = Frontend::hosted_map(
        Screens::from_xml(xml),
        StringTable::default(),
        Vec::new(),
        Vec::new(),
        SCREEN,
    );
    frontend.set_space(Space::VITA);
    let mut events = a_forced_event();
    events[0].forced_craft = None;
    frontend.set_campaign(events);
    (frontend, Input::new())
}

#[test]
fn change_craft_goes_to_the_team_screen_only_where_there_is_one() {
    let (mut frontend, mut input) = hosted_over(XML);
    tick(&mut frontend, &mut input, 0);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Square);
    assert!(frontend.machine().is(w2048::TEAM));

    // Omega ships no `team` screen: the button does nothing rather than leave
    // the map for a state nothing draws.
    let without = XML.replace(r#"name="team""#, r#"name="elsewhere""#);
    let (mut frontend, mut input) = hosted_over(&without);
    tick(&mut frontend, &mut input, 0);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Square);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
    assert!(frontend.event_card_open());
}
