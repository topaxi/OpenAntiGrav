//! The integration tests of this crate whose file name starts with `s` to
//! `z`, linked into one binary.
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

#[path = "../scenery_fx_ground_truth.rs"]
mod scenery_fx_ground_truth;
#[path = "../selection_screens_ground_truth.rs"]
mod selection_screens_ground_truth;
#[path = "../sfx_ground_truth.rs"]
mod sfx_ground_truth;
#[path = "../sfx_repeating_ground_truth.rs"]
mod sfx_repeating_ground_truth;
#[path = "../sfx_timeline_ground_truth.rs"]
mod sfx_timeline_ground_truth;
#[path = "../sfx_weapon_ground_truth.rs"]
mod sfx_weapon_ground_truth;
#[path = "../sfx_xfade_2048_ground_truth.rs"]
mod sfx_xfade_2048_ground_truth;
#[path = "../sfx_xfade_ground_truth.rs"]
mod sfx_xfade_ground_truth;
#[path = "../shadow_ground_truth.rs"]
mod shadow_ground_truth;
#[path = "../shadow_stability_ground_truth.rs"]
mod shadow_stability_ground_truth;
#[path = "../shake_blur_ground_truth.rs"]
mod shake_blur_ground_truth;
#[path = "../shield_shell_scroll_ground_truth.rs"]
mod shield_shell_scroll_ground_truth;
#[path = "../shine_ground_truth.rs"]
mod shine_ground_truth;
#[path = "../ship_spawn_ground_truth.rs"]
mod ship_spawn_ground_truth;
#[path = "../shuriken_ground_truth.rs"]
mod shuriken_ground_truth;
#[path = "../spawn_heading_ground_truth.rs"]
mod spawn_heading_ground_truth;
#[path = "../speed_plan_ground_truth.rs"]
mod speed_plan_ground_truth;
#[path = "../split_end_race_ground_truth.rs"]
mod split_end_race_ground_truth;
#[path = "../split_hud_ground_truth.rs"]
mod split_hud_ground_truth;
#[path = "../split_screen_ground_truth.rs"]
mod split_screen_ground_truth;
#[path = "../stall_rescue_ground_truth.rs"]
mod stall_rescue_ground_truth;
#[path = "../start_gantry_clock_inherit_ground_truth.rs"]
mod start_gantry_clock_inherit_ground_truth;
#[path = "../start_gantry_go_hold_ground_truth.rs"]
mod start_gantry_go_hold_ground_truth;
#[path = "../start_gantry_hd_laps_ground_truth.rs"]
mod start_gantry_hd_laps_ground_truth;
#[path = "../start_gantry_hd_mount_ground_truth.rs"]
mod start_gantry_hd_mount_ground_truth;
#[path = "../start_gantry_report_ground_truth.rs"]
mod start_gantry_report_ground_truth;
#[path = "../start_gantry_report_pure_ground_truth.rs"]
mod start_gantry_report_pure_ground_truth;
#[path = "../start_line_ground_truth.rs"]
mod start_line_ground_truth;
#[path = "../start_pose_ground_truth.rs"]
mod start_pose_ground_truth;
#[path = "../texture_detail_ground_truth.rs"]
mod texture_detail_ground_truth;
#[path = "../texture_stream_ground_truth.rs"]
mod texture_stream_ground_truth;
#[path = "../title_behaviour_data.rs"]
mod title_behaviour_data;
#[path = "../track_audio_2048_ground_truth.rs"]
mod track_audio_2048_ground_truth;
#[path = "../track_audio_ground_truth.rs"]
mod track_audio_ground_truth;
#[path = "../track_audio_hd_ground_truth.rs"]
mod track_audio_hd_ground_truth;
#[path = "../track_panel_ground_truth.rs"]
mod track_panel_ground_truth;
#[path = "../unlock_ground_truth.rs"]
mod unlock_ground_truth;
#[path = "../variant_unlock_ground_truth.rs"]
mod variant_unlock_ground_truth;
#[path = "../vita_2048_boot_ground_truth.rs"]
mod vita_2048_boot_ground_truth;
#[path = "../vita_2048_campaign_grid_ground_truth.rs"]
mod vita_2048_campaign_grid_ground_truth;
#[path = "../vita_2048_campaign_progress_ground_truth.rs"]
mod vita_2048_campaign_progress_ground_truth;
#[path = "../vita_2048_endrace_ground_truth.rs"]
mod vita_2048_endrace_ground_truth;
#[path = "../vita_2048_event_card_ground_truth.rs"]
mod vita_2048_event_card_ground_truth;
#[path = "../vita_2048_hud_ground_truth.rs"]
mod vita_2048_hud_ground_truth;
#[path = "../vita_2048_lighting_ground_truth.rs"]
mod vita_2048_lighting_ground_truth;
#[path = "../vita_2048_music_ground_truth.rs"]
mod vita_2048_music_ground_truth;
#[path = "../vita_2048_original_ai_ground_truth.rs"]
mod vita_2048_original_ai_ground_truth;
#[path = "../vita_2048_zone_scenery_ground_truth.rs"]
mod vita_2048_zone_scenery_ground_truth;
#[path = "../wall_collision_ground_truth.rs"]
mod wall_collision_ground_truth;
#[path = "../weapon_floor_sweep.rs"]
mod weapon_floor_sweep;
#[path = "../weapon_slowdown_ground_truth.rs"]
mod weapon_slowdown_ground_truth;
#[path = "../weapon_stamp_ground_truth.rs"]
mod weapon_stamp_ground_truth;
#[path = "../weapons_row_ground_truth.rs"]
mod weapons_row_ground_truth;
#[path = "../weather_ground_truth.rs"]
mod weather_ground_truth;
#[path = "../web_picker_table.rs"]
mod web_picker_table;
#[path = "../wreck_ground_truth.rs"]
mod wreck_ground_truth;
#[path = "../wreck_voice_ground_truth.rs"]
mod wreck_voice_ground_truth;
#[path = "../zone_airbrake_flaps_ground_truth.rs"]
mod zone_airbrake_flaps_ground_truth;
#[path = "../zone_announcer_ground_truth.rs"]
mod zone_announcer_ground_truth;
#[path = "../zone_craft_ground_truth.rs"]
mod zone_craft_ground_truth;
#[path = "../zone_grade_ground_truth.rs"]
mod zone_grade_ground_truth;
#[path = "../zone_ground_truth.rs"]
mod zone_ground_truth;
#[path = "../zone_sky_ground_truth.rs"]
mod zone_sky_ground_truth;
#[path = "../zone_start_ground_truth.rs"]
mod zone_start_ground_truth;

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
