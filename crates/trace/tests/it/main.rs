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

#[path = "../airtime_arc_ground_truth.rs"]
mod airtime_arc_ground_truth;
#[path = "../camera_lean_ground_truth.rs"]
mod camera_lean_ground_truth;
#[path = "../force_term_pose_walk_ground_truth.rs"]
mod force_term_pose_walk_ground_truth;
#[path = "../hover_contact_ground_truth.rs"]
mod hover_contact_ground_truth;
#[path = "../lap_capture_ground_truth.rs"]
mod lap_capture_ground_truth;
#[path = "../lap_window_ground_truth.rs"]
mod lap_window_ground_truth;
#[path = "../pad_crossing_ground_truth.rs"]
mod pad_crossing_ground_truth;
#[path = "../pads_ground_truth.rs"]
mod pads_ground_truth;
#[path = "../ride_height_ground_truth.rs"]
mod ride_height_ground_truth;
#[path = "../shove_wall_pose_walk_ground_truth.rs"]
mod shove_wall_pose_walk_ground_truth;
#[path = "../steady_state_vertical_ground_truth.rs"]
mod steady_state_vertical_ground_truth;
#[path = "../wall_contact_ground_truth.rs"]
mod wall_contact_ground_truth;
#[path = "../yaw_authority_ground_truth.rs"]
mod yaw_authority_ground_truth;

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
