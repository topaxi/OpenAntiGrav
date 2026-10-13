//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-unity <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here.

#[path = "../common/mod.rs"]
mod common;

#[path = "../bundle_ground_truth.rs"]
mod bundle_ground_truth;
#[path = "../classes_ground_truth.rs"]
mod classes_ground_truth;
#[path = "../export_samples.rs"]
mod export_samples;
#[path = "../fixture_track.rs"]
mod fixture_track;
#[path = "../hostile_real.rs"]
mod hostile_real;
#[path = "../install_census.rs"]
mod install_census;
#[path = "../known_items.rs"]
mod known_items;
#[path = "../oracle_ground_truth.rs"]
mod oracle_ground_truth;
#[path = "../serialized_ground_truth.rs"]
mod serialized_ground_truth;
#[path = "../synthetic.rs"]
mod synthetic;

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
