//! RACE REMIX as the official mod `oag.remix` (ADR-0083): with it on, RACEBOX carries its
//! REMIX row and its page launches through `launch_race` by its own keys; with it off, the
//! tree has neither and every title's menu still builds. Skips when the official-mods
//! submodule is not checked out.

use std::path::{Path, PathBuf};

use oag_game::mods::Mods;
use oag_ui::menu::{Action, Definition, Entry, For};

const PAGE: &str = "oag.remix/remix";

fn official() -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/mods/official");
    if dir.join("remix/oag-mod.toml").is_file() {
        Some(dir)
    } else {
        eprintln!("skipped: the official-mods submodule is not checked out");
        None
    }
}

fn tree(
    base: &Path,
    off: &[&str],
    title: &'static oag_title::Title,
    language: Option<&str>,
) -> Definition {
    let disabled = oag_game::settings::Mods {
        disabled: off.iter().map(ToString::to_string).collect(),
        ..Default::default()
    };
    let mods = Mods::load_from(&[base.to_path_buf()], &disabled).unwrap();
    mods.build(
        oag_ui::menu::BUILT_IN,
        language,
        Some(For {
            title,
            updated: false,
        }),
    )
    .unwrap()
    .definition
}

/// The labels of the rows attached under RACEBOX on the main page.
fn under_racebox(definition: &Definition) -> Vec<String> {
    let page = &definition.pages[definition.root];
    let racebox = page.row_with_id("racebox").expect("a RACEBOX row");
    (0..page.entries.len())
        .filter(|&row| page.is_attached(row) && page.family_of(row) == racebox)
        .map(|row| page.entries[row].label().to_string())
        .collect()
}

const TITLES: [&oag_title::Title; 5] = [
    oag_pulse::TITLE,
    oag_pure::TITLE,
    oag_hd::TITLE,
    oag_omega::TITLE,
    oag_2048::TITLE,
];

#[test]
fn with_the_mod_on_racebox_has_remix_and_with_it_off_nothing_and_every_title_builds() {
    let Some(base) = official() else { return };
    for title in TITLES {
        let on = tree(&base, &[], title, None);
        assert_eq!(under_racebox(&on), ["REPLAYS", "REMIX"], "{}", title.name);
        assert!(on.page(PAGE).is_some(), "{}", title.name);
        let off = tree(&base, &["oag.remix"], title, None);
        assert_eq!(under_racebox(&off), ["REPLAYS"], "{}", title.name);
        assert!(off.page(PAGE).is_none(), "{}", title.name);
    }
}

#[test]
fn the_page_starts_through_launch_race_by_its_own_mod_keys() {
    let Some(base) = official() else { return };
    let definition = tree(&base, &[], oag_pulse::TITLE, None);
    let page = definition.page(PAGE).unwrap();
    let inputs: Vec<_> = page
        .entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Run {
                action: Action::LaunchRace,
                inputs: Some(inputs),
                ..
            } => Some(inputs.clone()),
            _ => None,
        })
        .collect();
    let [inputs] = &inputs[..] else {
        panic!("one START row, not {inputs:?}");
    };
    for key in [
        &inputs.track_title,
        &inputs.track,
        &inputs.craft_title,
        &inputs.team,
        &inputs.variant,
        &inputs.mode,
        &inputs.class,
        &inputs.difficulty,
    ] {
        assert!(
            key.as_deref()
                .is_some_and(|key| key.starts_with("mod.oag.remix.")),
            "{key:?}"
        );
    }
    let start = page
        .meta
        .iter()
        .position(|meta| meta.id.as_deref() == Some("launch_remix"))
        .expect("the START row keeps the id launch_remix, which a drive script selects by");
    assert!(matches!(page.entries[start], Entry::Run { .. }));
}

/// The AI DIFFICULTY row is pinned to "no opponents" for exactly the known classes this
/// project's driver does not cover (`oag_ai::coverage`), so the mod's list cannot drift from
/// the code's set.
#[test]
fn the_ai_row_is_pinned_for_exactly_the_uncovered_classes() {
    let Some(base) = official() else { return };
    let definition = tree(&base, &[], oag_pulse::TITLE, None);
    let mut uncovered: Vec<String> = oag_tables::handling::SpeedClass::KNOWN
        .iter()
        .map(|class| class.as_str().to_lowercase())
        .filter(|class| !oag_ai::coverage::project_driver_covers(class))
        .collect();
    uncovered.sort();
    let pinned: Vec<_> = definition
        .page(PAGE)
        .unwrap()
        .entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Choice { setting, pins, .. }
                if setting == "mod.oag.remix.difficulty" && !pins.is_empty() =>
            {
                Some(pins)
            }
            _ => None,
        })
        .collect();
    let [pins] = &pinned[..] else {
        panic!("one pinned AI row, not {}", pinned.len());
    };
    let mut classes: Vec<String> = pins
        .iter()
        .filter(|pin| pin.when.setting == "mod.oag.remix.class")
        .flat_map(|pin| pin.when.values.iter().map(ToString::to_string))
        .collect();
    classes.sort();
    assert_eq!(classes, uncovered);
    assert!(pins.iter().all(|pin| pin.shows == "none"));
}

#[test]
fn the_page_reads_in_each_language_the_mod_ships() {
    let Some(base) = official() else { return };
    for (language, title) in [
        (None, "RACE REMIX"),
        (Some("German"), "RENN-REMIX"),
        (Some("French"), "COURSE REMIX"),
        (Some("PortugueseBR"), "REMIX DE CORRIDA"),
    ] {
        let definition = tree(&base, &[], oag_pulse::TITLE, language);
        assert_eq!(definition.page(PAGE).unwrap().title, title, "{language:?}");
    }
}

/// TEAM stays live in Zone and VARIANT greys there, as on the RACE page
/// (`menu_definition_settings::zone_leaves_the_team_rows_live_and_greys_the_variant_rows`).
#[test]
fn zone_leaves_team_live_and_greys_variant() {
    let Some(base) = official() else { return };
    let definition = tree(&base, &[], oag_pulse::TITLE, None);
    let page = definition.page(PAGE).unwrap();
    let row = |setting: &str| {
        page.entries
            .iter()
            .find(|entry| entry.setting() == Some(setting))
            .unwrap_or_else(|| panic!("no {setting} row"))
    };
    assert!(row("mod.oag.remix.team").disabled_by().is_none());
    let condition = row("mod.oag.remix.variant")
        .disabled_by()
        .expect("VARIANT has a condition");
    assert_eq!(condition.setting, "mod.oag.remix.mode");
    assert!(
        condition
            .values
            .contains(&oag_ui::menu::Value::Text("zone".to_string()))
    );
}
