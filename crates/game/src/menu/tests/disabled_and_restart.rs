//! Rows the definition greys out, rows whose values come from a supplied
//! source, the restart note the renderer row raises, and the definitions the
//! loader refuses outright.
//!
//! Split out of `menu/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

/// A disabled row is selectable and readable and does not move, which is
/// three separate things a player would notice.
#[test]
fn a_disabled_row_is_inert_until_the_row_that_disables_it_moves_off_the_value() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("display"), "the display page exists");
    let row = menu
        .page()
        .entries
        .iter()
        .position(|entry| entry.setting() == Some("display.frame_limit"))
        .expect("the frame limit is on the display page");

    menu.seed("display.vsync", &Value::Text("on".to_string()));
    menu.seed("display.frame_limit", &Value::Text("60".to_string()));
    for _ in 0..row {
        press(&mut menu, &[Button::Down]);
    }
    assert_eq!(menu.selected(), row);

    // Right does nothing, and says nothing: an event here would persist a
    // change the player did not make.
    assert_eq!(press(&mut menu, &[Button::Right]), Vec::new());
    assert_eq!(press(&mut menu, &[Button::Cross]), Vec::new());
    assert_eq!(
        menu.page().entries[row].chosen(),
        Some(Value::Text("60".to_string()))
    );

    // And under either of the other two modes it is an ordinary row
    // again - `smooth` especially, where the limiter is the only thing
    // stopping the GPU rendering frames that get discarded.
    menu.seed("display.vsync", &Value::Text("smooth".to_string()));
    let events = press(&mut menu, &[Button::Right]);
    assert_eq!(events.len(), 1, "{events:?}");
    assert_ne!(
        menu.page().entries[row].chosen(),
        Some(Value::Text("60".to_string()))
    );
}

/// **The first source whose answer is routinely nothing.** Monitors and
/// renderers always hold `default`; languages, circuits and race modes are
/// never empty on a real disc. `music_sources` is empty on every machine
/// with one Pulse disc, which is most of them, so the empty case is the
/// *common* one here rather than a corner - and it has to draw, be walked
/// past, and refuse to move, without panicking on a `rem_euclid(0)` or an
/// index into an empty list.
#[test]
fn a_row_whose_source_has_nothing_is_inert_rather_than_a_panic() {
    let mut menu = Menu::new(built_in());
    assert_eq!(
        menu.supply(ValueSource::MusicSources, &[]),
        1,
        "the AUDIO page has exactly one such row"
    );
    assert!(menu.open("audio"));
    let row = menu
        .page()
        .entries
        .iter()
        .position(|entry| entry.setting() == Some("audio.music_source"))
        .expect("the music source is on the audio page");
    for _ in 0..row {
        press(&mut menu, &[Button::Down]);
    }

    let entry = &menu.page().entries[row];
    assert_eq!(entry.value(), None, "an empty row shows no value");
    assert_eq!(entry.chosen(), None);

    for buttons in [&[Button::Left], &[Button::Right], &[Button::Cross]] {
        assert!(
            press(&mut menu, buttons).is_empty(),
            "an empty row must report no change"
        );
    }
    // And it still draws, label and all, rather than vanishing.
    let drawn = list(&menu, &|_| vec!["X"], None);
    assert!(
        drawn.iter().any(|draw| matches!(
            draw,
            Draw::Text { text, .. } if text == "MUSIC SOURCE"
        )),
        "the row is drawn"
    );
}

