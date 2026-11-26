//! Every menu seed names a key the written settings file holds.

use super::render_profile::{KNOWN_TITLES, PROFILE_KEYS, known_profiles};
use super::*;

/// Every seed names a setting the file can actually hold, which is what
/// stops a renamed field leaving a menu row showing its list's first
/// option instead of the player's own value.
///
/// The five relocated `graphics.*` rows are checked against every known
/// title's own `[render_profiles.<title>]` table rather than `[graphics]`
/// itself, since that is where `menu_seeds` actually reads and
/// `ensure_known_titles` actually writes them - see [`RenderProfile`].
/// `audio.music_source` is checked the same way against `[music_profiles]`.
#[test]
fn every_menu_seed_names_a_key_the_settings_file_has() {
    let mut settings = Settings::default();
    ensure_known_titles(&mut settings);
    // An unset `hud_safe_area` is not written (the title's own default), so the key is looked
    // for in a file that has one saved, as `language` is only present once picked.
    settings.graphics.hud_safe_area = Some(100);
    let written = toml::to_string_pretty(&settings).expect("serialise");
    let table: toml::Table = written.parse().expect("parse");

    for (setting, _) in menu_seeds(
        &settings,
        Anisotropy::default(),
        oag_pulse::TITLE,
        oag_disc::Platform::Psp,
    ) {
        let Some((section, key)) = setting.split_once('.') else {
            // `language` is a bare key, and only present once picked.
            assert_eq!(setting, "language");
            continue;
        };
        if setting == "audio.music_source" {
            for title in KNOWN_TITLES {
                let profile = table["music_profiles"]
                    .get(title.name)
                    .and_then(toml::Value::as_table)
                    .unwrap_or_else(|| panic!("no [music_profiles.{}]", title.name));
                assert!(
                    profile.contains_key(key),
                    "[music_profiles.{}] has no {key}",
                    title.name
                );
            }
            continue;
        }
        if section == "graphics" && PROFILE_KEYS.contains(&key) {
            let profiles = table
                .get("render_profiles")
                .and_then(toml::Value::as_table)
                .unwrap_or_else(|| panic!("no [render_profiles] table for {setting}"));
            for (title, platform) in known_profiles() {
                let profile_key = profile_key(title, platform);
                let profile = profiles
                    .get(&profile_key)
                    .and_then(toml::Value::as_table)
                    .unwrap_or_else(|| {
                        panic!("no [render_profiles.{profile_key}] table for {setting}")
                    });
                assert!(
                    profile.contains_key(key),
                    "[render_profiles.{profile_key}] has no {key}"
                );
            }
            continue;
        }
        let holder = table
            .get(section)
            .and_then(toml::Value::as_table)
            .unwrap_or_else(|| panic!("no [{section}] table for {setting}"));
        assert!(holder.contains_key(key), "[{section}] has no {key}");
    }
}
