//! Omega's in-race HUD: HD's eighteen-layout dialect, read off Omega's own
//! `Data\XML\*_HUD.xml` and `.gnf` atlases.
//!
//! # What this module replaces
//!
//! Until 2026-10-06 this module said Omega ships no per-mode composition and
//! carried one unread placeholder entry. **That was wrong, and the cause was a
//! truncated census**: `scripts/psarc.py` split a PS4 manifest on `\n`, but a
//! PS4 `.psarc` separates paths with NUL, so its listing came back as one
//! giant entry and a search for `arcade_hud.xml` found nothing. `data00.psarc`
//! ships the whole HD root set (`Data\XML\Arcade_HUD.xml`, `Elimination_HUD.xml`,
//! `TimeTrial_HUD.xml`, `SpeedLap_HUD.xml`, `Zone_HUD.xml`), and each composes
//! from the same file counts HD's does (17, 15, 14, 15, 5) with nothing missing
//! and nothing skipped (`crates/game/tests/omega_hud_ground_truth.rs`).
//!
//! # Which set a race reads: measured for the single-player managers
//!
//! `eboot.bin` (Ghidra `/omega/eboot-ps4-omega-eu.bin`) names these literals,
//! and the data xrefs from the single-player race managers are:
//!
//! | Layout | Referenced by |
//! | --- | --- |
//! | `Data\XML\Arcade_HUD.xml` | `RaceManager_ConstructArcadeHud` (`0x0129c58f`), the default branch of its "HUD Style" check; also `DemoRaceManager_Construct`, `MPTimeTrialRaceManager_Construct` |
//! | `Data\XML\TimeTrial_HUD.xml` | `SPTimeTrialRaceManager_Construct`, `SPFreePlayRaceManager_Construct`, `AIBatchRaceManager_Construct` |
//! | `Data\XML\SpeedLap_HUD.xml` | `SPTimeTrialRaceManager_Construct` (both files, picked by a runtime flag, as on 2048) |
//! | `Data\XML\Zone_HUD.xml` | `SPZoneRaceManager_Construct` |
//! | `Data\XML\Elimination_HUD.xml` | `SPEliminationRaceManager_Construct`, `MPEliminationRaceManager_Construct` |
//!
//! The "HUD Style" setting chooses `wo3_HUD\` (value `WIP3OUT`), `2097_HUD\`
//! or the bare root; the bare root is the default and is what this table
//! reads, as HD's does. **The `2048_hud\` set is reached through virtual
//! methods only** (`FUN_015ae4d0`, `FUN_015ac880`, no direct caller), which is
//! this title's 2048-lineage managers; which race selects them is not read, so
//! it is an open item and `Title::hud` stays one set per title, not per circuit
//! (compare `oag_omega::race::DEFAULTS`' particle directory, chosen the same
//! way). See `docs/ghidra/functions/ps4-omega-eu/race-hud-selection.md`.
//!
//! # Art
//!
//! Every atlas a root layout names (`.gtf`/`.mip` in the XML) ships as `.gnf`
//! under `Data\HUD\Textures\`, BC7, and the layouts are authored in Omega's own
//! pixel sizes (`fury_hud` is 2048 square here against HD's 1024, and the XML's
//! UVs reach 2010 against HD's 1005). **The rows are top-down: decoded straight
//! out of `gnf::Texture::decode` they equal HD's `.gtf` as `oag_hud::sprite`
//! draws it (reversed), mean absolute difference 0.02-0.7 of 255.** So unlike
//! the eight front-end `.gnf` of `FrontEnd::bottom_up_gnf`, no HUD atlas needs a
//! row reversal; see `docs/formats/omega-status.md`.

