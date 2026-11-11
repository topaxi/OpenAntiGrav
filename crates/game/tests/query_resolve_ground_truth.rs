//! A table-form `values_from` query is answered from the title its `of` setting names, and
//! re-listed when that setting or its companion changes; a mod's saved picks are settled
//! against the lists (ADR-0082). The disc-backed tests run both Pulse and Pure; the fixture
//! ones need no images.

use oag_game::{boot, remix};
use oag_ui::language::StringTable;
use oag_ui::menu::{Choice, Definition, Menu, Query, Value};

const HEADER: &str = r#"toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "choice"
label = "TITLE"
setting = "mod.t.title"
values = ["Alpha", "Beta"]
"#;

fn menu_with(rows: &str) -> Menu {
    let text = format!("{HEADER}{rows}");
    Menu::new(Definition::parse(&text, &StringTable::default()).expect("parses"))
}

fn tracks_row() -> String {
    "[[page.entry]]\nkind = \"choice\"\nlabel = \"TRACK\"\nsetting = \"mod.t.track\"\n\
     values_from = { query = \"tracks_of\", of = \"mod.t.title\" }\n"
        .to_string()
}

fn listed(menu: &Menu) -> Vec<String> {
    let page = &menu.definition().pages[0];
    let oag_ui::menu::Entry::Choice { values, .. } = &page.entries[1] else {
        panic!("a choice row");
    };
    values.iter().map(|v| v.value.clone()).collect()
}

fn lists(_: Query, args: remix::QueryArgs<'_>) -> Option<Vec<Choice>> {
    match args.title {
        "Alpha" => Some(vec![Choice::plain("a1"), Choice::plain("a2")]),
        "Beta" => Some(vec![Choice::plain("b1")]),
        _ => None,
    }
}

#[test]
fn a_row_is_relisted_when_the_setting_it_names_changes() {
    let mut menu = menu_with(&tracks_row());
    assert!(listed(&menu).is_empty(), "no list until the engine answers");
    menu.seed("mod.t.title", &Value::Text("Alpha".into()));
    remix::resolve_queries(&mut menu, None, lists);
    assert_eq!(listed(&menu), ["a1", "a2"]);
    menu.seed("mod.t.title", &Value::Text("Beta".into()));
    remix::resolve_queries(&mut menu, Some("mod.t.title"), lists);
    assert_eq!(listed(&menu), ["b1"]);
}

#[test]
fn another_setting_changing_leaves_the_list_alone() {
    let mut menu = menu_with(&tracks_row());
    menu.seed("mod.t.title", &Value::Text("Alpha".into()));
    remix::resolve_queries(&mut menu, None, lists);
    menu.seed("mod.t.title", &Value::Text("Beta".into()));
    remix::resolve_queries(&mut menu, Some("race.mode"), lists);
    assert_eq!(listed(&menu), ["a1", "a2"]);
}

#[test]
fn a_title_that_cannot_be_opened_keeps_the_row_on_its_list() {
    let mut menu = menu_with(&tracks_row());
    menu.seed("mod.t.title", &Value::Text("Alpha".into()));
    remix::resolve_queries(&mut menu, None, lists);
    remix::resolve_queries(&mut menu, None, |_, _| None);
    assert_eq!(listed(&menu), ["a1", "a2"]);
}

/// TITLE, then TEAM listing that title's teams for CLASS, then VARIANT listing TEAM's
/// variants: the chain a two-title page has.
const CHAIN: &str = r#"toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "choice"
label = "TITLE"
setting = "mod.t.title"
values = ["Alpha", "Beta"]
[[page.entry]]
kind = "choice"
label = "CLASS"
setting = "mod.t.class"
values = ["slow", "fast"]
default = "fast"
[[page.entry]]
kind = "choice"
label = "TEAM"
setting = "mod.t.team"
values_from = { query = "teams_of", of = "mod.t.title", class = "mod.t.class" }
[[page.entry]]
kind = "choice"
label = "VARIANT"
setting = "mod.t.variant"
values_from = { query = "variants_of", of = "mod.t.title", team = "mod.t.team" }
"#;

