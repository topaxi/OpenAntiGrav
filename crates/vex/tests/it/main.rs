//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-vex <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here.

#[path = "../wad_walk/mod.rs"]
mod wad_walk;

#[path = "../alpha_test_reference_ground_truth.rs"]
mod alpha_test_reference_ground_truth;
#[path = "../batch_position_gap_ground_truth.rs"]
mod batch_position_gap_ground_truth;
#[path = "../cloud_ground_truth.rs"]
mod cloud_ground_truth;
#[path = "../collision_ground_truth.rs"]
mod collision_ground_truth;
#[path = "../grid_camera_ground_truth.rs"]
mod grid_camera_ground_truth;
#[path = "../kdcol_ground_truth.rs"]
mod kdcol_ground_truth;
#[path = "../lane_band_census_ground_truth.rs"]
mod lane_band_census_ground_truth;
#[path = "../light_scale_ground_truth.rs"]
mod light_scale_ground_truth;
#[path = "../lighting_ground_truth.rs"]
mod lighting_ground_truth;
#[path = "../pads_ground_truth.rs"]
mod pads_ground_truth;
#[path = "../payload_coverage_ground_truth.rs"]
mod payload_coverage_ground_truth;
#[path = "../placed_psys_ground_truth.rs"]
mod placed_psys_ground_truth;
#[path = "../pvs_ground_truth.rs"]
mod pvs_ground_truth;
#[path = "../quake_ground_truth.rs"]
mod quake_ground_truth;
#[path = "../shadow_occluder_ground_truth.rs"]
mod shadow_occluder_ground_truth;
#[path = "../shadow_occluder_records.rs"]
mod shadow_occluder_records;
#[path = "../shield_texture_swizzle_ground_truth.rs"]
mod shield_texture_swizzle_ground_truth;
#[path = "../skycube_ground_truth.rs"]
mod skycube_ground_truth;
#[path = "../sound_emitter_ground_truth.rs"]
mod sound_emitter_ground_truth;
#[path = "../start_gantry_ground_truth.rs"]
mod start_gantry_ground_truth;
#[path = "../texture_stride_ground_truth.rs"]
mod texture_stride_ground_truth;
#[path = "../texture_swizzle_flag_ground_truth.rs"]
mod texture_swizzle_flag_ground_truth;
#[path = "../track_ground_truth.rs"]
mod track_ground_truth;
#[path = "../vex_batch_layout_ground_truth.rs"]
mod vex_batch_layout_ground_truth;
#[path = "../vex_class_ground_truth.rs"]
mod vex_class_ground_truth;
#[path = "../vex_layer_ground_truth.rs"]
mod vex_layer_ground_truth;
#[path = "../vex_ps2_ground_truth.rs"]
mod vex_ps2_ground_truth;
#[path = "../weather_ground_truth.rs"]
mod weather_ground_truth;

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