/// The layout each mode reads, [`oag_title::Title::hud`].
pub const LAYOUTS: &oag_title::HudLayouts = &oag_title::HudLayouts {
    arcade: r"Data\XML\Arcade_HUD.xml",
    time_trial: r"Data\XML\TimeTrial_HUD.xml",
    speed_lap: r"Data\XML\SpeedLap_HUD.xml",
    zone: r"Data\XML\Zone_HUD.xml",
    elimination: r"Data\XML\Elimination_HUD.xml",
};

/// How Omega's HUD sprites reach the screen, [`oag_title::Title::hud_art`].
///
/// **Measured**: `texture_extension` (every atlas of the five roots resolves
/// as `.gnf`, none under its authored spelling) and the layouts themselves.
/// **Chosen, not measured, inherited from HD's frame readings**: `always_on`,
/// `sights` (the widget names are checked to exist in Omega's layouts),
/// `runtime`, `shield_percent` and `message_slots` - nobody has run Omega.
/// `zone_speed_classes` is `None`: Omega's class names are unmeasured, so the
/// Zone ladder's class text draws nothing rather than HD's list.
pub const ART: &oag_title::HudArt = &oag_title::HudArt {
    texture_extension: Some(".gnf"),
    always_on: ALWAYS_ON,
    raster: false,
    sights: &oag_title::hud::Sights::Concentric {
        seeking: &[
            "MissileSightBG",
            "MissileSightOuter",
            "MissileSightInner",
            "MissileSightMiddle",
        ],
        locked: &["MissileSightLockedOnLines", "MissileSightLockedOnMiddle"],
        leach: Some([
            "LeachBeamSightBG",
            "LeachBeamSightOuter",
            "LeachBeamSightMiddle",
            "LeachBeamSightInner",
        ]),
    },
    pickup_backdrop_colour: None,
    pickup_colours: None,
    pickup_icon_models: None,
    pickup_icon_backdrop_model: None,
    pickup_icon_uv: None,
    zone_speed_classes: None,
    shield_percent: false,
    hud_font_role: "HUD",
    hud_small_font_role: Some("HUDSmall"),
    total_time_timed_modes_only: false,
    kill_column: false,
    message_slots: true,
    runtime: Some(RUNTIME),
};

const RUNTIME: &oag_title::hud::RuntimeHud = &oag_title::hud::RuntimeHud {
    shield: oag_title::hud::ShieldReadout {
        fill: "DamageBar",
        background: "DamageBarBg",
        text: "ShieldBarText",
        rgb: 0x0016_64FF,
        warning_rgb: 0x00FF_0000,
        critical_percent: 20,
        phases_per_second: 8,
    },
    lap_arc: oag_title::hud::SegmentArc {
        prefix: "LapBar",
        segments: 7,
    },
    place_arc: oag_title::hud::SegmentArc {
        prefix: "PosBar",
        segments: 8,
    },
    // Indicator not wired; the assist itself runs off Omega's own `<PilotAssist>`
    // table. See `docs/physics/pilot-assist.md`.
    assist: None,
};

/// Widgets drawn in every frame whatever the race state, HD's list. **Chosen,
/// not measured**; see [`ART`].
const ALWAYS_ON: &[&str] = &[
    "LapPanel",
    "PositionPanel",
    "TotalTimeBG",
    "PickupFarBackground",
    "PickupAbsorbBG",
    "PickupDamageBG",
    "DamageBarBg",
    "PickupBackground",
    "TimeIcon",
    "TimeIconVertBar",
    "ClockIcon",
    "BestLapImage",
    "SpeedBarBG",
    "ThrustBarBG",
    "SpeedBarOverlay",
    // Zone's ladder. Authored by the Zone roots alone; see the section above.
    "ZoneBG",
    "CurrentZonePanel",
    "ZonePlusLight0",
    "ZonePlusLight1",
    "ZonePlusLight2",
    "ZonePlusLight3",
    "ZonePlusLight4",
    "ZonePlusLight5",
    "ZonePlusLight6",
    "ZonePlusLight7",
    "ZonePlusLight8",
    "ZonePlusLight9",
    "ZonePlusLight10",
];
