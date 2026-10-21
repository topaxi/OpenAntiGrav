//! The integration tests of this crate whose file name starts with `h` to
//! `l`, linked into one binary.
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

mod hd_behind_glass_frame_ground_truth;
mod hd_bomb_blast_ground_truth;
mod hd_boot_ground_truth;
mod hd_campaign_ground_truth;
mod hd_cell_select_ground_truth;
mod hd_endrace_ground_truth;
mod hd_flyer_ground_truth;
mod hd_fury_backdrop_ground_truth;
mod hd_fury_ship_select_ground_truth;
mod hd_leach_ball_ground_truth;
mod hd_leach_strip_ground_truth;
mod hd_menu_ground_truth;
mod hd_missile_blast_ground_truth;
mod hd_movie_ground_truth;
mod hd_music_ground_truth;
mod hd_plasma_ball_ground_truth;
mod hd_psn_campaign_ground_truth;
mod hd_rest_flame_ground_truth;
mod hd_ship_preview_ground_truth;
mod hd_sun_occlusion_ground_truth;
mod hd_track_model_ground_truth;
mod hd_weapon_scene_ground_truth;
mod hud_safe_area_gate;
mod hud_scale_gate;
mod ipf_ground_truth;
mod language_reload_ground_truth;
mod lap_messages_ground_truth;
mod launcher_ground_truth;
mod loading_screen_ground_truth;

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
