//! [`SimSetup::headless`]: a race that needs no disc, no GPU and no track.
//!
//! **Not a fixture and not a stand-in for one.** It authors no geometry, no
//! handling and no weapon table, because a `SimSetup` that invented any of those
//! would be exactly the "plausible-looking stand-in" `CLAUDE.md` forbids - a
//! race built on it is not a race, and nothing should read a lap time off it.
//! What it *is* good for is proving the tick loop runs: every field is the
//! empty value its own type already defines, so the simulation steps a grid of
//! craft that fall in a vacuum and the state hash still moves.
//!
//! Its one real caller is the `oag-headless-sim` binary, which exists to show
//! that `Race`/`RaceSim` reach the tick loop with no renderer attached at all -
//! the cheap half of what a network server needs, which can now depend on this
//! crate without `oag-game` (`workspace-layout.md`'s rule 2).

use super::*;

impl SimSetup {
    /// A race with nothing in it: no track, no collision, no tables.
    ///
    /// Every craft has [`Handling::ZERO`], so nothing accelerates and nothing
    /// steers; the spline is empty, so no craft is ever located on it and no
    /// lap is ever counted. That is the point - the only thing this proves is
    /// that a tick *runs*, and it must not be mistaken for a race that means
    /// anything. See this module's own doc comment.
    #[must_use]
    pub fn headless(mode: Mode, seed: u64) -> Self {
        let ai = AiTrack {
            version: 0x105,
            paths: Vec::new(),
            junctions: Vec::new(),
        };
        Self {
            foreign: None,
            mode,
            eliminator_kill_target: None,
            laps_override: None,
            weapons_override: None,
            difficulty: oag_ai::Difficulty::default(),
            class: "VENOM".to_string(),
            // The whole grid, so a tick has eight craft to step rather than
            // one - which is what makes this a check of the per-slot work
            // rather than of slot 0 again.
            opponents: true,
            no_opponents: false,
            people: 1,
            seed,
            // An empty track, spelled out because `AiTrack` deliberately has
            // no `Default`: a track is something a disc authors, and a type
            // that hands one out for free is how an invented one gets shipped.
            // This one has no paths at all, which is a visible absence rather
            // than a plausible shape.
            ai: ai.clone(),
            spline: Spline::from_track(&ai),
            zone: None,
            // No closed ring, so no lap counter at all. The honest answer for
            // a track that is not there, and the branch `Race::tick` already
            // takes for a circuit that cannot count.
            course: None,
            start_position: None,
            laid_from_rear: false,
            grid_frame_from_sample: false,
            finished_thrust: None,
            original_tables: None,
            slot_teams: Vec::new(),
            row_teams: Vec::new(),
            sight: Sighting::default(),
            collision: CollisionWorld::new(),
            handling: Handling::ZERO,
            opponent_handling: Vec::new(),
            weapon_counts: oag_weapons::counts::WeaponCounts::default(),
            opponent_weapon_counts: Vec::new(),
            class_gravity_scale: 1.0,
            start_boost: None,
            pilot_assist: Default::default(),
            wall_restitution: None,
            // The cameras, all zeroed. Nothing here draws, and a camera whose
            // numbers were invented to look reasonable is the failure
            // `CLAUDE.md`'s "never invent what the assets already author"
            // names - zero is the honest absence.
            chase: ZERO_CHASE,
            muzzles: Vec::new(),
            cannon_flash: [None, None],
            zone_stages: None,
            countdown_voice: false,
            countdown_beat: false,
            launch_hover: None,
            hover_rig: None,
            craft_laws: None,
            autopilot_engine: oag_title::autopilot_engine::MEASURED,
            opponent_autopilot: false,
            destroyed: oag_title::destroyed::DestroyedCraft::PULSE,
            weapon_pad_glow: &oag_title::weapon_pad::WeaponPadGlow::Authored,
            leach_drain: oag_title::weapons::LeachDrain::PerTick,
            blast_push: oag_title::weapons::BlastPush::Posted,
            leach_beams: 8,
            quake_hit: oag_title::weapons::QuakeHit::WaveHeight {
                origin: oag_title::Origin::Chosen,
            },
            autopilot_absorb_cancels: true,
            autopilot_pad_grants: false,
            launch_normal_is_track_up: true,
            floor_arms_steer_to_ride_point: true,
            floor_arms_ride_launch_normal: false,
            plasma_release_at_display_scale: true,
            rocket_launch_at_display_scale: true,
            own_craft_box_hides_track: oag_title::weapons::OwnCraftBox::Falls,
            surface_probe_lengths: oag_title::weapons::ProbeLengths::PULSE,
            rocket_open_air: oag_title::weapons::RocketOpenAir::Falls,
            reset_triggers: oag_race::recovery::ResetTriggers::PULSE,
            contact_cues: oag_title::contact_cue::ContactCues::Rearmed,
            opponent_fire: oag_title::weapons::OpponentFire::PULSE,
            score_law: None,
            score_pace_terms: oag_title::eliminator::ScorePaceTerms::WITH_REPULSER_AND_SHURIKEN,
            flip_law: None,
            speedup_pads: Vec::new(),
            weapon_pads: Vec::new(),
            weapons: None,
            weapon_ai: None,
            detonator: None,
            detonator_pads: None,
            allowed_weapons: Vec::new(),
            pickup_roster: None,
            weapon_pad_refresh: 0.0,
            pose_override: None,
            field_poses: Vec::new(),
            capture_zone: None,
        }
    }
}

/// A chase camera with every term at zero. See [`SimSetup::headless`].
pub(super) const ZERO_CHASE: ChaseParams = ChaseParams {
    craft_scale: 0.0,
    view_sheared: false,
    fov: 0.0,
    lookat_height: 0.0,
    lookat_length: 0.0,
    pos_height: 0.0,
    pos_length: 0.0,
    spring_horiz: 0.0,
    spring_vert: 0.0,
};
