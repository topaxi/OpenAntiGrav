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
mod hd_amphiseum_wall_material_ground_truth;
mod hd_ship_hull_material_ground_truth;
mod hd_weapon_pad_split_ground_truth;
mod hull_lights_ground_truth;
mod lod_switch_ground_truth;
mod magstrip_ground_truth;
mod msaa_resolve;
mod pad_alpha_test_ground_truth;
mod ps2_blended_depth_ground_truth;
mod ps2_bloom_gain;
mod ps2_neon_clock_ground_truth;
mod ps2_start_gantry_ground_truth;
mod ps2_texture_binding_ground_truth;
mod psp_texture_levels_ground_truth;
mod pvs_placement_ground_truth;
mod pvs_section_view_ground_truth;
mod road_uv_ground_truth;
mod scenery_animation_ground_truth;
mod self_shadow_map;
mod shadow_map_coverage;
mod shake_ground_truth;
mod sun_occlusion_map;

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
