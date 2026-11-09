//! What a title puts on screen while it loads.
//!
//! An axis because the three titles in hand give three answers: Pulse's
//! `oag_pulse::loading` entry names were once reached for whatever disc had
//! booted, true while the other title was Pure (which ships neither), until HD
//! shipped **something else**.
//!
//! | title | what its loading screen is |
//! | --- | --- |
//! | Pulse | a procedural [wave](Wave) over a glow strip, with the disc's own 26 tips |
//! | Pure | nothing: neither the tips plugin nor the strip is on any of its archives |
//! | HD / Fury | a captioned title, one of five illustrated [features](Feature), and a bar |
//!
//! # Two shapes, not one field
//!
//! A Pulse tip and an HD feature are both "something to read while you wait" but
//! not the same data: a tip is a *string id alone*, drawn as prose over the wave
//! from a plugin the disc ships; a feature is an **image, a title and a
//! description together**, named by the executable rather than any file (HD's tips
//! plugin was cut before release). Merging them means a `Feature` whose image is
//! always `None` on one title and a `Wave` whose tips live elsewhere on the other.
//!
//! See `docs/formats/hd-loading.md` and
//! `docs/ghidra/functions/psp-pulse-usa/loading-screen.md`.

/// A title's loading screen, as far as its own disc states it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loading {
    /// The procedural wave and the tips beside it, when the title authors them.
    /// `None` on a title with no glow strip (both that are not Pulse).
    pub wave: Option<Wave>,
    /// The illustrated features this title rotates through, grouped by the
    /// front-end style that selects between them. Empty on both PSP titles; one
    /// entry per style the disc ships art for. A source with no saved choice gets the
    /// one matching the served front end's ground ([`FeatureStyle::dark_ground`]).
    /// See [`FeatureStyle`] and [`Feature`].
    pub features: &'static [FeatureStyle],
    /// The four front-end globals this screen draws with, when the title names
    /// them. **Names, not colours**: looked up in the source's own `FEGlobals`, so a
    /// disc that re-tints its front end re-tints this screen (HD's `HD_*` globals
    /// differ between archives, black-and-red in one and white-and-blue in
    /// another). See [`Palette`].
    pub palette: Option<Palette>,
    /// The marks the screen is framed with: arrows, rules, bracket corners and the
    /// bar's fill. Empty on a title that frames it with nothing (both PSP ones:
    /// Pulse draws a wave and prose over a cleared frame). See [`Chrome`].
    pub chrome: &'static [(Chrome, &'static str)],
    /// The string table id of the word this screen leads with, when the title has
    /// one. An **id**, not a word: the player reads the disc's text in their
    /// language, and spelling it here would put shipped content in this repository.
    /// `None` falls back to this build's heading, which Pulse's screen has always
    /// drawn.
    pub caption: Option<&'static str>,
    /// Which of [`Self::features`] a race may show, by mode. `None` on a title with
    /// no features and on one whose rule is unread: the screen draws from every
    /// feature it has. See [`Deck`].
    pub deck: Option<Deck>,
    /// The small labels the screen names its regions with, when the title draws
    /// them. See [`Labels`].
    pub labels: Option<Labels>,
    /// How the progression bar fills, when the title's loader drives it. See
    /// [`Progression`].
    pub progression: Option<Progression>,
    /// The entry name of this title's loading layout file (`screen = "loading"`,
    /// `schemas/v1/screen-layout.tosd`), read through the asset layers so a player's
    /// override replaces it by path. `None` where the screen is drawn by code: the
    /// layout this build chose, in the PSP's 480x272 grid
    /// (`docs/formats/screen-layout.md`).
    pub layout: Option<&'static str>,
}

/// How a progression bar fills: a first target, the loader's milestones, and the
/// easing the bar chases them with.
///
/// HD's bar is **driven by the load's own stages**, not a clock: the constructor
/// stores a first target (`0x002b9a28`), the loader calls
/// `LoadingScreen_SetProgressTarget` (`0x002b2c60`) with a fraction at each stage,
/// and every frame the shown column count moves toward the target by a rate
/// (`0x002b6c88`). See `oag_hd::loading::PROGRESSION`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progression {
    /// The target the bar is built with, as a fraction of the bar.
    pub start: f32,
    /// The target each load stage sets, in order (the first is what the first
    /// reported stage sets). Targets never go back.
    pub milestones: &'static [f32],
    /// Columns the shown count advances each frame, and what the rate returns to
    /// once the bar has caught its target.
    pub rate: f32,
    /// Columns across the bar, which scales every fraction above.
    pub columns: f32,
}

