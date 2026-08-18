//! Pure's in-race HUD layout entries.
//!
//! Entry names in the [ADR-0022] sense, the same division [`oag_pulse::hud`]
//! draws: *which* files this disc ships. Everything that reads them is
//! `oag_game::hud`.
//!
//! # Three names, all Pulse's, and that is a measurement
//!
//! Read directly off `pure-psp-eu.chd`'s `Data.wad` on 2026-08-18, one
//! `oag-wad cat` per name: `Data\XML\TimeTrial_HUD.xml`,
//! `Data\XML\Zone_HUD.xml` and `Data\XML\Arcade_HUD.xml` all return a layout,
//! spelled exactly as Pulse spells them. `Data\XML\SpeedLap_HUD.xml` returns
//! `no entry named ... (hash 1af0a646)`, the same answer Pulse's disc gives, so
//! speed lap draws the time trial's layout here too.
//!
//! **Written out rather than re-exported from `oag-pulse`.** A title package
//! states what its own disc ships; a table pointing at another title's constant
//! would say "the same as Pulse", which is a claim about Pulse rather than a
//! measurement of Pure, and it is precisely the shape that let every title be
//! served Pulse's HUD until finding S2 of the 2026-08-18 review. `oag_pure::race`
//! makes the same choice for `DEFAULT_TEAM`, with a test asserting the sharing
//! is deliberate.
//!
//! # No atlas, and no constant recording the absence
//!
//! Pure's HUD layouts name no texture at all - `oag_game::race::hud` reports it
//! per race and draws their sprites from `<Model>` geometry. So there is no
//! `ATLAS` here to match `oag_pulse::hud::ATLAS`, for the reason
//! [`crate::race`]'s docs give at length: an absent entry is a fact, and a
//! constant naming a file this disc does not carry would be an invention.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`oag_pulse::hud`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/pulse/src/hud.rs

/// The layout each mode this engine runs reads its HUD from, as
/// [`oag_title::Title::hud`] carries it.
pub const LAYOUTS: &oag_title::HudLayouts = &oag_title::HudLayouts {
    arcade: layouts::ARCADE,
    time_trial: layouts::TIME_TRIAL,
    speed_lap: layouts::TIME_TRIAL,
    zone: layouts::ZONE,
};

/// The three layouts this disc ships, by race mode.
pub mod layouts {
    /// Single race.
    pub const ARCADE: &str = r"Data\XML\Arcade_HUD.xml";
    /// Time trial, and speed lap with it - this disc ships no `SpeedLap_HUD.xml`.
    pub const TIME_TRIAL: &str = r"Data\XML\TimeTrial_HUD.xml";
    /// Zone mode.
    pub const ZONE: &str = r"Data\XML\Zone_HUD.xml";
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sharing with Pulse is deliberate and measured, not a leftover - the
    /// same assertion [`crate::race`] makes about the default team. A future
    /// reader should not "fix" these into a divergence, and should not collapse
    /// them into a re-export either: they are two discs that were both read.
    #[test]
    fn the_layouts_are_pulses_spellings_because_this_disc_carries_them() {
        assert_eq!(layouts::ARCADE, oag_pulse::hud::layouts::ARCADE);
        assert_eq!(layouts::TIME_TRIAL, oag_pulse::hud::layouts::TIME_TRIAL);
        assert_eq!(layouts::ZONE, oag_pulse::hud::layouts::ZONE);
        assert_eq!(
            LAYOUTS.speed_lap, LAYOUTS.time_trial,
            "no SpeedLap_HUD.xml is on this disc, so speed lap draws the time \
             trial's layout"
        );
    }
}
