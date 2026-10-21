//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-trace <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here, and
//! `every_test_file_is_listed` below fails until it is.

mod airtime_arc_ground_truth;
mod autopilot_ramp_pose_walk;
mod camera_lean_ground_truth;
mod collision_cage_ground_truth;
mod crest_pitch_pose_walk;
mod force_term_pose_walk_ground_truth;
mod height_miss_census;
mod hover_contact_ground_truth;
mod lap_capture_ground_truth;
mod lap_window_ground_truth;
mod pad_crossing_ground_truth;
mod pads_ground_truth;
mod ride_height_ground_truth;
mod shove_wall_pose_walk_ground_truth;
mod steady_state_vertical_ground_truth;
mod steer_lean_ground_truth;
mod wall_angular_pose_walk_ground_truth;
mod wall_contact_ground_truth;
mod yaw_authority_ground_truth;

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
