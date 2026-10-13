//! Every integration test of this crate, linked into one binary.
//!
//! Each `tests/<name>.rs` used to be its own test binary, and each one linked
//! the crate's whole dependency graph. One binary compiles the shared generic
//! code once and links once; `Cargo.toml` sets `autotests = false` and names
//! this file as the only `[[test]]`. A test keeps its name behind its file's
//! module: `cargo nextest run -p oag-game <file>::` runs one file's tests.
//!
//! A new test file is only compiled once it is listed here.

#[path = "../in_parallel/mod.rs"]
mod in_parallel;

#[path = "../absorb_ground_truth.rs"]
mod absorb_ground_truth;
#[path = "../absorb_mask_ground_truth.rs"]
mod absorb_mask_ground_truth;
#[path = "../ai_clean_lap_board.rs"]
mod ai_clean_lap_board;
#[path = "../ai_clean_lap_gate.rs"]
mod ai_clean_lap_gate;
#[path = "../ai_dekonstruct_black_board.rs"]
mod ai_dekonstruct_black_board;
#[path = "../ai_dekonstruct_black_ground_truth.rs"]
mod ai_dekonstruct_black_ground_truth;
#[path = "../ai_dekonstruct_symptoms_ground_truth.rs"]
mod ai_dekonstruct_symptoms_ground_truth;
#[path = "../ai_fork_split_ground_truth.rs"]
mod ai_fork_split_ground_truth;
#[path = "../ai_look_sweep.rs"]
mod ai_look_sweep;
#[path = "../ai_loss_attribution_board.rs"]
mod ai_loss_attribution_board;
#[path = "../ai_plan_probe.rs"]
mod ai_plan_probe;
#[path = "../ai_roll_ground_truth.rs"]
mod ai_roll_ground_truth;
#[path = "../ai_span_sweep.rs"]
mod ai_span_sweep;
#[path = "../ai_speed_plan_sweep.rs"]
mod ai_speed_plan_sweep;
#[path = "../airtime_budget_ground_truth.rs"]
mod airtime_budget_ground_truth;
#[path = "../asset_layers_ground_truth.rs"]
mod asset_layers_ground_truth;
#[path = "../billboard_adverts_ground_truth.rs"]
mod billboard_adverts_ground_truth;
#[path = "../bomb_orbit_ground_truth.rs"]
mod bomb_orbit_ground_truth;
#[path = "../boost_plume_ground_truth.rs"]
mod boost_plume_ground_truth;
#[path = "../boot_ground_truth.rs"]
mod boot_ground_truth;
#[path = "../boot_legal_wrap_ground_truth.rs"]
mod boot_legal_wrap_ground_truth;
#[path = "../branch_progress_ground_truth.rs"]
mod branch_progress_ground_truth;
#[path = "../campaign_cell_class_census_ground_truth.rs"]
mod campaign_cell_class_census_ground_truth;
#[path = "../campaign_medal.rs"]
mod campaign_medal;
#[path = "../campaign_pointer_ground_truth.rs"]
mod campaign_pointer_ground_truth;
#[path = "../campaign_skill_scale_ground_truth.rs"]
mod campaign_skill_scale_ground_truth;
#[path = "../cannon_ground_truth.rs"]
mod cannon_ground_truth;
#[path = "../cannon_tilt_ground_truth.rs"]
mod cannon_tilt_ground_truth;
#[path = "../chase_camera_ground_truth.rs"]
mod chase_camera_ground_truth;
#[path = "../circuit_length_ground_truth.rs"]
mod circuit_length_ground_truth;
#[path = "../countdown_voice_ground_truth.rs"]
mod countdown_voice_ground_truth;
#[path = "../craft_sticking_ground_truth.rs"]
mod craft_sticking_ground_truth;
#[path = "../damage_fx_ground_truth.rs"]
mod damage_fx_ground_truth;
#[path = "../destroy_camera_ground_truth.rs"]
mod destroy_camera_ground_truth;
#[path = "../difficulty_ground_truth.rs"]
mod difficulty_ground_truth;
#[path = "../dlc_ground_truth.rs"]
mod dlc_ground_truth;
#[path = "../eliminator_ai_stats_ground_truth.rs"]
mod eliminator_ai_stats_ground_truth;
#[path = "../eliminator_finish_ground_truth.rs"]
mod eliminator_finish_ground_truth;
#[path = "../eliminator_pads_ground_truth.rs"]
mod eliminator_pads_ground_truth;
#[path = "../endrace_trophy_ground_truth.rs"]
mod endrace_trophy_ground_truth;
#[path = "../endrace_voice_ground_truth.rs"]
mod endrace_voice_ground_truth;
#[path = "../falloff_survey_ground_truth.rs"]
mod falloff_survey_ground_truth;
#[path = "../fire_law_inherit_ground_truth.rs"]
mod fire_law_inherit_ground_truth;
#[path = "../font_border_ground_truth.rs"]
mod font_border_ground_truth;
#[path = "../font_roles_ground_truth.rs"]
mod font_roles_ground_truth;
#[path = "../foreign_conventions_ground_truth.rs"]
mod foreign_conventions_ground_truth;
#[path = "../foreign_race_ground_truth.rs"]
mod foreign_race_ground_truth;
#[path = "../frontend_grid_ground_truth.rs"]
mod frontend_grid_ground_truth;
#[path = "../gantry_mount_hd_circuits_ground_truth.rs"]
mod gantry_mount_hd_circuits_ground_truth;
#[path = "../german_ground_truth.rs"]
mod german_ground_truth;
#[path = "../glow_stamp_ground_truth.rs"]
mod glow_stamp_ground_truth;
#[path = "../grid_stagger_ground_truth.rs"]
mod grid_stagger_ground_truth;
#[path = "../grid_walk_ground_truth.rs"]
mod grid_walk_ground_truth;
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
#[path = "../mag_floor_scroll_ground_truth.rs"]
mod mag_floor_scroll_ground_truth;
#[path = "../maglock_ground_truth.rs"]
mod maglock_ground_truth;
#[path = "../magstrip_anchor_ground_truth.rs"]
mod magstrip_anchor_ground_truth;
#[path = "../magstrip_wire_2048_ground_truth.rs"]
mod magstrip_wire_2048_ground_truth;
#[path = "../magstrip_wire_hd_ground_truth.rs"]
mod magstrip_wire_hd_ground_truth;
#[path = "../main_menu_ground_truth.rs"]
mod main_menu_ground_truth;
#[path = "../medal_message_ground_truth.rs"]
mod medal_message_ground_truth;
#[path = "../menu_definition_settings.rs"]
mod menu_definition_settings;
#[path = "../menu_disabled_and_restart.rs"]
mod menu_disabled_and_restart;
#[path = "../menu_remix_ai_ground_truth.rs"]
mod menu_remix_ai_ground_truth;
#[path = "../menu_screen_filter.rs"]
mod menu_screen_filter;
#[path = "../menu_sfx_ground_truth.rs"]
mod menu_sfx_ground_truth;
#[path = "../menu_warnings_settings.rs"]
mod menu_warnings_settings;
#[path = "../menu_web_rows.rs"]
mod menu_web_rows;
#[path = "../mine_ground_truth.rs"]
mod mine_ground_truth;
#[path = "../mine_launch_audio_ground_truth.rs"]
mod mine_launch_audio_ground_truth;
#[path = "../mine_texture_scroll_ground_truth.rs"]
mod mine_texture_scroll_ground_truth;
#[path = "../missile_ground_truth.rs"]
mod missile_ground_truth;
#[path = "../mods_menu.rs"]
mod mods_menu;
#[path = "../mods_page.rs"]
mod mods_page;
#[path = "../movie_access_units_ground_truth.rs"]
mod movie_access_units_ground_truth;
#[path = "../movie_container_audio_ground_truth.rs"]
mod movie_container_audio_ground_truth;
#[path = "../movie_ground_truth.rs"]
mod movie_ground_truth;
#[path = "../mp4_movie_ground_truth.rs"]
mod mp4_movie_ground_truth;
#[path = "../music_resume_ground_truth.rs"]
mod music_resume_ground_truth;
#[path = "../name_voice_ground_truth.rs"]
mod name_voice_ground_truth;
#[path = "../novice_jump_ground_truth.rs"]
mod novice_jump_ground_truth;
#[path = "../novice_respawn_ground_truth.rs"]
mod novice_respawn_ground_truth;
#[path = "../off_track_rescue_ground_truth.rs"]
mod off_track_rescue_ground_truth;
#[path = "../omega_2048_era_craft_ground_truth.rs"]
mod omega_2048_era_craft_ground_truth;
#[path = "../omega_assist_hud_ground_truth.rs"]
mod omega_assist_hud_ground_truth;
#[path = "../omega_campaign_launch_ground_truth.rs"]
mod omega_campaign_launch_ground_truth;
#[path = "../omega_campaign_screens_ground_truth.rs"]
mod omega_campaign_screens_ground_truth;
#[path = "../omega_font_scale_ground_truth.rs"]
mod omega_font_scale_ground_truth;
#[path = "../omega_hud_ground_truth.rs"]
mod omega_hud_ground_truth;
#[path = "../omega_lightmap_ground_truth.rs"]
mod omega_lightmap_ground_truth;
#[path = "../omega_menu_backdrop_ground_truth.rs"]
mod omega_menu_backdrop_ground_truth;
#[path = "../omega_menu_blocks_ground_truth.rs"]
mod omega_menu_blocks_ground_truth;
#[path = "../omega_music_ground_truth.rs"]
mod omega_music_ground_truth;
#[path = "../omega_original_ai_ground_truth.rs"]
mod omega_original_ai_ground_truth;
#[path = "../omega_pvs_placement_ground_truth.rs"]
mod omega_pvs_placement_ground_truth;
#[path = "../omega_race_ground_truth.rs"]
mod omega_race_ground_truth;
#[path = "../omega_track_model_ground_truth.rs"]
mod omega_track_model_ground_truth;
#[path = "../omega_wem_ground_truth.rs"]
mod omega_wem_ground_truth;
#[path = "../opponent_fire_ground_truth.rs"]
mod opponent_fire_ground_truth;
#[path = "../opponent_weapons_ground_truth.rs"]
mod opponent_weapons_ground_truth;
#[path = "../original_ai_ground_truth.rs"]
mod original_ai_ground_truth;
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
#[path = "../scenery_fx_ground_truth.rs"]
mod scenery_fx_ground_truth;
#[path = "../selection_screens_ground_truth.rs"]
mod selection_screens_ground_truth;
#[path = "../sfx_ground_truth.rs"]
mod sfx_ground_truth;
#[path = "../sfx_repeating_ground_truth.rs"]
mod sfx_repeating_ground_truth;
#[path = "../sfx_timeline_ground_truth.rs"]
mod sfx_timeline_ground_truth;
#[path = "../sfx_weapon_ground_truth.rs"]
mod sfx_weapon_ground_truth;
#[path = "../sfx_xfade_2048_ground_truth.rs"]
mod sfx_xfade_2048_ground_truth;
#[path = "../sfx_xfade_ground_truth.rs"]
mod sfx_xfade_ground_truth;
#[path = "../shadow_ground_truth.rs"]
mod shadow_ground_truth;
#[path = "../shadow_stability_ground_truth.rs"]
mod shadow_stability_ground_truth;
#[path = "../shake_blur_ground_truth.rs"]
mod shake_blur_ground_truth;
#[path = "../shield_shell_scroll_ground_truth.rs"]
mod shield_shell_scroll_ground_truth;
#[path = "../shine_ground_truth.rs"]
mod shine_ground_truth;
#[path = "../ship_spawn_ground_truth.rs"]
mod ship_spawn_ground_truth;
#[path = "../shuriken_ground_truth.rs"]
mod shuriken_ground_truth;
#[path = "../spawn_heading_ground_truth.rs"]
mod spawn_heading_ground_truth;
#[path = "../speed_plan_ground_truth.rs"]
mod speed_plan_ground_truth;
#[path = "../split_end_race_ground_truth.rs"]
mod split_end_race_ground_truth;
#[path = "../split_hud_ground_truth.rs"]
mod split_hud_ground_truth;
#[path = "../split_screen_ground_truth.rs"]
mod split_screen_ground_truth;
#[path = "../stall_rescue_ground_truth.rs"]
mod stall_rescue_ground_truth;
#[path = "../start_gantry_clock_inherit_ground_truth.rs"]
mod start_gantry_clock_inherit_ground_truth;
#[path = "../start_gantry_go_hold_ground_truth.rs"]
mod start_gantry_go_hold_ground_truth;
#[path = "../start_gantry_hd_laps_ground_truth.rs"]
mod start_gantry_hd_laps_ground_truth;
#[path = "../start_gantry_hd_mount_ground_truth.rs"]
mod start_gantry_hd_mount_ground_truth;
#[path = "../start_gantry_report_ground_truth.rs"]
mod start_gantry_report_ground_truth;
#[path = "../start_gantry_report_pure_ground_truth.rs"]
mod start_gantry_report_pure_ground_truth;
#[path = "../start_line_ground_truth.rs"]
mod start_line_ground_truth;
#[path = "../start_pose_ground_truth.rs"]
mod start_pose_ground_truth;
#[path = "../texture_detail_ground_truth.rs"]
mod texture_detail_ground_truth;
#[path = "../texture_stream_ground_truth.rs"]
mod texture_stream_ground_truth;
#[path = "../title_behaviour_data.rs"]
mod title_behaviour_data;
#[path = "../track_audio_2048_ground_truth.rs"]
mod track_audio_2048_ground_truth;
#[path = "../track_audio_ground_truth.rs"]
mod track_audio_ground_truth;
#[path = "../track_audio_hd_ground_truth.rs"]
mod track_audio_hd_ground_truth;
#[path = "../track_panel_ground_truth.rs"]
mod track_panel_ground_truth;
#[path = "../unlock_ground_truth.rs"]
mod unlock_ground_truth;
#[path = "../variant_unlock_ground_truth.rs"]
mod variant_unlock_ground_truth;
#[path = "../vita_2048_boot_ground_truth.rs"]
mod vita_2048_boot_ground_truth;
#[path = "../vita_2048_campaign_grid_ground_truth.rs"]
mod vita_2048_campaign_grid_ground_truth;
#[path = "../vita_2048_campaign_progress_ground_truth.rs"]
mod vita_2048_campaign_progress_ground_truth;
#[path = "../vita_2048_endrace_ground_truth.rs"]
mod vita_2048_endrace_ground_truth;
#[path = "../vita_2048_event_card_ground_truth.rs"]
mod vita_2048_event_card_ground_truth;
#[path = "../vita_2048_hud_ground_truth.rs"]
mod vita_2048_hud_ground_truth;
#[path = "../vita_2048_lighting_ground_truth.rs"]
mod vita_2048_lighting_ground_truth;
#[path = "../vita_2048_music_ground_truth.rs"]
mod vita_2048_music_ground_truth;
#[path = "../vita_2048_original_ai_ground_truth.rs"]
mod vita_2048_original_ai_ground_truth;
#[path = "../vita_2048_zone_scenery_ground_truth.rs"]
mod vita_2048_zone_scenery_ground_truth;
#[path = "../wall_collision_ground_truth.rs"]
mod wall_collision_ground_truth;
#[path = "../weapon_floor_sweep.rs"]
mod weapon_floor_sweep;
#[path = "../weapon_slowdown_ground_truth.rs"]
mod weapon_slowdown_ground_truth;
#[path = "../weapon_stamp_ground_truth.rs"]
mod weapon_stamp_ground_truth;
#[path = "../weapons_row_ground_truth.rs"]
mod weapons_row_ground_truth;
#[path = "../weather_ground_truth.rs"]
mod weather_ground_truth;
#[path = "../web_picker_table.rs"]
mod web_picker_table;
#[path = "../wreck_ground_truth.rs"]
mod wreck_ground_truth;
#[path = "../wreck_voice_ground_truth.rs"]
mod wreck_voice_ground_truth;
#[path = "../zone_airbrake_flaps_ground_truth.rs"]
mod zone_airbrake_flaps_ground_truth;
#[path = "../zone_announcer_ground_truth.rs"]
mod zone_announcer_ground_truth;
#[path = "../zone_craft_ground_truth.rs"]
mod zone_craft_ground_truth;
#[path = "../zone_grade_ground_truth.rs"]
mod zone_grade_ground_truth;
#[path = "../zone_ground_truth.rs"]
mod zone_ground_truth;
#[path = "../zone_sky_ground_truth.rs"]
mod zone_sky_ground_truth;
#[path = "../zone_start_ground_truth.rs"]
mod zone_start_ground_truth;

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
