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

mod selection_screens_ground_truth;
mod sfx_ground_truth;
mod sfx_weapon_ground_truth;
mod shake_blur_ground_truth;
mod shield_shell_scroll_ground_truth;
mod shine_ground_truth;
mod spawn_heading_ground_truth;
mod split_end_race_ground_truth;
mod split_grid_ground_truth;
mod split_head_to_head_ground_truth;
mod split_hud_ground_truth;
mod split_recording_ground_truth;
mod split_screen_ground_truth;
mod texture_detail_ground_truth;
mod title_behaviour_data;
mod track_audio_2048_ground_truth;
mod track_audio_ground_truth;
mod track_panel_ground_truth;
mod unlock_ground_truth;
mod variant_unlock_ground_truth;
mod vita_2048_boot_ground_truth;
mod vita_2048_campaign_progress_ground_truth;
mod vita_2048_endrace_ground_truth;
mod vita_2048_event_card_ground_truth;
mod vita_2048_music_ground_truth;
mod weapon_stamp_ground_truth;
mod window_layout_ground_truth;
mod window_seats_grid_ground_truth;
mod wreck_ground_truth;
mod wreck_voice_ground_truth;
mod zone_airbrake_flaps_ground_truth;

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
