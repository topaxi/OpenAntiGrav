//! The menu definition this build ships: that it parses, that every page can
//! be left, and that every row on it offers only values the game will accept.
//!
//! Split out of `oag-ui`'s own `menu/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py` - and, unlike its siblings that stayed there,
//! moved to the composition root's own integration tests rather than with it:
//! checking the shipped definition against `oag_game::settings`/`perf`/`audio`
//! is an assertion that both crates state the same values, which only a
//! crate that can see both can make.

use oag_ui::menu::*;

/// The definition this build actually ships. Every test that can use it
/// does, so the file is exercised rather than a fixture standing in for it.
fn built_in() -> Definition {
    Definition::parse(BUILT_IN, &oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse")
}

/// The check that runs in CI. Unlike most of this repository's interesting
/// tests it needs no disc image, because the thing under test is ours.
#[test]
fn the_shipped_definition_is_internally_consistent() {
    let definition = built_in();
    assert!(!definition.pages.is_empty(), "a menu with no pages");
    definition.check().expect("reachability");

    for page in &definition.pages {
        assert!(!page.entries.is_empty(), "page {:?} has no rows", page.id);
        for entry in &page.entries {
            assert!(
                !entry.label().is_empty(),
                "page {:?} has an unlabelled row",
                page.id
            );
        }
    }
}

/// A row that names a `string_id` gets the table's answer, and its own
/// `label` when the table has nothing.
///
/// **The second half of this used to assert the opposite** - that a missing
/// id showed the *id* - on the argument that the id space is the disc's own.
/// The pilot editor's rows are the first in this build to name an id, those
/// ids are ours, and only `english.toml` carries them: under that rule a
/// French player read `OAG_PILOT_RENAME` off a row whose `label` says
/// `RENAME`. See `resolve`'s own comment for the whole reversal.
#[test]
fn a_string_id_is_resolved_against_the_table_parse_is_given() {
    let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "back"
label = "IGNORED"
string_id = "MENU_BACK"
"#;
    let found = oag_ui::language::StringTable::from_xml(
        r#"<StringTable><Entry ID="MENU_BACK" String="RETOUR"></Entry></StringTable>"#,
    );
    let definition = Definition::parse(text, &found).expect("parse");
    assert_eq!(definition.pages[0].entries[0].label(), "RETOUR");

    let absent = oag_ui::language::StringTable::default();
    let definition = Definition::parse(text, &absent).expect("parse");
    assert_eq!(
        definition.pages[0].entries[0].label(),
        "IGNORED",
        "no table has this id, so the row's own label is what a player sees - \
         showing the id would put OAG_PILOT_RENAME on screen in every language \
         but English"
    );
}

/// A page's own `title_string_id` is resolved the same way a row's
/// `string_id` is - one shared fallback rule, `resolved`, for both.
#[test]
fn a_title_string_id_is_resolved_against_the_table_parse_is_given() {
    let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
title = "IGNORED"
title_string_id = "PAGE_TITLE"
[[page.entry]]
kind = "back"
label = "BACK"
"#;
    let found = oag_ui::language::StringTable::from_xml(
        r#"<StringTable><Entry ID="PAGE_TITLE" String="TITRE"></Entry></StringTable>"#,
    );
    let definition = Definition::parse(text, &found).expect("parse");
    assert_eq!(definition.pages[0].title, "TITRE");

    let absent = oag_ui::language::StringTable::default();
    let definition = Definition::parse(text, &absent).expect("parse");
    assert_eq!(
        definition.pages[0].title, "IGNORED",
        "no table has this id, so the page's own title is what a player sees"
    );
}

/// Every page except the root must offer a way out, or a player who opens
/// it with a keyboard that has no cancel key is stuck in it. The root's way
/// out is `quit`, which is an action rather than a `back`.
#[test]
fn every_page_can_be_left_from_its_own_rows() {
    let definition = built_in();
    for (index, page) in definition.pages.iter().enumerate() {
        let leaves = page.entries.iter().any(|entry| {
            matches!(entry, Entry::Back { .. })
                || matches!(entry, Entry::Run { action, .. } if *action == Action::Quit)
        });
        assert!(
            leaves || index == definition.root,
            "page {:?} has no way back",
            page.id
        );
    }
}

/// The one class of mistake the format check cannot see: a `choice` whose
/// values are spelled by hand and drift from the list the game will accept.
/// Both of these fail at *race load*, deep inside an archive lookup, with a
/// message about a missing WAD entry - so they are pinned here, where the
/// message names the menu.
///
/// **Every matching row, not the first.** This used to `find_map`, which only
/// ever reached the RACE page's `race.class` row and left RACE REMIX's own
/// copy of the same setting unchecked - so a change to one and not the other
/// passed silently. `race.class` is authored twice on purpose, with two
/// different value sources, which is exactly the shape that needs all of them
/// looked at.
#[test]
fn the_race_page_offers_only_teams_and_classes_the_game_accepts() {
    let definition = built_in();
    let rows = |setting: &str| -> Vec<Vec<String>> {
        let found: Vec<Vec<String>> = definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .filter_map(|entry| match entry {
                Entry::Choice {
                    setting: key,
                    values,
                    ..
                } if key == setting => {
                    Some(values.iter().map(|value| value.value.clone()).collect())
                }
                _ => None,
            })
            .collect();
        assert!(!found.is_empty(), "no choice edits {setting:?}");
        found
    };

    for values in rows("race.team") {
        assert!(
            values.is_empty(),
            "the team row must be supplied, not spelled: the roster is what \
             the source offers plus what a mounted pack adds, and no list in \
             this repository can know either"
        );
    }

    // The same argument, reached later and for a different reason: the ladder
    // is a property of the *release*. Wipeout Pure's per-team
    // `handlingstats.xml` authors five `<Class>` rungs where Pulse's authors
    // four, so a hand-spelled list would be either wrong for Pure or silently
    // reused as if someone had measured it.
    //
    // Both rows are now supplied rather than spelled, from two *different*
    // sources: the RACE page offers the booted title's own ladder, RACE REMIX
    // offers the union across every title this machine can open. Neither may
    // carry a hand-written list again.
    let class_rows = rows("race.class");
    assert_eq!(
        class_rows.len(),
        2,
        "RACE and RACE REMIX should both author a race.class row"
    );
    for values in class_rows {
        assert!(
            values.is_empty(),
            "the speed-class row must be supplied, not spelled: the ladder is \
             the title's own, and Pure authors five rungs where Pulse authors \
             four"
        );
    }
}

/// The RACE page's VARIANT row exists for a title with a variant axis and is
/// dropped for one without. **Wipeout Pure authors neither `team_variants`
/// nor `guest_roster` nor `hull_variants` on any team**, so the row would
/// draw permanently empty and unusable there. **Wipeout Pulse authors none of
/// the first two, but does author the third** - see
/// `oag_title::race::HullVariant`'s own doc comment for the evidence - so its
/// row stays. RACE REMIX's own VARIANT row is untouched on every title:
/// `remix.team` can still name any title's team regardless of what booted.
#[test]
fn the_race_variant_row_is_dropped_for_a_title_with_no_variant_axis() {
    let variant_rows = |definition: &Definition, page_id: &str, setting: &str| {
        definition
            .pages
            .iter()
            .filter(|page| page.id == page_id)
            .flat_map(|page| page.entries.iter())
            .filter(|entry| entry.setting() == Some(setting))
            .count()
    };

    let mut pulse = built_in();
    pulse.drop_unavailable_race_variant(oag_pulse::TITLE);
    assert_eq!(
        variant_rows(&pulse, "race", "race.variant"),
        1,
        "Wipeout Pulse authors a real Normal/Concept hull_variants axis"
    );
    assert_eq!(
        variant_rows(&pulse, "remix", "remix.variant"),
        1,
        "RACE REMIX offers variants for a mixed grid regardless of the booted title"
    );

    let mut pure = built_in();
    pure.drop_unavailable_race_variant(oag_pure::TITLE);
    assert_eq!(
        variant_rows(&pure, "race", "race.variant"),
        0,
        "Wipeout Pure has no variant axis on any team"
    );

    let mut hd = built_in();
    hd.drop_unavailable_race_variant(oag_hd::TITLE);
    assert_eq!(
        variant_rows(&hd, "race", "race.variant"),
        1,
        "Wipeout HD/Fury's twelve teams carry a variant axis"
    );
}

/// Android drops QUIT (a phone app is left, not quit); every other row stays.
#[test]
fn drop_quit_removes_only_the_quit_rows() {
    let quits = |definition: &Definition| {
        definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .filter(|entry| {
                matches!(
                    entry,
                    Entry::Run {
                        action: Action::Quit,
                        ..
                    }
                )
            })
            .count()
    };
    let rows = |definition: &Definition| {
        definition
            .pages
            .iter()
            .map(|page| page.entries.len())
            .sum::<usize>()
    };
    let full = built_in();
    let mut android = built_in();
    android.drop_quit();
    assert!(quits(&full) > 0, "the built-in menu offers QUIT");
    assert_eq!(quits(&android), 0);
    assert_eq!(rows(&full) - rows(&android), quits(&full));
}

/// The mode row is supplied rather than spelled, so what it must agree with
/// is `mode_choices`, not a list in the definition file.
#[test]
fn the_supplied_mode_rows_are_exactly_the_modes_the_game_has() {
    let strings = oag_ui::language::StringTable::default();
    let stored: Vec<String> = mode_choices(&strings)
        .into_iter()
        .map(|choice| choice.value)
        .collect();
    assert_eq!(
        stored,
        oag_race::Mode::ALL
            .iter()
            .map(|mode| mode.name().to_string())
            .collect::<Vec<_>>(),
        "the supplied rows and Mode::ALL must be one list, in the same order"
    );
}

#[test]
fn a_mode_label_is_the_head_of_the_discs_event_text() {
    let strings = oag_ui::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="MSC_EVENT_ZONE" String="Zone: your ship accelerates automatically and the top speed increases."></Entry>
           </StringTable>"#,
    );
    assert_eq!(mode_label(oag_race::Mode::Zone, &strings), "Zone");
}