/// Alpha's teams are `a-slow`/`a-fast` per class, Beta's `b1`/`b2`; a team's variants are
/// its name with `-1` and `-2`.
fn chain_lists(query: Query, args: remix::QueryArgs<'_>) -> Option<Vec<Choice>> {
    match query {
        Query::TeamsOf => Some(match (args.title, args.companion) {
            ("Alpha", Some(class)) => vec![Choice::plain(format!("a-{class}"))],
            ("Beta", _) => vec![Choice::plain("b1"), Choice::plain("b2")],
            _ => return None,
        }),
        Query::VariantsOf => {
            let team = args.companion?;
            Some(vec![
                Choice::plain(format!("{team}-1")),
                Choice::plain(format!("{team}-2")),
            ])
        }
        _ => None,
    }
}

fn chain() -> Menu {
    Menu::new(Definition::parse(CHAIN, &StringTable::default()).expect("parses"))
}

fn saved<'a>(
    table: &'a oag_game::settings::mod_settings::ModSettings,
    key: &str,
) -> Option<&'a str> {
    oag_game::settings::mod_settings::get(table, &format!("mod.t.{key}"))
}

#[test]
fn a_fresh_profile_saves_what_every_row_shows_with_a_default_standing_for_an_unsaved_pick() {
    let mut menu = chain();
    let mut table = oag_game::settings::mod_settings::ModSettings::default();
    assert!(remix::settle_queries(
        &mut menu,
        &mut table,
        None,
        chain_lists
    ));
    assert_eq!(saved(&table, "title"), Some("Alpha"));
    assert_eq!(
        saved(&table, "class"),
        Some("fast"),
        "the row's default, not its first value"
    );
    assert_eq!(
        saved(&table, "team"),
        Some("a-fast"),
        "listed for the default class"
    );
    assert_eq!(saved(&table, "variant"), Some("a-fast-1"));
}

#[test]
fn a_saved_pick_survives_a_reopen_and_a_change_relists_down_the_chain() {
    let mut table = oag_game::settings::mod_settings::ModSettings::default();
    for (key, value) in [
        ("title", "Beta"),
        ("class", "slow"),
        ("team", "b2"),
        ("variant", "b2-2"),
    ] {
        oag_game::settings::mod_settings::set(&mut table, &format!("mod.t.{key}"), value);
    }
    let mut menu = chain();
    for (key, value) in oag_game::settings::mod_settings::seeds(&table) {
        menu.seed(&key, &value);
    }
    assert!(
        !remix::settle_queries(&mut menu, &mut table, None, chain_lists),
        "nothing moves"
    );
    assert_eq!(menu.value_of("mod.t.team"), Some("b2"));
    assert_eq!(menu.value_of("mod.t.variant"), Some("b2-2"));
    // TITLE to Alpha: TEAM is re-listed for the saved class, VARIANT for the team it falls to.
    oag_game::settings::mod_settings::set(&mut table, "mod.t.title", "Alpha");
    menu.seed("mod.t.title", &Value::Text("Alpha".into()));
    assert!(remix::settle_queries(
        &mut menu,
        &mut table,
        Some("mod.t.title"),
        chain_lists
    ));
    assert_eq!(saved(&table, "team"), Some("a-slow"));
    assert_eq!(saved(&table, "variant"), Some("a-slow-1"));
    // CLASS, TEAM's companion, re-lists TEAM too.
    oag_game::settings::mod_settings::set(&mut table, "mod.t.class", "fast");
    menu.seed("mod.t.class", &Value::Text("fast".into()));
    remix::settle_queries(&mut menu, &mut table, Some("mod.t.class"), chain_lists);
    assert_eq!(saved(&table, "team"), Some("a-fast"));
    assert_eq!(saved(&table, "variant"), Some("a-fast-1"));
}

#[test]
fn a_row_left_with_no_list_keeps_its_saved_pick() {
    let mut table = oag_game::settings::mod_settings::ModSettings::default();
    oag_game::settings::mod_settings::set(&mut table, "mod.t.team", "b2");
    let mut menu = chain();
    remix::settle_queries(&mut menu, &mut table, None, |_, _| None);
    assert_eq!(saved(&table, "team"), Some("b2"));
}

