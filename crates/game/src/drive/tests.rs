use super::*;
use oag_gameplay::input::Button;

fn rows(spec: &[(&str, Option<&str>, Option<&str>)]) -> Vec<Row> {
    spec.iter()
        .enumerate()
        .map(|(index, (label, id, setting))| Row {
            index,
            id: id.map(str::to_string),
            label: (*label).to_string(),
            kind: "action",
            setting: setting.map(str::to_string),
            enabled: true,
            ..Row::default()
        })
        .collect()
}

fn screen(focus: usize, step: Option<(&'static str, &'static str)>, rows: Vec<Row>) -> Screen {
    let row = &rows[focus];
    Screen {
        kind: "page".into(),
        id: "race".into(),
        focus: Some(Focus {
            index: focus,
            id: row.id.clone(),
            label: row.label.clone(),
            setting: row.setting.clone(),
        }),
        step,
        rows,
        ..Screen::default()
    }
}

#[test]
fn a_script_parses_every_step_with_its_line() {
    let script = Script::parse(
        "until stage=menu within 99  # boot\nselect \"LAUNCH RACE\"; press cross down\nset race.mode=time_trial\nticks 5\nscreenshot a.png\nexpect page!=race\nquit",
    )
    .unwrap();
    let kinds: Vec<String> = script
        .steps
        .iter()
        .map(|(n, s)| format!("{n}:{s}"))
        .collect();
    assert_eq!(kinds.len(), 8);
    assert_eq!(kinds[0], "1:until stage=menu within 99");
    assert_eq!(kinds[1], "2:select \"LAUNCH RACE\" within 1800");
    assert!(matches!(&script.steps[2].1, Step::Press(b) if b == &[Button::Cross, Button::Down]));
    assert!(
        matches!(&script.steps[3].1, Step::Set { setting, value, .. } if setting == "race.mode" && value == "time_trial")
    );
}

#[test]
fn a_bad_script_names_its_line_and_the_fault() {
    for (text, fragment) in [
        ("press cross\nfly away", "line 2"),
        ("until nonsense=1", "not a state key"),
        ("press wibble", "not a button"),
        ("select a b", "one row"),
        ("until stage=menu within soon", "not a number"),
        ("select \"open", "never closed"),
    ] {
        let why = Script::parse(text).unwrap_err();
        assert!(why.contains(fragment), "{text:?} -> {why}");
    }
}

#[test]
fn selecting_walks_down_a_list_and_finds_the_forward_button_by_pressing() {
    let list = rows(&[
        ("A", Some("a"), None),
        ("B", None, Some("x.b")),
        ("C", Some("c"), None),
    ]);
    let mut select = Select::new("c");
    // The screen does not say which way is forward: Down is tried first, and moves.
    assert_eq!(
        select.next(&screen(0, None, list.clone())),
        Ok(Next::Press(Button::Down))
    );
    assert_eq!(
        select.next(&screen(1, None, list.clone())),
        Ok(Next::Press(Button::Down))
    );
    assert_eq!(select.next(&screen(2, None, list)), Ok(Next::Done));
}

#[test]
fn a_press_that_moves_nothing_tries_the_next_direction() {
    let list = rows(&[("A", Some("a"), None), ("B", Some("b"), None)]);
    let mut select = Select::new("b");
    assert_eq!(
        select.next(&screen(0, None, list.clone())),
        Ok(Next::Press(Button::Down))
    );
    // A strip: Down did nothing, so Right is tried.
    assert_eq!(
        select.next(&screen(0, None, list.clone())),
        Ok(Next::Press(Button::Right))
    );
    assert_eq!(select.next(&screen(1, None, list)), Ok(Next::Done));
}

#[test]
fn a_screen_that_names_its_buttons_is_not_probed() {
    let list = rows(&[("A", Some("a"), None), ("B", Some("b"), None)]);
    let mut select = Select::new("b");
    assert_eq!(
        select.next(&screen(0, Some(("right", "left")), list)),
        Ok(Next::Press(Button::Right))
    );
}

#[test]
fn a_row_that_is_not_there_fails_with_the_rows_that_are() {
    let list = rows(&[("A", Some("a"), None), ("B", None, Some("x.b"))]);
    let why = Select::new("zzz").next(&screen(0, None, list)).unwrap_err();
    assert!(
        why.contains("no row \"zzz\"") && why.contains("\"a\"") && why.contains("\"x.b\""),
        "{why}"
    );
}

#[test]
fn a_row_is_found_by_id_setting_label_or_index() {
    let list = rows(&[("Alpha", Some("a"), None), ("Beta", None, Some("x.b"))]);
    let s = screen(0, None, list);
    for target in ["a", "x.b", "beta", "#1"] {
        assert!(Select::new(target).row(&s).is_some(), "{target}");
    }
}

#[test]
fn a_choice_is_stepped_until_it_reads_the_value_and_refuses_one_it_lacks() {
    let mut list = rows(&[("MODE", None, Some("race.mode"))]);
    list[0].kind = "choice";
    list[0].values = vec![
        Opt {
            value: "a".into(),
            label: "Alpha".into(),
            unavailable: None,
        },
        Opt {
            value: "b".into(),
            label: "Beta".into(),
            unavailable: None,
        },
        Opt {
            value: "/x/c.toml".into(),
            label: "C".into(),
            unavailable: Some("still checking".into()),
        },
    ];
    list[0].value = Some("a".into());
    let mut set = SetChoice::new("race.mode", "Beta");
    assert_eq!(
        set.next(&screen(0, None, list.clone())),
        Ok(Next::Press(Button::Right))
    );
    list[0].value = Some("b".into());
    assert_eq!(set.next(&screen(0, None, list.clone())), Ok(Next::Done));
    let Ok(Next::Wait(why)) =
        SetChoice::new("race.mode", "c.toml").next(&screen(0, None, list.clone()))
    else {
        panic!(
            "an option that cannot be used yet is waited for, and a file is named by its file name"
        );
    };
    assert!(why.contains("cannot use it yet: still checking"), "{why}");
    let Ok(Next::Wait(why)) = SetChoice::new("race.mode", "gamma").next(&screen(0, None, list))
    else {
        panic!("a value the row lacks is waited for, in case a thread is still filling the list");
    };
    assert!(
        why.contains("does not offer") && why.contains("a=\"Alpha\""),
        "{why}"
    );
}

#[test]
fn a_state_line_is_json_and_a_condition_reads_it() {
    let state = State {
        tick: 7,
        stage: "menu".into(),
        page: Some("race".into()),
        screen: Some(screen(0, None, rows(&[("START", Some("go"), None)]))),
        ..State::default()
    };
    let line = state.to_line();
    assert!(
        line.starts_with("{\"tick\":7,\"stage\":\"menu\",\"page\":\"race\""),
        "{line}"
    );
    let script = Script::parse("expect focus=go page=RACE stage!=race").unwrap();
    let Step::Expect { conds } = &script.steps[0].1 else {
        panic!()
    };
    assert!(state.satisfies(conds));
    assert!(state.same_as(&State {
        tick: 99,
        ..state.clone()
    }));
}