/// Three ways the disc can fail to yield a label, all of which have to end
/// with a readable row rather than a paragraph or a blank.
#[test]
fn a_mode_label_falls_back_rather_than_printing_prose() {
    let absent = oag_ui::language::StringTable::default();
    assert_eq!(
        mode_label(oag_race::Mode::TimeTrial, &absent),
        oag_race::Mode::TimeTrial.fallback_label()
    );

    let no_colon = oag_ui::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="MSC_EVENT_TT" String="Beat the clock in this solo race and make every corner count"></Entry>
           </StringTable>"#,
    );
    assert_eq!(
        mode_label(oag_race::Mode::TimeTrial, &no_colon),
        oag_race::Mode::TimeTrial.fallback_label(),
        "a description with no colon put its whole first clause in the row"
    );

    let empty = oag_ui::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="MSC_EVENT_SL" String=": focus all your efforts"></Entry>
           </StringTable>"#,
    );
    assert_eq!(
        mode_label(oag_race::Mode::SpeedLap, &empty),
        oag_race::Mode::SpeedLap.fallback_label()
    );
}

/// `oag_race::Mode::fallback_label` is hardcoded English, and the invented-UI-text
/// thread names that as a gap - but `mode_label` already reads through the
/// same merged table [`oag_ui::strings::overlay`] writes a project override
/// into, so naming [`oag_race::Mode::string_id`] in a project file already
/// overrides the fallback with no further wiring, even with no disc entry at
/// all. `oag_ui::language::load::load_strings` is what performs the merge at boot; this proves
/// the read side alone, beside the disc-absent case the test above proves.
#[test]
fn a_project_override_of_a_mode_id_wins_over_the_hardcoded_fallback() {
    let mut strings = oag_ui::language::StringTable::default();
    strings.merge(std::collections::HashMap::from([(
        oag_race::Mode::TimeTrial.string_id().to_string(),
        "AGAINST THE CLOCK".to_string(),
    )]));
    assert_eq!(
        mode_label(oag_race::Mode::TimeTrial, &strings),
        "AGAINST THE CLOCK"
    );
}

