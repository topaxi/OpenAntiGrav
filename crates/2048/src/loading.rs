//! 2048's loading screen: HD's feature screen, re-spelt for the Vita's `.gxt`, with no mode icon.
//!
//! Read off the v1.04 `eboot.elf` (`docs/ghidra/functions/vita-2048-eu-v104/loading-screen.md`,
//! `docs/formats/2048-loading.md`). `LoadingScreen_LoadAssets` (`0x81281ab2`) is HD's thread
//! body and its kind 7 loads `corner`, `dot`, `square`, `line`, `Title_Arrow_HD` and
//! `Subtitle_Arrow_HD` as `.gxt`, then the one illustration the feature draw picked, and
//! `LoadingScreen_Draw` (`0x812830c8`) places them at HD's operands.
//!
//! **Chosen, not measured: that a race load shows this screen.** No call site of the v1.04
//! executable starts kind 7; a race load starts kind 4 (an event card or a controls still and a
//! percentage box), which is not built. This is the screen 2048's own data authors in full.
//!
//! What differs from HD's, each read off the same functions:
//!
//! - **One styling.** The loader names both `Barrel_Roll.gxt` and `Barrel_Roll_fury.gxt` and
//!   picks between them on the byte at `0x8151cae8`, which is `0` in the image and has no writer
//!   in the program, so the base (white-and-blue) art is the one the original draws. The `_fury`
//!   pictures ship and are named but unreachable, so [`FEATURES`] carries the base style alone.
//! - **The ground is white, chosen, not measured.** Kind 7 fills no ground (`HD_BG`, which the
//!   constructor resolves, is read by no draw of kind 7); the base pictures are opaque white to
//!   their edge, which only shows nothing against a white ground. [`PALETTE`] therefore takes
//!   `HD_White` for the ground, the one white global of 2048's `HD_Colours` block.
//! - **No mode icon.** Kind 7 draws the panel's label (`FE_MODE_ICON`) and its four brackets
//!   and nothing inside them; there is no clip and no still to name.
//! - **The brand is HD's literal**, `WIPEOUT(R) HD` (`0x814d0e24`), in the layout file.
//! - **The colours are the four `HD_*` globals**, resolved by the constructor
//!   (`LoadingScreen_Construct`, `0x8128074c`) the way HD's are.
//!
//! The layout is `LoadingScreen_2048.toml`. **Chosen, not measured**: the bar fills with the time
//! estimate (`progression: None`; 2048's own maximum, 94.62, and rate, 0.115, are not HD's
//! 149.4 and 0.1, and how they map onto the bar's 166 columns is unread).

/// 2048's loading screen. See the module doc.
pub static LOADING: oag_title::Loading = oag_title::Loading {
    wave: None,
    features: FEATURES,
    chrome: CHROME,
    palette: Some(PALETTE),
    caption: Some(CAPTION),
    deck: Some(DECK),
    labels: Some(LABELS),
    progression: None,
    layout: Some(LAYOUT),
};

/// The layout file: HD's operands without the mode icon. Served from the fallback layer.
pub const LAYOUT: &str = r"Data\XML\LoadingScreen_2048.toml";

/// The styling the executable picks (flag `0x8151cae8` is `0`).
pub const STYLE: &str = "HD";

/// The five features, in HD's order, in the one styling the executable reaches. The order is the executable's
/// `Feature type` index (`LoadingScreen_Construct`'s draw and the text builder agree).
pub const FEATURES: &[oag_title::loading::FeatureStyle] = &[oag_title::loading::FeatureStyle {
    name: STYLE,
    dark_ground: false,
    mode_icons: &[],
    mode_stills: oag_title::loading::ModeStills::NONE,
    features: &[
        feature(
            "data/fe/images/barrel_roll.gxt",
            "MAN_2_BR",
            "FE_INSTRUCTIONS",
            "FE_BR_INST",
        ),
        feature(
            "data/fe/images/side_shift_tap.gxt",
            "MAN_2_SS",
            "FE_INSTRUCTIONS",
            "FE_SS_INST",
        ),
        feature(
            "data/fe/images/pilot_assist.gxt",
            "FE_PILOT_ASSIST",
            "ONL_CON_DESC",
            "FE_PA_INST",
        ),
        feature(
            "data/fe/images/absorb.gxt",
            "IG_HUD_ABSORB",
            "FE_INSTRUCTIONS",
            "FE_ABSORB_INST",
        ),
        feature(
            "data/fe/images/flip.gxt",
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

/// The ink, accent and trough are the globals the constructor looks up (`FE_HD_Grey`,
/// `FE_HD_Blue`, `FE_HD_LightGrey`), in HD's roles, which 2048's draw function matches (the
/// workhorse ink is `+0x8ec`, the lit and unlit bar tiles `+0x8f0` and `+0x8f4`). The ground is
/// **chosen, not measured**: see the module doc.
pub const PALETTE: oag_title::loading::Palette = oag_title::loading::Palette {
    background: "HD_White",
    ink: "HD_Grey",
    accent: "HD_Blue",
    dim: "HD_LightGrey",
};

/// The marks the screen is framed with, in the order state 7 loads them.
pub const CHROME: &[(oag_title::loading::Chrome, &str)] = &[
    (
        oag_title::loading::Chrome::Corner,
        "data/fe/images/corner.gxt",
    ),
    (oag_title::loading::Chrome::Dot, "data/fe/images/dot.gxt"),
    (
        oag_title::loading::Chrome::Square,
        "data/fe/images/square.gxt",
    ),
    (oag_title::loading::Chrome::Rule, "data/fe/images/line.gxt"),
    (
        oag_title::loading::Chrome::TitleArrow,
        "data/fe/images/title_arrow_hd.gxt",
    ),
    (
        oag_title::loading::Chrome::SubtitleArrow,
        "data/fe/images/subtitle_arrow_hd.gxt",
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

/// Which features a race may show, by the mode id the draw compares: HD's rows, because
/// `LoadingScreen_Construct` (`0x8128074c`) tests the same ids. Mode 6 draws from `{0, 1}` and
/// `0xe` from `{1}` by direct compare; `GameMode_IsEliminatorFamily` is `8` or `0x14`
/// (and two further 2048 mode flags this table cannot key by id) with `% 5`, and
/// `0x81001982` is `0xd` or `0x15` with `% 3` over `{0, 1, 3}`. Every other mode is
/// `rand() % 4`: there is no Fury-content flag, so the base width is four.
pub const DECK: oag_title::loading::Deck = oag_title::loading::Deck {
    modes: &[
        (8, &[0, 1, 2, 3, 4]),
        (0x14, &[0, 1, 2, 3, 4]),
        (0xd, &[0, 1, 3]),
        (0x15, &[0, 1, 3]),
        (0xe, &[1]),
        (6, &[0, 1]),
    ],
    otherwise_fury: &[0, 1, 2, 3],
    otherwise_base: &[0, 1, 2, 3],
    fury_content: true,
};
