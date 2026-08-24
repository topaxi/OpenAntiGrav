//! Wipeout HD's loading screen: a full-screen still, and the tips plugin that
//! was cut before release.
//!
//! # The screen the code has and the screen the disc has are not the same
//!
//! HD's executable still carries Pulse's whole loading-plugin machinery.
//! `FrontendRoot.cpp`'s plugin list is in `EBOOT.elf` verbatim -
//! `Data\Plugins\frontend`, `Data\Plugins\billboards`, `Data\Plugins\grids`,
//! `Data\Plugins\music`, `Data\Plugins\loading`, `Data\Plugins\news` - and so
//! are `PI_LoadingScreen` and `PI_LoadingScreen_Item.cpp`, which are the node
//! kind and the source file behind Pulse's 26 rotating tips.
//!
//! **Three of those six plugin directories are not on the retail disc**:
//! `music`, `loading` and `news`. Counted over the whole PSARC manifest, the
//! only `Data\Plugins\*` directories that exist are `frontend`, `billboards`,
//! `grids`, `languages` and `downloads`. So HD's tips were cut with their
//! plugin, and there is nothing to read them out of. Nor is there a glow strip:
//! `LoadingPulseOverlay.mip` is a Pulse entry and is on no HD archive. **This
//! title draws no wave and no tip, and that is the measurement rather than an
//! unfinished port** - see [`oag_title::Loading`].
//!
//! `/Data/FE/Images/WP_HD_DEMO_Loading_screen1.gtf` is in the executable too
//! and likewise absent from retail, which is the same story from the other end:
//! the demo build had a loading still of its own and the shipping one does not
//! use it.
//!
//! # What retail *does* ship, and how the code reaches it
//!
//! `LoadingScreenThread` sits in `EBOOT.elf` between `Loading Screen Finished`
//! and this run of format strings, in this order:
//!
//! ```text
//! /USRDIR/GameAd.bik
//! AdServer sent "%s"  at  "%s"
//! AdServer Not Found
//! Using loading screen ad "%s"
//! ...
//! LoadingScreenThread
//! Loading Screen Finished
//! /Data/FE/Ads/LSAD_%s_16x9_01.gtf
//! /Data/FE/Ads/LSAD_%s_16x9_02.gtf
//! /Data/FE/Ads/LSAD_%s_16x9_03.gtf
//! /Data/FE/Ads/LSAD_%s_4x3_01.gtf
//! /Data/FE/Ads/LSAD_%s_4x3_02.gtf
//! /Data/FE/Ads/LSAD_%s_4x3_03.gtf
//! ```
//!
//! `LoadingScreenAdServer.cpp` is the file name beside them. **HD's loading
//! screen was an advertising slot** - DoubleFusion's, whose SPRX the same
//! executable loads (`Loading DoubleFusion SPRX file "%s"`) - served over the
//! network, with the twelve `LSAD_*` stills on the disc as what it falls back
//! to when `AdServer Not Found`.
//!
//! And they are on the disc, all twelve of them, in `DATA06`: `%s` expands to
//! `HD` and `FURY`, which are exactly the two values `OPT_FE_STYLE` offers (see
//! `docs/formats/hd-frontend.md`). Every one is **1024x512 DXT1**, 262,272
//! bytes, one mip, no cubemap - a full-screen still, and the largest thing in
//! the front end's image set by an order of magnitude.
//!
//! # What they depict, which is what settles the ad reading
//!
//! **An empty billboard.** Decoded, all twelve are one subject in three camera
//! framings: a roadside hoarding at an angle with a flat grey panel where its
//! poster would be, over a striped sky - pale blue on the `HD` six and black on
//! the `FURY` six, which is two palettes of one scene and exactly what
//! `OPT_FE_STYLE` selecting between them predicts.
//!
//! The grey panel is the point: these are the frame an advertisement is
//! composited *into*, not finished artwork. `/USRDIR/GameAd.bik`, the served
//! content the executable names, is **not on the disc** in or out of the
//! PSARCs - `/USRDIR/` is where the console writes, so it was fetched at
//! runtime. A disc played today has the frame and nothing to put in it, and
//! that is what this draws. Filling the panel would be inventing an advert.
//!
//! The 2009 press coverage describes the same object from the front - "a small
//! rectangular video ... with designs that looked like they fit in with WipEout
//! HD's sleek, futuristic styling" - which is the frame and the panel seen
//! running. **What it also implies is a limitation here**: nobody reported an
//! empty hoarding in the year before the ads were patched in, so the original
//! probably drew this frame only *with* an advert and something else without.
//! What that something else is has not been read - `LOADING SCREEN TYPE type ==
//! %i` is the likely switch. This draws the frame unconditionally because it is
//! the only loading-screen picture on the disc. See
//! `docs/formats/hd-loading.md`.
//!
//! Confidence **86** that these are what the retail loading screen draws: the
//! path template, the thread name and the fall-back message are read off the
//! executable and all twelve files resolve and decode, but the call site was
//! not traced and nothing has been watched running. **84** that the network ad
//! is the primary and these the fallback rather than the other way round: that
//! was 62 on the strings alone and decoding the twelve raised it, for the
//! reason the section above gives. **90** that the loading plugin and the glow
//! strip are absent from retail - a manifest count and a pair of misses.
//!
//! See `docs/formats/hd-loading.md`.

