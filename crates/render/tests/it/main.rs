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

#[path = "../archive_cache/mod.rs"]
mod archive_cache;

#[path = "../airbrake_flaps_ground_truth.rs"]
mod airbrake_flaps_ground_truth;
#[path = "../alpha_test_cutout.rs"]
mod alpha_test_cutout;
#[path = "../anim_upload_census_ground_truth.rs"]
mod anim_upload_census_ground_truth;
#[path = "../animated_uv_ground_truth.rs"]
mod animated_uv_ground_truth;
#[path = "../authored_uv_ground_truth.rs"]
mod authored_uv_ground_truth;
#[path = "../billboard_slots_ground_truth.rs"]
mod billboard_slots_ground_truth;
#[path = "../blend_class_ground_truth.rs"]
mod blend_class_ground_truth;
#[path = "../blink_lights_ground_truth.rs"]
mod blink_lights_ground_truth;
#[path = "../bloom_gain.rs"]
mod bloom_gain;
#[path = "../collision_capture.rs"]
mod collision_capture;
#[path = "../gantry_mount_ground_truth.rs"]
mod gantry_mount_ground_truth;
#[path = "../glow_mask_ground_truth.rs"]
mod glow_mask_ground_truth;
#[path = "../hd_airbrake_flaps_ground_truth.rs"]
mod hd_airbrake_flaps_ground_truth;
#[path = "../hd_ambient_role_census_ground_truth.rs"]
mod hd_ambient_role_census_ground_truth;
#[path = "../hd_amphiseum_wall_material_ground_truth.rs"]
mod hd_amphiseum_wall_material_ground_truth;
#[path = "../hd_behind_glass_ground_truth.rs"]
mod hd_behind_glass_ground_truth;
#[path = "../hd_emissive_glow_ground_truth.rs"]
mod hd_emissive_glow_ground_truth;
#[path = "../hd_emissive_lit_path.rs"]
mod hd_emissive_lit_path;
#[path = "../hd_gantry_curve_replay_ground_truth.rs"]
mod hd_gantry_curve_replay_ground_truth;
#[path = "../hd_holo_billboard_ground_truth.rs"]
mod hd_holo_billboard_ground_truth;
#[path = "../hd_holo_billboard_lit_path.rs"]
mod hd_holo_billboard_lit_path;
#[path = "../hd_hull_colour_factor_ground_truth.rs"]
mod hd_hull_colour_factor_ground_truth;
#[path = "../hd_hull_glow_role_ground_truth.rs"]
mod hd_hull_glow_role_ground_truth;
#[path = "../hd_light_bar_ground_truth.rs"]
mod hd_light_bar_ground_truth;
#[path = "../hd_light_cone_ground_truth.rs"]
mod hd_light_cone_ground_truth;
#[path = "../hd_light_cone_lit_path.rs"]
mod hd_light_cone_lit_path;
#[path = "../hd_lightmapped_albedo_ground_truth.rs"]
mod hd_lightmapped_albedo_ground_truth;
#[path = "../hd_lod_ground_truth.rs"]
mod hd_lod_ground_truth;
#[path = "../hd_mag_wave_ground_truth.rs"]
mod hd_mag_wave_ground_truth;
#[path = "../hd_mag_wave_lit_path.rs"]
mod hd_mag_wave_lit_path;
#[path = "../hd_original_speed_pad_ground_truth.rs"]
mod hd_original_speed_pad_ground_truth;
#[path = "../hd_pad_ne_ground_truth.rs"]
mod hd_pad_ne_ground_truth;
#[path = "../hd_pad_ne_lit_path.rs"]
mod hd_pad_ne_lit_path;
#[path = "../hd_quake_lift.rs"]
mod hd_quake_lift;
#[path = "../hd_scenery_animation_ground_truth.rs"]
mod hd_scenery_animation_ground_truth;
#[path = "../hd_ship_hull_material_ground_truth.rs"]
mod hd_ship_hull_material_ground_truth;
#[path = "../hd_speed_pad_owner_ground_truth.rs"]
mod hd_speed_pad_owner_ground_truth;
#[path = "../hd_spu_vertex_light.rs"]
mod hd_spu_vertex_light;
#[path = "../hd_vertex_scroll_ground_truth.rs"]
mod hd_vertex_scroll_ground_truth;
#[path = "../hd_vineta_ceiling_ground_truth.rs"]
mod hd_vineta_ceiling_ground_truth;
#[path = "../hd_water_ground_truth.rs"]
mod hd_water_ground_truth;
#[path = "../hd_weapon_pad_glow_ground_truth.rs"]
mod hd_weapon_pad_glow_ground_truth;
#[path = "../hd_weapon_pad_split_ground_truth.rs"]
mod hd_weapon_pad_split_ground_truth;
#[path = "../hd_zone_wave_census_ground_truth.rs"]
mod hd_zone_wave_census_ground_truth;
#[path = "../hull_lights_ground_truth.rs"]
mod hull_lights_ground_truth;
#[path = "../lod_switch_ground_truth.rs"]
mod lod_switch_ground_truth;
#[path = "../magstrip_ground_truth.rs"]
mod magstrip_ground_truth;
#[path = "../moving_normal_inverse_transpose.rs"]
mod moving_normal_inverse_transpose;
#[path = "../msaa_load_keeps_view.rs"]
mod msaa_load_keeps_view;
#[path = "../msaa_resolve.rs"]
mod msaa_resolve;
#[path = "../msaa_resolve_viewport.rs"]
mod msaa_resolve_viewport;
#[path = "../node_anim_pipeline.rs"]
mod node_anim_pipeline;
#[path = "../omega_nova_prelit.rs"]
mod omega_nova_prelit;
#[path = "../omega_transparency_ground_truth.rs"]
mod omega_transparency_ground_truth;
#[path = "../pad_alpha_test_ground_truth.rs"]
mod pad_alpha_test_ground_truth;
#[path = "../ps2_bloom_gain.rs"]
mod ps2_bloom_gain;
#[path = "../ps2_texture_binding_ground_truth.rs"]
mod ps2_texture_binding_ground_truth;
#[path = "../psp2_airbrake_flaps_ground_truth.rs"]
mod psp2_airbrake_flaps_ground_truth;
#[path = "../psp2_glow_ground_truth.rs"]
mod psp2_glow_ground_truth;
#[path = "../psp2_scenery_animation_ground_truth.rs"]
mod psp2_scenery_animation_ground_truth;
#[path = "../psp2_texture_dedupe_ground_truth.rs"]
mod psp2_texture_dedupe_ground_truth;
#[path = "../psp_slope_lod.rs"]
mod psp_slope_lod;
#[path = "../psp_texture_levels_ground_truth.rs"]
mod psp_texture_levels_ground_truth;
#[path = "../pvs_placement_ground_truth.rs"]
mod pvs_placement_ground_truth;
#[path = "../pvs_section_view_ground_truth.rs"]
mod pvs_section_view_ground_truth;
#[path = "../road_uv_ground_truth.rs"]
mod road_uv_ground_truth;
#[path = "../scenery_animation_ground_truth.rs"]
mod scenery_animation_ground_truth;
#[path = "../self_shadow_map.rs"]
mod self_shadow_map;
#[path = "../shadow_map_coverage.rs"]
mod shadow_map_coverage;
#[path = "../shake_ground_truth.rs"]
mod shake_ground_truth;
#[path = "../sun_occlusion_map.rs"]
mod sun_occlusion_map;
#[path = "../velocity_target.rs"]
mod velocity_target;
#[path = "../weapon_pad_order_ground_truth.rs"]
mod weapon_pad_order_ground_truth;
#[path = "../zone_recolour.rs"]
mod zone_recolour;

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
