//! `graphics.shadow_resolution` in the per-title render profile: default, round trip, fallback.

use super::render_profile::known_profiles;
use super::tests::load_text;
use super::*;
use oag_display::display::ShadowResolution;

const PULSE: &str = "Wipeout Pulse (PSP)";

#[test]
fn a_profile_defaults_to_the_originals_sharpness() {
    assert_eq!(
        RenderProfile::default().shadow_resolution,
        ShadowResolution::Original
    );
    for (title, _) in known_profiles() {
        assert_eq!(
            RenderProfile::for_title(title).shadow_resolution,
            ShadowResolution::Original,
            "{}",
            title.name
        );
    }
}

#[test]
fn every_shadow_resolution_round_trips_through_the_settings_file() {
    for resolution in ShadowResolution::ALL {
        let text = format!("[render_profiles.\"{PULSE}\"]\nshadow_resolution = \"{resolution}\"\n");
        let (loaded, _) = load_text(&text);
        assert_eq!(loaded.render_profiles[PULSE].shadow_resolution, resolution);
        let written = render(&loaded).expect("renders");
        assert!(
            written.contains(&format!("shadow_resolution = \"{resolution}\"")),
            "{resolution} is written back"
        );
        let (again, _) = load_text(&written);
        assert_eq!(again.render_profiles[PULSE].shadow_resolution, resolution);
    }
}

#[test]
fn the_menu_seed_carries_the_stored_shadow_resolution() {
    let (title, platform) = known_profiles().next().expect("a title is known");
    let mut settings = Settings::default();
    settings
        .render_profiles
        .entry(profile_key(title, platform))
        .or_default()
        .shadow_resolution = ShadowResolution::Ultra;
    let seeds = menu_seeds(&settings, Anisotropy::default(), title, platform);
    let seeded = seeds
        .iter()
        .find(|(key, _)| *key == "graphics.shadow_resolution")
        .expect("the SHADOW RESOLUTION row is seeded on every title");
    assert_eq!(seeded.1, oag_ui::menu::Value::Text("ultra".to_string()));
}
