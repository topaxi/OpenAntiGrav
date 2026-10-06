//! Omega's in-race HUD, unread - see this module's own "why inert" note.
//!
//! Every other title's per-mode HUD is one composed XML document per mode
//! (`arcade_hud.xml`, `timetrial_hud.xml`, ... on HD; `2048_hud\*_HUD.xml` on
//! 2048). **Omega ships neither scheme where this lane looked.** `data00.psarc`
//! and `data08.psarc` carry no `arcade_hud.xml`/`timetrial_hud.xml`/
//! `speedlap_hud.xml`/`zone_hud.xml` at all; instead `Data\xml\` holds a set of
//! *fragment* files with no per-mode composition (`hud_damage_indicator.xml`,
//! `hud_pickups.xml`, `hud_proximity.xml`, `hud_sights.xml`,
//! `hud_ready_go.xml`, `hud_elim_lap_counters.xml`, `hud_elim_positions.xml`,
//! `hud_detonatorweapons.xml`) alongside `2048_hud\`/`2097_hud\`/`wo3_hud\`/
//! `splitscreen_hud\`/`splitscreenzone_hud\`/`duel_hud\` subtrees carrying
//! more fragments and VR-suffixed extensions. Working out which fragments
//! compose which mode's HUD is real reverse-engineering, out of scope for
//! this lane (the brief allows skipping this module "unless trivially copied
//! and verified" - this was neither).
//!
//! # Why the placeholder below is provably inert
//!
//! [`oag_title::Title::hud`]/[`Title::hud_art`] are not `Option`, so
//! [`crate::TITLE`] has to supply *something*. `crates/raceplay/src/hud.rs`
//! is the **only** reader of `Title::hud`'s fields anywhere in `oag-game`,
//! and it is reached exclusively from a race that has already started
//! loading. Omega's race path refuses by name before that point (see
//! `crates/omega/src/race.rs`'s own doc comment and the boot report this
//! crate's `frontend` module wires up), so this placeholder is never
//! dereferenced for anything this lane draws.

/// One real, archive-confirmed entry, repeated across every field
/// [`oag_title::HudLayouts`] requires - not a composed layout, and not a
/// claim that any of the five modes actually draws this file. See the
/// module doc.
const PLACEHOLDER: &str = r"Data\xml\hud_pickups.xml";

/// Placeholder HUD layout entries - see the module doc.
pub const LAYOUTS: &oag_title::HudLayouts = &oag_title::HudLayouts {
    arcade: PLACEHOLDER,
    time_trial: PLACEHOLDER,
    speed_lap: PLACEHOLDER,
    zone: PLACEHOLDER,
    elimination: PLACEHOLDER,
};

/// Placeholder HUD art rules - every field here is `None`/empty on the same
/// "measurement gap, not invented" terms `oag_2048::hud::ART`'s own
/// `always_on: &[]` already established as an accepted shape for an unread
/// HUD, not a shortcut new to this crate.
pub const ART: &oag_title::HudArt = &oag_title::HudArt {
    texture_extension: None,
    always_on: &[],
    raster: false,
    sights: &oag_title::hud::Sights::Unread,
    pickup_backdrop_colour: None,
    pickup_colours: None,
    pickup_icon_models: None,
    pickup_icon_backdrop_model: None,
    pickup_icon_uv: None,
    zone_speed_classes: None,
    // Pulse's reading, and the stated fallback for "every title that has not
    // had its own frame checked" - `oag_title::HudArt::shield_percent`'s own
    // doc comment.
    shield_percent: true,
    // Placeholder, on the same "provably inert" terms as every other field
    // in this constant - see the module doc. Not measured on Omega's own
    // plugins; carries the four-title shared literal rather than an empty
    // string so a future reader who does wire this up starts from the
    // common answer rather than from nothing.
    hud_font_role: "HUD",
    hud_small_font_role: Some("HUDSmall"),
    total_time_timed_modes_only: false,
    kill_column: false,
    message_slots: false,
    runtime: None,
};