/// `every_settings_row_is_one_the_game_seeds` skips any page that is not
/// `display` or `graphics`, so the race rows are not covered by it. The mode
/// row is the one that would break silently: a `choice` whose setting nothing
/// seeds opens on whatever the file happens to hold rather than on the saved
/// value.
#[test]
fn the_mode_row_is_seeded_by_the_settings_module() {
    let settings = oag_game::settings::Settings::default();
    let seeds = oag_game::settings::menu_seeds(
        &settings,
        oag_mesh::mesh_render::Anisotropy::default(),
        oag_pulse::TITLE,
        oag_disc::Platform::Psp,
    );
    assert!(
        seeds.iter().any(|(key, _)| *key == "race.mode"),
        "nothing seeds race.mode, so the menu cannot open on the saved mode"
    );
}

/// Every row on the CONTROLS page is seeded, for the same reason the mode row
/// above needs its own test: `every_settings_row_is_one_the_game_seeds` sweeps
/// `display` and `graphics` and nothing else.
///
/// Written as a sweep of the page rather than as two named rows so that a row
/// added later is covered the day it lands. TRIGGER SENSITIVITY is the one that
/// would break quietly - an unseeded `choice` opens on its list's *first* value,
/// so it would read 50 whatever the file said, and the first press would move a
/// player at 100 straight to 75.
#[test]
fn every_controls_row_is_seeded_by_the_settings_module() {
    let definition = built_in();
    let page = definition
        .pages
        .iter()
        .find(|page| page.id == "controls")
        .expect("a controls page");
    let seeds = oag_game::settings::menu_seeds(
        &oag_game::settings::Settings::default(),
        oag_mesh::mesh_render::Anisotropy::default(),
        oag_pulse::TITLE,
        oag_disc::Platform::Psp,
    );

    for entry in &page.entries {
        let Some(setting) = entry.setting() else {
            continue;
        };
        assert!(
            seeds.iter().any(|(seeded, _)| *seeded == setting),
            "{setting} is on the controls page and is not seeded"
        );
    }
}

