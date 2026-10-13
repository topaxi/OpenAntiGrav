//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-rcs <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here, and
//! `every_test_file_is_listed` below fails until it is.

#[path = "../rcsmodel_common/mod.rs"]
mod rcsmodel_common;

#[path = "../coverage_ground_truth.rs"]
mod coverage_ground_truth;
#[path = "../gxp_ground_truth.rs"]
mod gxp_ground_truth;
#[path = "../hd_gantry_curve_ground_truth.rs"]
mod hd_gantry_curve_ground_truth;
#[path = "../hd_lineage_blend_ground_truth.rs"]
mod hd_lineage_blend_ground_truth;
#[path = "../hd_pvs_ground_truth.rs"]
mod hd_pvs_ground_truth;
#[path = "../magstrip_arc_fp_ground_truth.rs"]
mod magstrip_arc_fp_ground_truth;
#[path = "../omega_animation_ground_truth.rs"]
mod omega_animation_ground_truth;
#[path = "../omega_declaration_ground_truth.rs"]
mod omega_declaration_ground_truth;
#[path = "../omega_material_param_ground_truth.rs"]
mod omega_material_param_ground_truth;
#[path = "../omega_nodes_ground_truth.rs"]
mod omega_nodes_ground_truth;
#[path = "../points2_ground_truth.rs"]
mod points2_ground_truth;
#[path = "../psp2_animation_ground_truth.rs"]
mod psp2_animation_ground_truth;
#[path = "../psp2_engine_trail_ground_truth.rs"]
mod psp2_engine_trail_ground_truth;
#[path = "../psp2_material_param_ground_truth.rs"]
mod psp2_material_param_ground_truth;
#[path = "../psp2_pvs_ground_truth.rs"]
mod psp2_pvs_ground_truth;
#[path = "../psp2_rcsmodel_ground_truth.rs"]
mod psp2_rcsmodel_ground_truth;
#[path = "../psp2_rcsmodel_material_ground_truth.rs"]
mod psp2_rcsmodel_material_ground_truth;
#[path = "../rcsmaterial_ground_truth.rs"]
mod rcsmaterial_ground_truth;
#[path = "../rcsmodel_decl_ground_truth.rs"]
mod rcsmodel_decl_ground_truth;
#[path = "../rcsmodel_ground_truth.rs"]
mod rcsmodel_ground_truth;
#[path = "../rcsmodel_material_ground_truth.rs"]
mod rcsmodel_material_ground_truth;
#[path = "../rcsmodel_stride_ground_truth.rs"]
mod rcsmodel_stride_ground_truth;
#[path = "../rcsmodel_surface_ground_truth.rs"]
mod rcsmodel_surface_ground_truth;
#[path = "../rcsmodel_vertex_ground_truth.rs"]
mod rcsmodel_vertex_ground_truth;
#[path = "../specular_power_ground_truth.rs"]
mod specular_power_ground_truth;
#[path = "../start_gantry_2048_ground_truth.rs"]
mod start_gantry_2048_ground_truth;
#[path = "../zone_shader_census_ground_truth.rs"]
mod zone_shader_census_ground_truth;

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
