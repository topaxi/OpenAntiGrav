//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-texture <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here.

#[path = "../fnt_ground_truth.rs"]
mod fnt_ground_truth;
#[path = "../fnt_hd_ground_truth.rs"]
mod fnt_hd_ground_truth;
#[path = "../gtf_ground_truth.rs"]
mod gtf_ground_truth;
#[path = "../gxt_ground_truth.rs"]
mod gxt_ground_truth;
#[path = "../mip_fnt_coverage_ground_truth.rs"]
mod mip_fnt_coverage_ground_truth;
#[path = "../omega_gnf_ground_truth.rs"]
mod omega_gnf_ground_truth;
#[path = "../ps2_texture_ground_truth.rs"]
mod ps2_texture_ground_truth;
#[path = "../ship_skin_ground_truth.rs"]
mod ship_skin_ground_truth;

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