impl Progression {
    /// The target, in columns, once `stage` stages have been reported: the first
    /// target at `0`, then one milestone per stage.
    #[must_use]
    pub fn target_after(&self, stage: usize) -> f32 {
        let fraction = match stage {
            0 => self.start,
            n => self.milestones.get(n - 1).copied().unwrap_or(1.0),
        };
        fraction * self.columns
    }
}

/// The four labels HD's loading screen writes over its regions, each behind a
/// square bullet.
///
/// **Drawn by the game itself, not a debug overlay.** `LoadingScreen_BuildText`
/// (`0x002b7cd0`) resolves these four from the string table by id; `LoadingScreen_Draw`
/// places them and the brand line (a literal on HD, a string id on Omega, so it lives in
/// the layout file) at fixed positions with `HD_Grey` and a 16 by 16 `square.gtf` bullet
/// (`docs/ghidra/functions/ps3-hdfury-eu/loading-screen.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Labels {
    /// The string table id of the label over the feature illustration.
    pub feature_image: &'static str,
    /// The id of the label over the feature's prose.
    pub feature_description: &'static str,
    /// The id of the label over the progression bar.
    pub progression_bar: &'static str,
    /// The id of the label over the mode icon.
    pub mode_icon: &'static str,
}

/// Which features a loading screen may draw, by the mode about to be raced.
///
/// **A law read out of HD's loading-screen constructor**, not a rotation: it
/// reduces a `rand()` result modulo a range the mode selects, and one mode skips
/// an index. So the law is the *set* of feature indices each mode may draw,
/// indexing [`FeatureStyle::features`], drawn from uniformly. See
/// `oag_hd::loading::DECK`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deck {
    /// Modes with a deck of their own: `(mode id, feature indices)`.
    /// The mode ids are the source executable's own, not any crate's enum.
    pub modes: &'static [(u32, &'static [u8])],
    /// Every other mode's deck when the source carries the Fury content.
    pub otherwise_fury: &'static [u8],
    /// Every other mode's deck when it does not.
    pub otherwise_base: &'static [u8],
    /// Whether this source carries the Fury content, which picks between the two.
    pub fury_content: bool,
}

impl Deck {
    /// The feature indices a race in `mode` may show. `None` mode (a screen not
    /// covering a race) gets every index either default deck names.
    #[must_use]
    pub fn indices(&self, mode: Option<u32>) -> &'static [u8] {
        let otherwise = if self.fury_content {
            self.otherwise_fury
        } else {
            self.otherwise_base
        };
        mode.and_then(|id| self.modes.iter().find(|(own, _)| *own == id))
            .map_or(otherwise, |(_, deck)| deck)
    }
}

/// The two entries the procedural wave needs. Only Pulse authors one. The numbers
/// that *drive* it are `oag_pulse::loading`'s constants (read out of one release's
/// executable; a second title authoring a wave would author its own).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wave {
    /// The entry holding the `<PI_LoadingScreen>` tips.
    pub tips: &'static str,
    /// The strip every column of the wave samples.
    pub glow_strip: &'static str,
}

/// The four front-end globals a loading screen resolves, by name.
///
/// **Recovered from the executable**, in this order: HD's loading-screen
/// constructor looks up `FE_HD_BG`, `FE_HD_Grey`, `FE_HD_Blue` and
/// `FE_HD_LightGrey` through the palette lookup and keeps the four at object offset
/// `0x95c`. The `FE_` prefix is the lookup's; the globals are `HD_BG` etc. in
/// `skin.xml`.
///
/// # The roles are the usage counts, not the names
///
/// Each palette names its colours relative to its own ground: where `HD_BG` is
/// white, `HD_Grey` is the *dark* ink; where black, the *light* one. So "light
/// grey" is not the bright one and cannot be read as the text colour. What settles
/// it is how often the original's draw function reads each, per screen variant:
/// `HD_BG` **once**, `HD_Grey` about **ten** times, `HD_Blue` **once**,
/// `HD_LightGrey` **once**: one ground, one workhorse and two single-element
/// colours, with `HD_Grey` the only legible ink against both grounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The screen's ground. Read once, before the variant branch.
    pub background: &'static str,
    /// Everything written and every mark drawn: text, rules, bracket corners.
    /// Everything written and every mark drawn: text, rules, bracket corners. The
    /// workhorse, and the only one of the four legible against both grounds.
    pub ink: &'static str,
    /// The accent: Fury's red, the base game's pale blue. One element.
    pub accent: &'static str,
    /// The one translucent colour: the **unfilled dots of the progression bar**,
    /// the accent filling them from the left. On the served archive a half-alpha
    /// dark grey, confidence 88, identified from a live frame of the original's
    /// fade-in (`docs/ghidra/functions/ps3-hdfury-eu/loading-screen.md`,
    /// "`HD_LightGrey` is the unfilled dots of the progression bar").
    pub dim: &'static str,
}