/// A machine that has both discs gets the three values, and the row keeps
/// whatever the settings file seeded it with - which is what stops the
/// first nudge of the row persisting `auto` over a player's `ps2`.
#[test]
fn a_supplied_music_source_row_keeps_the_value_it_was_seeded_with() {
    let mut menu = Menu::new(built_in());
    let offered: Vec<Choice> = crate::audio::MusicSource::ALL
        .iter()
        .map(|source| Choice::plain(source.name()))
        .collect();
    assert_eq!(menu.supply(ValueSource::MusicSources, &offered), 1);
    menu.seed("audio.music_source", &Value::Text("ps2".to_string()));

    assert!(menu.open("audio"));
    let entry = menu
        .page()
        .entries
        .iter()
        .find(|entry| entry.setting() == Some("audio.music_source"))
        .expect("the row");
    assert_eq!(entry.chosen(), Some(Value::Text("ps2".to_string())));
}

/// The visible half: a disabled row draws dim, so "this does nothing" is
/// something a player can see rather than something they discover.
#[test]
fn a_disabled_row_is_drawn_dimmed_even_when_it_is_selected() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("display"));
    let row = menu
        .page()
        .entries
        .iter()
        .position(|entry| entry.setting() == Some("display.frame_limit"))
        .expect("the frame limit is on the display page");
    for _ in 0..row {
        press(&mut menu, &[Button::Down]);
    }

    let label_colour = |menu: &Menu| {
        let list = list(menu, &|_| vec!["X"], None);
        list.iter()
            .find_map(|draw| match draw {
                Draw::Text { color, text, .. } if text == "FRAME LIMIT" => Some(*color),
                _ => None,
            })
            .expect("the row is drawn")
    };

    menu.seed("display.vsync", &Value::Text("on".to_string()));
    assert_eq!(label_colour(&menu), DIMMED);
    menu.seed("display.vsync", &Value::Text("off".to_string()));
    assert_eq!(label_colour(&menu), skin().selected());
    menu.seed("display.vsync", &Value::Text("smooth".to_string()));
    assert_eq!(label_colour(&menu), skin().selected());
}

/// `disabled_by` naming something nothing edits is a row that is never
/// greyed out, which looks exactly like a working one.
#[test]
fn a_condition_on_a_setting_nothing_edits_is_refused() {
    let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "choice"
label = "LIMIT"
setting = "a.limit"
disabled_by = { setting = "a.nothing", value = "on" }
values = ["1", "2"]
"#;
    let e = Definition::parse(text).expect_err("must not load");
    assert!(matches!(e, Error::BadCondition { .. }), "{e}");
    assert!(e.to_string().contains("a.nothing"), "{e}");
}

/// And a value that row can never hold is the same failure one level down:
/// the setting exists, the condition is simply unreachable, and the row
/// stays live forever.
#[test]
fn a_condition_on_a_value_no_row_can_hold_is_refused() {
    let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "choice"
label = "VSYNC"
setting = "a.vsync"
values = ["off", "on"]
[[page.entry]]
kind = "choice"
label = "LIMIT"
setting = "a.limit"
disabled_by = { setting = "a.vsync", value = "onn" }
values = ["1", "2"]
"#;
    let e = Definition::parse(text).expect_err("must not load");
    assert!(matches!(e, Error::BadCondition { .. }), "{e}");
    assert!(e.to_string().contains("onn"), "{e}");
    assert!(e.to_string().contains("off, on"), "{e}");
}

/// A number cannot be told apart from the text a choice row stores, so it
/// is refused rather than guessed at.
#[test]
fn a_condition_value_that_is_not_text_or_a_flag_is_refused() {
    let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "choice"
label = "LIMIT"
setting = "a.limit"
disabled_by = { setting = "a.limit", value = 60 }
values = ["1", "2"]
"#;
    let e = Definition::parse(text).expect_err("must not load");
    assert!(matches!(e, Error::BadCondition { .. }), "{e}");
}

/// And on a kind that cannot be adjusted it would parse and do nothing,
/// which is the whole class of mistake this loader exists to refuse.
#[test]
fn only_an_adjustable_row_may_be_disabled() {
    let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "toggle"
label = "VSYNC"
setting = "a.vsync"
[[page.entry]]
kind = "back"
label = "BACK"
disabled_by = { setting = "a.vsync", value = true }
"#;
    let e = Definition::parse(text).expect_err("must not load");
    assert!(matches!(e, Error::BadEntry { .. }), "{e}");
    assert!(e.to_string().contains("disabled_by"), "{e}");
}

