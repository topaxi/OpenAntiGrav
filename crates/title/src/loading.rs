//! What a title puts on screen while it loads.
//!
//! An axis because the three titles in hand give three different answers, and
//! the third is what forced it. Until 2026-08-24 `oag_game::loading` reached
//! for `oag_pulse::loading`'s two entry names whatever disc had booted, on the
//! reasoning - written into that module - that "a title that ships neither has
//! nothing to put in a table". That was true while the other title was Pure,
//! which ships neither. Wipeout HD ships **something else**, so there are now
//! three answers rather than one and an absence:
//!
//! | title | what its loading screen is |
//! | --- | --- |
//! | Pulse | a procedural [wave](Wave) over a glow strip, with the disc's own 26 tips |
//! | Pure | nothing: neither the tips plugin nor the strip is on any of its archives |
//! | HD / Fury | a captioned title, one of five illustrated [features](Feature), and a bar |
//!
//! # The two shapes are the same idea twice, and are still not one field
//!
//! A Pulse tip and an HD feature are both "something to read while you wait",
//! and it is tempting to call them one thing. They are not the same data: a
//! Pulse tip is a *string id alone*, drawn as prose over the wave, and pulled
//! from a plugin the disc ships; an HD feature is an **image, a title and a
//! description together**, named by the executable rather than by any file, and
//! HD's own tips plugin was cut before release. Merging them would mean a
//! `Feature` whose image is always `None` on one title and a `Wave` whose tips
//! live somewhere else on the other, which is two absences pretending to be one
//! type.
//!
//! See `docs/formats/hd-loading.md` and
//! `docs/ghidra/functions/psp-pulse-usa/loading-screen.md`.

/// A title's loading screen, as far as its own disc states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loading {
    /// The procedural wave and the tips beside it, when the title authors them.
    ///
    /// `None` on a title that ships no glow strip, which is both of the ones
    /// that are not Pulse.
    pub wave: Option<Wave>,
    /// The illustrated features this title rotates through, grouped by the
    /// front-end style that selects between them.
    ///
    /// Empty on both PSP titles, which have no such thing. One entry per style
    /// the disc ships art for, the first being what a source with no saved
    /// choice gets. See [`FeatureStyle`] and [`Feature`].
    pub features: &'static [FeatureStyle],
    /// The four front-end globals this screen draws with, when the title names
    /// them.
    ///
    /// **Names, not colours.** They are looked up in the source's own
    /// `FEGlobals`, so a disc that re-tints its front end re-tints this screen
    /// with it - which Wipeout HD does: its `HD_*` globals differ between
    /// archives, black-and-red in one and white-and-blue in another. See
    /// [`Palette`].
    pub palette: Option<Palette>,
    /// The marks the screen is framed with: arrows, rules, bracket corners and
    /// the bar's own fill.
    ///
    /// Empty on a title that frames its loading screen with nothing, which is
    /// both PSP ones - Pulse draws a wave and prose over a cleared frame. See
    /// [`Chrome`].
    pub chrome: &'static [(Chrome, &'static str)],
    /// The string table id of the word this screen leads with, when the title
    /// has one.
    ///
    /// An **id**, not a word: what a player reads is the disc's own text in
    /// their own language, and spelling it here would put shipped content in
    /// this repository. `None` falls back to this build's own heading, which is
    /// what Pulse's screen has always drawn.
    pub caption: Option<&'static str>,
}

/// The two entries the procedural wave needs.
///
/// Only Pulse authors one. The numbers that *drive* it are not here: they are
/// `oag_pulse::loading`'s own constants, because they were read out of one
/// release's executable and a second title authoring a wave would author its
/// own. This is the pair of archive entries, which is the part a loader needs
/// before it can ask anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wave {
    /// The entry holding the `<PI_LoadingScreen>` tips.
    pub tips: &'static str,
    /// The strip every column of the wave samples.
    pub glow_strip: &'static str,
}

/// The four front-end globals a loading screen resolves, by name.
///
/// **Recovered from the executable**, in this order: Wipeout HD's loading-screen
/// constructor looks up `FE_HD_BG`, `FE_HD_Grey`, `FE_HD_Blue` and
/// `FE_HD_LightGrey` through the palette lookup and keeps the four results
/// together at object offset `0x95c`. The `FE_` prefix is the lookup's; the
/// globals themselves are `HD_BG` and friends in `skin.xml`.
///
/// **Which of the four goes where is not recovered.** The constructor stores
/// them and the drawing code that reads them has not been traced, so the roles
/// below are this project's reading of a screenshot: the ground is `BG`, the
/// text is `LightGrey`, the rules and bracket marks are `Grey`, and `Blue` is
/// the accent. That is a mapping of four known colours onto four visible roles
/// rather than a guess at unknown values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The screen's ground.
    pub background: &'static str,
    /// The rules and the bracket marks.
    pub rule: &'static str,
    /// The accent.
    pub accent: &'static str,
    /// The text.
    pub text: &'static str,
}

/// One piece of a loading screen's own chrome.
///
/// Named rather than positional so a caller asks for the mark it wants instead
/// of indexing a list whose order is some executable's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chrome {
    /// The marker before the screen's own title.
    TitleArrow,
    /// The marker before a feature's name.
    SubtitleArrow,
    /// The horizontal rules.
    Rule,
    /// The bracket marks round a region.
    Corner,
    /// The progression bar's fill.
    Dot,
}

/// One front-end style's worth of features, named as the disc names the style.
///
/// **Wipeout HD ships every feature illustration twice** - white-and-blue for
/// the base game and black-and-red for Fury - and its own `OPT_FE_STYLE` offers
/// exactly two values, `HD` and `FURY`. So the style is a real axis of the
/// disc's data rather than a preference this project invented, and
/// [`Loading::features`] carries one of these per style instead of a single
/// list somebody has to know the styling of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureStyle {
    /// What the disc calls this style, and what a settings file stores.
    pub name: &'static str,
    /// Its five, in the order the executable names them.
    pub features: &'static [Feature],
}

/// One thing the loading screen teaches while it waits: a picture, a name and a
/// paragraph.
///
/// Wipeout HD's screen is built around exactly one of these at a time - its own
/// widgets are named `FEATURE IMAGE` and `FEATURE DESCRIPTION` - and which one
/// is up is what its `Feature type == %i` line reports. See
/// `oag_hd::loading::FEATURES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Feature {
    /// The archive entry holding the illustration.
    pub image: &'static str,
    /// The string table id of the feature's name, or `None` where the id has
    /// not been recovered.
    ///
    /// `None` is a real state rather than a gap waiting to be filled: two of
    /// HD's five have an unambiguous `FE_*` id and three do not, and guessing
    /// between the candidates would be putting a name on screen this project
    /// cannot vouch for. A feature with no title id draws its description
    /// alone.
    pub title: Option<&'static str>,
    /// The string table id of the paragraph under it.
    pub description: &'static str,
}
