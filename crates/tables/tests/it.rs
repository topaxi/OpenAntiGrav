//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-tables <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here, and
//! `every_test_file_is_listed` below fails until it is.

mod common;

mod ai_control_stats_ground_truth;
mod ai_race_full_ground_truth;
mod ai_race_stats_ground_truth;
mod ai_stats_lineage_ground_truth;
mod effectsettings_ground_truth;
mod enginelight_ground_truth;
mod envsettings_ground_truth;
mod envsettings_nova;
mod fury_backdrop_ground_truth;
mod handling_ground_truth;
mod hd_weapons_ground_truth;
mod pure_campaign_ground_truth;
mod pure_flow_ground_truth;
mod pure_weapons_ground_truth;
mod race_campaign_ground_truth;
mod speed_ladder_ground_truth;
mod start_gantry_pure_ground_truth;
mod track_stats_ground_truth;
mod trackstartup_ground_truth;
mod weapon_ai_ground_truth;
mod weapons_ground_truth;

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