/// The RENDERER row is the one setting on either page that a running game
/// cannot act on, so it is the one row that has to say so. Pinned to the
/// asset because losing the field is invisible: the row keeps working, keeps
/// storing, and simply stops explaining why nothing changed.
#[test]
fn the_renderer_row_says_a_restart_is_needed() {
    let definition = built_in();
    let entry = definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some("graphics.renderer"))
        .expect("nothing edits graphics.renderer");
    let restart = entry.restart().expect("the renderer defers to a restart");
    assert!(
        restart.message.to_uppercase().contains("RESTART"),
        "the message has to say the word: {:?}",
        restart.message
    );
    // Every other row on the two settings pages applies this run. A third
    // one appearing here is not necessarily wrong, but it is a claim about
    // what the game can do live and it should be made deliberately.
    //
    // `graphics.anti_aliasing` is the second, and deliberately: `off`,
    // `fxaa` and `smaa` are live, the same as every other row, but moving
    // to or between the two MSAA levels rebuilds every scene pipeline, so
    // that half of the row genuinely cannot apply this frame. See
    // ADR-0013 and `Session::open_menus`, which tells the two cases
    // apart by sample count rather than treating the whole row as
    // deferred.
    //
    // `graphics.boost_fov_kick` is the third, for the same shape of
    // reason as MSAA: `Race::set_boost_fov_kick` is only ever called once,
    // at `Race::start`, so a race already running keeps whatever it was
    // built with until the next one starts.
    let deferred: Vec<&str> = definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .filter(|entry| entry.restart().is_some())
        .filter_map(Entry::setting)
        .collect();
    assert_eq!(
        deferred,
        [
            "graphics.renderer",
            "graphics.anti_aliasing",
            "graphics.boost_fov_kick"
        ]
    );
}

/// The note is about what the *game* is doing, not about what the settings
/// file holds - which is why it stays quiet until it has been told, and why
/// it goes quiet again when the row comes back.
#[test]
fn the_restart_note_appears_only_once_the_row_leaves_what_is_running() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("graphics"), "the graphics page exists");
    let adapters = ["default", "vulkan: Card", "vulkan: Other (cpu)"]
        .map(|name| Choice::plain(name.to_string()));
    menu.supply(ValueSource::Renderers, &adapters);
    menu.seed(
        "graphics.renderer",
        &Value::Text("vulkan: Card".to_string()),
    );
    let row = |menu: &Menu| {
        menu.page()
            .entries
            .iter()
            .find(|entry| entry.setting() == Some("graphics.renderer"))
            .expect("the renderer is on the graphics page")
            .clone()
    };

    // Seeded, listed, and nothing has said what is running: silent. The
    // alternative is a note on a menu nobody has touched.
    assert!(menu.restart_note(&row(&menu)).is_none());

    assert!(menu.in_effect(
        "graphics.renderer",
        &[Value::Text("vulkan: Card".to_string())]
    ));
    assert!(
        menu.restart_note(&row(&menu)).is_none(),
        "the row still holds what the game is drawing with"
    );

    // Moving it is the whole point, and the row is *not* greyed: it is the
    // way back from an adapter that will not draw.
    let events = press(&mut menu, &[Button::Right]);
    assert_eq!(events.len(), 1, "{events:?}");
    assert!(menu.restart_note(&row(&menu)).is_some());

    // And back again, because a note that never clears teaches a player to
    // ignore it.
    press(&mut menu, &[Button::Left]);
    assert!(menu.restart_note(&row(&menu)).is_none());
}