/// The sensitivity row offers exactly the percentages the type does.
///
/// The list is in `assets/ui/menu.toml` and the values it must agree with are
/// `settings::TriggerSensitivity::OFFERED`. A spare value in the row is a
/// choice that fails to apply, and a missing one is a setting a player cannot
/// reach from the menus at all.
#[test]
fn the_sensitivity_row_offers_what_the_type_offers() {
    let definition = built_in();
    let page = definition
        .pages
        .iter()
        .find(|page| page.id == "controls")
        .expect("a controls page");
    let offered: Vec<String> = oag_game::settings::TriggerSensitivity::OFFERED
        .iter()
        .map(ToString::to_string)
        .collect();

    let values: Vec<String> = page
        .entries
        .iter()
        .find_map(|entry| match entry {
            Entry::Choice {
                setting, values, ..
            } if setting == "controls.trigger_sensitivity" => {
                Some(values.iter().map(|v| v.value.clone()).collect())
            }
            _ => None,
        })
        .expect("a trigger sensitivity row");
    assert_eq!(values, offered);
}

/// The RACE page opens on a time trial, and that is what the row's *first*
/// value plus the settings default together have to say. Either one alone
/// would leave the other free to drift.
#[test]
fn the_mode_row_starts_on_the_time_trial() {
    let definition = built_in();
    let race = definition
        .pages
        .iter()
        .find(|page| page.id == "race")
        .expect("a race page");
    let source = race
        .entries
        .iter()
        .find_map(|entry| match entry {
            Entry::Choice {
                setting, source, ..
            } if setting == "race.mode" => Some(*source),
            _ => None,
        })
        .expect("a mode row");
    assert_eq!(
        source,
        Some(ValueSource::RaceModes),
        "the mode row spells its values instead of taking them off the disc"
    );

    let strings = oag_ui::language::StringTable::default();
    assert_eq!(
        mode_choices(&strings)
            .first()
            .map(|choice| choice.value.clone()),
        Some(oag_race::Mode::TimeTrial.name().to_string())
    );
    assert_eq!(
        oag_game::settings::Race::default().mode,
        oag_race::Mode::TimeTrial.name(),
        "the row opens on a time trial but the saved default is something else"
    );
}

