//! [`Options`]' `Default` and its source-opening helpers: split out of `options.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

impl Default for Options {
    fn default() -> Self {
        Self {
            source: oag_source::source::DEFAULT_IMAGE.to_string(),
            craft_source: None,
            difficulty: oag_ai::Difficulty::default(),
            skill_level: None,
            // Empty rather than the search path: a default that read the
            // filesystem would make two runs of the same test differ by what
            // the developer happens to have downloaded. The composition root
            // fills this in from `source::resolve_dlc`.
            dlc: Vec::new(),
            track: None,
            foreign_track: None,
            extensions: Vec::new(),
            find_extensions: None,
            recorded_mods: None,
            find_texture_packs: None,
            no_overrides: false,
            boost_zoom: None,
            trail_hits: TrailHits::Off,
            wingtip_contrails: WingtipContrails::Off,
            own_shield: false,
            own_rockets: false,
            water: oag_display::display::Water::default(),
            shadow_resolution: oag_display::display::ShadowResolution::default(),
            scene_precision: oag_display::display::ScenePrecision::default(),
            circuit_lighting: oag_display::display::CircuitLighting::default(),
            reflection_resolution: oag_display::display::ReflectionResolution::default(),
            reflection_timed: None,
            team: None,
            hull_variant: None,
            skin: None,
            livery: None,
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
            campaign: false,
            eliminator_kill_target: None,
            eliminator_score_target: None,
            laps_override: None,
            weapons_override: None,
            no_driver_for_class: false,
            server_source: false,
            zone_stage: None,
            exposure_hold: None,
            ribbon: false,
            no_effects: false,
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
            flame_seconds: None,
            camera: None,
            fov_override: None,
            demo: false,
        }
    }
}

impl Options {
    /// Where the player's override directories are looked for when this race opens a source:
    /// the default search path, or none under [`Self::no_overrides`] (the fallback layer alone).
    #[must_use]
    pub fn override_bases(&self) -> Vec<std::path::PathBuf> {
        if self.no_overrides {
            Vec::new()
        } else {
            oag_source::layers::overrides_search_path()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_race_without_overrides_looks_in_no_override_directory() {
        let mut options = Options::default();
        assert!(!options.override_bases().is_empty());
        options.no_overrides = true;
        assert!(options.override_bases().is_empty());
    }
}
