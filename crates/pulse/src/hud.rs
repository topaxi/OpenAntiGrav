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

/// How Pulse's HUD sprites reach the screen, as [`oag_title::Title::hud_art`]
/// carries it.
///
/// All three rows are Pulse's own answers, which until 2026-08-25 were `const`s
/// inside `oag_game::hud` that every title was served. See
/// [`oag_title::HudArt`] for what each row is and for HD's disagreement on all
/// three.
pub const ART: &oag_title::HudArt = &oag_title::HudArt {
    // The layouts name `.mip` and this disc carries `.mip`. The PS2 pressing's
    // `.pct` rewrite is `crate::read_image`'s job and applies to every source.
    texture_extension: None,
    always_on: ALWAYS_ON,
    // Four instances of one corner-bracket model plus the closed box at the
    // middle, bound by `HudSight_Bind` (`0x0881b604`) in this order - which is
    // also the order `sight::BRACKET_ROTATIONS` is indexed by. See
    // `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`.
    sights: &oag_title::hud::Sights::Brackets {
        brackets: [
            "missile_sight_1",
            "missile_sight_2",
            "missile_sight_3",
            "missile_sight_4",
        ],
        inner: "missile_sight_inner",
    },
    pickup_backdrop_colour: Some(PICKUP_BACKDROP_COLOUR),
};

/// The layout constant substituted for the pickup backdrop's authored colour.
///
/// # Why a substitution is needed at all
///
/// **Drawing the backdrop and its icon exactly as this disc authors them
/// produces an opaque white hexagon with the icon invisible inside it**, and
/// that is not a bug in the reader - it is what the shipped data says,
/// measured:
///
/// - `PickupBackground` samples a **filled hexagon whose alpha is 255** across
///   2,424 of its 2,492 opaque pixels, and `TurboIcon` samples a glyph in the
///   same white with antialiased edges. Both are pure white masks; all the
///   colour is meant to come from the tint.
/// - Both are authored `Color="FEConst->HudColour1"`, and `HudColour1` is
///   `0xFFFFFFFF` - **opaque white**.
///
/// So an opaque white hexagon is drawn and then an opaque white glyph is drawn
/// on top of it, and the second is invisible against the first. The original
/// plainly does not look like that, so **it must set at least one of the two
/// colours at runtime** - which is unrecovered. `0x0883b3b8` is the known place
/// the runtime reaches into these widgets (it forces the icon id) and is where
/// to look.
///
/// # What is substituted, and why this one
///
/// `HudBGColour`, the only background colour the layout defines - `0x40000000`,
/// a quarter-alpha black - applied to the backdrop while the icon keeps its
/// authored white. **Every value still comes off the player's own disc**; what
/// is ours is the choice of which constant, and it is flagged here rather than
/// hidden. The precedent is the front end's title colour, substituted the same
/// way and for the same reason while the widget behind it is unbuilt - see
/// `docs/ui/menus-original.md`.
///
/// # And it is Pulse's answer, not the dialect's
///
/// A `const` in `oag_game::hud` until 2026-08-25, applied to every title. HD
/// authors `PickupBackground` as a hexagon **outline** rather than a filled
/// one, so its icon is legible in the colour its own layout gives it, and
/// Pulse's substitution turned HD's backdrop into a quarter-alpha smudge. See
/// [`oag_title::HudArt::pickup_backdrop_colour`].
///
/// Recorded in `docs/gameplay/pickups.md`. A reference frame of *this* game's
/// own pickup box would settle it and has not been taken; HD's has.
pub const PICKUP_BACKDROP_COLOUR: &str = "HudBGColour";

/// The sprite widgets Pulse draws whenever its HUD is up.
///
/// The two bars, their backgrounds, their end marks and the time icon. What is
/// left in the layouts is a warning, an opponent tag or a mode-specific piece,
/// none of which has anything driving it - see `docs/ui/hud.md`. The pickup
/// widgets are the exception and are **not** here, because they are conditional
/// on what the craft is holding rather than always on.
pub const ALWAYS_ON: &[&str] = &[
    "SpeedBarBg",
    "ShieldBarBg",
    "SpeedBar",
    "ShieldBar",
    "SpeedBarMark",
    "ShieldBarMark",
    "TimeIcon",
];

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
