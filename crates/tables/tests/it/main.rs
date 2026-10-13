//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-tables <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here.

#[path = "../common/mod.rs"]
mod common;

#[path = "../ai_control_stats_ground_truth.rs"]
mod ai_control_stats_ground_truth;
#[path = "../ai_race_full_ground_truth.rs"]
mod ai_race_full_ground_truth;
#[path = "../ai_race_stats_ground_truth.rs"]
mod ai_race_stats_ground_truth;
#[path = "../ai_stats_lineage_ground_truth.rs"]
mod ai_stats_lineage_ground_truth;
#[path = "../effectsettings_ground_truth.rs"]
mod effectsettings_ground_truth;
#[path = "../enginelight_ground_truth.rs"]
mod enginelight_ground_truth;
#[path = "../envsettings_ground_truth.rs"]
mod envsettings_ground_truth;
#[path = "../envsettings_nova.rs"]
mod envsettings_nova;
#[path = "../fury_backdrop_ground_truth.rs"]
mod fury_backdrop_ground_truth;
#[path = "../handling_ground_truth.rs"]
mod handling_ground_truth;
#[path = "../hd_weapons_ground_truth.rs"]
mod hd_weapons_ground_truth;
#[path = "../pure_campaign_ground_truth.rs"]
mod pure_campaign_ground_truth;
#[path = "../pure_flow_ground_truth.rs"]
mod pure_flow_ground_truth;
#[path = "../pure_weapons_ground_truth.rs"]
mod pure_weapons_ground_truth;
#[path = "../race_campaign_ground_truth.rs"]
mod race_campaign_ground_truth;
#[path = "../speed_ladder_ground_truth.rs"]
mod speed_ladder_ground_truth;
#[path = "../start_gantry_pure_ground_truth.rs"]
mod start_gantry_pure_ground_truth;
#[path = "../track_stats_ground_truth.rs"]
mod track_stats_ground_truth;
#[path = "../trackstartup_ground_truth.rs"]
mod trackstartup_ground_truth;
#[path = "../weapon_ai_ground_truth.rs"]
mod weapon_ai_ground_truth;
#[path = "../weapons_ground_truth.rs"]
mod weapons_ground_truth;

/// A `tests/*.rs` file missing from the list above compiles into nothing, and
/// its tests stop running without a word.
#[test]
fn every_test_file_is_listed() {
    let listing = include_str!("main.rs");
    let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for entry in std::fs::read_dir(&tests).expect("tests/ is readable") {
        let path = entry.expect("tests/ entry").path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let file = path.file_name().and_then(|f| f.to_str()).expect("UTF-8");
            assert!(
                listing.contains(&format!("#[path = \"../{file}\"]")),
                "tests/{file} is not listed in tests/it/main.rs"
            );
        }
    }
}
