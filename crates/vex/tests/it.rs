//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-vex <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here, and
//! `every_test_file_is_listed` below fails until it is.

mod wad_walk;

mod alpha_test_reference_ground_truth;
mod batch_position_gap_ground_truth;
mod cloud_ground_truth;
mod collision_ground_truth;
mod grid_camera_ground_truth;
mod kdcol_ground_truth;
mod lane_band_census_ground_truth;
mod light_scale_ground_truth;
mod lighting_ground_truth;
mod mesh_depth_point_ground_truth;
mod pads_ground_truth;
mod payload_coverage_ground_truth;
mod placed_psys_ground_truth;
mod pvs_ground_truth;
mod quake_ground_truth;
mod shadow_occluder_ground_truth;
mod shadow_occluder_records;
mod shield_texture_swizzle_ground_truth;
mod skycube_ground_truth;
mod sound_emitter_ground_truth;
mod start_gantry_ground_truth;
mod texture_stride_ground_truth;
mod texture_swizzle_flag_ground_truth;
mod track_ground_truth;
mod vex_batch_layout_ground_truth;
mod vex_class_ground_truth;
mod vex_layer_ground_truth;
mod vex_ps2_ground_truth;
mod weather_ground_truth;

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
