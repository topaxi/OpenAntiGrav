//! The integration tests of this crate whose file name starts with `h` to
//! `l`, linked into one binary.
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

mod perturb;

mod hd_ai_craft_laws_ground_truth;
mod hd_air_pose_walk;
mod hd_chenghou_jumps_sweep;
mod hd_engine_flare_ground_truth;
mod hd_env_probes_ground_truth;
mod hd_envsettings_carry_ground_truth;
mod hd_flare_spike_names_ground_truth;
mod hd_flyby_ground_truth;
mod hd_grid_hover_ground_truth;
mod hd_hud_ground_truth;
mod hd_livery_ground_truth;
mod hd_mix_ground_truth;
mod hd_opponent_variant_ground_truth;
mod hd_original_ai_ground_truth;
mod hd_pad_illumination_ground_truth;
mod hd_pickup_icon_ground_truth;
mod hd_record_target_ground_truth;
mod hd_rim_glow_ground_truth;
mod hd_rocket_smoke_ground_truth;
mod hd_sky_tint_ground_truth;
mod hd_tiny_sun_direction_ground_truth;
mod hd_trackwall_ground_truth;
mod hd_zone_hover_ground_truth;
mod head_to_head_people_ground_truth;
mod hillside_rain_ground_truth;
mod hud_layout_ground_truth;
mod hud_style_ground_truth;
mod hull_gather_ground_truth;
mod hull_light_scale_ground_truth;
mod intro_camera_ground_truth;
mod language_offered_ground_truth;
mod lap_times_ground_truth;
mod launch_boost_ground_truth;
mod leach_energy_ground_truth;
mod leach_tube_ground_truth;
mod livery_ground_truth;
mod lock_sight_ground_truth;

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
