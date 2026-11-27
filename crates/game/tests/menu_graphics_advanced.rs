//! The GRAPHICS page and the ADVANCED page under it: which rows sit where, on
//! the title with every row and on the titles that drop some. Moving a row is
//! presentation only, so each row keeps its setting key.

use oag_gameplay::input::{Button, Input};
use oag_ui::menu::*;

const TOP: [&str; 12] = [
    "graphics.renderer",
    "graphics.render_scale",
    "graphics.target_fps",
    "graphics.minimum_resolution",
    "graphics.reconstruction",
    "graphics.upscale_sharpness",
    "graphics.msaa",
    "graphics.anisotropy",
    "graphics.camera_view",
    "action:benchmark",
    "submenu:graphics_advanced",
    "back",
];

fn built_in() -> Definition {
    Definition::built_in(&oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse")
}

fn rows(definition: &Definition, id: &str) -> Vec<String> {
    let page = definition.page(id).unwrap_or_else(|| panic!("{id} exists"));
    page.entries
        .iter()
        .map(|entry| match entry {
            Entry::Submenu { target, .. } => format!("submenu:{}", definition.pages[*target].id),
            Entry::Back { .. } => "back".to_string(),
            Entry::Run { action, .. } => format!("action:{}", action.name()),
            other => other.setting().expect("a setting row").to_string(),
        })
        .collect()
}

fn advanced(
    pulse_like: bool,
    weather: bool,
    trail_hits: bool,
    encoded: bool,
    materials: bool,
    glass_target: bool,
) -> Vec<&'static str> {
    // WINGTIP CONTRAILS is on the titles with no `wingtip` class of their own: the PSP pair, as
    // TRAIL ORDER and BOOST ZOOM are.
    let wingtip = pulse_like;
    let mut rows = vec![
        "graphics.motion_blur",
        "graphics.motion_blur_resolution",
        "graphics.shadows",
        "graphics.shadow_resolution",
    ];
    // SCENE PRECISION is on the title whose scene stores encoded values: HD.
    if encoded {
        rows.push("graphics.scene_precision");
    }
    // CIRCUIT LIGHTING is on the titles whose circuits are shaded from their own materials.
    if materials {
        rows.push("graphics.circuit_lighting");
    }
    // REFLECTION RESOLUTION is on the titles that draw a screen-space glass or sea target.
    if glass_target {
        rows.push("graphics.reflection_resolution");
    }
    rows.extend([
        "graphics.water",
        "graphics.model_detail",
        "graphics.texture_detail",
        // Kept while no device has said whether it takes ASTC (`oag_game::texture_cache`).
        "graphics.texture_compression",
    ]);
    if pulse_like {
        rows.push("graphics.trail_order");
        rows.push("graphics.scenery_sort");
        rows.push("graphics.trail_length");
    }
    if weather {
        rows.push("graphics.weather");
    }
    rows.extend([
        "graphics.screen_filter",
        "graphics.screen_filter_strength",
        "graphics.fov",
        "graphics.boost_fov_kick",
    ]);
    if pulse_like {
        rows.push("graphics.boost_zoom");
    }
    if trail_hits {
        rows.push("graphics.trail_hits");
    }
    if wingtip {
        rows.push("graphics.wingtip_contrails");
        // OWN SHIELD is on the same pair: the titles whose shield is not HD's.
        rows.push("graphics.own_shield");
        // OWN ROCKETS too: the titles whose rocket is drawn from PSP-era effects.
        rows.push("graphics.own_rockets");
    }
    rows.extend(["graphics.cpu_governor", "graphics.perf_overlay", "back"]);
    rows
}

#[test]
fn the_graphics_page_keeps_the_original_and_performance_rows_and_opens_advanced() {
    for title in [oag_pulse::TITLE, oag_pure::TITLE, oag_hd::TITLE] {
        let mut definition = built_in();
        definition.drop_unavailable_boost_zoom(title);
        definition.drop_unavailable_trail_hits(title);
        definition.drop_unavailable_scene_precision(title);
        definition.drop_unavailable_circuit_lighting(title);
        definition.drop_unavailable_reflection_resolution(title);
        definition.drop_unavailable_wingtip_contrails(title);
        definition.drop_unavailable_own_shield(title);
        definition.drop_unavailable_own_rockets(title);
        definition.drop_unavailable_trail_order(title);
        definition.drop_unavailable_scenery_sort(title);
        definition.drop_unavailable_trail_length(title);
        definition.drop_unavailable_weather(title);
        assert_eq!(rows(&definition, "graphics"), TOP, "{}", title.name);
    }
}

#[test]
fn advanced_holds_the_look_extras_and_the_mixed_rows_in_order() {
    // TRAIL HITS stays on every title whose original never shows the burst: all but HD.
    for (title, pulse_like, trail_hits) in [
        (oag_pulse::TITLE, true, true),
        (oag_pure::TITLE, true, true),
        (oag_hd::TITLE, false, false),
        (oag_omega::TITLE, false, true),
        (oag_2048::TITLE, false, true),
    ] {
        let mut definition = built_in();
        definition.drop_unavailable_boost_zoom(title);
        definition.drop_unavailable_trail_hits(title);
        definition.drop_unavailable_scene_precision(title);
        definition.drop_unavailable_circuit_lighting(title);
        definition.drop_unavailable_reflection_resolution(title);
        definition.drop_unavailable_wingtip_contrails(title);
        definition.drop_unavailable_own_shield(title);
        definition.drop_unavailable_own_rockets(title);
        definition.drop_unavailable_trail_order(title);
        definition.drop_unavailable_scenery_sort(title);
        definition.drop_unavailable_trail_length(title);
        definition.drop_unavailable_weather(title);
        assert_eq!(
            rows(&definition, "graphics_advanced"),
            advanced(
                pulse_like,
                title.weather.is_played(),
                trail_hits,
                title.looks.blends_encoded.applies_everywhere(),
                title.looks.material_render_state.applies_everywhere(),
                title.looks.screen_space_target.applies_everywhere(),
            ),
            "{}",
            title.name
        );
    }
}

