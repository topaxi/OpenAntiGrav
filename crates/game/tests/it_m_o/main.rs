//! The integration tests of this crate whose file name starts with `m` to
//! `o`, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. `Cargo.toml` sets `autotests = false`
//! and lists one `[[test]]` per first-letter range instead: few enough links
//! to keep that saving, and enough binaries that a cold build compiles them in
//! parallel and an edit to one test file recompiles only its range. A test
//! keeps its name behind its file's module: `cargo nextest run -p oag-game
//! <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed in the range its first
//! letter falls in, and `every_test_file_is_listed` below fails until it is.

#[path = "../mag_floor_scroll_ground_truth.rs"]
mod mag_floor_scroll_ground_truth;
#[path = "../maglock_ground_truth.rs"]
mod maglock_ground_truth;
#[path = "../magstrip_anchor_ground_truth.rs"]
mod magstrip_anchor_ground_truth;
#[path = "../magstrip_wire_2048_ground_truth.rs"]
mod magstrip_wire_2048_ground_truth;
#[path = "../magstrip_wire_hd_ground_truth.rs"]
mod magstrip_wire_hd_ground_truth;
#[path = "../main_menu_ground_truth.rs"]
mod main_menu_ground_truth;
#[path = "../medal_message_ground_truth.rs"]
mod medal_message_ground_truth;
#[path = "../menu_definition_settings.rs"]
mod menu_definition_settings;
#[path = "../menu_disabled_and_restart.rs"]
mod menu_disabled_and_restart;
#[path = "../menu_remix_ai_ground_truth.rs"]
mod menu_remix_ai_ground_truth;
#[path = "../menu_screen_filter.rs"]
mod menu_screen_filter;
#[path = "../menu_sfx_ground_truth.rs"]
mod menu_sfx_ground_truth;
#[path = "../menu_warnings_settings.rs"]
mod menu_warnings_settings;
#[path = "../menu_web_rows.rs"]
mod menu_web_rows;
#[path = "../mine_ground_truth.rs"]
mod mine_ground_truth;
#[path = "../mine_launch_audio_ground_truth.rs"]
mod mine_launch_audio_ground_truth;
#[path = "../mine_texture_scroll_ground_truth.rs"]
mod mine_texture_scroll_ground_truth;
#[path = "../missile_ground_truth.rs"]
mod missile_ground_truth;
#[path = "../mods_menu.rs"]
mod mods_menu;
#[path = "../mods_page.rs"]
mod mods_page;
#[path = "../movie_access_units_ground_truth.rs"]
mod movie_access_units_ground_truth;
#[path = "../movie_container_audio_ground_truth.rs"]
mod movie_container_audio_ground_truth;
#[path = "../movie_ground_truth.rs"]
mod movie_ground_truth;
#[path = "../mp4_movie_ground_truth.rs"]
mod mp4_movie_ground_truth;
#[path = "../music_resume_ground_truth.rs"]
mod music_resume_ground_truth;
#[path = "../name_voice_ground_truth.rs"]
mod name_voice_ground_truth;
#[path = "../novice_jump_ground_truth.rs"]
mod novice_jump_ground_truth;
#[path = "../novice_respawn_ground_truth.rs"]
mod novice_respawn_ground_truth;
#[path = "../off_track_rescue_ground_truth.rs"]
mod off_track_rescue_ground_truth;
#[path = "../omega_2048_era_craft_ground_truth.rs"]
mod omega_2048_era_craft_ground_truth;
#[path = "../omega_assist_hud_ground_truth.rs"]
mod omega_assist_hud_ground_truth;
#[path = "../omega_campaign_launch_ground_truth.rs"]
mod omega_campaign_launch_ground_truth;
#[path = "../omega_campaign_screens_ground_truth.rs"]
mod omega_campaign_screens_ground_truth;
#[path = "../omega_font_scale_ground_truth.rs"]
mod omega_font_scale_ground_truth;
#[path = "../omega_hud_ground_truth.rs"]
mod omega_hud_ground_truth;
#[path = "../omega_lightmap_ground_truth.rs"]
mod omega_lightmap_ground_truth;
#[path = "../omega_menu_backdrop_ground_truth.rs"]
mod omega_menu_backdrop_ground_truth;
#[path = "../omega_menu_blocks_ground_truth.rs"]
mod omega_menu_blocks_ground_truth;
#[path = "../omega_music_ground_truth.rs"]
mod omega_music_ground_truth;
#[path = "../omega_original_ai_ground_truth.rs"]
mod omega_original_ai_ground_truth;
#[path = "../omega_pvs_placement_ground_truth.rs"]
mod omega_pvs_placement_ground_truth;
#[path = "../omega_race_ground_truth.rs"]
mod omega_race_ground_truth;
#[path = "../omega_track_model_ground_truth.rs"]
mod omega_track_model_ground_truth;
#[path = "../omega_wem_ground_truth.rs"]
mod omega_wem_ground_truth;
#[path = "../opponent_fire_ground_truth.rs"]
mod opponent_fire_ground_truth;
#[path = "../opponent_weapons_ground_truth.rs"]
mod opponent_weapons_ground_truth;
#[path = "../original_ai_ground_truth.rs"]
mod original_ai_ground_truth;

/// A `tests/*.rs` file missing from every `tests/it*/main.rs` compiles into
/// nothing, and its tests stop running without a word.
#[test]
fn every_test_file_is_listed() {
    let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut listings = String::new();
    for entry in std::fs::read_dir(&tests).expect("tests/ is readable") {
        let path = entry.expect("tests/ entry").path();
        let name = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or_default();
        if path.is_dir() && name.starts_with("it") {
            listings += &std::fs::read_to_string(path.join("main.rs")).unwrap_or_default();
        }
    }
    for entry in std::fs::read_dir(&tests).expect("tests/ is readable") {
        let path = entry.expect("tests/ entry").path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let file = path.file_name().and_then(|f| f.to_str()).expect("UTF-8");
            let listed = listings
                .matches(&format!("#[path = \"../{file}\"]"))
                .count();
            assert_eq!(
                listed, 1,
                "tests/{file} is listed {listed} times in tests/it*/main.rs, not once"
            );
        }
    }
}