/// Anisotropy's values have to be exactly what `Anisotropy` parses - a typo
/// here would persist a setting the next run refuses to load.
#[test]
fn the_anisotropy_row_offers_only_levels_that_parse() {
    let definition = built_in();
    let Some(Entry::Choice { values, .. }) = definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some("graphics.anisotropy"))
    else {
        panic!("nothing edits graphics.anisotropy");
    };
    for option in values {
        option
            .value
            .parse::<oag_mesh::mesh_render::Anisotropy>()
            .unwrap_or_else(|e| panic!("{:?}: {e}", option.value));
    }
}

/// The same drift guard as the one above, for every row on DISPLAY and
/// GRAPHICS. A value here that `display` cannot parse would be ignored at
/// runtime with a message, which is the failure a player meets as "this row
/// does nothing".
#[test]
fn the_two_settings_pages_offer_only_values_that_parse() {
    use oag_display::display::{
        Aspect, BoostFovKick, Brightness, CameraView, Fov, Gamma, Size, WindowMode,
    };
    let definition = built_in();
    let values = |setting: &str| -> Vec<String> {
        definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .find(|entry| entry.setting() == Some(setting))
            .and_then(|entry| match entry {
                Entry::Choice { values, .. } => {
                    Some(values.iter().map(|v| v.value.clone()).collect())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no choice edits {setting:?}"))
    };

    let aspects = values("display.aspect");
    for name in &aspects {
        name.parse::<Aspect>().unwrap_or_else(|e| panic!("{e}"));
    }
    assert_eq!(
        aspects.len(),
        Aspect::ALL.len(),
        "every aspect should be offerable"
    );

    let modes = values("display.window_mode");
    for name in &modes {
        name.parse::<WindowMode>().unwrap_or_else(|e| panic!("{e}"));
    }
    assert_eq!(modes.len(), WindowMode::ALL.len());

    let scales: Vec<oag_display::display::Scale> = values("graphics.render_scale")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        scales,
        oag_display::display::Scale::OFFERED,
        "the render-scale rows and `Scale::OFFERED` must be one list"
    );

    let reconstructions: Vec<oag_display::display::Reconstruction> =
        values("graphics.reconstruction")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
    assert_eq!(
        reconstructions,
        oag_display::display::Reconstruction::ALL,
        "the reconstruction rows and `Reconstruction::ALL` must be one list"
    );

    let sharpness: Vec<oag_display::display::Sharpness> = values("graphics.upscale_sharpness")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        sharpness,
        oag_display::display::Sharpness::OFFERED,
        "the sharpness rows and `Sharpness::OFFERED` must be one list"
    );

    let msaa: Vec<oag_display::display::Msaa> = values("graphics.msaa")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        msaa,
        oag_display::display::Msaa::ALL,
        "the MSAA rows and `Msaa::ALL` must be one list"
    );

    let texture_detail: Vec<oag_mesh::mesh_render::TextureDetail> =
        values("graphics.texture_detail")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
    assert_eq!(
        texture_detail,
        oag_mesh::mesh_render::TextureDetail::ALL,
        "the TEXTURE DETAIL rows and `TextureDetail::ALL` must be one list"
    );

    let sizes: Vec<Size> = values("display.window_size")
        .iter()
        .map(|name| name.parse::<Size>().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        sizes,
        Size::OFFERED,
        "the window-size rows and `Size::OFFERED` must be one list"
    );

    let overlays: Vec<oag_present::perf::Overlay> = values("graphics.perf_overlay")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        overlays,
        oag_present::perf::Overlay::ALL,
        "the overlay rows and `Overlay::ALL` must be one list"
    );

    let limits: Vec<oag_present::perf::FrameLimit> = values("display.frame_limit")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        limits,
        oag_present::perf::FrameLimit::OFFERED,
        "the frame-limit rows and `FrameLimit::OFFERED` must be one list"
    );

    let vsync: Vec<oag_present::perf::Vsync> = values("display.vsync")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        vsync,
        oag_present::perf::Vsync::ALL,
        "the vsync rows and `Vsync::ALL` must be one list"
    );

    let brightness: Vec<Brightness> = values("display.brightness")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        brightness,
        Brightness::OFFERED,
        "the brightness rows and `Brightness::OFFERED` must be one list"
    );

    let gamma: Vec<Gamma> = values("display.gamma")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        gamma,
        Gamma::OFFERED,
        "the gamma rows and `Gamma::OFFERED` must be one list"
    );

    let music: Vec<oag_sound::Volume> = values("audio.music_volume")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        music,
        oag_sound::Volume::OFFERED,
        "the music-volume rows and `Volume::OFFERED` must be one list"
    );

    let fov: Vec<Fov> = values("graphics.fov")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        fov,
        Fov::OFFERED,
        "the field-of-view rows and `Fov::OFFERED` must be one list"
    );

    let boost_fov_kick: Vec<BoostFovKick> = values("graphics.boost_fov_kick")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        boost_fov_kick,
        BoostFovKick::OFFERED,
        "the boost-fov-kick rows and `BoostFovKick::OFFERED` must be one list"
    );

    // And this one is stronger than the others in the list: `CameraView::ALL`
    // is the *recovered cycle order*, so a row list that merely held the same
    // three values in a different order would put the menu and the in-race
    // cycle button out of step. Equality of the sequences is what rules that
    // out.
    let camera_view: Vec<CameraView> = values("graphics.camera_view")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        camera_view,
        CameraView::ALL,
        "the camera-view rows and `CameraView::ALL` must be one list, in the cycle's order"
    );
}

