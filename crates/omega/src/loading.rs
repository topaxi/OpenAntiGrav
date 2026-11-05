//! Omega's loading screen: HD's screen, carried forward and re-spelt for the PS4's `.gnf`.
//!
//! The patch `eboot.bin`'s string pool holds one run that is this screen's asset list, in the
//! order HD's holds its own (`docs/formats/omega-loading.md`): `FE_LOADINGDOT`, the four region
//! labels (`FE_MODE_ICON`, `FE_PROG_BAR`, `FE_FEATURE_IMAGE`, `FE_FEATURE_DESC`), the feature
//! ids (`MAN_2_BR`, `FE_BR_INST`, `IG_HUD_ABSORB`, `FE_ABSORB_INST`, `FE_FLIP`, `FE_FLIP_INST`,
//! `MAN_2_SS`, `FE_SS_INST`, `FE_PILOT_ASSIST`, `ONL_CON_DESC`, `FE_PA_INST`), the six marks
//! (`corner`, `dot`, `square`, `line`, `Title_Arrow_HD`, `Subtitle_Arrow_HD`) and the five
//! illustrations, each as `.gnf` and `.2x.gnf`. Every id resolves in `data09.psarc`'s English
//! table and every picture is in `data00`/`data08`.
//!
//! What differs from HD's, each read off the same run:
//!
//! - **One styling.** The run names `Barrel_Roll.gnf` and no `_fury` file, and the front end
//!   always takes `WOHD_Colours` (`frontend::MENU_SKIN`'s `colour_template`), so the screen
//!   is HD's white-and-blue. The `_fury` pictures ship but nothing here names them.
//! - **A still, not a clip, in the mode icon's panel.** No Omega archive ships a `.bik`; the
//!   constructor (`FUN_01708fe0` of the base `eboot.bin`) picks one `.2x.gnf` per mode id
//!   ([`MODE_STILLS`]) and the draw function (`FUN_017062e0`, state 8) places it at
//!   (1518, 708), 230 by 234, in a 1920 by 1080 grid.
//! - **The brand is a string id**, `MSC_WOOM_CAPS`, not HD's literal (the layout file's).
//! - **Three further pictures**, `LoadingScreen_{racer,motion,wipeout}_controls.gnf`, and
//!   `loading_screen_demo_bar.gnf`, are named beside it; which screen draws them is unread,
//!   so nothing here does.
//! - **The tint is unread.** The run names `White2048` and `Blue2048` just before the
//!   caption, and the `HD_*` globals not at all; which role each takes is unread, so the
//!   screen is tinted with HD's four, as the menus around it are.
//!
//! The layout is `LoadingScreen_Omega.toml`: state 8 of the draw function places the rules,
//! arrows, captions, brackets and the picture window at the operands `LoadingScreen_HD.toml`
//! holds, to the pixel. **Chosen, not measured**: the deck is every feature for every mode
//! (`deck: None`; the feature type is an argument the constructor takes, and its caller is
//! unread) and the bar fills with the time estimate (`progression: None`; the loader's stages
//! are unread).

/// Omega's loading screen. See the module doc.
pub static LOADING: oag_title::Loading = oag_title::Loading {
    wave: None,
    features: FEATURES,
    chrome: CHROME,
    palette: Some(PALETTE),
    caption: Some(CAPTION),
    deck: None,
    labels: Some(LABELS),
    progression: None,
    layout: Some(LAYOUT),
};

/// The layout file: HD's, with the brand a string id. Served from the fallback layer.
pub const LAYOUT: &str = r"Data\XML\LoadingScreen_Omega.toml";

/// The one styling the disc's front end runs in (`WOHD_Colours`).
pub const STYLE: &str = "HD";