#[test]
fn confirming_the_advanced_row_opens_the_page() {
    let mut menu = Menu::new(built_in());
    assert!(menu.open("graphics"));
    let last_but_one = menu.page().entries.len() - 2;
    for _ in 0..last_but_one {
        let mut input = Input::new();
        input.begin_frame(1 << Button::Down.index());
        menu.update(&mut input);
        let mut input = Input::new();
        input.begin_frame(0);
        menu.update(&mut input);
    }
    let mut input = Input::new();
    input.begin_frame(1 << Button::Cross.index());
    menu.update(&mut input);
    assert_eq!(menu.page().id, "graphics_advanced");
}

#[test]
fn target_fps_offers_default_after_the_fixed_rates_and_shows_the_live_rate() {
    let definition = built_in();
    let page = definition.page("graphics").expect("graphics page");
    let values: Vec<(String, String)> = page
        .entries
        .iter()
        .find_map(|entry| match entry {
            Entry::Choice {
                setting, values, ..
            } if setting == "graphics.target_fps" => Some(
                values
                    .iter()
                    .map(|v| (v.value.clone(), v.label.clone()))
                    .collect(),
            ),
            _ => None,
        })
        .expect("a TARGET FPS row");
    let names: Vec<&str> = values.iter().map(|(value, _)| value.as_str()).collect();
    assert_eq!(
        names,
        [
            "off", "original", "30", "60", "90", "100", "120", "144", "default"
        ]
    );
    assert!(
        values[8].1.eq_ignore_ascii_case("default"),
        "{}",
        values[8].1
    );

    let mut menu = Menu::new(definition);
    menu.relabel_choice("graphics.target_fps", "default", "DEFAULT (144)");
    let page = menu.definition().page("graphics").expect("graphics page");
    let labelled = page.entries.iter().any(|entry| match entry {
        Entry::Choice {
            setting, values, ..
        } if setting == "graphics.target_fps" => values
            .iter()
            .any(|v| v.value == "default" && v.label == "DEFAULT (144)"),
        _ => false,
    });
    assert!(labelled);
}

/// TARGET FPS offers `original` only where the title has a width throttle of its own (HD),
/// keeps every other value everywhere, and greys MINIMUM RESOLUTION while it is chosen.
#[test]
fn target_fps_offers_original_only_where_the_title_throttles_its_width() {
    let values = |definition: &Definition| -> Vec<String> {
        definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .filter_map(|entry| match entry {
                oag_ui::menu::Entry::Choice {
                    setting, values, ..
                } if setting == "graphics.target_fps" => Some(values),
                _ => None,
            })
            .flat_map(|values| values.iter().map(|choice| choice.value.clone()))
            .collect()
    };
    let every = values(&built_in());
    assert!(every.iter().any(|value| value == "original"), "{every:?}");
    let without: Vec<String> = every
        .iter()
        .filter(|value| *value != "original")
        .cloned()
        .collect();
    for (title, offered) in [
        (oag_pulse::TITLE, &without),
        (oag_pure::TITLE, &without),
        (oag_hd::TITLE, &every),
        (oag_omega::TITLE, &without),
        (oag_2048::TITLE, &without),
    ] {
        let mut definition = built_in();
        definition.drop_unavailable_width_throttle(title);
        assert_eq!(&values(&definition), offered, "{}", title.name);
    }
    let floor = built_in()
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some("graphics.minimum_resolution"))
        .and_then(|entry| entry.disabled_by().cloned())
        .expect("MINIMUM RESOLUTION is greyed by something");
    assert_eq!(floor.setting, "graphics.target_fps");
    assert!(
        floor
            .values
            .contains(&oag_ui::menu::Value::Text("original".to_string())),
        "{:?}",
        floor.values
    );
}

#[test]
fn frame_limit_ends_with_refresh_rate_and_shows_the_live_rate() {
    let mut menu = Menu::new(built_in());
    menu.relabel_choice("display.frame_limit", "refresh_rate", "REFRESH RATE (144)");
    let found = menu.definition().pages.iter().flat_map(|page| &page.entries).any(|entry| {
        matches!(entry, Entry::Choice { setting, values, .. }
            if setting == "display.frame_limit"
                && values.last().is_some_and(|v| v.value == "refresh_rate" && v.label == "REFRESH RATE (144)"))
    });
    assert!(found);
}

/// REFLECTION RESOLUTION is offered by the titles that draw the glass or sea target (HD, 2048,
/// Omega) and dropped by the two that never do.
#[test]
fn the_reflection_resolution_row_is_on_the_titles_with_a_glass_target() {
    for (title, shown) in [
        (oag_pulse::TITLE, false),
        (oag_pure::TITLE, false),
        (oag_hd::TITLE, true),
        (oag_omega::TITLE, true),
        (oag_2048::TITLE, true),
    ] {
        let mut definition = built_in();
        definition.drop_unavailable_reflection_resolution(title);
        assert_eq!(
            rows(&definition, "graphics_advanced")
                .iter()
                .any(|row| row == "graphics.reflection_resolution"),
            shown,
            "{}",
            title.name
        );
    }
}
