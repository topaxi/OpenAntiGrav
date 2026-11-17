//! `graphics.water` in the per-title render profile: default, round trip, fallback.

use super::render_profile::known_profiles;
use super::tests::load_text;
use super::*;
use oag_display::display::Water;

const PULSE: &str = "Wipeout Pulse (PSP)";

#[test]
fn a_profile_defaults_to_the_titles_own_law() {
    assert_eq!(RenderProfile::default().water, Water::Original);
    for (title, platform) in known_profiles() {
        assert_eq!(
            Settings::default()
                .render_profiles
                .get(&profile_key(title, platform))
                .map_or(Water::Original, |p| p.water),
            Water::Original,
            "{}",
            title.name
        );
    }
}

#[test]
fn every_water_value_round_trips_through_the_settings_file() {
    for water in Water::ALL {
        let text = format!("[render_profiles.\"{PULSE}\"]\nwater = \"{water}\"\n");
        let (loaded, _) = load_text(&text);
        assert_eq!(loaded.render_profiles[PULSE].water, water);
        let written = render(&loaded).expect("renders");
        assert!(
            written.contains(&format!("water = \"{water}\"")),
            "{water} is written back"
        );
        let (again, _) = load_text(&written);
        assert_eq!(again.render_profiles[PULSE].water, water);
    }
}

#[test]
fn an_unknown_water_value_falls_back_to_original_and_keeps_its_neighbours() {
    let text = format!("[render_profiles.\"{PULSE}\"]\nwater = \"tidal\"\nmsaa = \"4x\"\n");
    let (loaded, after) = load_text(&text);
    let profile = &loaded.render_profiles[PULSE];
    assert_eq!(profile.water, Water::Original);
    assert_eq!(profile.msaa.to_string(), "4x");
    assert_eq!(after, text, "the player's file is left alone");
}

#[test]
fn the_menu_seed_carries_the_stored_water_value() {
    let (title, platform) = known_profiles().next().expect("a title is known");
    let mut settings = Settings::default();
    settings
        .render_profiles
        .entry(profile_key(title, platform))
        .or_default()
        .water = Water::Off;
    let seeds = menu_seeds(&settings, Anisotropy::default(), title, platform);
    let seeded = seeds
        .iter()
        .find(|(key, _)| *key == "graphics.water")
        .expect("the WATER row is seeded on every title");
    assert_eq!(seeded.1, oag_ui::menu::Value::Text("off".to_string()));
}