/// Every row that edits a setting has to be one `apply_setting` handles and
/// `menu_seeds` fills in, or it is a row that moves and does nothing - which
/// is the failure this whole file is arranged to prevent.
///
/// Checked against the seeds rather than against `main.rs`, which this
/// crate's library half cannot see: a setting the seeds do not know would
/// also open its row on the list's first option instead of the player's
/// own value, so the two lists have to agree anyway.
///
/// **Every page, not two of them.** This swept `display` and `graphics` only
/// until 2026-08-24, and `audio.sfx_volume` sat unseeded on the AUDIO page for
/// as long as the row existed - reading `0` at every setting, because that is
/// the head of its own value list. A guard that covers the pages somebody
/// happened to be working on is a guard that lets the next page through.
#[test]
fn every_settings_row_is_one_the_game_seeds() {
    let definition = built_in();
    // With a language picked, because `menu_seeds` only offers that key once
    // one has been - and the LANGUAGE row exists whether or not it has.
    let settings = oag_game::settings::Settings {
        language: Some("English".to_string()),
        ..Default::default()
    };
    let seeded: Vec<&str> = oag_game::settings::menu_seeds(
        &settings,
        oag_mesh::mesh_render::Anisotropy::default(),
        oag_pulse::TITLE,
        oag_disc::Platform::Psp,
    )
    .into_iter()
    .map(|(setting, _)| setting)
    .collect();

    // The AI PILOTS page's own four rows are not settings at all - which
    // pilot and which axis are on screen is not persisted, so nothing in
    // `settings::menu_seeds` could know them (see `oag_raceplay::pilots`'s own
    // module doc on why a pilot file is not a setting). They are supplied
    // and seeded by `Session::supply_pilot_menu`/`resupply_pilot_bounds`
    // instead. Exempted by name rather than by page, so an ordinary setting
    // later added to this same page still has to earn its place here.
    const PILOT_EDITOR_ROWS: [&str; 4] =
        ["pilot.selected", "pilot.axis", "pilot.low", "pilot.high"];

    for page in &definition.pages {
        for entry in &page.entries {
            let Some(setting) = entry.setting() else {
                continue;
            };
            if PILOT_EDITOR_ROWS.contains(&setting) {
                continue;
            }
            assert!(
                seeded.contains(&setting),
                "{setting} is on the {} page and is not seeded",
                page.id
            );
        }
    }

    // And the settings pages exist, so a rename in the definition cannot make
    // the loop above vacuous.
    for id in ["display", "graphics", "audio", "controls"] {
        assert!(
            definition.pages.iter().any(|page| page.id == id),
            "no {id} page"
        );
    }
}

