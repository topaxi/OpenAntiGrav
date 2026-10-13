//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-assets <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here.

#[path = "../hd_psarc_ground_truth.rs"]
mod hd_psarc_ground_truth;
#[path = "../omega_psarc_ground_truth.rs"]
mod omega_psarc_ground_truth;
#[path = "../pmf_ground_truth.rs"]
mod pmf_ground_truth;
#[path = "../ps2_font_ground_truth.rs"]
mod ps2_font_ground_truth;
#[path = "../ps2_image_ground_truth.rs"]
mod ps2_image_ground_truth;
#[path = "../pulse_title_ground_truth.rs"]
mod pulse_title_ground_truth;

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
