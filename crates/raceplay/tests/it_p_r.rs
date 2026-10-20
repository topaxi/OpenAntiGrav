//! The integration tests of this crate whose file name starts with `p` to
//! `r`, linked into one binary.
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

mod in_parallel;
mod perturb;
mod pickup_icon_ground_truth;
mod pilot_assist_ground_truth;
mod pilot_assist_levels_ground_truth;
mod plasma_ground_truth;
mod portuguese_br_ground_truth;
mod post_finish_ground_truth;
mod projectile_tilt_ground_truth;
mod prompt_pilots;
mod ps2_hud_sights_ground_truth;
mod ps2_shield_ground_truth;
mod psp2_sky_fog_ground_truth;
mod psys_inventory_ground_truth;
mod pulse_gantry_circuits_ground_truth;
mod pulse_variant_titles_ground_truth;
mod pure_race_ground_truth;
mod quake_orientation_ground_truth;
mod quake_ripple_ground_truth;
mod quiet_replay_ground_truth;
mod race_finish_ground_truth;
mod race_load_read_memo_ground_truth;
mod ram_ground_truth;
mod replay_ground_truth;
mod repulser_fork_ground_truth;
mod repulser_guard_band_ground_truth;
mod reset_zone_ground_truth;
mod reversed_grid_ground_truth;
mod rocket_floor_trace;
mod rocket_launch_ground_truth;

mod pilot_schema;

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
