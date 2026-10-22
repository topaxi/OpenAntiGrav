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
//! and nothing skipped (`crates/raceplay/tests/omega_hud_ground_truth.rs`).
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
    skins: SKINS,
    ..BASE
};

/// The disc's `HUD Style` list (`HD`, `2097`, `WIP3OUT`; default `HD`) with the roots
/// Omega's race managers load for each.
///
/// **Read off `eboot.bin`'s `*_HUD.xml` literals** (`strings`, 2026-10-16): unlike HD's,
/// Omega names all five modes in both retro skins, `Elimination` included, and the
/// skinned split pairs of both cuts, Eliminator pairs too. Mode-for-mode this is the
/// shape `RaceManager_ConstructArcadeHud` reads for Arcade
/// (`docs/ghidra/functions/ps4-omega-eu/race-hud-selection.md`); the other managers'
/// branches are read off the literals alone, **confidence 70**. The loyalty figures are
/// the `PI_HUD` rows' (`definition.xml`, base and patch), the same as HD's.
pub const SKINS: &[oag_title::HudSkin] = &[
    oag_title::HudSkin {
        name: "HD",
        layouts: &BASE,
        font_role: None,
        art: None,
        loyalty: 0,
    },
    oag_title::HudSkin {
        name: "2097",
        layouts: &oag_title::HudLayouts {
            arcade: r"Data\XML\2097_HUD\Arcade_HUD.xml",
            time_trial: r"Data\XML\2097_HUD\TimeTrial_HUD.xml",
            speed_lap: r"Data\XML\2097_HUD\SpeedLap_HUD.xml",
            zone: r"Data\XML\2097_HUD\Zone_HUD.xml",
            elimination: r"Data\XML\2097_HUD\Elimination_HUD.xml",
            split: Some(SPLIT_2097),
            ..BASE
        },
        font_role: Some("2097HUD"),
        art: Some(&RETRO_ART),
        loyalty: 6000,
    },
    oag_title::HudSkin {
        name: "WIP3OUT",
        layouts: &oag_title::HudLayouts {
            arcade: r"Data\XML\wo3_HUD\Arcade_HUD.xml",
            time_trial: r"Data\XML\wo3_HUD\TimeTrial_HUD.xml",
            speed_lap: r"Data\XML\wo3_HUD\SpeedLap_HUD.xml",
            zone: r"Data\XML\wo3_HUD\Zone_HUD.xml",
            elimination: r"Data\XML\wo3_HUD\Elimination_HUD.xml",
            split: Some(SPLIT_WO3),
            ..BASE
        },
        font_role: Some("wo3HUD"),
        art: Some(&RETRO_ART),
        loyalty: 10000,
    },
];

/// [`ART`] for the retro skins: the same sprite, font and sight rules, without the default
/// skin's runtime shield and lap-arc readings, which the retro layouts do not author
/// (`oag_title::HudSkin::art`).
const RETRO_ART: oag_title::HudArt = oag_title::HudArt {
    runtime: None,
    lap_one_shift: None,
    lit_alpha: None,
    // Unmeasured on the retro skins (`hd-hud-leftovers` item 3 asks it): no shadow, authored quads.
    text_shadow: None,
    half_unit_glyph_quads: false,
    bar_tip: None,
    boost_smear: None,
    ..*ART
};

/// The default skin's roots with no skin list, what every other skin overrides.
const BASE: oag_title::HudLayouts = oag_title::HudLayouts {
    arcade: r"Data\XML\Arcade_HUD.xml",
    time_trial: r"Data\XML\TimeTrial_HUD.xml",
    speed_lap: r"Data\XML\SpeedLap_HUD.xml",
    zone: r"Data\XML\Zone_HUD.xml",
    elimination: r"Data\XML\Elimination_HUD.xml",
    split: Some(SPLIT),
    split_layout: None,
    skins: &[],
};

/// The 2097 skin's split pairs, both cuts.
const SPLIT_2097: oag_title::hud::SplitLayouts = oag_title::hud::SplitLayouts {
    arcade: [
        "Data/xml/2097_hud/splitscreen_hud/splitscreen_hud_0.xml",
        "Data/xml/2097_hud/splitscreen_hud/splitscreen_hud_1.xml",
    ],
    elimination: [
        "Data/xml/2097_hud/splitscreen_hud/splitscreen_elimination_hud_0.xml",
        "Data/xml/2097_hud/splitscreen_hud/splitscreen_elimination_hud_1.xml",
    ],
    view_height: 540,
    vertical: Some(oag_title::hud::VerticalLayouts {
        arcade: [
            "Data/xml/2097_hud/splitscreen_hud/vert_splitscreen_hud_0.xml",
            "Data/xml/2097_hud/splitscreen_hud/vert_splitscreen_hud_1.xml",
        ],
        elimination: [
            "Data/xml/2097_hud/splitscreen_hud/vert_splitscreen_elimination_hud_0.xml",
            "Data/xml/2097_hud/splitscreen_hud/vert_splitscreen_elimination_hud_1.xml",
        ],
        view_width: 960,
    }),
};