/// HD's loading screen: a caption, one illustrated feature, and a bar.
///
/// `wave: None` is the measurement this module opens with: the tips plugin and
/// the glow strip are both absent from every archive, and the running game says
/// so itself - `FileSystem::Open FAILED for 'Data\Plugins\loading\Definition.xml'`
/// on the `TTY.log` of every boot.
pub static LOADING: oag_title::Loading = oag_title::Loading {
    wave: None,
    features: FEATURES,
    chrome: CHROME,
    palette: Some(PALETTE),
    caption: Some(CAPTION),
};

/// The five features the screen rotates through, **in the order the executable
/// names them**.
///
/// That order is not this crate's arrangement: the ten entries sit in one run
/// in `EBOOT.elf`, `_fury` and plain alternating, and `Feature type == 2` on
/// the `TTY.log` picks out `Pilot_Assist` - which is what both a screenshot of
/// a running Fury race and this project's own RPCS3 capture show on screen.
/// So the index is the position in this list, corroborated twice.
///
/// **The plain entries rather than the `_fury` ones.** Each feature ships
/// twice, the same axis `OPT_FE_STYLE` selects everywhere else on this title -
/// black-and-red for Fury against the base game's white-and-blue. This build
/// has no row for that setting yet, so it takes the base game's, the same way
/// [`BACKDROP`] and [`crate::names::FRONT_END_MUSIC`] take theirs, and
/// [`FEATURES_FURY`] records the other five rather than losing them.
///
/// **Three of the five title ids are `None`, and that is a refusal rather than
/// a gap.** `FE_PILOT_ASSIST` and `FE_FLIP` are unambiguous - the `FE_`
/// namespace, the exact words on screen. There is no `FE_BR`, `FE_SS` or
/// `FE_ABSORB`, so the other three titles come from somewhere else: the
/// candidates that hold the right words are `MAN_2_BR`, `MAN_2_SS` /
/// `OPT_CTRL_SS` and `IG_HUD_ABSORB` / `MSC_ABSORB`, and picking between them
/// would be putting a name on screen this project cannot vouch for. See
/// `docs/formats/hd-loading.md`.
///
/// Confidence **88** on the set and the order, **80** on the two named title
/// ids, **95** on the five descriptions: `FE_PA_INST` is verbatim the paragraph
/// in a screenshot of the running game, and the other four are its immediate
/// namesakes.
pub const FEATURES: &[oag_title::loading::FeatureStyle] = &[
    oag_title::loading::FeatureStyle {
        name: STYLE_HD,
        features: &[
            feature(r"Data\FE\Images\Barrel_Roll.gtf", None, "FE_BR_INST"),
            feature(r"Data\FE\Images\Side_Shift_Tap.gtf", None, "FE_SS_INST"),
            feature(
                r"Data\FE\Images\Pilot_Assist.gtf",
                Some("FE_PILOT_ASSIST"),
                "FE_PA_INST",
            ),
            feature(r"Data\FE\Images\Absorb.gtf", None, "FE_ABSORB_INST"),
            feature(r"Data\FE\Images\Flip.gtf", Some("FE_FLIP"), "FE_FLIP_INST"),
        ],
    },
    oag_title::loading::FeatureStyle {
        name: STYLE_FURY,
        features: &[
            feature(r"Data\FE\Images\Barrel_Roll_fury.gtf", None, "FE_BR_INST"),
            feature(
                r"Data\FE\Images\Side_Shift_Tap_fury.gtf",
                None,
                "FE_SS_INST",
            ),
            feature(
                r"Data\FE\Images\Pilot_Assist_fury.gtf",
                Some("FE_PILOT_ASSIST"),
                "FE_PA_INST",
            ),
            feature(r"Data\FE\Images\Absorb_fury.gtf", None, "FE_ABSORB_INST"),
            feature(
                r"Data\FE\Images\Flip_fury.gtf",
                Some("FE_FLIP"),
                "FE_FLIP_INST",
            ),
        ],
    },
];

