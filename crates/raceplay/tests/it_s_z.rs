//! The integration tests of this crate whose file name starts with `s` to
//! `z`, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. `Cargo.toml` sets `autotests = false`
//! and lists one `[[test]]` per first-letter range instead: few enough links
//! to keep that saving, and enough binaries that a cold build compiles them in
//! parallel and an edit to one test file recompiles only its range. A test
//! keeps its name behind its file's module: `cargo nextest run -p oag-raceplay
//! <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed in the range its first
//! letter falls in, and `every_test_file_is_listed` below fails until it is.

mod perturb;

mod scenery_fx_ground_truth;
mod sfx_repeating_ground_truth;
mod sfx_timeline_ground_truth;
mod sfx_xfade_2048_ground_truth;
mod sfx_xfade_ground_truth;
mod shadow_ground_truth;
mod shadow_stability_ground_truth;
mod ship_spawn_ground_truth;
mod shuriken_ground_truth;
mod speed_plan_ground_truth;
mod stall_rescue_ground_truth;
mod start_gantry_clock_inherit_ground_truth;
mod start_gantry_go_hold_ground_truth;
mod start_gantry_hd_laps_ground_truth;
mod start_gantry_hd_mount_ground_truth;
mod start_gantry_omega_ground_truth;
mod start_gantry_report_ground_truth;
mod start_gantry_report_pure_ground_truth;
mod start_line_ground_truth;
mod start_pose_ground_truth;
#[cfg(feature = "ballisticng")]
mod survey;
mod texture_stream_ground_truth;
mod track_audio_hd_ground_truth;
mod track_props_ground_truth;
mod trackfile_ground_truth;
mod vita_2048_campaign_grid_ground_truth;
mod vita_2048_countdown_ground_truth;
mod vita_2048_hud_event_ground_truth;
mod vita_2048_lighting_ground_truth;
mod vita_2048_original_ai_ground_truth;
mod vita_2048_zone_scenery_ground_truth;
mod wall_collision_ground_truth;
mod wall_penalty_ground_truth;
mod weapon_floor_sweep;
mod weapon_slowdown_ground_truth;
mod weapons_row_ground_truth;
mod weather_ground_truth;
mod weather_splash_ground_truth;
mod web_picker_table;
mod window_flyby_ground_truth;
mod zone_announcer_ground_truth;
mod zone_craft_ground_truth;
mod zone_grade_ground_truth;
mod zone_ground_truth;
mod zone_hover_sweep;
mod zone_sky_ground_truth;
mod zone_start_ground_truth;

/// A `tests/*.rs` file missing from every `tests/it*.rs` listing compiles into
/// nothing, and its tests stop running without a word.
#[test]
fn every_test_file_is_listed() {
    let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let files: Vec<(String, String)> = std::fs::read_dir(&tests)
        .expect("tests/ is readable")
        .map(|entry| entry.expect("tests/ entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .map(|path| {
            let stem = path.file_stem().and_then(|f| f.to_str()).expect("UTF-8");
            let text = std::fs::read_to_string(&path).expect("test file is readable");
            (stem.to_string(), text)
        })
        .collect();
    let listings: Vec<&str> = files
        .iter()
        .filter(|(_, text)| text.contains("fn every_test_file_is_listed()"))
        .map(|(_, text)| text.as_str())
        .collect();
    for (stem, text) in &files {
        if text.contains("fn every_test_file_is_listed()") {
            continue;
        }
        let line = format!("\nmod {stem};\n");
        let listed: usize = listings.iter().map(|l| l.matches(&line).count()).sum();
        assert_eq!(
            listed, 1,
            "tests/{stem}.rs is listed {listed} times in tests/it*.rs, not once"
        );
    }
}
