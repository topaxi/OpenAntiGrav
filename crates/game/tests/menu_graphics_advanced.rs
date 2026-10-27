//! The GRAPHICS page and the ADVANCED page under it: which rows sit where, on
//! the title with every row and on the titles that drop some. Moving a row is
//! presentation only, so each row keeps its setting key.

use oag_gameplay::input::{Button, Input};
use oag_ui::menu::*;

const TOP: [&str; 11] = [
    "graphics.renderer",
    "graphics.render_scale",
    "graphics.target_fps",
    "graphics.minimum_resolution",
    "graphics.reconstruction",
    "graphics.upscale_sharpness",
    "graphics.msaa",
    "graphics.anisotropy",
    "graphics.camera_view",
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
            other => other.setting().expect("a setting row").to_string(),
        })
        .collect()
}

fn advanced(pulse_like: bool) -> Vec<&'static str> {
    let mut rows = vec![
        "graphics.motion_blur",
        "graphics.motion_blur_resolution",
        "graphics.shadows",
        "graphics.model_detail",
        "graphics.texture_detail",
    ];
    if pulse_like {
        rows.push("graphics.trail_order");
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
    rows.extend(["graphics.cpu_governor", "graphics.perf_overlay", "back"]);
    rows
}

#[test]
fn the_graphics_page_keeps_the_original_and_performance_rows_and_opens_advanced() {
    for title in [oag_pulse::TITLE, oag_pure::TITLE, oag_hd::TITLE] {
        let mut definition = built_in();
        definition.drop_unavailable_boost_zoom(title);
        definition.drop_unavailable_trail_order(title);
        assert_eq!(rows(&definition, "graphics"), TOP, "{}", title.name);
    }
}

#[test]
fn advanced_holds_the_look_extras_and_the_mixed_rows_in_order() {
    for (title, pulse_like) in [
        (oag_pulse::TITLE, true),
        (oag_pure::TITLE, true),
        (oag_hd::TITLE, false),
        (oag_omega::TITLE, false),
        (oag_2048::TITLE, false),
    ] {
        let mut definition = built_in();
        definition.drop_unavailable_boost_zoom(title);
        definition.drop_unavailable_trail_order(title);
        assert_eq!(
            rows(&definition, "graphics_advanced"),
            advanced(pulse_like),
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
