//! Wipeout 2048's in-race HUD layout entries.
//!
//! Entry names in the [ADR-0022] sense - *which* files this package ships -
//! the same division [`oag_hd::hud`] draws. Everything that reads them is
//! `oag_game::hud`.
//!
//! # Four skin sets, and the root one is the default
//!
//! The archive holds 26 `*_HUD.xml` documents in four groups: five directly
//! under `Data\XML\`, and one set each under `2048_hud\`, `2097_hud\` and
//! `wo3_hud\`. That is Wipeout HD's own shape with one added - HD ships the
//! same root set plus `wo3_hud` and `2097_hud` as *skins* - so the root set is
//! read as the default here for the same reason it is there.
//!
//! **Which skin 2048 selects, and on what, is unread**, and `2048_hud\` being
//! the one named after this title is exactly the kind of coincidence that
//! looks like an answer. It is recorded and not acted on. Nothing here has
//! opened any of the four.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`oag_hd::hud`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/hud.rs

/// The layout each mode this engine runs reads its HUD from, as
/// [`oag_title::Title::hud`] carries it.
pub const LAYOUTS: &oag_title::HudLayouts = &oag_title::HudLayouts {
    arcade: layouts::ARCADE,
    time_trial: layouts::TIME_TRIAL,
    speed_lap: layouts::SPEED_LAP,
    zone: layouts::ZONE,
};

/// How 2048's HUD sprites reach the screen, as [`oag_title::Title::hud_art`]
/// carries it.
///
/// **Two of the three rows are honestly unread.** No layout here has been
/// composed, so nothing is known about which widgets are up whenever the HUD
/// is or how its reticle is built - and copying HD's answers, which would
/// compose to *something*, is how a wrong picture survives review.
pub const ART: &oag_title::HudArt = &oag_title::HudArt {
    // **Measured.** The Vita's texture container is `.gxt`, and the archive
    // carries `Data\HUD\Textures\missile_reticule.gxt` under the same stem
    // HD's layouts name - so the substitution rule is the same one HD needs,
    // with a different extension on the end.
    texture_extension: Some(TEXTURE_EXTENSION),
    always_on: ALWAYS_ON,
    sights: &oag_title::hud::Sights::Unread,
    // `None` is "draw what the layout authors", which is the conservative
    // answer for a title whose layouts nothing has opened - Pulse's colour
    // substitution is the one that would need evidence.
    pickup_backdrop_colour: None,
};

/// What a layout's texture reference becomes on this title.
///
/// See [`ART`]; `oag_title::hud::with_extension` is the rule that applies it.
pub const TEXTURE_EXTENSION: &str = "gxt";

/// The sprite widgets 2048 draws whenever its HUD is up: **unread**.
///
/// Empty because no layout of this title has been composed, *not* because a
/// composition came back with nothing - which is what
/// `oag_pure::hud::ALWAYS_ON`'s emptiness means and is a different claim.
pub const ALWAYS_ON: &[&str] = &[];

/// The layout entries themselves.
pub mod layouts {
    /// Single race.
    pub const ARCADE: &str = r"Data\XML\Arcade_HUD.xml";
    /// Time trial.
    pub const TIME_TRIAL: &str = r"Data\XML\TimeTrial_HUD.xml";
    /// Speed lap - a file of its own here, unlike on either PSP title.
    pub const SPEED_LAP: &str = r"Data\XML\SpeedLap_HUD.xml";
    /// Zone.
    pub const ZONE: &str = r"Data\XML\Zone_HUD.xml";
}
