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
    deck: Some(DECK),
    labels: Some(LABELS),
};

/// The five features the screen draws from, **in the order the executable
/// names them**.
///
/// The index is the one `Feature type == %i` reports. All three of its
/// consumers agree on it, which is what makes it a measurement rather than a
/// reading: `LoadingScreen_Construct` stores it at `object+0x444`, the
/// illustration loader `0x002b86a0` indexes the ten `.gtf` slots by it, and the
/// text builder `0x002b7cd0` indexes the title, heading and paragraph ids by it.
/// Two live frames pin it from the other side: an RPCS3 boot logged
/// `Feature type == 1` over a Side Shift screen, another `== 2` over Pilot
/// Assist.
///
/// **The plain entries rather than the `_fury` ones.** Each feature ships
/// twice, the same axis `OPT_FE_STYLE` selects everywhere else on this title -
/// black-and-red for Fury against the base game's white-and-blue. The loader
/// picks by `FrontEnd_IsFuryStyle()`, and so does this build, by
/// `settings.display.front_end_style`.
///
/// **All five title ids are read.** Three were left unguessed until 2026-10-04,
/// when the text builder's own TOC slots were resolved: the names are
/// `MAN_2_BR`, `MAN_2_SS` and `IG_HUD_ABSORB`, existing strings the screen
/// reuses rather than `FE_`-namespaced ones of its own. The heading over the
/// paragraph is `FE_INSTRUCTIONS` for four features and `ONL_CON_DESC` for
/// Pilot Assist, which is why the same screen reads `INSTRUCTIONS` in one frame
/// and `DESCRIPTION` in the next.
///
/// Confidence **92** on the set, the order and every id: slot-by-slot out of
/// two functions' own TOCs, and two live frames agree on indices 1 and 2.
/// Indices 0, 3 and 4 are read, not seen.
pub const FEATURES: &[oag_title::loading::FeatureStyle] = &[
    oag_title::loading::FeatureStyle {
        name: STYLE_HD,
        features: &[
            feature(
                r"Data\FE\Images\Barrel_Roll.gtf",
                "MAN_2_BR",
                "FE_INSTRUCTIONS",
                "FE_BR_INST",
            ),
            feature(
                r"Data\FE\Images\Side_Shift_Tap.gtf",
                "MAN_2_SS",
                "FE_INSTRUCTIONS",
                "FE_SS_INST",
            ),
            feature(
                r"Data\FE\Images\Pilot_Assist.gtf",
                "FE_PILOT_ASSIST",
                "ONL_CON_DESC",
                "FE_PA_INST",
            ),
            feature(
                r"Data\FE\Images\Absorb.gtf",
                "IG_HUD_ABSORB",
                "FE_INSTRUCTIONS",
                "FE_ABSORB_INST",
            ),
            feature(
                r"Data\FE\Images\Flip.gtf",
                "FE_FLIP",
                "FE_INSTRUCTIONS",
                "FE_FLIP_INST",
            ),
        ],
    },
    oag_title::loading::FeatureStyle {
        name: STYLE_FURY,
        features: &[
            feature(
                r"Data\FE\Images\Barrel_Roll_fury.gtf",
                "MAN_2_BR",
                "FE_INSTRUCTIONS",
                "FE_BR_INST",
            ),
            feature(
                r"Data\FE\Images\Side_Shift_Tap_fury.gtf",
                "MAN_2_SS",
                "FE_INSTRUCTIONS",
                "FE_SS_INST",
            ),
            feature(
                r"Data\FE\Images\Pilot_Assist_fury.gtf",
                "FE_PILOT_ASSIST",
                "ONL_CON_DESC",
                "FE_PA_INST",
            ),
            feature(
                r"Data\FE\Images\Absorb_fury.gtf",
                "IG_HUD_ABSORB",
                "FE_INSTRUCTIONS",
                "FE_ABSORB_INST",
            ),
            feature(
                r"Data\FE\Images\Flip_fury.gtf",
                "FE_FLIP",
                "FE_INSTRUCTIONS",
                "FE_FLIP_INST",
            ),
        ],
    },
];