/// The base game's style, as `OPT_FE_STYLE`'s own first entry spells it.
///
/// White and blue. First in [`FEATURES`], so a source with no saved choice gets
/// it.
pub const STYLE_HD: &str = "HD";

/// Fury's style, as `OPT_FE_STYLE`'s second entry spells it.
///
/// Black and red. **Both sets are on the disc in full**, which is what makes
/// this a real choice rather than a preference with one answer: five plain
/// entries and five `_fury` ones, every one of them a shipped `.gtf`.
pub const STYLE_FURY: &str = "FURY";

/// One row of [`FEATURES`], so the table above reads as a table.
const fn feature(
    image: &'static str,
    title: Option<&'static str>,
    description: &'static str,
) -> oag_title::loading::Feature {
    oag_title::loading::Feature {
        image,
        title,
        description,
    }
}

/// The four globals the loading screen's constructor resolves.
///
/// Read out of `EBOOT.elf`: the constructor at `0x002b3bb0` loads these four
/// names from its own TOC, in this order, passes each through the palette
/// lookup and keeps the results together at object offset `0x95c`. The `FE_`
/// prefix the code carries is the lookup's; the globals in `skin.xml` are
/// `HD_BG` and friends.
///
/// **The values differ by archive**, which is the same open question
/// `docs/formats/hd-frontend.md` carries about `skin.xml`'s six copies -
/// `DATA06`'s `HD_BG` is `0xffffffff` and `DATA00`'s is not, so the served copy
/// decides whether this screen is white-and-blue or black-and-red. That
/// correspondence with `OPT_FE_STYLE` is suggestive and unproven; the loader
/// reads whichever copy is served rather than choosing one.
///
/// Confidence **90** on the set and the order - four `lwz` from one TOC block,
/// in one basic block, into four adjacent fields. **60** on which colour plays
/// which role, which is a reading of a screenshot; see
/// [`oag_title::loading::Palette`].
pub const PALETTE: oag_title::loading::Palette = oag_title::loading::Palette {
    background: "HD_BG",
    rule: "HD_Grey",
    accent: "HD_Blue",
    text: "HD_LightGrey",
};

/// The chrome the screen is framed with, named in the same run of the
/// executable as the illustrations.
///
/// `Title_Arrow_HD` is the marker before `LOADING...`, `Subtitle_Arrow_HD` the
/// one before a feature's name, `line` the horizontal rules, `corner` the
/// bracket marks round each region and `dot` the progression bar's own fill.
/// All six are tiny - 256 to 1,152 bytes - and all six are in `DATA02`.
///
/// Ordered as the executable names them, and carried as a table because the
/// screen wants them together: `oag_game::loading::Assets` builds one sheet out
/// of these and the illustration, so a single `Draw::Sprite` atlas holds
/// everything the screen draws.
pub const CHROME: &[(oag_title::loading::Chrome, &str)] = &[
    (
        oag_title::loading::Chrome::TitleArrow,
        r"Data\FE\Images\Title_Arrow_HD.gtf",
    ),
    (
        oag_title::loading::Chrome::SubtitleArrow,
        r"Data\FE\Images\Subtitle_Arrow_HD.gtf",
    ),
    (oag_title::loading::Chrome::Rule, r"Data\FE\Images\line.gtf"),
    (
        oag_title::loading::Chrome::Corner,
        r"Data\FE\Images\corner.gtf",
    ),
    (oag_title::loading::Chrome::Dot, r"Data\FE\Images\dot.gtf"),
];

/// The string table id of the word this screen leads with.
///
/// `FE_LOADINGDOT`, which every one of the disc's sixteen languages carries -
/// `LOADING...` in english, `LÄDT ....` in german. **Confirmed on screen**: a
/// running Fury race heads this screen `LOADING... VINETA K`, which is this
/// string followed by the circuit's own name out of the same table. The id
/// rather than the word, so what a player reads is the disc's own text in their
/// own language and never appears in this repository.
///
/// `MSC_LOADING` is deliberately *not* this: it belongs to
/// `AutoLoadingProfileScreen` in `memorystickbootscreens_psp.xml`.
pub const CAPTION: &str = "FE_LOADINGDOT";