/// The Wipeout 3 skin's split pairs, both cuts.
const SPLIT_WO3: oag_title::hud::SplitLayouts = oag_title::hud::SplitLayouts {
    arcade: [
        "Data/xml/wo3_hud/splitscreen_hud/splitscreen_hud_0.xml",
        "Data/xml/wo3_hud/splitscreen_hud/splitscreen_hud_1.xml",
    ],
    elimination: [
        "Data/xml/wo3_hud/splitscreen_hud/splitscreen_elimination_hud_0.xml",
        "Data/xml/wo3_hud/splitscreen_hud/splitscreen_elimination_hud_1.xml",
    ],
    view_height: 540,
    vertical: Some(oag_title::hud::VerticalLayouts {
        arcade: [
            "Data/xml/wo3_hud/splitscreen_hud/vert_splitscreen_hud_0.xml",
            "Data/xml/wo3_hud/splitscreen_hud/vert_splitscreen_hud_1.xml",
        ],
        elimination: [
            "Data/xml/wo3_hud/splitscreen_hud/vert_splitscreen_elimination_hud_0.xml",
            "Data/xml/wo3_hud/splitscreen_hud/vert_splitscreen_elimination_hud_1.xml",
        ],
        view_width: 960,
    }),
};

/// The two-player split-screen layouts Omega ships, the same four files HD and 2048
/// ship (`splitscreen_hud_{0,1}.xml` and `splitscreen_elimination_hud_{0,1}.xml`): one
/// 1920x1080 layout per player, the second the first shifted 540 down. **Wired the
/// way HD's are** (`oag_hd::hud::SPLIT`), whose HUD grid Omega shares; the row is
/// **ported**, and looked at on 2026-10-13: a two-view Single Race on
/// `data/extracted/ps4` draws each player's own layout in its half. The four roots
/// are also checked to compose to the sprites HD's do, in
/// `crates/game/tests/split_hud_ground_truth.rs`.
pub const SPLIT: oag_title::hud::SplitLayouts = oag_title::hud::SplitLayouts {
    arcade: [
        "Data/xml/splitscreen_hud/splitscreen_hud_0.xml",
        "Data/xml/splitscreen_hud/splitscreen_hud_1.xml",
    ],
    elimination: [
        "Data/xml/splitscreen_hud/splitscreen_elimination_hud_0.xml",
        "Data/xml/splitscreen_hud/splitscreen_elimination_hud_1.xml",
    ],
    view_height: 540,
    // Omega ships HD's `vert_` files in `data00`, and its race box authors the cut that
    // chooses them (`SplitScreen`, `RB_VERT`): the same four names, the same 960 column.
    vertical: Some(oag_title::hud::VerticalLayouts {
        arcade: [
            "Data/xml/splitscreen_hud/vert_splitscreen_hud_0.xml",
            "Data/xml/splitscreen_hud/vert_splitscreen_hud_1.xml",
        ],
        elimination: [
            "Data/xml/splitscreen_hud/vert_splitscreen_elimination_hud_0.xml",
            "Data/xml/splitscreen_hud/vert_splitscreen_elimination_hud_1.xml",
        ],
        view_width: 960,
    }),
};

