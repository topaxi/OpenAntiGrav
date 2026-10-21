//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-render <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here, and
//! `every_test_file_is_listed` below fails until it is.

mod archive_cache;
mod track_albedo;

mod airbrake_flaps_ground_truth;
mod alpha_test_cutout;
mod anim_upload_census_ground_truth;
mod animated_uv_ground_truth;
mod authored_uv_ground_truth;
mod billboard_slots_ground_truth;
mod blend_class_ground_truth;
mod blink_lights_ground_truth;
mod bloom_gain;
mod collision_capture;
mod gantry_mount_ground_truth;
mod glow_mask_ground_truth;
mod hd_airbrake_flaps_ground_truth;
mod hd_ambient_role_census_ground_truth;
mod hd_amphiseum_wall_material_ground_truth;
mod hd_behind_glass_ground_truth;
mod hd_emissive_glow_ground_truth;
mod hd_emissive_lit_path;
mod hd_gantry_curve_replay_ground_truth;
mod hd_holo_billboard_ground_truth;
mod hd_holo_billboard_lit_path;
mod hd_hull_colour_factor_ground_truth;
mod hd_hull_env_probe;
mod hd_hull_glow_role_ground_truth;
mod hd_hull_normal_lit_path;
mod hd_light_bar_ground_truth;
mod hd_light_cone_ground_truth;
mod hd_light_cone_lit_path;
mod hd_lightmapped_albedo_ground_truth;
mod hd_lod_ground_truth;
mod hd_mag_wave_ground_truth;
mod hd_mag_wave_lit_path;
mod hd_original_speed_pad_ground_truth;
mod hd_pad_ne_ground_truth;
mod hd_pad_ne_lit_path;
mod hd_program_glow_ground_truth;
mod hd_program_glow_mask;
mod hd_quake_lift;
mod hd_scenery_animation_ground_truth;
mod hd_shadowed_track_redraw;
mod hd_ship_hull_material_ground_truth;
mod hd_speed_pad_owner_ground_truth;
mod hd_spu_vertex_light;
mod hd_track_surface_ground_truth;
mod hd_vertex_scroll_ground_truth;
mod hd_vineta_ceiling_ground_truth;
mod hd_water_ground_truth;
mod hd_weapon_pad_glow_ground_truth;
mod hd_weapon_pad_split_ground_truth;
mod hd_zone_light_cells_ground_truth;
mod hd_zone_wave_census_ground_truth;
mod hull_lights_ground_truth;
mod lod_switch_ground_truth;
mod magstrip_ground_truth;
mod moving_normal_inverse_transpose;
mod msaa_load_keeps_view;
mod msaa_resolve;
mod msaa_resolve_viewport;
mod node_anim_pipeline;
mod omega_nova_prelit;
mod omega_pvs_placement_ground_truth;
mod omega_transparency_ground_truth;
mod pad_alpha_test_ground_truth;
mod ps2_bloom_gain;
mod ps2_start_gantry_ground_truth;
mod ps2_texture_binding_ground_truth;
mod psp2_airbrake_flaps_ground_truth;
mod psp2_glow_ground_truth;
mod psp2_scenery_animation_ground_truth;
mod psp2_texture_dedupe_ground_truth;
mod psp_slope_lod;
mod psp_texture_levels_ground_truth;
mod pvs_placement_ground_truth;
mod pvs_section_view_ground_truth;
mod road_uv_ground_truth;
mod scenery_animation_ground_truth;
mod self_shadow_map;
mod shadow_map_coverage;
mod shake_ground_truth;
mod sun_occlusion_map;
mod velocity_target;
mod weapon_pad_order_ground_truth;
mod zone_recolour;

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