/// A game that let wgpu pick is on `default` *and* on whatever wgpu picked.
/// Naming that adapter changes the settings file and nothing about the
/// picture, so a note telling the player to restart for it would be false.
#[test]
fn naming_the_adapter_the_default_already_resolved_to_is_not_a_change() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("graphics"));
    let adapters =
        ["default", "vulkan: Card", "vulkan: Other"].map(|name| Choice::plain(name.to_string()));
    menu.supply(ValueSource::Renderers, &adapters);
    menu.seed("graphics.renderer", &Value::Text("default".to_string()));
    menu.in_effect(
        "graphics.renderer",
        &[
            Value::Text("default".to_string()),
            Value::Text("vulkan: Card".to_string()),
        ],
    );
    let row = |menu: &Menu| {
        menu.page()
            .entries
            .iter()
            .find(|entry| entry.setting() == Some("graphics.renderer"))
            .expect("the renderer is on the graphics page")
            .clone()
    };

    // `default` -> `vulkan: Card`, the adapter it already resolved to.
    press(&mut menu, &[Button::Right]);
    assert_eq!(
        row(&menu).chosen(),
        Some(Value::Text("vulkan: Card".to_string()))
    );
    assert!(menu.restart_note(&row(&menu)).is_none());

    // One further along is a different card, and that does need a restart.
    press(&mut menu, &[Button::Right]);
    assert!(menu.restart_note(&row(&menu)).is_some());
}

/// The visible half: an amber marker in the margin and one line under the
/// rows, the same channel a warning uses, because to a player they are the
/// same sentence - this row is not doing what it says.
#[test]
fn a_restart_note_is_drawn_in_the_margin_and_under_the_rows() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("graphics"));
    let adapters = ["default", "vulkan: Card"].map(|name| Choice::plain(name.to_string()));
    menu.supply(ValueSource::Renderers, &adapters);
    menu.seed("graphics.renderer", &Value::Text("default".to_string()));
    menu.in_effect("graphics.renderer", &[Value::Text("default".to_string())]);

    let amber = |menu: &Menu| {
        list(menu, &|_| vec!["X"], None)
            .into_iter()
            .filter_map(|draw| match draw {
                Draw::Text { color, text, .. } if color == WARNING => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert!(amber(&menu).is_empty(), "nothing has been changed yet");

    press(&mut menu, &[Button::Right]);
    let drawn = amber(&menu);
    assert_eq!(drawn.len(), 2, "a marker and a message: {drawn:?}");
    assert_eq!(drawn[0], "!");
    assert!(drawn[1].to_uppercase().contains("RESTART"), "{drawn:?}");
}

/// `Menu::warning` picks between the anti-aliasing row's two declared
/// warnings rather than assuming there is only one - the runtime half of
/// `Entry::warnings` being a list. Their conditions are disjoint by
/// construction (one fires below render scale 100, the other only at 200),
/// so exactly one, or neither, ever applies at a time.
#[test]
fn the_anti_aliasing_row_shows_whichever_of_its_two_warnings_currently_applies() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("graphics"));
    let row = |menu: &Menu| {
        menu.page()
            .entries
            .iter()
            .find(|entry| entry.setting() == Some("graphics.anti_aliasing"))
            .expect("anti-aliasing is on the graphics page")
            .clone()
    };
    menu.seed("graphics.anti_aliasing", &Value::Text("fxaa".to_string()));

    menu.seed("graphics.upscaler", &Value::Text("off".to_string()));
    menu.seed("graphics.render_scale", &Value::Text("100".to_string()));
    assert!(
        menu.warning(&row(&menu)).is_none(),
        "neither warning applies at 100% with the upscaler off"
    );

    menu.seed("graphics.upscaler", &Value::Text("fsr1".to_string()));
    menu.seed("graphics.render_scale", &Value::Text("50".to_string()));
    let entry = row(&menu);
    let warning = menu
        .warning(&entry)
        .expect("fxaa below render scale 100 with fsr1 selected fights the upscaler");
    assert!(
        warning.message.to_uppercase().contains("UPSCALER"),
        "{warning:?}"
    );

    menu.seed("graphics.upscaler", &Value::Text("off".to_string()));
    menu.seed("graphics.render_scale", &Value::Text("200".to_string()));
    let entry = row(&menu);
    let warning = menu
        .warning(&entry)
        .expect("fxaa at 200% render scale is redundant on its own");
    assert!(
        warning.message.to_uppercase().contains("REDUNDANT"),
        "{warning:?}"
    );
}