/// How Omega's HUD sprites reach the screen, [`oag_title::Title::hud_art`].
///
/// **Measured**: `texture_extension` (every atlas of the five roots resolves
/// as `.gnf`, none under its authored spelling) and the layouts themselves.
/// **Chosen, not measured, inherited from HD's frame readings**: `always_on`,
/// `sights` (the widget names are checked to exist in Omega's layouts),
/// `runtime` and `shield_percent` - nobody has run Omega. `message_slots` and
/// `lap_hud` are read off its own executable (see their comments).
/// `zone_speed_classes` is `None`: Omega's class names are unmeasured, so the
/// Zone ladder's class text draws nothing rather than HD's list.
pub const ART: &oag_title::HudArt = &oag_title::HudArt {
    texture_extension: Some(".gnf"),
    always_on: ALWAYS_ON,
    raster: false,
    ink_aligned_text: true,
    half_unit_glyph_quads: true,
    // Inherited from HD: Omega draws HD's own HUD files through HD's engine, and the draw code is
    // not checkable (PS4 build, no capture path).
    lit_alpha: Some(oag_title::pre_race::Sourced::inherited(
        oag_title::hud::LitAlpha { byte: 0xCC },
        "hd",
    )),
    // Inherited from HD, with the lit alpha above.
    text_shadow: Some(oag_title::pre_race::Sourced::inherited(
        oag_title::hud::TextShadow { dx: -1, dy: -1 },
        "hd",
    )),
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
    bar_models: None,
    time_ladder_captions: None,
    pickup_icon_uv: None,
    zone_speed_classes: None,
    shield_percent: false,
    speed_full_kmh: 700,
    hud_font_role: "HUD",
    hud_small_font_role: Some("HUDSmall"),
    total_time_timed_modes_only: false,
    kill_column: false,
    message_slots: true,
    // **Measured** in the PS4 executable (`/omega/eboot-ps4-omega-eu.bin`, inside the
    // `0x0125e9b0` HUD update, read 2026-10-12): the message drain at
    // `0x01269f64`-`0x0126a1af` is HD's `0x91d20` shape (flag word at status `+0x138`,
    // slots of `0x18` bytes, the `100.0` waiting sentinel, `IG_HUD_PLAP`/
    // `NEWLAP_REC`/`FIN_LAP` and the new-lap-record line skipped in mode 5), and the
    // gap display at `0x0126812a`-`0x01268398` is HD's `0x907b0` (count-up with no
    // live-craft test in mode 5 and 10, the same four colours, a 3.5 s hold, no place
    // stop). **Chosen, not measured, from HD**: the split table (unread), so
    // `stamps_need_two_live`, one split a lap and a 100-crossing table. See
    // `docs/ghidra/functions/ps4-omega-eu/hud-message-triggers.md`.
    lap_hud: Some(oag_title::lap_hud::LapHud {
        perfect_lap: "IG_HUD_PLAP",
        new_lap_record: "IG_HUD_NEWLAP_REC",
        final_lap: "IG_HUD_FIN_LAP",
        record_line_in_time_trial: false,
        // Omega names `RACE_Perfect_lap`, a Wwise event; its banks are not `SBlk`.
        voices: oag_title::lap_hud::LapVoices::NONE,
        speed_lap_medal: None,
        tint: oag_title::lap_hud::MessageTint::GoodBad,
        gap: oag_title::lap_hud::SplitGapRule {
            text: oag_title::lap_hud::GapText::Signed,
            green_hold_needs_first_place: false,
            count_up_alone_in_timed_modes: true,
            stamps_need_two_live: true,
            splits_per_lap: 1,
            table_crossings: 100,
        },
    }),
    runtime: Some(RUNTIME),
    lap_one_shift: None,
    bar_tip: None,
    boost_smear: None,
    // Inherited from HD: Omega's race HUD is HD's own files, whose roots are `<Screen name="HUD">`
    // (read off `data00.psarc`, both the default and the `2048_hud` sets). The consumer in the
    // PS4 executable is not located, and no PS4 emulator can check it.
    sway: Some(oag_title::pre_race::Sourced::inherited(
        oag_title::hud::HudSway { x_per_lean: -15 },
        "hd",
    )),
    // Checked 2026-10-21: the patch front end (`data09.psarc`) authors no `Safe Area Setting`.
    safe_area: None,
    // The four widgets are Omega's own: `Data\XML\HUD_assist_indicator.xml`
    // (`HUD_Elim_assist_indicator.xml` in Elimination) is included by Arcade,
    // TimeTrial, SpeedLap and Elimination, with HD's names and nesting, so no
    // fragment is spliced. Zone's root carries none, as HD's does not. The
    // readout rule (3 s, 1.5 s, 4 Hz) is HD's and 2048's shared function. See
    // `docs/physics/pilot-assist.md`.
    resume_countdown: oag_title::hud::ResumeCountdown::Text,
    assist: Some(oag_title::hud::AssistIndicator {
        background: "AssistIndicatorBG",
        main: "AssistIndicatorMain",
        left: &["AssistIndicatorLeft", "AssistIndicatorLeftBig"],
        right: &["AssistIndicatorRight", "AssistIndicatorRightBig"],
        fragment: None,
    }),
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
    // `Hud_UpdatePickup` (`0x0008df30`) resets `PickupBackground`, `PickupDamageBG` and
    // `PickupAbsorbBG` to white each tick and writes `0x5affffff` over them with nothing to
    // show (hud-readouts.md, "The pickup bar's idle dim"). Omega inherits HD's.
    pickup_idle: Some(oag_title::hud::PickupIdle {
        backdrop: "PickupBackground",
        hit_pills: &["PickupDamageBG", "PickupAbsorbBG"],
        argb: 0x5aff_ffff,
    }),
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
    "SpeedBar",
    "ThrustBar",
    "SpeedBarOverlay",
    // The retro `HUD Style` skins' bars and their frames (`2097_hud`, `wo3_hud`); a name a
    // layout does not author is simply not found, so the default skin is unchanged.
    "speedbarbg",
    "topdots",
    "bottomdots",
    "nrgbg",
    "SpeedBarWo3",
    "ThrustBarWo3",
    "nrgBarWo3",
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
