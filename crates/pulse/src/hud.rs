//! The HUD's atlas and its five layouts.
//!
//! Entry names, in the [ADR-0022] sense: which files Pulse ships and what is in
//! each. Everything that *reads* them - the `<Image>`/`<Text>` widget model, the
//! `<Item>` offset handling, the draw list - is `oag_hud` and stays there,
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
    elimination: layouts::ELIMINATION,
};

/// How Pulse's HUD sprites reach the screen, as [`oag_title::Title::hud_art`]
/// carries it.
///
/// All three rows are Pulse's own answers, which until 2026-08-25 were `const`s
/// inside `oag_hud` that every title was served. See
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
        // The LeachBeam's own four, all four instancing
        // `Data\HUD\leachbeam_sight.vex` - a hollow arrowhead - and bound by the
        // same slot run at `+0x108` … `+0x114`. No inner: only the Missile gets
        // a closed box.
        leach: Some([
            "leachbeam_sight_1",
            "leachbeam_sight_2",
            "leachbeam_sight_3",
            "leachbeam_sight_4",
        ]),
    },
    pickup_backdrop_colour: Some(PICKUP_BACKDROP_COLOUR),
    pickup_colours: Some(PICKUP_COLOURS),
    // Pulse's icons are `<Image>` sprites (`pickup_icon_name`'s `<Type>Icon`),
    // not `<Mode3D><Model>`s - this field is Pure's own dialect.
    pickup_icon_models: None,
    pickup_icon_backdrop_model: None,
    // Pulse names a widget per weapon (`pickup_icon_name`) - this field is
    // 2048's own dialect, one widget with a rewritten UV.
    pickup_icon_uv: None,
    // `None`: no Zone speed-class ladder has been read on this title.
    zone_speed_classes: None,
    // Pulse's own reference frame reads `100%`. See `oag_title::HudArt::shield_percent`.
    shield_percent: true,
    // Measured: every one of Pulse's own language plugins names
    // `<Font><Values name="HUD" ...Src="...\PulseHud.fnt">` and `HUDSmall`
    // -> `small.fnt` - see `oag_ui::language::roles`' own doc table (role
    // spelling is the format's, not one plugin's; `PI012` is English only on
    // the USA pressing per `oag_ui::language`'s module doc, so this cites the
    // role rather than one plugin id). See `oag_title::HudArt::hud_font_role`.
    hud_font_role: "HUD",
    hud_small_font_role: Some("HUDSmall"),
    // Measured: `PlayerStatus_Update` leaves the clock target at -1 outside Time Trial,
    // Speed Lap, Free Play and Multiplayer Time Trial, and `Hud_UpdateTimeCluster` hides
    // both widgets on it. Seen on a live Eliminator frame. See
    // `oag_title::HudArt::total_time_timed_modes_only`.
    total_time_timed_modes_only: true,
    kill_column: true,
    message_slots: true,
    runtime: None,
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
/// A `const` in `oag_hud` until 2026-08-25, applied to every title. HD
/// authors `PickupBackground` as a hexagon **outline** rather than a filled
/// one, so its icon is legible in the colour its own layout gives it, and
/// Pulse's substitution turned HD's backdrop into a quarter-alpha smudge. See
/// [`oag_title::HudArt::pickup_backdrop_colour`].
///
/// Recorded in `docs/gameplay/pickups.md`. Reference frames of this game's own
/// pickup box have now been taken - see [`PICKUP_COLOURS`], which draws the
/// backdrop close to the colour those frames show, for the eleven weapons they
/// settled the *category* of. This constant is what the remaining two
/// (`Bomb`, `Mine`) still fall back to.
pub const PICKUP_BACKDROP_COLOUR: &str = "HudBGColour";

