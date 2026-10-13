//! The integration tests of this crate whose file name starts with `p` to
//! `r`, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. `Cargo.toml` sets `autotests = false`
//! and lists one `[[test]]` per first-letter range instead: few enough links
//! to keep that saving, and enough binaries that a cold build compiles them in
//! parallel and an edit to one test file recompiles only its range. A test
//! keeps its name behind its file's module: `cargo nextest run -p oag-game
//! <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed in the range its first
//! letter falls in, and `every_test_file_is_listed` below fails until it is.

#[path = "../in_parallel/mod.rs"]
mod in_parallel;

#[path = "../pad_join_end_to_end.rs"]
mod pad_join_end_to_end;
#[path = "../picker_glyphs_ground_truth.rs"]
mod picker_glyphs_ground_truth;
#[path = "../picker_pitch_ground_truth.rs"]
mod picker_pitch_ground_truth;
#[path = "../pickup_icon_ground_truth.rs"]
mod pickup_icon_ground_truth;
#[path = "../pilot_assist_ground_truth.rs"]
mod pilot_assist_ground_truth;
#[path = "../pilot_assist_levels_ground_truth.rs"]
mod pilot_assist_levels_ground_truth;
#[path = "../plasma_cues_audio_ground_truth.rs"]
mod plasma_cues_audio_ground_truth;
#[path = "../plasma_ground_truth.rs"]
mod plasma_ground_truth;
#[path = "../portuguese_br_ground_truth.rs"]
mod portuguese_br_ground_truth;
#[path = "../post_finish_ground_truth.rs"]
mod post_finish_ground_truth;
#[path = "../projectile_tilt_ground_truth.rs"]
mod projectile_tilt_ground_truth;
#[path = "../prompt_glyphs_ground_truth.rs"]
mod prompt_glyphs_ground_truth;
#[path = "../prompt_pilots.rs"]
mod prompt_pilots;
#[path = "../ps2_glow_mask_ground_truth.rs"]
mod ps2_glow_mask_ground_truth;
#[path = "../ps2_shield_ground_truth.rs"]
mod ps2_shield_ground_truth;
#[path = "../ps2_ship_skin_ground_truth.rs"]
mod ps2_ship_skin_ground_truth;
#[path = "../ps2_source_ground_truth.rs"]
mod ps2_source_ground_truth;
#[path = "../ps2_start_gantry_ground_truth.rs"]
mod ps2_start_gantry_ground_truth;
#[path = "../psp2_sky_fog_ground_truth.rs"]
mod psp2_sky_fog_ground_truth;
#[path = "../psp_shield_model_ground_truth.rs"]
mod psp_shield_model_ground_truth;
#[path = "../psys_inventory_ground_truth.rs"]
mod psys_inventory_ground_truth;
#[path = "../pulse_frame_ground_truth.rs"]
mod pulse_frame_ground_truth;
#[path = "../pulse_menu_ticker_ground_truth.rs"]
mod pulse_menu_ticker_ground_truth;
#[path = "../pulse_settings_ground_truth.rs"]
mod pulse_settings_ground_truth;
#[path = "../pulse_variant_titles_ground_truth.rs"]
mod pulse_variant_titles_ground_truth;
#[path = "../pure_boot_ground_truth.rs"]
mod pure_boot_ground_truth;
#[path = "../pure_chain_walk_ground_truth.rs"]
mod pure_chain_walk_ground_truth;
#[path = "../pure_dlc_ground_truth.rs"]
mod pure_dlc_ground_truth;
#[path = "../pure_frame_ground_truth.rs"]
mod pure_frame_ground_truth;
#[path = "../pure_menu_art_ground_truth.rs"]
mod pure_menu_art_ground_truth;
#[path = "../pure_music_ground_truth.rs"]
mod pure_music_ground_truth;
#[path = "../pure_original_ai_ground_truth.rs"]
mod pure_original_ai_ground_truth;
#[path = "../pure_race_ground_truth.rs"]
mod pure_race_ground_truth;
#[path = "../quake_orientation_ground_truth.rs"]
mod quake_orientation_ground_truth;
#[path = "../quake_ripple_ground_truth.rs"]
mod quake_ripple_ground_truth;
#[path = "../race_end_photo_load_ground_truth.rs"]
mod race_end_photo_load_ground_truth;
#[path = "../race_finish_ground_truth.rs"]
mod race_finish_ground_truth;
#[path = "../race_ground_truth.rs"]
mod race_ground_truth;
#[path = "../race_load_read_memo_ground_truth.rs"]
mod race_load_read_memo_ground_truth;
#[path = "../race_remix_ground_truth.rs"]
mod race_remix_ground_truth;
#[path = "../ram_ground_truth.rs"]
mod ram_ground_truth;
#[path = "../record_readout_ground_truth.rs"]
mod record_readout_ground_truth;
#[path = "../remix_2048_sources_ground_truth.rs"]
mod remix_2048_sources_ground_truth;
#[path = "../replay_ground_truth.rs"]
mod replay_ground_truth;
#[path = "../repulser_fork_ground_truth.rs"]
mod repulser_fork_ground_truth;
#[path = "../repulser_guard_band_ground_truth.rs"]
mod repulser_guard_band_ground_truth;
#[path = "../reset_zone_ground_truth.rs"]
mod reset_zone_ground_truth;
#[path = "../reversed_grid_ground_truth.rs"]
mod reversed_grid_ground_truth;
#[path = "../rocket_floor_trace.rs"]
mod rocket_floor_trace;
#[path = "../rocket_launch_ground_truth.rs"]
mod rocket_launch_ground_truth;
#[path = "../roster_declared_ground_truth.rs"]
mod roster_declared_ground_truth;

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
