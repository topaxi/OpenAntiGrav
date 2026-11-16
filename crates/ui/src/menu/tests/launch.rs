//! `launch_race` rows that name their inputs by setting key.

use super::*;

fn parse(rows: &str) -> Result<Definition, Error> {
    Definition::parse(
        &format!(
            "toml-schema = {{ location = \"https://oag.topaxi.com/mods/schemas/v1/menu.tosd\", version = \"1.0.0\" }}\n\
             root = \"main\"\n[[page]]\nid = \"main\"\ntitle = \"MAIN\"\n{rows}"
        ),
        &crate::language::StringTable::default(),
    )
}

#[test]
fn a_launch_race_row_hands_its_keys_to_the_game() {
    let mut menu = Menu::new(
        parse(
            "[[page.entry]]\nkind = \"action\"\nlabel = \"GO\"\naction = \"launch_race\"\n\
             launch = { track_title = \"mod.a.b.title\", track = \"mod.a.b.track\", mode = \"race.mode\" }\n",
        )
        .expect("a launch table on launch_race parses"),
    );
    let events = press(&mut menu, &[Button::Cross]);
    assert_eq!(
        events,
        [MenuEvent::Launch(LaunchInputs {
            track_title: Some("mod.a.b.title".into()),
            track: Some("mod.a.b.track".into()),
            mode: Some("race.mode".into()),
            ..LaunchInputs::default()
        })]
    );
}

#[test]
fn a_launch_race_row_without_keys_is_the_race_pages_own_launch() {
    let mut menu = Menu::new(
        parse("[[page.entry]]\nkind = \"action\"\nlabel = \"GO\"\naction = \"launch_race\"\n")
            .unwrap(),
    );
    assert_eq!(
        press(&mut menu, &[Button::Cross]),
        [MenuEvent::Fired(Action::LaunchRace)]
    );
}

#[test]
fn a_launch_table_on_another_action_is_refused() {
    let err = parse(
        "[[page.entry]]\nkind = \"action\"\nlabel = \"GO\"\naction = \"quit\"\nlaunch = { track = \"x.y\" }\n",
    )
    .expect_err("only launch_race takes a launch table");
    assert!(err.to_string().contains("launch_race"), "{err}");
}

#[test]
fn an_unknown_launch_input_is_refused() {
    assert!(
        parse(
            "[[page.entry]]\nkind = \"action\"\nlabel = \"GO\"\naction = \"launch_race\"\nlaunch = { bogus = \"x\" }\n",
        )
        .is_err()
    );
}