/// The pickup backdrop's own colour, per weapon - what [`PICKUP_BACKDROP_COLOUR`]
/// was a placeholder for.
///
/// # Four frames, not a guess at one
///
/// A PSP under Xvfb (`Xvfb :97`, `PPSSPPSDL --appendconfig=/tmp/debugger.ini`,
/// `pulse-psp-usa.chd`, 2026-09-04 - see
/// `docs/reverse-engineering/ppsspp-debugger.md`) was driven onto Talon's
/// Junction's own weapon pads (`scripts/psp-drive.py place --pad-class
/// weapon`) and, once, sat on its Time Trial start line - which this disc's
/// own event text says hands out a free `Turbo` every lap, `Turbo` being the
/// one pickup a placement cannot be aimed at - and screenshotted (`import
/// -window root`) while holding what it picked up. Four frames: `ShieldIcon`
/// and `AutopilotIcon` on green, `MissileIcon` on magenta, `TurboIcon` on a
/// third, brighter-looking green shot against open sky rather than a tunnel
/// interior. Each colour below is the dominant cluster of the hexagon's
/// interior pixels, away from the glyph and the antialiased edge, from the
/// two `Shield`/`Autopilot` frames - not a single sample, and not the
/// `Turbo` frame, for the reason its own paragraph gives.
///
/// # Eleven weapons share two colours, and the other two are named
///
/// `Data\Plugins\loading\Definition.xml` points every weapon's *loading
/// screen* tip at its own `Data\Defaults\Loading\Pulse\<Name>.mip`, and each
/// of those authors the same hexagon-and-glyph picture the HUD does - same
/// glyph shapes, confirmed by eye against `PulseHUD.mip`'s own thirteen. Its
/// hexagon is a **flat, unblurred fill**, and sampling all thirteen files
/// (`oag-wad cat 0x<hash>`, hashed by `oag_formats::wad::hash_name`) turns up
/// exactly **three** distinct fills: `Bomb` and `Mine` are `(0, 0, 255)`,
/// eight of the rest are `(250, 1, 189)`, and `AutoPilot`/`Shield`/`Turbo` are
/// `(0, 255, 0)`. That is a category, not thirteen independent authored
/// colours, and it is the *only* source for the grouping below - nothing here
/// has an in-race frame of `Rocket`, `Quake`, `Cannon`, `Plasma`, `LeachBeam`,
/// `Repulser` or `Shuriken`, only of the two other members of their pink
/// category.
///
/// # The `Turbo` frame reads differently, and why is not established
///
/// The `Shield`/`Autopilot` pair and the `Missile` frame all read as an
/// interior flat enough to look opaque, and an early version of this comment
/// called that settled. **The `Turbo` frame does not match them**: shot
/// against bright sky rather than a dark tunnel, its hexagon reads `(62, 177,
/// 120)` - a real blue channel where the other two green frames read
/// `22`-`47`. One ruled-out explanation and two still-live ones:
///
/// - **Ruled out: a whole-frame exposure or tonemap difference.** The same
///   screenshot's `TimeIcon` - a widget authored `Color="FEConst->
///   HudColour2"`, `0xFF7DEFC0` - samples `(127, 244, 195)` against the
///   declared `(125, 239, 192)`, within antialiasing noise. A widget with no
///   substitution reads correctly, unscaled, in the very frame the pickup
///   backdrop reads high - so this is not the whole HUD being brighter that
///   frame.
/// - **Ruled out: a different layout.** `TimeTrial_HUD.xml` (this frame's
///   own layout, `Turbo` being Time Trial's own free-lap grant per
///   `docs/gameplay/pickups.md`) authors `PickupBackground` and `TurboIcon`
///   with the identical `x`, `y`, size, `U`/`V` and `Color="FEConst->
///   HudColour1"` that `Arcade_HUD.xml` does - checked directly, not
///   assumed.
/// - **Still live: the backdrop blends with the scene behind it.** A
///   translucent hexagon reads brighter over open sky than over a dark
///   tunnel, which is the shape of what was measured.
/// - **Still live: `Turbo`'s own runtime colour genuinely differs from
///   `Shield`/`Autopilot`'s**, despite the loading screen filing all three
///   under one `(0, 255, 0)`. The loading screen is independently authored
///   art (see the ratio note below) and settles the *category*, not that
///   every member of it is byte-identical at runtime.
///
/// Neither live hypothesis is confirmed: a single shared alpha checked
/// against all four frames' own nearby-pixel background estimates does not
/// fit cleanly on one value, on any channel weighting tried, which could mean
/// there is no single alpha, or could mean a screen-space "nearby" pixel is
/// not what is actually occluded and every estimate here is too coarse to
/// solve for one. The loading screen's own fill running brighter than every
/// in-race frame (Missile's red `123/250 ≈ 0.49`, its blue `44/189 ≈ 0.49`;
/// Shield's green `108/255 ≈ 0.42`) is consistent with either hypothesis too.
/// So this table draws each colour **opaque**, from the two frames that agree
/// with each other (`Shield`, `Autopilot`) rather than from an average that
/// would let the `Turbo` anomaly quietly move the number - which is not a
/// claim that the original draws it opaque, only the simplest thing this
/// build can draw without picking between two unconfirmed explanations.
///
/// # Bomb and Mine are the one category with no frame at all
///
/// Neither weapon pad reachable from Talon's Junction's own start line handed
/// out an explosive in the placements taken. An earlier version of this table
/// extrapolated `(0, 0, 125)` for the pair by scaling the loading screen's
/// `(0, 0, 255)` by the ~0.49 ratio measured above - and that scaling is
/// exactly the relationship the paragraph above just found does not hold
/// cleanly even for the two categories it was measured on, so extrapolating
/// it a third time to a category with no frame at all compounds an already
/// shaky number. **This build draws no colour for `Bomb`/`Mine`** and falls
/// back to [`PICKUP_BACKDROP_COLOUR`] instead, rather than ship a number
/// nothing here has seen and this same table just found reason to doubt.
///
/// Confidence **70** for `GREEN`: two frames of two different weapons
/// (`Shield`, `Autopilot`) agreeing closely, on the real disc, sampled rather
/// than eyeballed - docked below the 90s a clean measurement earns because a
/// third frame of a third category member (`Turbo`) does not agree with them
/// and why is unresolved, per the section above. Confidence **60** for
/// `PINK`: the same disc, the same method, but one frame of one weapon
/// (`Missile`) standing for all eight members its loading-screen category
/// carries, with no second frame to check it against the way `GREEN` has.
/// Both are opaque approximations rather than a solved blend - see above.
///
/// Open: a weapon pad placement that comes up `Bomb` or `Mine`; a second
/// frame each of `Rocket`, `Quake`, `Cannon`, `Plasma`, `LeachBeam`,
/// `Repulser` or `Shuriken` to check against `Missile`'s; and a capture
/// harness that can read the pixel actually occluded by the hexagon rather
/// than estimate it from a screen-space neighbour, which is what the `Turbo`
/// anomaly above needs to resolve either way. See `docs/gameplay/pickups.md`.
///
/// Positional rather than keyed by `oag_tables::weapons::Weapon`: this crate
/// is deliberately tables only, with no non-test edge to `oag-formats` (see
/// this file's `Cargo.toml`), so the index is a data contract with
/// `oag_hud` - which does own that dependency - the same way
/// [`oag_title::HudArt::pickup_colours`] documents it.
pub const PICKUP_COLOURS: [Option<u32>; 14] = {
    // Values are `0xAARRGGBB`, drawn opaque - see this constant's own doc
    // comment for why that is a simplification and not a measurement of the
    // backdrop's real alpha.
    const PINK: Option<u32> = Some(0xFF7B_2D5D);
    const GREEN: Option<u32> = Some(0xFF1A_7021);
    // `Bomb`/`Mine` have no frame at all - see this constant's own doc
    // comment - so this category falls back to `PICKUP_BACKDROP_COLOUR`.
    const BLUE: Option<u32> = None;
    [
        PINK,  // Rocket
        PINK,  // Missile
        PINK,  // Quake
        PINK,  // Cannon
        GREEN, // Turbo
        GREEN, // Shield
        GREEN, // Autopilot
        PINK,  // Plasma
        BLUE,  // Bomb
        BLUE,  // Mine
        PINK,  // LeachBeam
        PINK,  // Repulser
        PINK,  // Shuriken
        // Pure's weapon; no Pulse table authors it, so no Pulse pad ever
        // hands one out and no frame colour was ever measured for it.
        None, // Disruptor
    ]
};

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
