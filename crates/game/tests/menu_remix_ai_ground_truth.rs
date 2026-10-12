//! The shipped menu definition's remix AI row against the controller coverage.

use oag_ui::menu::*;

fn built_in() -> Definition {
    Definition::parse(BUILT_IN, &oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse")
}

/// RACE REMIX's AI DIFFICULTY row is pinned to "no opponents" for exactly the
/// known classes this project's driver does not cover (`oag_ai::coverage`), so
/// the TOML list cannot drift from the code's set.
#[test]
fn the_remix_ai_row_is_pinned_for_exactly_the_uncovered_classes() {
    let definition = built_in();
    let mut uncovered: Vec<String> = oag_tables::handling::SpeedClass::KNOWN
        .iter()
        .map(|class| class.as_str().to_lowercase())
        .filter(|class| !oag_ai::coverage::project_driver_covers(class))
        .collect();
    uncovered.sort();
    let mut found = 0;
    for entry in definition.pages.iter().flat_map(|page| page.entries.iter()) {
        let Entry::Choice { setting, pins, .. } = entry else {
            continue;
        };
        if setting != "ai.difficulty" || pins.is_empty() {
            continue;
        }
        found += 1;
        let mut pinned: Vec<String> = pins
            .iter()
            .filter(|pin| pin.when.setting == "race.class")
            .flat_map(|pin| pin.when.values.iter().map(ToString::to_string))
            .collect();
        pinned.sort();
        assert_eq!(pinned, uncovered);
        assert!(pins.iter().all(|pin| pin.shows == "none"));
    }
    assert_eq!(found, 1, "exactly the RACE REMIX page pins the AI row");
}
