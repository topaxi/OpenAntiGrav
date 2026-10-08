//! [`Setup::headless`]: a race that needs no disc, no GPU and no track.
//!
//! **Not a fixture and not a stand-in for one.** It authors no geometry, no
//! handling and no weapon table, because a `Setup` that invented any of those
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

impl Setup {
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
            trail_sparks: false,
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
            hd_trail: None,
            // No shield to draw either, and `Palette` has no measured "empty"
            // constant of its own - only Pulse's and HD's real, measured
            // colours - so a zeroed literal is the honest absence here,
            // matching every other visual field on this constructor rather
            // than picking one title's real palette for a race that draws
            // nothing.
            shield_palette: oag_render::shield::Palette {
                activation: [0.0; 4],
                target: [0.0; 4],
                hit: [0.0; 4],
                tints_shell: true,
            },
            // Nothing draws on this constructor, Pulse's mechanism included -
            // see `shield_palette` above for the same reasoning.
            hd_plasma_blast: false,
            hd_bomb_blast: false,
            hd_missile_blast: false,
            pulse_laid_pose: false,
            grid_frame_from_sample: false,
            screen_flash: false,
            absorb_burst: None,
            absorb_anchors: Vec::new(),
            hit_spark_anchors: Vec::new(),
            weapon_spark_anchors: Vec::new(),
            wreck_anchors: Vec::new(),
            magstrip_wake: None,
            magstrip_pob: false,
            leach_strip: None,
            destroy_stations: Vec::new(),
            finished_thrust: None,
            intro_camera: None,
            slot_teams: Vec::new(),
            collision: CollisionWorld::new(),
            handling: Handling::ZERO,
            // Zero travel at zero speed: the flaps neither open nor move.
            airbrake_graphics: oag_gameplay::AirbrakeGraphics {
                amount: 0.0,
                up_speed: 0.0,
                down_speed: 0.0,
            },
            class_gravity_scale: 1.0,
            start_boost: None,
            // The cameras, all zeroed. Nothing here draws, and a camera whose
            // numbers were invented to look reasonable is the failure
            // `CLAUDE.md`'s "never invent what the assets already author"
            // names - zero is the honest absence.
            chase: ZERO_CHASE,
            chase_close: ZERO_CHASE,
            internal: InternalParams {
                fov: 0.0,
                headtilt: None,
                height: 0.0,
                length: 0.0,
                pitch: 0.0,
            },
            nozzles: Vec::new(),
            engine_lights: Vec::new(),
            spu_vertex_lights: true,
            spark_anchors: Vec::new(),
            collision_fx: Vec::new(),
            scenery_fx: Default::default(),
            effects: psys::Library::default(),
            handles: Default::default(),
            sounds: oag_sound::sfx::Banks::default(),
            track_emitters: oag_sound::sfx::TrackEmitters::default(),
            announcer: oag_sound::sfx::Announcer::default(),
            class_announcer: oag_sound::sfx::ClassAnnouncer::default(),
            zone_stages: None,
            countdown_voice: false,
            speedup_pads: Vec::new(),
            weapon_pads: Vec::new(),
            weapons: None,
            weapon_ai: None,
            allowed_weapons: Vec::new(),
            weapon_pad_refresh: 0.0,
            pose_override: None,
            camera_override: None,
        }
    }
}

/// A chase camera with every term at zero. See [`Setup::headless`].
const ZERO_CHASE: ChaseParams = ChaseParams {
    craft_scale: 0.0,
    fov: 0.0,
    lookat_height: 0.0,
    lookat_length: 0.0,
    pos_height: 0.0,
    pos_length: 0.0,
    spring_horiz: 0.0,
    spring_vert: 0.0,
};