/// The definition's own answer to "classic vsync makes the limiter
/// meaningless, and the other two modes do not".
///
/// Pinned here because the halves live in different files: the pairing is
/// asserted in `assets/ui/menu.toml`, and the loop that honours it is in
/// `main.rs` against `Vsync::paces_itself`. Both have to name the same
/// value or the row greys out at the wrong time, and a row that lost its
/// `disabled_by` altogether would still parse.
#[test]
fn the_frame_limit_is_disabled_by_classic_vsync_alone() {
    let definition = built_in();
    let entry = definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some("display.frame_limit"))
        .expect("nothing edits graphics.frame_limit");
    let condition = entry.disabled_by().expect("the limiter has a condition");
    assert_eq!(condition.setting, "display.vsync");
    assert_eq!(condition.values, vec![Value::Text("on".to_string())]);

    // The same value from the other side, so the menu and the loop cannot
    // drift into disagreeing about which mode paces itself.
    let [Value::Text(name)] = condition.values.as_slice() else {
        panic!("the vsync row stores one text value");
    };
    let mode: oag_present::perf::Vsync = name.parse().expect("a real vsync mode");
    assert!(mode.paces_itself());
    for other in oag_present::perf::Vsync::ALL {
        assert_eq!(
            other.paces_itself(),
            other == mode,
            "{other} is the wrong side of the condition"
        );
    }
}

