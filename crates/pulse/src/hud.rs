//! The HUD's atlas and its five layouts.
//!
//! Entry names, in the [ADR-0022] sense: which files Pulse ships and what is in
//! each. Everything that *reads* them - the `<Image>`/`<Text>` widget model, the
//! `<Item>` offset handling, the draw list - is `oag_game::hud` and stays there,
//! because a second title with a different atlas would reuse all of it.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The archive entry holding the atlas every HUD sprite samples.
///
/// One texture serves **84 of the 110** `Src=` references across the five
/// layouts; the other 26 are `<Mode3D>` models (`missile_sight_outer` x8,
/// `leachbeam_sight` x8, `Pulse_Ready_Go` x4, `Cockpit_321GO` x4,
/// `missile_sight_inner` x2). It ships **twice** on the PSP disc, in `FE.wad`
/// and in `Data.wad`, byte-identical at 66,576 bytes.
///
/// The counts read `84 of the 100` and `16` until 2026-08-09; 84 + 16 = 100 is
/// arithmetic that agrees with itself and not with the disc.
pub const ATLAS: &str = r"Data\HUD\Textures\PulseHUD.mip";

/// The layout each mode this engine runs reads its HUD from, as
/// [`oag_title::Title::hud`] carries it.
///
/// The constants below are the same values, kept as named items because that is
/// where their evidence is written down; this is the table the engine reads.
///
/// **`speed_lap` is the time trial's, and that is measured rather than assumed**:
/// this disc ships no `SpeedLap_HUD.xml` - the name hashes to `1af0a646` and no
/// entry carries it - which is why `docs/ui/hud.md` counts five layouts for six
/// modes. HD ships a separate one, which is the divergence that made
/// `oag_title::HudLayouts` a type.
pub const LAYOUTS: &oag_title::HudLayouts = &oag_title::HudLayouts {
    arcade: layouts::ARCADE,
    time_trial: layouts::TIME_TRIAL,
    speed_lap: layouts::TIME_TRIAL,
    zone: layouts::ZONE,
};

/// The five shipped layouts, by race mode.
///
/// Widget counts are `<Image>` plus `<Text>`, measured off the USA PSP disc and
/// pinned in `crates/game/tests/hud_layout_ground_truth.rs`. Most widgets are
/// inactive in any given frame, so these are sizes of the layout rather than of
/// what is on screen.
pub mod layouts {
    /// Single race and tournament: 35 images, 35 texts, 11 models.
    pub const ARCADE: &str = r"Data\XML\Arcade_HUD.xml";
    /// Eliminator: 33 images, 35 texts, 11 models.
    pub const ELIMINATION: &str = r"Data\XML\Elimination_HUD.xml";
    /// Time trial and speed lap: 10 images, 25 texts, 2 models.
    pub const TIME_TRIAL: &str = r"Data\XML\TimeTrial_HUD.xml";
    /// Zone mode: 7 images, 24 texts, 2 models.
    pub const ZONE: &str = r"Data\XML\Zone_HUD.xml";
    /// Multiplayer tag: no images, 8 texts, no models - the floating name tags
    /// and nothing else.
    pub const MP_TAG: &str = r"Data\XML\MPTag_HUD.xml";
}