/// A row nothing supplies is silent, which is the one way this mechanism
/// can fail invisibly - so `in_effect` reports whether anybody took it, and
/// the composition root says so on stderr.
#[test]
fn telling_a_setting_no_row_defers_reports_it() {
    let mut menu = Menu::new(built_in());
    assert!(menu.in_effect("graphics.renderer", &[Value::Text("default".to_string())]));
    assert!(
        !menu.in_effect("display.vsync", &[Value::Text("off".to_string())]),
        "the vsync row applies live and declares no restart"
    );
    assert!(!menu.in_effect("nothing.at.all", &[Value::Flag(true)]));
}

/// A marker with nothing to read is a puzzle: it says something is wrong
/// and not what to do about it.
#[test]
fn a_restart_required_with_no_message_is_refused() {
    let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "choice"
label = "RENDERER"
setting = "a.renderer"
values = ["one", "two"]
restart_required = ""
"#;
    let e = Definition::parse(text).expect_err("must not load");
    assert!(matches!(e, Error::BadEntry { .. }), "{e}");
    assert!(e.to_string().contains("restart_required"), "{e}");
}

/// And on a kind that cannot be adjusted it would parse and do nothing -
/// the same refusal `disabled_by` gets, for the same reason.
#[test]
fn only_an_adjustable_row_may_need_a_restart() {
    let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "back"
label = "BACK"
restart_required = "RESTART THE GAME"
"#;
    let e = Definition::parse(text).expect_err("must not load");
    assert!(matches!(e, Error::BadEntry { .. }), "{e}");
}

#[test]
fn a_version_this_build_does_not_know_is_refused() {
    let error = Definition::parse("version = 99\nroot = \"main\"").expect_err("refused");
    assert!(matches!(error, Error::Version { found: 99 }), "{error}");
}

#[test]
fn a_dangling_target_is_refused() {
    let error = Definition::parse(
        r#"
        version = 1
        root = "main"
        [[page]]
        id = "main"
        [[page.entry]]
        kind = "submenu"
        label = "NOWHERE"
        target = "does_not_exist"
        "#,
    )
    .expect_err("refused");
    assert!(matches!(error, Error::NoSuchPage { .. }), "{error}");
}

#[test]
fn an_unknown_action_is_refused_and_says_what_is_known() {
    let error = Definition::parse(
        r#"
        version = 1
        root = "main"
        [[page]]
        id = "main"
        [[page.entry]]
        kind = "action"
        label = "DO IT"
        action = "make_tea"
        "#,
    )
    .expect_err("refused");
    let message = error.to_string();
    assert!(message.contains("make_tea"), "{message}");
    assert!(message.contains("launch_race"), "{message}");
}

/// The orphan case: a page that parses and resolves, and that nothing links
/// to. This is the only one `resolve` cannot catch a row at a time.
#[test]
fn a_page_nothing_links_to_is_refused() {
    let error = Definition::parse(
        r#"
        version = 1
        root = "main"
        [[page]]
        id = "main"
        [[page.entry]]
        kind = "action"
        label = "QUIT"
        action = "quit"
        [[page]]
        id = "orphan"
        [[page.entry]]
        kind = "back"
        label = "BACK"
        "#,
    )
    .expect_err("refused");
    assert!(
        matches!(&error, Error::Unreachable(id) if id == "orphan"),
        "{error}"
    );
}