/// The RACE page's TEAM, VARIANT and TRACK rows exist only on a title whose
/// front end has no selection screens of its own. Wipeout Pulse's race box
/// picks all three on `Track Creation` and `Team Selection`
/// (`docs/ui/selection-screens.md`), so its RACE page keeps MODE, SPEED
/// CLASS, AI DIFFICULTY, START and BACK and nothing that those screens
/// already ask. Pure's own `Track Selection`/`Team Selection` picked up the
/// same treatment 2026-09-10, once `oag_pure::FRONT_END.race_box` was wired
/// (`docs/formats/race-setup.md`); it never authored a VARIANT row to begin
/// with (`RaceDefaults::has_team_variants`). HD authors its two screens in
/// files of their own (`FrontEnd::track_select`/`team_select`, read
/// 2026-09-29) and drops the same rows; Wipeout 2048, which reads neither,
/// keeps all three.
#[test]
fn the_rows_the_selection_screens_pick_are_dropped_on_a_title_that_has_them() {
    let settings_on_race = |definition: &Definition| -> Vec<String> {
        definition
            .pages
            .iter()
            .filter(|page| page.id == "race")
            .flat_map(|page| page.entries.iter())
            .filter_map(|entry| entry.setting().map(str::to_string))
            .collect()
    };

    let mut pulse = built_in();
    pulse.drop_rows_picked_on_screen(oag_pulse::TITLE);
    let kept = settings_on_race(&pulse);
    for gone in ["race.team", "race.variant", "race.track"] {
        assert!(
            !kept.contains(&gone.to_string()),
            "{gone} stays on Pulse: {kept:?}"
        );
    }
    for stays in ["race.mode", "race.class"] {
        assert!(
            kept.contains(&stays.to_string()),
            "{stays} dropped on Pulse: {kept:?}"
        );
    }
    // The race box's own two lists are read on Pulse alone.
    for stays in ["race.kill_target", "race.weapons"] {
        assert!(
            kept.contains(&stays.to_string()),
            "{stays} dropped on Pulse"
        );
    }
    // RECORDS and RACE REMIX keep their own circuit rows.
    let elsewhere = pulse
        .pages
        .iter()
        .filter(|page| page.id != "race")
        .flat_map(|page| page.entries.iter())
        .filter(|entry| entry.setting() == Some("race.track"))
        .count();
    assert!(elsewhere >= 1, "RECORDS keeps its race.track row");

    let mut pure = built_in();
    pure.drop_rows_picked_on_screen(oag_pure::TITLE);
    let kept = settings_on_race(&pure);
    for gone in ["race.team", "race.track"] {
        assert!(
            !kept.contains(&gone.to_string()),
            "{gone} stays on Pure: {kept:?}"
        );
    }
    for stays in ["race.mode", "race.class"] {
        assert!(
            kept.contains(&stays.to_string()),
            "{stays} dropped on Pure: {kept:?}"
        );
    }

    for gone in ["race.kill_target", "race.weapons"] {
        assert!(
            !settings_on_race(&pure).contains(&gone.to_string()),
            "{gone} stays on Pure, which reads no such list"
        );
    }

    let mut hd = built_in();
    hd.drop_rows_picked_on_screen(oag_hd::TITLE);
    let kept = settings_on_race(&hd);
    for gone in ["race.team", "race.track", "race.variant"] {
        assert!(
            !kept.contains(&gone.to_string()),
            "{gone} stays on HD: {kept:?}"
        );
    }

    let mut vita = built_in();
    vita.drop_rows_picked_on_screen(oag_2048::TITLE);
    for stays in ["race.team", "race.track"] {
        assert!(
            settings_on_race(&vita).contains(&stays.to_string()),
            "{stays} dropped on 2048: {:?}",
            settings_on_race(&vita)
        );
    }
}

/// The three touch rows live on their own page under CONTROLS, and a machine
/// with no touchscreen loses only the way in.
#[test]
fn the_touch_rows_sit_on_their_own_page_and_desktop_drops_the_way_in() {
    let settings = |definition: &Definition, id: &str| -> Vec<String> {
        definition
            .pages
            .iter()
            .find(|page| page.id == id)
            .unwrap_or_else(|| panic!("no page {id}"))
            .entries
            .iter()
            .filter_map(|entry| entry.setting().map(str::to_string))
            .collect()
    };
    let full = built_in();
    for key in [
        "controls.touch_scheme",
        "controls.touch_go_zones",
        "controls.touch_opacity",
    ] {
        assert!(settings(&full, "touch_controls").iter().any(|s| s == key));
        for other in ["graphics", "controls"] {
            assert!(
                !settings(&full, other).iter().any(|s| s == key),
                "{key} on {other}"
            );
        }
    }
    let way_in = |definition: &Definition| {
        let touch = definition
            .pages
            .iter()
            .position(|page| page.id == "touch_controls")
            .expect("the page");
        definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .filter(|entry| matches!(entry, Entry::Submenu { target, .. } if *target == touch))
            .count()
    };
    assert_eq!(way_in(&full), 1);
    let mut desktop = built_in();
    desktop.drop_touch_controls();
    assert_eq!(way_in(&desktop), 0);
    assert!(desktop.pages.iter().any(|page| page.id == "touch_controls"));
}
