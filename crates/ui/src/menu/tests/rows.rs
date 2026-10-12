//! What a row does when it is activated: choices cycle, toggles flip,
//! seeding and supplied lists land where the caller meant, actions fire.
//!
//! Split out of `menu/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

#[test]
fn a_choice_cycles_and_reports_every_step() {
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Cross]);

    let events = press(&mut menu, &[Button::Right]);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "graphics.anisotropy".to_string(),
            value: Value::Text("4x".to_string()),
        }]
    );
    // And wraps backwards off the start rather than sticking.
    press(&mut menu, &[Button::Left]);
    let events = press(&mut menu, &[Button::Left]);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "graphics.anisotropy".to_string(),
            value: Value::Text("16x".to_string()),
        }]
    );
}

/// One button has to be enough to change everything, for a player on a pad
/// who never finds left and right.
#[test]
fn activating_a_choice_steps_it_forward() {
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Cross]);
    let events = press(&mut menu, &[Button::Cross]);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "graphics.anisotropy".to_string(),
            value: Value::Text("4x".to_string()),
        }]
    );
}

#[test]
fn a_toggle_flips_and_reads_out_as_on_or_off() {
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Cross]);
    press(&mut menu, &[Button::Down]);

    assert_eq!(
        menu.page().entries[1].value(),
        Some(Value::Flag(false)),
        "a toggle starts off unless it is seeded"
    );
    let events = press(&mut menu, &[Button::Right]);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "graphics.switch".to_string(),
            value: Value::Flag(true),
        }]
    );
    assert_eq!(Value::Flag(true).to_string(), "ON");
}

#[test]
fn seeding_moves_a_row_to_the_value_the_caller_holds() {
    let mut menu = Menu::new(fixture());
    assert!(menu.seed("graphics.anisotropy", &Value::Text("16x".to_string())));
    press(&mut menu, &[Button::Cross]);
    assert_eq!(
        menu.page().entries[0].value(),
        Some(Value::Text("16x".to_string()))
    );
}

/// A config file naming a value the menus do not offer must not be able to
/// put an unreachable option on the list.
#[test]
fn seeding_a_value_the_row_does_not_offer_changes_nothing() {
    let mut menu = Menu::new(fixture());
    assert!(!menu.seed("graphics.anisotropy", &Value::Text("64x".to_string())));
    press(&mut menu, &[Button::Cross]);
    assert_eq!(
        menu.page().entries[0].value(),
        Some(Value::Text("off".to_string())),
        "the row stays on a value the player can actually reach"
    );
}

/// The bug this ordering rule exists to stop: a row seeded to the player's
/// value, then handed its list, must not silently fall back to whatever is
/// first. It would draw wrong *and* the next nudge would persist the wrong
/// value as a deliberate choice.
#[test]
fn supplying_a_list_keeps_the_value_the_row_was_already_on() {
    let definition = Definition::parse(
        r#"
        toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
        root = "main"
        [[page]]
        id = "main"
        [[page.entry]]
        kind = "choice"
        label = "LANGUAGE"
        setting = "language"
        values_from = "languages"
        "#,
        &crate::language::StringTable::default(),
    )
    .expect("parse");
    let mut menu = Menu::new(definition);

    let offered =
        |names: &[&str]| -> Vec<Choice> { names.iter().map(|n| Choice::plain(*n)).collect() };
    menu.supply(ValueSource::Languages, &offered(&["French", "English"]));
    assert!(menu.seed("language", &Value::Text("English".to_string())));
    assert_eq!(
        menu.page().entries[0].chosen(),
        Some(Value::Text("English".to_string()))
    );

    // Supplied again, the seeded value survives even though it is not first.
    menu.supply(
        ValueSource::Languages,
        &offered(&["French", "German", "English"]),
    );
    assert_eq!(
        menu.page().entries[0].chosen(),
        Some(Value::Text("English".to_string())),
        "a re-supplied list must not reset the row to its first option"
    );

    // And a list that no longer has it falls to the first, which is the only
    // thing left to fall to.
    menu.supply(ValueSource::Languages, &offered(&["French", "German"]));
    assert_eq!(
        menu.page().entries[0].chosen(),
        Some(Value::Text("French".to_string()))
    );
}

/// A disc-supplied row stores an id and shows a name, and the event carries
/// the id. Persisting the label would write words a player read into a
/// settings file, where the next run would not recognise them.
#[test]
fn a_supplied_row_shows_its_label_and_reports_its_value() {
    let definition = Definition::parse(
        r#"
        toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
        root = "main"
        [[page]]
        id = "main"
        [[page.entry]]
        kind = "choice"
        label = "TRACK"
        setting = "race.track"
        values_from = "tracks"
        "#,
        &crate::language::StringTable::default(),
    )
    .expect("parse");
    let mut menu = Menu::new(definition);
    menu.supply(
        ValueSource::Tracks,
        &[
            Choice::labelled("16_Track", "A Circuit"),
            Choice::labelled("32_Track", "A Circuit Reversed"),
        ],
    );

    assert_eq!(
        menu.page().entries[0].value(),
        Some(Value::Text("A Circuit".to_string())),
        "the row draws the name"
    );
    let events = press(&mut menu, &[Button::Right]);
    assert_eq!(
        events,
        vec![MenuEvent::Changed {
            setting: "race.track".to_string(),
            value: Value::Text("32_Track".to_string()),
        }],
        "and reports the id"
    );
}