/// One piece of a loading screen's own chrome. Named rather than positional so a
/// caller asks for the mark it wants instead of indexing a list in some
/// executable's order.
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
    /// The square bullet before a region's label.
    Square,
}

/// One front-end style's worth of features, named as the disc names the style.
///
/// **HD ships every feature illustration twice**, white-and-blue for the base game
/// and black-and-red for Fury, and its `OPT_FE_STYLE` offers exactly two values,
/// `HD` and `FURY`. So the style is an axis of the disc's data, not a preference
/// this project invented, and [`Loading::features`] carries one of these per style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureStyle {
    /// What the disc calls this style, and what a settings file stores.
    pub name: &'static str,
    /// Whether this style is the one a front end served with a dark ground
    /// (`HD_BG` near black) is in.
    ///
    /// The front end's menus have no setting to follow: they are drawn in the
    /// style of the archive the source serves, so a source with no saved choice
    /// takes the style whose flag matches that ground, not the first of the
    /// list. See [`Palette::background`].
    pub dark_ground: bool,
    /// Its five, in the order the executable names them.
    pub features: &'static [Feature],
    /// The Bink clip the mode icon's panel plays, by the race's mode id (the
    /// source executable's ids, as [`Deck::modes`] keys them). Empty on a title with
    /// no such panel. A mode with no row draws no icon: which clip each unnamed id
    /// gets is unread (`oag_hd::loading::FEATURES`).
    pub mode_icons: &'static [(u32, &'static str)],
    /// The still the mode icon's panel shows, where the title draws a picture and not a clip.
    /// [`ModeStills::NONE`] on a title that plays clips or has no such panel.
    pub mode_stills: ModeStills,
}

/// The picture the mode icon's panel shows, by the race's mode id (the source
/// executable's ids, as [`Deck::modes`] keys them).
///
/// Omega's constructor picks one texture per mode (`oag_omega::loading`); HD plays a clip
/// instead ([`FeatureStyle::mode_icons`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeStills {
    /// A mode's entry, or `None` for a mode that draws no picture.
    pub modes: &'static [(u32, Option<&'static str>)],
    /// The entry of every mode `modes` does not name; `None` draws nothing.
    pub otherwise: Option<&'static str>,
}

impl ModeStills {
    /// A title with no mode stills.
    pub const NONE: Self = Self {
        modes: &[],
        otherwise: None,
    };

    /// The entry the panel shows for `mode`, if any.
    #[must_use]
    pub fn entry(&self, mode: Option<u32>) -> Option<&'static str> {
        mode.and_then(|mode| self.modes.iter().find(|(id, _)| *id == mode))
            .map_or(self.otherwise, |(_, entry)| *entry)
    }

    /// Every distinct entry this table can show.
    pub fn entries(&self) -> impl Iterator<Item = &'static str> + '_ {
        let mut seen: Vec<&'static str> = Vec::new();
        self.modes
            .iter()
            .filter_map(|(_, entry)| *entry)
            .chain(self.otherwise)
            .filter(move |entry| {
                let fresh = !seen.contains(entry);
                seen.push(entry);
                fresh
            })
    }
}

/// One thing the loading screen teaches while it waits: a picture, a name and a
/// paragraph.
///
/// HD's screen is built around one at a time (its widgets are `FEATURE IMAGE` and
/// `FEATURE DESCRIPTION`), and which is up is what its `Feature type == %i` line
/// reports (`oag_hd::loading::FEATURES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Feature {
    /// The archive entry holding the illustration.
    pub image: &'static str,
    /// The string table id of the feature's name, or `None` where a title has no
    /// name for one. All five of HD's are named (the text builder's slots were
    /// read); three reuse ids from other namespaces (`MAN_2_BR`, `MAN_2_SS`,
    /// `IG_HUD_ABSORB`).
    pub title: Option<&'static str>,
    /// The string table id of the paragraph under it.
    pub description: &'static str,
    /// The string table id of the heading over the paragraph, not the same on every
    /// feature: `FE_INSTRUCTIONS` over four, `ONL_CON_DESC` over Pilot Assist.
    pub heading: Option<&'static str>,
}
