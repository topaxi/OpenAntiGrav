//! The integration tests of this crate whose file name starts with `p` to
//! `r`, linked into one binary.
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

mod pad_join_end_to_end;
mod pause_resume_ground_truth;
mod picker_flaps_ground_truth;
mod picker_glyphs_ground_truth;
mod picker_pitch_ground_truth;
mod plasma_cues_audio_ground_truth;
mod pose_hold_ground_truth;
mod pose_lean_ground_truth;
mod ps2_glow_mask_ground_truth;
mod ps2_ship_skin_ground_truth;
mod ps2_source_ground_truth;
mod pulse_menu_ticker_ground_truth;
mod pulse_settings_ground_truth;
mod pure_boot_ground_truth;
mod pure_chain_walk_ground_truth;
mod pure_dlc_ground_truth;
mod pure_menu_art_ground_truth;
mod pure_music_ground_truth;
mod pure_original_ai_ground_truth;
mod pure_sign_glow_ground_truth;
mod race_end_photo_load_ground_truth;
mod race_ground_truth;
mod race_remix_ground_truth;
mod record_readout_ground_truth;
mod render_bench_ground_truth;
mod resume_countdown_ground_truth;
mod roster_declared_ground_truth;

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
