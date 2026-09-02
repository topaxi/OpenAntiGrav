//! The menu definition this build ships: that it parses, that every page can
//! be left, and that every row on it offers only values the game will accept.
//!
//! Split out of `menu/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

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
#[test]
fn the_race_page_offers_only_teams_and_classes_the_game_accepts() {
    let definition = built_in();
    let values = |setting: &str| -> Vec<String> {
        #[allow(clippy::redundant_closure_for_method_calls)]
        definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .find_map(|entry| match entry {
                Entry::Choice {
                    setting: key,
                    values,
                    ..
                } if key == setting => Some(values.iter().map(|v| v.value.clone()).collect()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no choice edits {setting:?}"))
    };

    assert!(
        values("race.team").is_empty(),
        "the team row must be supplied, not spelled: the roster is what the \
         source offers plus what a mounted pack adds, and no list in this \
         repository can know either"
    );
    for name in values("race.class") {
        assert!(
            oag_physics::SpeedClass::from_name(&name).is_some(),
            "{name:?} is not a speed class"
        );
    }
    assert_eq!(
        values("race.class").len(),
        oag_physics::SpeedClass::ALL.len(),
        "every speed class should be offerable"
    );
}

/// The mode row is supplied rather than spelled, so what it must agree with
/// is `mode_choices`, not a list in the definition file.
#[test]
fn the_supplied_mode_rows_are_exactly_the_modes_the_game_has() {
    let strings = crate::language::StringTable::default();
    let stored: Vec<String> = super::mode_choices(&strings)
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
    let strings = crate::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="MSC_EVENT_ZONE" String="Zone: your ship accelerates automatically and the top speed increases."></Entry>
           </StringTable>"#,
    );
    assert_eq!(super::mode_label(oag_race::Mode::Zone, &strings), "Zone");
}

/// Three ways the disc can fail to yield a label, all of which have to end
/// with a readable row rather than a paragraph or a blank.
#[test]
fn a_mode_label_falls_back_rather_than_printing_prose() {
    let absent = crate::language::StringTable::default();
    assert_eq!(
        super::mode_label(oag_race::Mode::TimeTrial, &absent),
        oag_race::Mode::TimeTrial.fallback_label()
    );

    let no_colon = crate::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="MSC_EVENT_TT" String="Beat the clock in this solo race and make every corner count"></Entry>
           </StringTable>"#,
    );
    assert_eq!(
        super::mode_label(oag_race::Mode::TimeTrial, &no_colon),
        oag_race::Mode::TimeTrial.fallback_label(),
        "a description with no colon put its whole first clause in the row"
    );

    let empty = crate::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="MSC_EVENT_SL" String=": focus all your efforts"></Entry>
           </StringTable>"#,
    );
    assert_eq!(
        super::mode_label(oag_race::Mode::SpeedLap, &empty),
        oag_race::Mode::SpeedLap.fallback_label()
    );
}

/// `every_settings_row_is_one_the_game_seeds` skips any page that is not
/// `display` or `graphics`, so the race rows are not covered by it. The mode
/// row is the one that would break silently: a `choice` whose setting nothing
/// seeds opens on whatever the file happens to hold rather than on the saved
/// value.
#[test]
fn the_mode_row_is_seeded_by_the_settings_module() {
    let settings = crate::settings::Settings::default();
    let seeds = crate::settings::menu_seeds(
        &settings,
        oag_render::mesh_render::Anisotropy::default(),
        oag_pulse::TITLE.name,
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
    let seeds = crate::settings::menu_seeds(
        &crate::settings::Settings::default(),
        oag_render::mesh_render::Anisotropy::default(),
        oag_pulse::TITLE.name,
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
    let offered: Vec<String> = crate::settings::TriggerSensitivity::OFFERED
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

    let strings = crate::language::StringTable::default();
    assert_eq!(
        super::mode_choices(&strings)
            .first()
            .map(|choice| choice.value.clone()),
        Some(oag_race::Mode::TimeTrial.name().to_string())
    );
    assert_eq!(
        crate::settings::Race::default().mode,
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
            .parse::<oag_render::mesh_render::Anisotropy>()
            .unwrap_or_else(|e| panic!("{:?}: {e}", option.value));
    }
}

/// The same drift guard as the one above, for every row on DISPLAY and
/// GRAPHICS. A value here that `display` cannot parse would be ignored at
/// runtime with a message, which is the failure a player meets as "this row
/// does nothing".
#[test]
fn the_two_settings_pages_offer_only_values_that_parse() {
    use crate::display::{
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

    let scales: Vec<crate::display::Scale> = values("graphics.render_scale")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        scales,
        crate::display::Scale::OFFERED,
        "the render-scale rows and `Scale::OFFERED` must be one list"
    );

    let upscalers: Vec<crate::display::Upscaler> = values("graphics.upscaler")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        upscalers,
        crate::display::Upscaler::ALL,
        "the upscaler rows and `Upscaler::ALL` must be one list"
    );

    let sharpness: Vec<crate::display::Sharpness> = values("graphics.upscale_sharpness")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        sharpness,
        crate::display::Sharpness::OFFERED,
        "the sharpness rows and `Sharpness::OFFERED` must be one list"
    );

    let anti_aliasing: Vec<crate::display::AntiAliasing> = values("graphics.anti_aliasing")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        anti_aliasing,
        crate::display::AntiAliasing::ALL,
        "the anti-aliasing rows and `AntiAliasing::ALL` must be one list"
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

    let overlays: Vec<crate::perf::Overlay> = values("graphics.perf_overlay")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        overlays,
        crate::perf::Overlay::ALL,
        "the overlay rows and `Overlay::ALL` must be one list"
    );

    let limits: Vec<crate::perf::FrameLimit> = values("display.frame_limit")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        limits,
        crate::perf::FrameLimit::OFFERED,
        "the frame-limit rows and `FrameLimit::OFFERED` must be one list"
    );

    let vsync: Vec<crate::perf::Vsync> = values("display.vsync")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        vsync,
        crate::perf::Vsync::ALL,
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

    let music: Vec<crate::audio::Volume> = values("audio.music_volume")
        .iter()
        .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(
        music,
        crate::audio::Volume::OFFERED,
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
    let settings = crate::settings::Settings {
        language: Some("English".to_string()),
        ..Default::default()
    };
    let seeded: Vec<&str> = crate::settings::menu_seeds(
        &settings,
        oag_render::mesh_render::Anisotropy::default(),
        oag_pulse::TITLE.name,
    )
    .into_iter()
    .map(|(setting, _)| setting)
    .collect();

    for page in &definition.pages {
        for entry in &page.entries {
            let Some(setting) = entry.setting() else {
                continue;
            };
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
    let mode: crate::perf::Vsync = name.parse().expect("a real vsync mode");
    assert!(mode.paces_itself());
    for other in crate::perf::Vsync::ALL {
        assert_eq!(
            other.paces_itself(),
            other == mode,
            "{other} is the wrong side of the condition"
        );
    }
}