/// The five features, in HD's order, each as the `.gnf` the executable names.
pub const FEATURES: &[oag_title::loading::FeatureStyle] = &[oag_title::loading::FeatureStyle {
    name: STYLE,
    dark_ground: false,
    mode_icons: &[],
    mode_stills: MODE_STILLS,
    features: &[
        feature(
            r"Data\FE\Images\Barrel_Roll.gnf",
            "MAN_2_BR",
            "FE_INSTRUCTIONS",
            "FE_BR_INST",
        ),
        feature(
            r"Data\FE\Images\Side_Shift_Tap.gnf",
            "MAN_2_SS",
            "FE_INSTRUCTIONS",
            "FE_SS_INST",
        ),
        feature(
            r"Data\FE\Images\Pilot_Assist.gnf",
            "FE_PILOT_ASSIST",
            "ONL_CON_DESC",
            "FE_PA_INST",
        ),
        feature(
            r"Data\FE\Images\Absorb.gnf",
            "IG_HUD_ABSORB",
            "FE_INSTRUCTIONS",
            "FE_ABSORB_INST",
        ),
        feature(
            r"Data\FE\Images\Flip.gnf",
            "FE_FLIP",
            "FE_INSTRUCTIONS",
            "FE_FLIP_INST",
        ),
    ],
}];

const fn feature(
    image: &'static str,
    title: &'static str,
    heading: &'static str,
    description: &'static str,
) -> oag_title::loading::Feature {
    oag_title::loading::Feature {
        image,
        title: Some(title),
        description,
        heading: Some(heading),
    }
}

/// HD's four globals, **chosen, not measured** for Omega (module doc).
pub const PALETTE: oag_title::loading::Palette = oag_title::loading::Palette {
    background: "HD_BG",
    ink: "HD_Grey",
    accent: "HD_Blue",
    dim: "HD_LightGrey",
};

/// The marks the screen is framed with.
pub const CHROME: &[(oag_title::loading::Chrome, &str)] = &[
    (
        oag_title::loading::Chrome::TitleArrow,
        r"Data\FE\Images\Title_Arrow_HD.gnf",
    ),
    (
        oag_title::loading::Chrome::SubtitleArrow,
        r"Data\FE\Images\Subtitle_Arrow_HD.gnf",
    ),
    (oag_title::loading::Chrome::Rule, r"Data\FE\Images\line.gnf"),
    (
        oag_title::loading::Chrome::Corner,
        r"Data\FE\Images\corner.gnf",
    ),
    (oag_title::loading::Chrome::Dot, r"Data\FE\Images\dot.gnf"),
    (
        oag_title::loading::Chrome::Square,
        r"Data\FE\Images\square.gnf",
    ),
];

/// The word the screen leads with.
pub const CAPTION: &str = "FE_LOADINGDOT";

/// The four region labels, by string id.
pub const LABELS: oag_title::loading::Labels = oag_title::loading::Labels {
    feature_image: "FE_FEATURE_IMAGE",
    feature_description: "FE_FEATURE_DESC",
    progression_bar: "FE_PROG_BAR",
    mode_icon: "FE_MODE_ICON",
};

const SINGLE_RACE: &str = r"Data\FE\NewImages\callout\race_mode.2x.gnf";

/// The picture of the mode icon's panel, by mode id, as the constructor's `switch` picks it:
/// the white `callout` glyph of the mode, which the layout tints with the accent. The base
/// styling takes these; the other branch (`Data/FE/Images/<mode>.2x.gnf`, untinted) is
/// taken when a flag the front end sets to 1 and nothing here names is.
///
/// Mode ids `0xd` and `0x15` (Zone Battle) and `0xe` (Detonator) pick a different, untinted
/// file in the same `switch`, so they draw nothing here rather than a wrong glyph.
pub const MODE_STILLS: oag_title::loading::ModeStills = oag_title::loading::ModeStills {
    modes: &[
        (4, Some(r"Data\FE\NewImages\callout\tournament_mode.2x.gnf")),
        (
            0x11,
            Some(r"Data\FE\NewImages\callout\tournament_mode.2x.gnf"),
        ),
        (5, Some(r"Data\FE\NewImages\callout\timetrial_mode.2x.gnf")),
        (6, Some(r"Data\FE\NewImages\callout\zone_mode.2x.gnf")),
        (8, Some(r"Data\FE\NewImages\callout\combat_mode.2x.gnf")),
        (0x14, Some(r"Data\FE\NewImages\callout\combat_mode.2x.gnf")),
        (10, Some(r"Data\FE\NewImages\callout\speed_mode.2x.gnf")),
        (0xd, None),
        (0x15, None),
        (0xe, None),
    ],
    otherwise: Some(SINGLE_RACE),
};