/// Which of the five a race may show, by the executable's own mode id.
///
/// Read out of `LoadingScreen_Construct` (`0x002b3bb0`), where
/// `FUN_006762f8` - which is libc `rand()`, unseeded here and so a random
/// source rather than a frame counter - is reduced modulo a range the mode
/// picks. Read as **sets of indices**, because one deck is not a prefix:
///
/// | mode id | what it is | feature indices |
/// | ---: | --- | --- |
/// | `8`, `0x14` | `SPElimination`, `MPElimination` | `0 1 2 3 4` |
/// | `0xd`, `0x15` | unnamed, `MPArcade` | `0 1 3` (skips Pilot Assist) |
/// | `0xe` | unnamed | `1` always |
/// | `6` | unnamed | `0 1` |
/// | anything else | `3` = `SPArcade`, `5` = `SPTimeTrial`, ... | `0 1 2 3`, or `0 1 2` without the Fury content |
///
/// The last row's size is `((b - 1) >> 31) + 4` for the byte `b` at
/// `0x00b979fd`: four when it is at least one. **That byte is the Fury-content
/// flag**: three readers agree - `0x00029948` forces `FE_Style` to `HD` and
/// skips `PlayedFuryBefore` while it is clear, `0x00183b38` skips
/// `BackgroundAnimFury_Load` while it is clear, and `0x001a8f50` swaps a
/// `0x8000000`-flagged menu row for `FE_FURY_REQUIRED` while it is clear. Its
/// writer is unread (no store reaches it through its own TOC slot; it is passed
/// by address to about twenty functions), but its **value was read live**:
/// `00 01 00 00` at `0x00b979fc` on an RPCS3 boot of the EU Fury disc, so
/// [`oag_title::loading::Deck::fury_content`] is `true`, and a base-game-only
/// source would be `false`.
///
/// Setting the byte at `0x009384e1` sends every mode to the last row. Its
/// reader is `LoadingScreen_Construct` alone, so it is not modelled.
///
/// Confidence **88**: the moduli and branch targets are in the decompile and
/// the last row's size is corroborated live (`Feature type == 3` was logged
/// in a `GetMode()==3` race, which a modulus of three could not produce).
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
    otherwise_base: &[0, 1, 2],
    fury_content: true,
};

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
/// in one basic block, into four adjacent fields. **80** on the roles, which
/// come from how often the draw function reads each and from the values
/// themselves rather than from the names; see [`oag_title::loading::Palette`],
/// and note that the one role still unidentified is `HD_LightGrey`'s.
pub const PALETTE: oag_title::loading::Palette = oag_title::loading::Palette {
    background: "HD_BG",
    ink: "HD_Grey",
    accent: "HD_Blue",
    dim: "HD_LightGrey",
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
    (
        oag_title::loading::Chrome::Square,
        r"Data\Fe\Images\square.gtf",
    ),
];

/// The five labels the screen writes over its regions.
///
/// Read out of `LoadingScreen_BuildText` (`0x002b7cd0`) and
/// `LoadingScreen_Draw` (`0x002b61c8`), confidence 88: the brand is the literal
/// `WIPEOUT\xc2\xae HD` at `0x007a0748` and the other four are string ids the
/// text builder resolves (`FE_FEATURE_IMAGE`, `FE_FEATURE_DESC`, `FE_PROG_BAR`,
/// `FE_MODE_ICON`), each drawn behind a `square.gtf` bullet. An earlier reading
/// called them a debug overlay; the strings are in the retail string table.
pub const LABELS: oag_title::loading::Labels = oag_title::loading::Labels {
    brand: "WIPEOUT\u{ae} HD",
    feature_image: "FE_FEATURE_IMAGE",
    feature_description: "FE_FEATURE_DESC",
    progression_bar: "FE_PROG_BAR",
    mode_icon: "FE_MODE_ICON",
};

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