/// The full-screen frame the **advertisement** was composited into, recorded
/// and deliberately not drawn.
///
/// **This was mistaken for the loading screen itself and is not.** The mistake
/// is worth keeping written down, because everything that led to it was true:
/// `LoadingScreenThread`, `Loading Screen Finished` and the six
/// `/Data/FE/Ads/LSAD_%s_*.gtf` templates sit together in `EBOOT.elf`, all
/// twelve files are on the disc, and every one decodes to a 1024x512 DXT1
/// hoarding with a blank grey panel. What the reading missed is that the run of
/// strings around them is the *whole* screen's asset list - `corner.gtf`,
/// `dot.gtf`, `line.gtf`, `Title_Arrow_HD.gtf`, `Subtitle_Arrow_HD.gtf` and the
/// ten feature images are in the same block - so the ad is one element of the
/// screen rather than the screen.
///
/// **RPCS3 settled it.** On a disc with no ad server the `TTY.log` reads
/// `DIGA State "STATE_PRE_ADVERT"` then `AdServer Not Found`, and the frame is
/// never drawn; the screen is the caption, the feature and the bar. `DIGA` is
/// Double Fusion's in-game advertising, whose SPRX the same executable loads,
/// and `/USRDIR/GameAd.bik` - the served video that filled the blank panel - is
/// **not on the disc**, because it was fetched at runtime by a service that has
/// not answered since 2009.
///
/// So this is the ad slot, it is empty, and drawing it would put a screen on
/// display that the original shows only with an advert in it.
pub const BACKDROP: &str = r"Data\FE\Ads\LSAD_HD_16x9_01.gtf";

/// Every loading still the disc carries, by the two axes that select it.
///
/// Recorded rather than reachable, for the reason [`BACKDROP`] gives: all
/// twelve are real, all twelve decode, and which one the original serves is a
/// question about a shut-down ad server and an unwired setting. Keeping the
/// list here is what stops the other eleven being re-derived from the
/// executable next time.
///
/// All twelve are 1024x512 DXT1, 262,272 bytes, measured on
/// `hdfury-ps3-eu-dec.iso`.
pub const BACKDROP_VARIANTS: &[(&str, &str)] = &[
    ("HD 16:9 1", r"Data\FE\Ads\LSAD_HD_16x9_01.gtf"),
    ("HD 16:9 2", r"Data\FE\Ads\LSAD_HD_16x9_02.gtf"),
    ("HD 16:9 3", r"Data\FE\Ads\LSAD_HD_16x9_03.gtf"),
    ("HD 4:3 1", r"Data\FE\Ads\LSAD_HD_4x3_01.gtf"),
    ("HD 4:3 2", r"Data\FE\Ads\LSAD_HD_4x3_02.gtf"),
    ("HD 4:3 3", r"Data\FE\Ads\LSAD_HD_4x3_03.gtf"),
    ("Fury 16:9 1", r"Data\FE\Ads\LSAD_FURY_16x9_01.gtf"),
    ("Fury 16:9 2", r"Data\FE\Ads\LSAD_FURY_16x9_02.gtf"),
    ("Fury 16:9 3", r"Data\FE\Ads\LSAD_FURY_16x9_03.gtf"),
    ("Fury 4:3 1", r"Data\FE\Ads\LSAD_FURY_4x3_01.gtf"),
    ("Fury 4:3 2", r"Data\FE\Ads\LSAD_FURY_4x3_02.gtf"),
    ("Fury 4:3 3", r"Data\FE\Ads\LSAD_FURY_4x3_03.gtf"),
];

/// The looping animation the *selection* screens show while a preview loads.
///
/// **Recorded here and deliberately not wired**, because it is not this
/// screen's. Both of its uses in the front-end XML are a `<Movie
/// name="FlyByMovie" ... repeat="true" preload="false">` on a track- or
/// ship-selection screen, standing in until that screen's own preview arrives,
/// and in the executable the string sits in a run of track-selection assets
/// (`TrackDirectionSheet.gtf`, every circuit's `TrackSelectEmblem_BW.gtf`)
/// rather than anywhere near `LoadingScreenThread`. Calling it "the loading
/// indicator" would be a guess dressed as a wiring.
pub const SELECTION_PLACEHOLDER_MOVIE: &str = r"Data\FE\Images\Loading_Icon.bik";
