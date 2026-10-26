//! The integration tests of this crate whose file name starts with `a` to
//! `g`, linked into one binary.
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

mod add_on_circuits_ground_truth;
mod field_capture_ground_truth;
mod in_parallel;
mod perturb;

mod absorb_ground_truth;
mod ai_clean_lap_board;
mod ai_clean_lap_gate;
mod ai_contact_where;
mod ai_dekonstruct_black_board;
mod ai_dekonstruct_black_ground_truth;
mod ai_dekonstruct_symptoms_ground_truth;
mod ai_fork_split_ground_truth;
mod ai_look_sweep;
mod ai_loss_attribution_board;
mod ai_plan_probe;
mod ai_roll_ground_truth;
mod ai_span_sweep;
mod airtime_budget_ground_truth;
mod asset_layers_ground_truth;
mod autopilot_cancel_ground_truth;
mod autopilot_original_ground_truth;
mod between_ticks_ground_truth;
mod billboard_adverts_ground_truth;
mod boost_plume_ground_truth;
mod branch_progress_ground_truth;
mod cage_effect_sweep;
mod campaign_skill_scale_ground_truth;
mod cannon_ground_truth;
mod cannon_tilt_ground_truth;
mod chase_camera_ground_truth;
mod circuit_length_ground_truth;
mod cockpit_hit_noise_ground_truth;
mod countdown_voice_hd_ground_truth;
mod craft_sticking_ground_truth;
mod damage_fx_ground_truth;
mod destroy_camera_ground_truth;
mod difficulty_ground_truth;
mod eliminator_finish_ground_truth;
mod eliminator_pads_ground_truth;
mod falloff_survey_ground_truth;
mod fire_law_inherit_ground_truth;
mod font_border_ground_truth;
mod foreign_conventions_ground_truth;
mod foreign_ground_truth;
mod foreign_race_ground_truth;
mod gantry_mount_hd_circuits_ground_truth;
mod german_ground_truth;
mod grid_stagger_ground_truth;
mod grid_walk_ground_truth;

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