fn source(name: &str) -> Option<String> {
    oag_testdata::image(name).map(|path| path.display().to_string())
}

fn options(source: &str) -> boot::Options {
    boot::Options {
        language: Some("English".to_string()),
        source: source.to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-query-resolve-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    }
}

/// What the RACE page lists for the source: the boot shell's circuits, labelled.
fn race_list(source: &str) -> Vec<(String, String)> {
    let (shell, _archives, _title) = boot::load_shell(&options(source)).expect("the shell");
    shell
        .tracks
        .iter()
        .map(|track| {
            let label = oag_raceplay::catalogue::label(
                track,
                &shell.circuit_names,
                &shell.strings,
                &shell.tracks,
            );
            (track.id.clone(), label)
        })
        .collect()
}

fn query_list(source: &str, title: &'static oag_title::Title) -> Vec<(String, String)> {
    let catalogue = remix::catalogue(source).expect("catalogue");
    remix::query_choices(Query::TracksOf, &catalogue, title, None)
        .expect("tracks_of answers")
        .into_iter()
        .map(|c| (c.value, c.label))
        .collect()
}

const PULSE: &str = "pulse-psp-eu.chd";
const PURE: &str = "pure-psp-eu.chd";

#[test]
#[ignore = "needs disc images in data/images/"]
fn tracks_of_the_booted_title_is_the_race_list_on_pulse_and_pure() {
    for (image, title) in [(PULSE, oag_pulse::TITLE), (PURE, oag_pure::TITLE)] {
        let Some(source) = source(image) else {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "{image} missing"
            );
            continue;
        };
        let race = race_list(&source);
        assert!(!race.is_empty(), "{image} offers circuits");
        assert_eq!(query_list(&source, title), race, "{image}");
    }
}

#[test]
#[ignore = "needs disc images in data/images/"]
fn a_mod_row_lists_the_circuits_of_the_title_its_setting_names() {
    let (Some(pulse), Some(pure)) = (source(PULSE), source(PURE)) else {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "images missing"
        );
        return;
    };
    let pulse_name = oag_pulse::TITLE.name;
    let pure_name = oag_pure::TITLE.name;
    let text = format!(
        "{}[[page.entry]]\nkind = \"choice\"\nlabel = \"TITLE\"\nsetting = \"mod.t.which\"\nvalues = [\"{pulse_name}\", \"{pure_name}\"]\n\
         [[page.entry]]\nkind = \"choice\"\nlabel = \"TRACK\"\nsetting = \"mod.t.track\"\n\
         values_from = {{ query = \"tracks_of\", of = \"mod.t.which\" }}\n",
        HEADER
    );
    let mut menu = Menu::new(Definition::parse(&text, &StringTable::default()).expect("parses"));
    let answer = |query: Query, args: remix::QueryArgs<'_>| {
        let (source, t) = if args.title == pulse_name {
            (&pulse, oag_pulse::TITLE)
        } else {
            (&pure, oag_pure::TITLE)
        };
        remix::query_choices(query, &remix::catalogue(source).ok()?, t, None)
    };
    let ids = |menu: &Menu| -> Vec<String> {
        let oag_ui::menu::Entry::Choice { values, .. } = &menu.definition().pages[0].entries[2]
        else {
            panic!("a choice row");
        };
        values.iter().map(|v| v.value.clone()).collect()
    };
    menu.seed("mod.t.which", &Value::Text(pulse_name.into()));
    remix::resolve_queries(&mut menu, None, answer);
    let on_pulse = ids(&menu);
    assert_eq!(
        on_pulse,
        query_list(&pulse, oag_pulse::TITLE)
            .into_iter()
            .map(|t| t.0)
            .collect::<Vec<_>>()
    );
    menu.seed("mod.t.which", &Value::Text(pure_name.into()));
    remix::resolve_queries(&mut menu, Some("mod.t.which"), answer);
    let on_pure = ids(&menu);
    assert_eq!(
        on_pure,
        query_list(&pure, oag_pure::TITLE)
            .into_iter()
            .map(|t| t.0)
            .collect::<Vec<_>>()
    );
    assert_ne!(on_pulse, on_pure, "the two titles race different circuits");
}