/// A row whose source turned out to have nothing draws no value and does
/// not move, rather than panicking on an empty list.
#[test]
fn a_row_with_nothing_supplied_is_inert() {
    let definition = Definition::parse(
        r#"
        toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
        root = "main"
        [[page]]
        id = "main"
        [[page.entry]]
        kind = "choice"
        label = "TRACK"
        setting = "race.track"
        values_from = "tracks"
        "#,
        &crate::language::StringTable::default(),
    )
    .expect("parse");
    let mut menu = Menu::new(definition);
    menu.supply(ValueSource::Tracks, &[]);

    assert_eq!(menu.page().entries[0].value(), None);
    assert!(press(&mut menu, &[Button::Right]).is_empty());
    assert!(press(&mut menu, &[Button::Cross]).is_empty());
}

#[test]
fn a_choice_cannot_declare_both_a_list_and_a_source() {
    let error = Definition::parse(
        r#"
        toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
        root = "main"
        [[page]]
        id = "main"
        [[page.entry]]
        kind = "choice"
        label = "TRACK"
        setting = "race.track"
        values = ["16_Track"]
        values_from = "tracks"
        "#,
        &crate::language::StringTable::default(),
    )
    .expect_err("refused");
    assert!(error.to_string().contains("alternatives"), "{error}");
}

#[test]
fn seeding_a_key_nothing_edits_says_so() {
    let mut menu = Menu::new(fixture());
    assert!(!menu.seed("graphics.nothing", &Value::Flag(true)));
}

#[test]
fn an_action_row_fires_rather_than_navigating() {
    let mut menu = Menu::new(fixture());
    press(&mut menu, &[Button::Down]);
    let events = press(&mut menu, &[Button::Cross]);
    assert_eq!(events, vec![MenuEvent::Fired(Action::Quit)]);
    assert_eq!(menu.page().id, "main", "firing does not move the cursor");
}

/// Backing out of the root is the menus saying they are done, not a no-op
/// and not a crash.
#[test]
fn backing_out_of_the_root_closes_the_menus() {
    let mut menu = Menu::new(fixture());
    let events = press(&mut menu, &[Button::Circle]);
    assert_eq!(events, vec![MenuEvent::Closed]);
    assert_eq!(menu.depth(), 1, "and leaves the stack alone");
}

/// A title whose original words the campaign row differently has the row
/// renamed in place, and a title that does not keeps the row as written.
#[test]
fn the_campaign_row_is_worded_by_the_title_and_keeps_its_place() {
    let strings = crate::language::StringTable::default();
    let label_of = |definition: &Definition| {
        definition
            .page("main")
            .unwrap()
            .entries
            .iter()
            .enumerate()
            .find_map(|(index, entry)| match entry {
                Entry::Run {
                    action: Action::OpenCampaign,
                    label,
                } => Some((index, label.clone())),
                _ => None,
            })
            .unwrap()
    };
    let built_in = Definition::parse(BUILT_IN, &strings).unwrap();
    let (place, kept) = label_of(&built_in);
    let mut pulse = built_in.clone();
    pulse.rename_campaign_row(oag_pulse::TITLE, &strings);
    assert_eq!(label_of(&pulse), (place, kept), "Pulse keeps RACE CAMPAIGN");
    let mut pure = built_in;
    pure.rename_campaign_row(oag_pure::TITLE, &strings);
    assert_eq!(label_of(&pure), (place, "SINGLE PLAYER".to_string()));
}

/// An option that cannot be used is still offered: the row steps onto it, draws
/// its value dimmed with the reason under the rows, and steps off again. A
/// `disabled_by` row would refuse the step and strand the player on it.
#[test]
fn an_unusable_option_is_listed_dimmed_with_its_reason_and_can_be_left() {
    let definition = Definition::parse(
        r#"
        toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
        root = "main"
        [[page]]
        id = "main"
        title = "MAIN"
        [[page.entry]]
        kind = "choice"
        label = "TRACK"
        setting = "remix.track"
        values_from = "remix_tracks"
        "#,
        &crate::language::StringTable::default(),
    )
    .expect("parses");
    let mut menu = Menu::new(definition);
    menu.supply(
        ValueSource::RemixTracks,
        &[
            Choice::labelled("good", "GOOD"),
            Choice::labelled("bad", "BAD").unusable("no lap loop"),
        ],
    );
    let drawn = |menu: &Menu| list(menu, &no_bindings, None);
    let note = |draws: &[Draw]| {
        draws
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if text == "! no lap loop"))
    };
    let value_color = |draws: &[Draw], value: &str| {
        draws.iter().find_map(|d| match d {
            Draw::Text { text, color, .. } if text == value => Some(*color),
            Draw::FacedText { text, color, .. } if text == value => Some(*color),
            _ => None,
        })
    };

    assert!(!note(&drawn(&menu)), "a usable option says nothing");
    let events = press(&mut menu, &[Button::Right]);
    assert_eq!(events.len(), 1, "the row steps onto the unusable option");
    let on_bad = drawn(&menu);
    assert!(note(&on_bad), "its reason shows under the rows");
    assert_eq!(value_color(&on_bad, "BAD"), Some(DIMMED));
    assert!(
        menu.reserves_unavailable_note(),
        "the note's line is kept clear"
    );

    press(&mut menu, &[Button::Left]);
    assert!(!note(&drawn(&menu)), "stepping back clears it");
}
