//! What Wipeout 2048's race load draws: the percentage box, the rule and the seven-triangle meter,
//! driven by the loader's own law.
//!
//! Kind 4 of the executable's `LoadingScreen` class
//! (`docs/ghidra/functions/vita-2048-eu-v104/race-load.md`, `docs/formats/2048-loading.md`). The
//! launched event's card behind the box is the front end's own painter, not a loading-screen
//! asset; this table carries what the box itself needs. Kind 7, the HD feature screen the
//! class also carries, is started by no call site and is not drawn.

/// 2048's race load: a caption, a box and a meter; no wave, no feature, no chrome.
pub static LOADING: oag_title::Loading = oag_title::Loading {
    wave: None,
    features: &[],
    chrome: &[],
    palette: Some(PALETTE),
    caption: Some("FE_LOADINGDOT"),
    deck: None,
    labels: None,
    progression: Some(PROGRESSION),
    layout: Some(r"Data\XML\LoadingScreen_2048.toml"),
};

/// The three `Skin.xml` globals the box draws with (`Blue2048` the box, rule, caption and
/// meter, `White2048` the readout), and the ground the card sits on. The ground is the lightest
/// of the skin's colours: the original's is the blurred campaign map, which this build does
/// not draw behind a launch.
pub const PALETTE: oag_title::loading::Palette = oag_title::loading::Palette {
    background: "White2048",
    ink: "Blue2048",
    accent: "White2048",
    dim: "Grey2048",
};

/// The meter's law, read off `LoadingScreen_Start`, `_SetCeiling` and `_SetRate`: a first
/// ceiling of 94.62 columns of 166 (57 %), three raises, a rate of 0.115 a step, a step per
/// 1/30 s, the rate rescaled by the gap on a raise and frozen by the race manager. Confidence
/// 92 (`race-load.md`, "The meter's law", reproduced numerically on two captures).
///
/// Which of this build's load stages is which milestone is **chosen, not measured**: the
/// original's three callers are unread. The first three stages are 0.75, 0.95 and 0.98 and the
/// fourth, the crafts built, stands for the race manager's constructor that freezes the rate.
pub const PROGRESSION: oag_title::loading::Progression = oag_title::loading::Progression {
    start: 94.62 / 166.0,
    milestones: &[0.75, 0.95, 0.98],
    rate: 0.115,
    columns: 166.0,
    rules: oag_title::loading::Rules {
        raise: oag_title::loading::Raise::Rescale,
        frames_per_step: 2,
        freezes_rate: true,
    },
};
