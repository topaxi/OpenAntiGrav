//! `settings.toml` against `schemas/v1/settings.tosd` (`docs/formats/toml-schema.md`): what the
//! game writes is accepted, and a key the schema lacks or a value of the wrong type is named.

use oag_game::settings::{self, Settings};
use oag_schema::validate::accepts;

fn written(settings: &Settings) -> String {
    settings::render(settings).expect("serialises")
}

fn verdict(text: &str) -> Result<(), Vec<String>> {
    accepts(settings::SCHEMA_KIND, 1, text)
}

/// Every table and every optional key set, so a field the schema lacks is written and refused.
fn populated() -> Settings {
    let mut s = settings::fresh();
    s.graphics.hud_safe_area = Some(90);
    s.source.image = Some("/games/pulse.chd".into());
    s.source.dlc = vec!["/games/dlc".into()];
    s.log.file = Some("/tmp/oag.log".into());
    s.log.filter = Some("info".into());
    s.dev.unlock_everything = Some(false);
    s.mods.disabled.insert("oag.example".into());
    s.replay.record = false;
    s.mod_settings
        .entry("oag.example".into())
        .or_default()
        .insert("difficulty".into(), "hard".into());
    s.language = Some("English".into());
    s.audio.music_folders = vec![oag_sound::pool::MusicFolder {
        path: "/music".into(),
        enabled: true,
    }];
    s.audio.music_switches.insert("folder:/music".into(), true);
    s.audio
        .choose_spatial(Some("usb-codec"), oag_audio::SpatialChoice::Headphones);
    s.race.window_monitors = vec!["DP-1".into()];
    s.race.window_panes = vec!["a".into()];
    s.online.address = "example.org:4433".into();
    s
}

#[test]
fn a_fresh_settings_file_validates() {
    let text = written(&settings::fresh());
    verdict(&text).unwrap_or_else(|e| panic!("{e:#?}\n{text}"));
}

#[test]
fn a_settings_file_with_every_section_populated_validates() {
    let text = written(&populated());
    for table in [
        "[mods]",
        "[replay]",
        "[dev]",
        "[log]",
        "[source]",
        "[mod_settings.\"oag.example\"]",
        "[audio.music_switches]",
        "[[audio.music_folders]]",
        "[[audio.spatial_devices]]",
        "hud_safe_area",
        "language",
    ] {
        assert!(text.contains(table), "{table} missing from:\n{text}");
    }
    verdict(&text).unwrap_or_else(|e| panic!("{e:#?}\n{text}"));
}

#[test]
fn the_file_declares_the_schema_it_is_read_as() {
    let text = written(&settings::fresh());
    let table: toml::Table = text.parse().unwrap();
    let header: oag_schema::Header = table[oag_schema::TABLE].clone().try_into().unwrap();
    oag_schema::check(Some(&header), settings::SCHEMA_KIND, 1).unwrap();
    let back: Settings = toml::from_str(&text).expect("the loader reads what it wrote");
    assert_eq!(written(&back), text);
}

#[test]
fn a_file_without_the_header_and_a_minimal_one_validate() {
    verdict("").unwrap();
    verdict("[audio]\nmusic_volume = 50\n").unwrap();
}

#[test]
fn the_web_canvas_size_and_the_older_spellings_validate() {
    verdict("[display]\ncanvas_size = \"fit\"\nvsync = true\n").unwrap();
    verdict("[controls]\npilot_assist = false\n").unwrap();
}

#[test]
fn an_unknown_key_is_named() {
    let errors = verdict("[display]\nbrightnes = 100\n")
        .unwrap_err()
        .join("\n");
    assert!(errors.contains("brightnes"), "{errors}");
    let errors = verdict("[render_profiles.\"X (Y)\"]\nrender_scal = 100\n")
        .unwrap_err()
        .join("\n");
    assert!(errors.contains("render_scal"), "{errors}");
}

#[test]
fn a_wrong_type_is_named() {
    let errors = verdict("[display]\nbrightness = \"bright\"\n")
        .unwrap_err()
        .join("\n");
    assert!(errors.contains("brightness"), "{errors}");
    let errors = verdict("[mod_settings.\"a.b\"]\nk = 3\n")
        .unwrap_err()
        .join("\n");
    assert!(errors.contains('k'), "{errors}");
}

#[test]
fn a_remembered_spatial_audio_choice_survives_a_settings_reload() {
    use oag_audio::SpatialChoice;
    let mut s = settings::fresh();
    s.audio
        .choose_spatial(Some("usb-codec"), SpatialChoice::Headphones);
    let mut back: settings::Settings = toml::from_str(&written(&s)).expect("read");
    back.audio.recall_spatial("usb-codec");
    assert_eq!(back.audio.spatial_audio, SpatialChoice::Headphones);
    back.audio.recall_spatial("speakers");
    assert_eq!(back.audio.spatial_audio, SpatialChoice::Auto);
}
