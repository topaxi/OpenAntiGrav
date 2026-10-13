//! The integration tests of this crate whose file name starts with `h` to
//! `l`, linked into one binary.
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

#[path = "../hd_ai_craft_laws_ground_truth.rs"]
mod hd_ai_craft_laws_ground_truth;
#[path = "../hd_behind_glass_frame_ground_truth.rs"]
mod hd_behind_glass_frame_ground_truth;
#[path = "../hd_bomb_blast_ground_truth.rs"]
mod hd_bomb_blast_ground_truth;
#[path = "../hd_boot_ground_truth.rs"]
mod hd_boot_ground_truth;
#[path = "../hd_campaign_ground_truth.rs"]
mod hd_campaign_ground_truth;
#[path = "../hd_cell_select_ground_truth.rs"]
mod hd_cell_select_ground_truth;
#[path = "../hd_endrace_ground_truth.rs"]
mod hd_endrace_ground_truth;
#[path = "../hd_engine_flare_ground_truth.rs"]
mod hd_engine_flare_ground_truth;
#[path = "../hd_envsettings_carry_ground_truth.rs"]
mod hd_envsettings_carry_ground_truth;
#[path = "../hd_flyby_ground_truth.rs"]
mod hd_flyby_ground_truth;
#[path = "../hd_flyer_ground_truth.rs"]
mod hd_flyer_ground_truth;
#[path = "../hd_fury_backdrop_ground_truth.rs"]
mod hd_fury_backdrop_ground_truth;
#[path = "../hd_fury_ship_select_ground_truth.rs"]
mod hd_fury_ship_select_ground_truth;
#[path = "../hd_grid_hover_ground_truth.rs"]
mod hd_grid_hover_ground_truth;
#[path = "../hd_hud_ground_truth.rs"]
mod hd_hud_ground_truth;
#[path = "../hd_hud_runtime_ground_truth.rs"]
mod hd_hud_runtime_ground_truth;
#[path = "../hd_leach_ball_ground_truth.rs"]
mod hd_leach_ball_ground_truth;
#[path = "../hd_leach_strip_ground_truth.rs"]
mod hd_leach_strip_ground_truth;
#[path = "../hd_livery_ground_truth.rs"]
mod hd_livery_ground_truth;
#[path = "../hd_menu_ground_truth.rs"]
mod hd_menu_ground_truth;
#[path = "../hd_missile_blast_ground_truth.rs"]
mod hd_missile_blast_ground_truth;
#[path = "../hd_mix_ground_truth.rs"]
mod hd_mix_ground_truth;
#[path = "../hd_movie_ground_truth.rs"]
mod hd_movie_ground_truth;
#[path = "../hd_music_ground_truth.rs"]
mod hd_music_ground_truth;
#[path = "../hd_opponent_variant_ground_truth.rs"]
mod hd_opponent_variant_ground_truth;
#[path = "../hd_original_ai_ground_truth.rs"]
mod hd_original_ai_ground_truth;
#[path = "../hd_pad_illumination_ground_truth.rs"]
mod hd_pad_illumination_ground_truth;
#[path = "../hd_pickup_icon_ground_truth.rs"]
mod hd_pickup_icon_ground_truth;
#[path = "../hd_plasma_ball_ground_truth.rs"]
mod hd_plasma_ball_ground_truth;
#[path = "../hd_psn_campaign_ground_truth.rs"]
mod hd_psn_campaign_ground_truth;
#[path = "../hd_rim_glow_ground_truth.rs"]
mod hd_rim_glow_ground_truth;
#[path = "../hd_rocket_smoke_ground_truth.rs"]
mod hd_rocket_smoke_ground_truth;
#[path = "../hd_ship_preview_ground_truth.rs"]
mod hd_ship_preview_ground_truth;
#[path = "../hd_sky_tint_ground_truth.rs"]
mod hd_sky_tint_ground_truth;
#[path = "../hd_track_model_ground_truth.rs"]
mod hd_track_model_ground_truth;
#[path = "../hd_trackwall_ground_truth.rs"]
mod hd_trackwall_ground_truth;
#[path = "../hd_weapon_scene_ground_truth.rs"]
mod hd_weapon_scene_ground_truth;
#[path = "../hud_layout_ground_truth.rs"]
mod hud_layout_ground_truth;
#[path = "../hud_scale_gate.rs"]
mod hud_scale_gate;
#[path = "../intro_camera_ground_truth.rs"]
mod intro_camera_ground_truth;
#[path = "../ipf_ground_truth.rs"]
mod ipf_ground_truth;
#[path = "../language_offered_ground_truth.rs"]
mod language_offered_ground_truth;
#[path = "../language_reload_ground_truth.rs"]
mod language_reload_ground_truth;
#[path = "../lap_messages_ground_truth.rs"]
mod lap_messages_ground_truth;
#[path = "../lap_times_ground_truth.rs"]
mod lap_times_ground_truth;
#[path = "../launch_boost_ground_truth.rs"]
mod launch_boost_ground_truth;
#[path = "../launcher_ground_truth.rs"]
mod launcher_ground_truth;
#[path = "../leach_energy_ground_truth.rs"]
mod leach_energy_ground_truth;
#[path = "../leach_tube_ground_truth.rs"]
mod leach_tube_ground_truth;
#[path = "../livery_ground_truth.rs"]
mod livery_ground_truth;
#[path = "../loading_screen_ground_truth.rs"]
mod loading_screen_ground_truth;
#[path = "../lock_sight_ground_truth.rs"]
mod lock_sight_ground_truth;

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
