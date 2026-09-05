use std::collections::BTreeMap;

use winit::keyboard::{Key, NamedKey};

use super::*;

#[test]
fn the_default_table_agrees_with_map_key() {
    let bindings = Bindings::default();
    for (_, key) in keys::candidates() {
        assert_eq!(bindings.resolve(&key), keys::map_key(&key));
    }
}

#[test]
fn a_rebind_moves_the_key_and_reports_who_it_came_from() {
    let mut bindings = Bindings::default();
    // "S" is Down by default; give it to Square instead.
    assert_eq!(bindings.rebind(Button::Square, "S"), Some(Button::Down));
    assert_eq!(bindings.names_for(Button::Square), ["S"]);
    // Down loses nothing else it held - only "S" moved.
    assert_eq!(bindings.names_for(Button::Down), ["DOWN"]);
}

#[test]
fn rebinding_a_key_already_on_the_target_button_steals_nothing_but_still_replaces() {
    let mut bindings = Bindings::default();
    // "UP" already produces Up, so nothing is taken from another button -
    // but the row still ends up with exactly the one key just pressed, "W"
    // included, per the module doc's "replaces, does not add".
    assert_eq!(bindings.rebind(Button::Up, "UP"), None);
    assert_eq!(bindings.names_for(Button::Up), ["UP"]);
}

#[test]
fn a_rebind_replaces_every_key_the_button_held_rather_than_adding() {
    let mut bindings = Bindings::default();
    // Up starts with two keys ("UP", "W"); rebinding it to a third it does
    // not already hold leaves exactly the new one.
    bindings.rebind(Button::Up, "Q");
    assert_eq!(bindings.names_for(Button::Up), ["Q"]);
    // Both of Up's old keys are unbound now, not handed to anything else.
    assert!(bindings.resolve(&Key::Named(NamedKey::ArrowUp)).is_none());
    assert!(bindings.resolve(&Key::Character("w".into())).is_none());
    // And "Q"'s old owner, L, lost it to the steal.
    assert!(bindings.names_for(Button::L).is_empty());
}

#[test]
fn a_button_left_with_no_key_is_a_real_state_not_a_failure() {
    let mut bindings = Bindings::default();
    bindings.rebind(Button::Down, "UP");
    bindings.rebind(Button::Down, "W");
    // Up has now had both its default keys stolen.
    assert!(bindings.names_for(Button::Up).is_empty());
    for (_, key) in keys::candidates() {
        assert_ne!(bindings.resolve(&key), Some(Button::Up));
    }
}

#[test]
fn an_unrecognised_name_does_not_change_the_table() {
    let mut bindings = Bindings::default();
    assert_eq!(bindings.rebind(Button::Up, "F1"), None);
    assert_eq!(bindings, Bindings::default());
}

#[test]
fn to_pairs_names_every_candidate_and_flags_the_unbound_ones() {
    let mut bindings = Bindings::default();
    // Down's replace drops its own default keys, "DOWN" and "S", and takes
    // "UP" from Up.
    bindings.rebind(Button::Down, "UP");
    let pairs = bindings.to_pairs();
    assert_eq!(pairs.len(), keys::candidates().len());
    assert_eq!(pairs.get("UP").map(String::as_str), Some("down"));
    assert_eq!(pairs.get("DOWN").map(String::as_str), Some(UNBOUND));
    assert_eq!(pairs.get("S").map(String::as_str), Some(UNBOUND));
}

#[test]
fn a_rebind_round_trips_through_pairs() {
    let mut bindings = Bindings::default();
    bindings.rebind(Button::Square, "S");
    let (restored, ignored) = Bindings::from_pairs(&bindings.to_pairs());
    assert!(ignored.is_empty());
    assert_eq!(restored, bindings);
}

#[test]
fn a_missing_entry_falls_back_to_the_default_for_that_key_alone() {
    let mut pairs = Bindings::default().to_pairs();
    // As if this file predates a candidate this build added - "Q" was never
    // written at all, not written as "none".
    pairs.remove("Q");
    let (bindings, ignored) = Bindings::from_pairs(&pairs);
    assert!(ignored.is_empty());
    assert_eq!(bindings.names_for(Button::L), ["Q"]);
}

#[test]
fn an_unparseable_value_is_reported_and_the_default_kept() {
    let mut pairs = Bindings::default().to_pairs();
    pairs.insert("UP".to_string(), "sideways".to_string());
    let (bindings, ignored) = Bindings::from_pairs(&pairs);
    assert_eq!(ignored, ["UP"]);
    // The default for UP - Up - not the garbage value, and not unbound.
    assert_eq!(
        bindings.resolve(&Key::Named(NamedKey::ArrowUp)),
        Some(Button::Up)
    );
}

#[test]
fn an_unknown_key_name_in_the_file_is_silently_dropped() {
    // A newer build's candidate this one has never heard of. Not one of the
    // eighteen this build iterates, so it is never looked at at all - the
    // same way an unknown TOML table field already is elsewhere in
    // `settings.rs`.
    let mut pairs = Bindings::default().to_pairs();
    pairs.insert("F5".to_string(), "cross".to_string());
    let (bindings, ignored) = Bindings::from_pairs(&pairs);
    assert!(ignored.is_empty());
    assert_eq!(bindings, Bindings::default());
}

#[test]
fn from_pairs_on_an_empty_map_is_the_default_table() {
    let (bindings, ignored) = Bindings::from_pairs(&BTreeMap::new());
    assert!(ignored.is_empty());
    assert_eq!(bindings, Bindings::default());
}
