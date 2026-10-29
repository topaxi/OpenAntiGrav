//! [`Options`]' `Default`: split out of `options.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

impl Default for Options {
    fn default() -> Self {
        Self {
            source: oag_source::source::DEFAULT_IMAGE.to_string(),
            craft_source: None,
            difficulty: oag_ai::Difficulty::default(),
            // Empty rather than the search path: a default that read the
            // filesystem would make two runs of the same test differ by what
            // the developer happens to have downloaded. The composition root
            // fills this in from `source::resolve_dlc`.
            dlc: Vec::new(),
            track: None,
            foreign_track: None,
            extensions: Vec::new(),
            boost_zoom: None,
            team: None,
            hull_variant: None,
            skin: None,
            // Empty for the same reason `dlc` is: the ids come off the
            // player's own disc, and `load` may not invent one. The
            // composition root fills it from the catalogue.
            opponent_teams: Vec::new(),
            // Empty is "no override anywhere" - see the field's own doc
            // comment. Only `race::load_event` ever sets this.
            grid_teams: Vec::new(),
            seat_picks: Vec::new(),
            zone_model: false,
            hull_shine: true,
            hull_wreck: true,
            track_shine: true,
            // The rung every measured title shares, named rather than
            // defaulted from an enum: a title whose ladder was never read must
            // not silently inherit one that was.
            class: SpeedClass::VENOM.as_str().to_string(),
            mode: Mode::default(),
            language: None,
            hud_style: None,
            add_on_campaign: None,
            eliminator_kill_target: None,
            laps_override: None,
            weapons_override: None,
            no_driver_for_class: false,
            server_source: false,
            zone_stage: None,
            exposure_hold: None,
            ribbon: false,
            collision: false,
            opponents: false,
            views: 1,
            people: 0,
            trail_sparks: false,
            seed: None,
            pose: None,
            field: Vec::new(),
            hud: None,
            world_clock_offset: 0.0,
            gantry_seconds: None,
            camera: None,
        }
    }
}
