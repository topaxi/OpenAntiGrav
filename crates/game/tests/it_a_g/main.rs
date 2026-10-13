//! The integration tests of this crate whose file name starts with `a` to
//! `g`, linked into one binary.
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

#[path = "../in_parallel/mod.rs"]
mod in_parallel;

#[path = "../absorb_ground_truth.rs"]
mod absorb_ground_truth;
#[path = "../absorb_mask_ground_truth.rs"]
mod absorb_mask_ground_truth;
#[path = "../ai_clean_lap_board.rs"]
mod ai_clean_lap_board;
#[path = "../ai_clean_lap_gate.rs"]
mod ai_clean_lap_gate;
#[path = "../ai_dekonstruct_black_board.rs"]
mod ai_dekonstruct_black_board;
#[path = "../ai_dekonstruct_black_ground_truth.rs"]
mod ai_dekonstruct_black_ground_truth;
#[path = "../ai_dekonstruct_symptoms_ground_truth.rs"]
mod ai_dekonstruct_symptoms_ground_truth;
#[path = "../ai_fork_split_ground_truth.rs"]
mod ai_fork_split_ground_truth;
#[path = "../ai_look_sweep.rs"]
mod ai_look_sweep;
#[path = "../ai_loss_attribution_board.rs"]
mod ai_loss_attribution_board;
#[path = "../ai_plan_probe.rs"]
mod ai_plan_probe;
#[path = "../ai_roll_ground_truth.rs"]
mod ai_roll_ground_truth;
#[path = "../ai_span_sweep.rs"]
mod ai_span_sweep;
#[path = "../ai_speed_plan_sweep.rs"]
mod ai_speed_plan_sweep;
#[path = "../airtime_budget_ground_truth.rs"]
mod airtime_budget_ground_truth;
#[path = "../asset_layers_ground_truth.rs"]
mod asset_layers_ground_truth;
#[path = "../billboard_adverts_ground_truth.rs"]
mod billboard_adverts_ground_truth;
#[path = "../bomb_orbit_ground_truth.rs"]
mod bomb_orbit_ground_truth;
#[path = "../boost_plume_ground_truth.rs"]
mod boost_plume_ground_truth;
#[path = "../boot_ground_truth.rs"]
mod boot_ground_truth;
#[path = "../boot_legal_wrap_ground_truth.rs"]
mod boot_legal_wrap_ground_truth;
#[path = "../branch_progress_ground_truth.rs"]
mod branch_progress_ground_truth;
#[path = "../campaign_cell_class_census_ground_truth.rs"]
mod campaign_cell_class_census_ground_truth;
#[path = "../campaign_medal.rs"]
mod campaign_medal;
#[path = "../campaign_pointer_ground_truth.rs"]
mod campaign_pointer_ground_truth;
#[path = "../campaign_skill_scale_ground_truth.rs"]
mod campaign_skill_scale_ground_truth;
#[path = "../cannon_ground_truth.rs"]
mod cannon_ground_truth;
#[path = "../cannon_tilt_ground_truth.rs"]
mod cannon_tilt_ground_truth;
#[path = "../chase_camera_ground_truth.rs"]
mod chase_camera_ground_truth;
#[path = "../circuit_length_ground_truth.rs"]
mod circuit_length_ground_truth;
#[path = "../countdown_voice_ground_truth.rs"]
mod countdown_voice_ground_truth;
#[path = "../craft_sticking_ground_truth.rs"]
mod craft_sticking_ground_truth;
#[path = "../damage_fx_ground_truth.rs"]
mod damage_fx_ground_truth;
#[path = "../destroy_camera_ground_truth.rs"]
mod destroy_camera_ground_truth;
#[path = "../difficulty_ground_truth.rs"]
mod difficulty_ground_truth;
#[path = "../dlc_ground_truth.rs"]
mod dlc_ground_truth;
#[path = "../eliminator_ai_stats_ground_truth.rs"]
mod eliminator_ai_stats_ground_truth;
#[path = "../eliminator_finish_ground_truth.rs"]
mod eliminator_finish_ground_truth;
#[path = "../eliminator_pads_ground_truth.rs"]
mod eliminator_pads_ground_truth;
#[path = "../endrace_trophy_ground_truth.rs"]
mod endrace_trophy_ground_truth;
#[path = "../endrace_voice_ground_truth.rs"]
mod endrace_voice_ground_truth;
#[path = "../falloff_survey_ground_truth.rs"]
mod falloff_survey_ground_truth;
#[path = "../fire_law_inherit_ground_truth.rs"]
mod fire_law_inherit_ground_truth;
#[path = "../font_border_ground_truth.rs"]
mod font_border_ground_truth;
#[path = "../font_roles_ground_truth.rs"]
mod font_roles_ground_truth;
#[path = "../foreign_conventions_ground_truth.rs"]
mod foreign_conventions_ground_truth;
#[path = "../foreign_race_ground_truth.rs"]
mod foreign_race_ground_truth;
#[path = "../frontend_grid_ground_truth.rs"]
mod frontend_grid_ground_truth;
#[path = "../gantry_mount_hd_circuits_ground_truth.rs"]
mod gantry_mount_hd_circuits_ground_truth;
#[path = "../german_ground_truth.rs"]
mod german_ground_truth;
#[path = "../glow_stamp_ground_truth.rs"]
mod glow_stamp_ground_truth;
#[path = "../grid_stagger_ground_truth.rs"]
mod grid_stagger_ground_truth;
#[path = "../grid_walk_ground_truth.rs"]
mod grid_walk_ground_truth;

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
