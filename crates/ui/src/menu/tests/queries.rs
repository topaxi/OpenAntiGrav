//! `values_from` as a fixed name or as a query table (ADR-0082).

use super::*;
use crate::menu::{Query, QueryRef};

const HEADER: &str = r#"toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/menu.tosd", version = "1.0.0" }
root = "main"
[[page]]
id = "main"
"#;

fn choice(values_from: &str) -> String {
    format!(
        "{HEADER}[[page.entry]]\nkind = \"choice\"\nlabel = \"X\"\nsetting = \"x\"\nvalues_from = {values_from}\n"
    )
}

fn parse(text: &str) -> Result<Definition, crate::menu::definition::Error> {
    Definition::parse(text, &crate::language::StringTable::default())
}

fn row(definition: &Definition) -> (Option<ValueSource>, Option<QueryRef>) {
    let Entry::Choice { source, query, .. } = &definition.pages[0].entries[0] else {
        panic!("a choice row");
    };
    (*source, query.clone())
}

#[test]
fn every_fixed_name_still_parses_to_its_own_source() {
    for name in [
        "languages",
        "tracks",
        "monitors",
        "window_sizes",
        "fullscreen_resolutions",
        "renderers",
        "race_modes",
        "music_sources",
        "front_end_styles",
        "hud_styles",
        "teams",
        "titles",
        "craft_titles",
        "remix_tracks",
        "remix_teams",
        "speed_classes",
        "remix_speed_classes",
        "remix_ai_difficulty",
        "race_variant",
        "remix_variant",
        "pilots",
        "pilot_axes",
        "pilot_axis_low",
        "pilot_axis_high",
        "screen_filters",
        "kill_targets",
        "weapons",
        "mods",
        "mod_switch",
        "music_pool",
    ] {
        let source = ValueSource::parse(name).unwrap_or_else(|| panic!("{name} parses"));
        assert_eq!(source.name(), name);
        let definition = parse(&choice(&format!("{name:?}"))).unwrap();
        assert_eq!(row(&definition), (Some(source), None), "{name}");
    }
}

#[test]
fn a_query_with_an_empty_of_is_the_fixed_name_the_booted_title_gets() {
    for (query, source) in [
        (Query::TitlesAvailable, ValueSource::Titles),
        (Query::TracksOf, ValueSource::Tracks),
        (Query::TeamsOf, ValueSource::Teams),
        (Query::VariantsOf, ValueSource::RaceVariant),
        (Query::SpeedClassesUnion, ValueSource::RemixSpeedClasses),
        (Query::AiDifficulties, ValueSource::RemixAiDifficulty),
    ] {
        assert_eq!(Query::parse(query.name()), Some(query));
        for table in [
            format!("{{ query = {:?} }}", query.name()),
            format!("{{ query = {:?}, of = \"\" }}", query.name()),
        ] {
            let definition = parse(&choice(&table)).unwrap();
            assert_eq!(row(&definition), (Some(source), None), "{table}");
        }
    }
}

#[test]
fn custom_tracks_has_no_supplier_yet_and_is_still_a_row_with_a_list_to_come() {
    let definition = parse(&choice(r#"{ query = "custom_tracks" }"#)).unwrap();
    assert_eq!(row(&definition), (None, None));
}

#[test]
fn a_query_naming_a_setting_keeps_it_for_the_caller() {
    let text = format!(
        "{}[[page.entry]]\nkind = \"choice\"\nlabel = \"T\"\nsetting = \"pick.title\"\nvalues = [\"a\"]\n",
        choice(r#"{ query = "tracks_of", of = "pick.title" }"#)
    );
    let definition = parse(&text).unwrap();
    assert_eq!(
        row(&definition),
        (
            None,
            Some(QueryRef {
                query: Query::TracksOf,
                of: "pick.title".to_string()
            })
        )
    );
}

#[test]
fn an_unknown_query_names_the_row_and_the_known_queries() {
    let error = parse(&choice(r#"{ query = "tracks_off" }"#))
        .unwrap_err()
        .to_string();
    assert!(error.contains("page \"main\" entry 0"), "{error}");
    assert!(error.contains("\"tracks_off\""), "{error}");
    assert!(error.contains("tracks_of"), "{error}");
    let error = parse(&choice(r#""nonsense""#)).unwrap_err().to_string();
    assert!(error.contains("page \"main\" entry 0"), "{error}");
    assert!(error.contains("\"nonsense\""), "{error}");
}

#[test]
fn a_bad_of_is_a_load_error_naming_the_row() {
    let unwritten = parse(&choice(
        r#"{ query = "tracks_of", of = "nobody.writes.this" }"#,
    ))
    .unwrap_err()
    .to_string();
    assert!(unwritten.contains("page \"main\" entry 0"), "{unwritten}");
    assert!(unwritten.contains("nobody.writes.this"), "{unwritten}");
    let argument = parse(&choice(r#"{ query = "ai_difficulties", of = "x" }"#))
        .unwrap_err()
        .to_string();
    assert!(argument.contains("takes no `of`"), "{argument}");
    let stray = parse(&choice(r#"{ query = "tracks_of", with = "x" }"#));
    assert!(stray.is_err(), "an unknown key in the table is refused");
}

#[test]
fn a_table_and_literal_values_are_alternatives() {
    let text = format!("{}values = [\"a\"]\n", choice(r#"{ query = "tracks_of" }"#));
    let error = parse(&text).unwrap_err().to_string();
    assert!(error.contains("alternatives"), "{error}");
}
