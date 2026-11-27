//! The view's particle and effect state a blurred shot keeps at its tick.
//!
//! The ticks a blur replays emit, age and re-seed every effect the race draws, so a shot that did
//! not put them back left the next shot (and the race) other sparks than it found. What belongs
//! together: a pool and every handle into it, a source and the generator it draws from.
//!
//! **Every field of [`RaceView`] is named below**, as restored or as live, and the destructure has
//! no `..`: a field added to the view does not compile until it is sorted into one list. Restored
//! is any state a tick advances that is drawn as an effect, or holds a handle into one that is.
//! Live is configuration, the camera and everything the shot or the session manages itself:
//! `incoming_raises` is the HUD's (`HudMemory` in `shot.rs` keeps it with the rest of the HUD's
//! state), and `hull_shake` and `boost_kick` ride the camera.

use super::*;

/// What [`remember`] took, put back by calling it.
pub(super) struct EffectsMemory(Box<dyn FnOnce(&mut RaceView)>);

impl EffectsMemory {
    /// Puts the remembered fields back.
    pub(super) fn put_back(self, view: &mut RaceView) {
        (self.0)(view);
    }
}

impl std::fmt::Debug for EffectsMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EffectsMemory")
    }
}

macro_rules! sort_fields {
    (restored: [$($restored:ident),* $(,)?], live: [$($live:ident),* $(,)?] $(,)?) => {
        /// Clones the restored fields of `view`; the result puts them back.
        pub(super) fn remember(view: &RaceView) -> EffectsMemory {
            let RaceView {
                $($restored: _,)*
                $($live: _,)*
            } = view;
            $(let $restored = view.$restored.clone();)*
            EffectsMemory(Box::new(move |view: &mut RaceView| {
                $(view.$restored = $restored;)*
            }))
        }
    };
}

sort_fields! {
    restored: [
        shield, cockpit_hit, stage, stage_rng, surfaces, scenery_fx,
        contact, pair_contacts, field_sparks, floor_sparks, hit_sparks, damage_fx,
        exhaust, exhaust_rng, hd_trail, hd_trail_red, hd_flame, hd_sprite,
        trail_inside, trail_hit_rest, engine_light_jitter, engine_light_rng,
        engine_flare, projectile_flare, projectile_flare_orbit, plasma_launch,
        quake_effect, emp_effect, emp_fork_effect, quake_point, quake_surface, repulser_effects, repulser_fields, emp_repulsor,
        leach_beam_effect, leach_charge_effect, leach_beam_ribbon, leach_beam_rng,
        leach_ball_elapsed, leach_strip, magstrip_wake, mag_floor_fx,
        rocket_smoke, missile_smoke, wingtip,
        plasma_blasts, bomb_blasts, hd_bomb_blasts, hd_missile_blasts, hd_missile_streaks,
        disruptor_halos, disruptor_craft, screen_flash, craft_flashes, zone_end, ship_debris,
        wreck_fx, absorb_bursts, absorb_started, absorb_overlay, absorb_shell,
    ],
    live: [
        camera_view, camera, camera_jumps, transit_seats, poses, correction, correction_smoothing,
        hull_shake, hull_shake_law, hull_draw_factor, hull_shake_on, camera_shake_on,
        player_tags_on, track_sections, camera_override, fov_override, capture_hud,
        hud_waits_for_go, now_playing, world_clock_offset, gantry_seconds, flame_seconds, intro, finish_camera,
        hd_trail_active, spectates_from_start, photo, picture, restores, chase_params, chase_far, chase_close,
        viewpoints, internal_params, zoom, zoom_borrowed, ps2_trail, hd_plasma_blast,
        plasma_blast_law, disruptor_halo_law, disruptor_craft_law, pulse_laid_pose,
        laid_pose_scaled, craft_blast_carry, forced_blast_velocity, destroy_camera, absorb_burst, absorb_anchors, hull_reach,
        trail_sparks, campaign_event, score_pass, medal_targets, shield_was_up, huds, pair_spoil,
        split_gap, incoming_raises, nozzles, nozzle_axes, engine_lights, spu_vertex_lights,
        hull_points, hd_weapon_lights, spark_anchors, collision_fx, effects, handles, mine_blasts,
        hd_bomb_blast, bomb_shockwave_fades, hd_missile_blast, detonator_cannon_lights, detonator_emp_light, detonator_emp_repulsor, detonator_rounds_as_cannon,
        score_feedback,
        detonator_mine_spin,
        detonator_ammo_gauge, ammo_gauge_shown, ammo_gauge_speed, detonator_hull_charge,
        hull_charge_shown, sounds, track_emitters, announcer,
        class_announcer, rumble, boost_kick, boost_fov_kick, flaps, grid_flaps, flap_phase,
        flap_graphics, sight, sight_screens, sight_state, sight_aim, sight_fov, model_detail,
        texture_detail, trail_order, scenery_sort, weather, mine_poses, mine_pins,
    ],
}
