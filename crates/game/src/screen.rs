//! The front end read out of its own XML.
//!
//! Pulse's front end is data, not code. `Data\Plugins\PI001\GUI\Skin.xml` in
//! `Data.wad` holds the boot screens, the `FEGlobals` colour and layout
//! variables, and a `LoadXML` list that pulls in every menu. This module turns
//! the expanded XML into a model the boot sequence can drive.
//!
//! Two conventions from the format matter here:
//!
//! - **`Values` carries its parent's attributes.** `<Movie><Values src="..."/>`
//!   means the `Movie` has a `src`. Attributes also appear directly on the
//!   element, so both are merged, with the element's own winning.
//! - **`FEGlobals->Name` is an indirection** into the `<Variable global="Name">`
//!   list. [`Screens::resolve`] follows it.
//!
//! See `docs/formats/fexml.md` and `docs/architecture/frontend-boot.md`.

use std::collections::HashMap;

use crate::input::button_from_name;

// The XML tree parser itself is a format concern and lives in `oag-formats`
// next to the dictionary expander, so the front end and the handling-stats
// decoder share one copy rather than growing two. Re-exported because this
// module is where the rest of the crate reaches for it.
pub use oag_formats::fexml::{Node, parse};
use oag_gameplay::input::Button;

/// Container extensions a `Movie` widget's `src` may already carry.
///
/// The PS2 front-end XML spells them lower case; the disc's own ISO 9660
/// directory spells the files upper case, so the test ignores case rather than
/// picking a side.
/// `.bik` is Wipeout HD's, and it is here for the same reason the PS2's two
/// are: its `<Movie>` widget names `Data/FE/Images/StudioLiverpool.bik`,
/// extension included, and appending `.PMF` to that would ask for nothing at
/// all.
pub const MOVIE_EXTENSIONS: [&str; 4] = [".pmf", ".pss", ".ipf", ".bik"];

fn has_movie_extension(src: &str) -> bool {
    let lower = src.to_ascii_lowercase();
    MOVIE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// One `Movie` widget's attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movie {
    /// The `src` attribute.
    ///
    /// A base name with **no extension** on the PSP. On the PS2 it carries its
    /// own - see [`Movie::entry_name`].
    pub src: String,
    /// Whether the movie has sound.
    pub sound: bool,
    /// Play as soon as the screen is entered.
    pub autostart: bool,
    /// Loop.
    pub repeat: bool,
    /// Fire the screen's `AutoRedirect` when the movie finishes.
    pub autoredirect: bool,
    /// Whether the `localised` attribute was present and true.
    pub localised: bool,
}

impl Movie {
    /// The archive entry name, assembled the way the widget assembles it.
    ///
    /// `Movie_ParseAttributes` appends `.PMF` when `localised` is absent, and
    /// `_US.PMF` otherwise. The PSP intro's `src` is `Data\Movies\Intro`, which
    /// is why the filename never turned up in a string search of the
    /// executable.
    ///
    /// **The PS2 build diverges, and its own XML is the evidence**: its `src`
    /// values are `Data\Movies\Intro.pss` and `Data\Movies\Backdrop.ipf`,
    /// extension included, and `SCES_547.48` matches on exactly those spellings
    /// (`FUN_0019b168` tests for `\Intro.pss` and `\Backdrop.ipf` before
    /// rewriting them to the region's cut). Appending `.PMF` to those would ask
    /// for `Data\Movies\Backdrop.ipf.PMF`, which is nothing at all - so a `src`
    /// that already names its container is taken as it stands. See
    /// [`MOVIE_EXTENSIONS`].
    #[must_use]
    pub fn entry_name(&self) -> String {
        if has_movie_extension(&self.src) {
            return self.src.clone();
        }
        if self.localised {
            format!("{}_US.PMF", self.src)
        } else {
            format!("{}.PMF", self.src)
        }
    }

    fn from_node(node: &Node) -> Self {
        Self {
            src: node.value("src").unwrap_or_default().to_string(),
            sound: node.flag("sound").unwrap_or(false),
            autostart: node.flag("autostart").unwrap_or(false),
            repeat: node.flag("repeat").unwrap_or(false),
            autoredirect: node.flag("autoredirect").unwrap_or(false),
            localised: node.flag("localised").unwrap_or(false),
        }
    }
}

/// One `Text` widget.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Text {
    /// Widget name, when it has one.
    pub name: Option<String>,
    /// String-table id to look up.
    pub idstring: Option<String>,
    /// A literal string, used instead of `idstring` by a few screens.
    pub string: Option<String>,
    /// Font name: `Default`, `Small`, `Title`, `Menu`, `HUD`.
    pub font: String,
    /// Position in the PSP's 480x272 screen space.
    pub x: f32,
    /// Position in the PSP's 480x272 screen space.
    pub y: f32,
    /// Scale multiplier.
    pub scale: f32,
    /// ARGB, as the XML writes it.
    pub color: u32,
    /// `left`, `right` or `centre`.
    pub align: String,
    /// Whether the widget starts hidden.
    pub start_enabled: bool,
    /// Whether the widget throbs once visible, rather than sitting still.
    ///
    /// The one measured case is `BOOT_PRESS_START` on `Show Logo`: its period
    /// and depth are read off a real capture in
    /// `docs/architecture/frontend-boot.md` (confidence 75) and hard-coded in
    /// `frontend::draw`'s `PULSE_PERIOD`/`PULSE_FLOOR` rather than re-derived
    /// per widget, since no other `pulse="true"` widget has been captured yet
    /// to say whether they share the same numbers.
    pub pulse: bool,
    /// Seconds after the widget's screen appears before it starts drawing at
    /// all, from the `delay` attribute. Zero when absent, which draws at once
    /// exactly as before this field existed.
    ///
    /// Every widget carries a `delay` in the XML this crate has seen (Pure's
    /// too), not only pulsing ones - see [`Self::pulse`]'s sibling gap noted
    /// on [`Skin::collect_widgets`]'s own doc comment, `delay`/`transition`
    /// together. Only a `pulse` widget acts on it here; a non-pulsing widget's
    /// `delay` is parsed and otherwise unused, the same trade `Image::auto_load`
    /// already makes for a flag this crate reads but does not yet act on.
    pub delay: f32,
    /// The pixel width to wrap at, when `widthlimited="true"` is set.
    ///
    /// Taken from the nearest enclosing `Viewport`'s own `width` - the only
    /// number the XML gives a `widthlimited` text to wrap against. `None`
    /// when `widthlimited` is absent or false, and also when it is true but
    /// nothing wraps the widget in a `Viewport` (nothing to wrap against).
    /// `Show Logo`'s `USLegalText`/`BOOT_LEGAL` is the one measured case:
    /// `Viewport` gives `width="400"`, `x="40"` matching the text's own `x`,
    /// so viewport-width and width-minus-inset agree and cannot be told
    /// apart from this one sample. Confidence 60 on "viewport width, not
    /// inset" as the general rule; 95 that it is right for this screen. See
    /// `docs/architecture/frontend-boot.md`.
    pub wrap_width: Option<f32>,
}

/// One `Redirect`: a button, and the screen it goes to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Redirect {
    /// Widget name, when it has one. `AutoRedirect` is fired by a finishing
    /// movie; `LanguageAutoRedirect` by the language menu.
    pub name: Option<String>,
    /// Abstract button index that fires it forward, if any.
    pub forward: Option<Button>,
    /// Abstract button index that fires it backward, if any.
    pub backward: Option<Button>,
    /// Target screen name from the `Default` child's `goto`.
    pub goto: Option<String>,
}

/// An `Image` widget that names a texture.
///
/// The solid-colour kind, which has a `color` and no `src`, is a [`Fill`] and
/// goes to [`Screen::fills`] instead.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// Widget name, when it has one.
    pub name: Option<String>,
    /// The texture's archive entry name, with the extension the XML gives.
    pub src: String,
    /// Left edge. Absent in the XML means zero, as it does for `Text`.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Authored width, when the XML gives one. Absent means the texture's own.
    pub width: Option<f32>,
    /// Authored height, likewise.
    pub height: Option<f32>,
    /// Modulating colour as ARGB. White when unstated, which leaves the
    /// texture's own colours alone.
    pub color: u32,
    /// The `U` attribute: left edge of the sub-rect this widget samples out
    /// of `src`, in the texture's own pixels. `None` when the XML gives none,
    /// which means the whole texture.
    ///
    /// **Several widgets share one texture this way on the real discs.**
    /// `ArrowSelect` samples `FETextures.mip` at `U="53" V="0"`; Pure's
    /// `Title Screen->Press start button` samples `FETextures_startscreen.mip`
    /// at `U="0" V="15"`, and two more widgets on that same screen sample
    /// different rects of the same file (`U="0" V="0"` and `U="26" V="15"`) -
    /// without this, every one of them would draw the file's own top-left
    /// corner instead of its own patch.
    pub u: Option<f32>,
    /// The `V` attribute, likewise.
    pub v: Option<f32>,
    /// The `TxtrWidth` attribute: width of the sampled sub-rect. `None` falls
    /// back to the placed texture's own width at draw time - every `U`/`V`
    /// measured on the real discs is paired with a `TxtrWidth`/`TxtrHeight`
    /// beside it, so nothing has been observed to need a different default.
    pub texture_width: Option<f32>,
    /// The `TxtrHeight` attribute, likewise.
    pub texture_height: Option<f32>,
    /// The `AutoLoad` attribute.
    ///
    /// Recorded but not acted on: this build loads every referenced texture up
    /// front, so there is nothing yet for a load-on-demand flag to change. It is
    /// parsed rather than dropped because the attribute is real.
    pub auto_load: bool,
}

/// A solid-colour `Image` widget: no `src`, so nothing to sample.
///
/// **Carries its own rect since 2026-08-25** - it used to be a bare ARGB
/// value on [`Screen::fills`], because every one measured until then was a
/// genuine full-screen backdrop (`Title Screen`'s and `Show Logo`'s own
/// white/black background, both authored at exactly `480x272`), so
/// `draw_backdrops` drawing every fill across the whole screen cost nothing.
/// That stopped being free the moment `<Animation>` started being recursed
/// into: `Demo_Definition.xml`, shared verbatim by both Pulse's and Pure's
/// disc, wraps two colour-only `Image`s at `width="347" height="1"` and
/// `width="151" height="1"` - a decorative underline, not a backdrop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fill {
    /// Left edge. Absent in the XML means zero, as it does for [`Image`].
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Authored width, when the XML gives one. Absent means the whole
    /// screen, since every colour-only `Image` that omits it is a
    /// full-screen backdrop on the discs measured - the same reading
    /// [`Image::width`] gives a `src` widget with none.
    pub width: Option<f32>,
    /// Authored height, likewise.
    pub height: Option<f32>,
    /// ARGB, resolved through `FEGlobals->` the same as every other colour
    /// this format authors.
    pub color: u32,
}

/// A `Menu` widget: a named, selectable list, and where/how its rows draw.
///
/// The picker's `Menu` has no per-row positions of its own - `DisplayLanguages`
/// populates it with one row per language the disc offers, and every row shares
/// this one widget's `x`, `scale`, `color` and `align`, stepping down from `y`
/// by however tall a row is. Nothing in the XML states a row height; see
/// `docs/architecture/frontend-boot.md` for how that gap is closed.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    /// The `name` attribute, `"Language"` on the picker.
    pub name: String,
    /// Left edge of every row.
    pub x: f32,
    /// Top edge of the first row.
    pub y: f32,
    /// Scale multiplier, shared by every row.
    pub scale: f32,
    /// ARGB, as the XML writes it, shared by every row.
    pub color: u32,
    /// `left`, `right` or `centre`.
    pub align: String,
    /// Font name, e.g. `Default`.
    pub font: String,
}

/// One screen, flattened out of the tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Screen {
    /// The `name` attribute.
    pub name: String,
    /// The `type` attribute, when present. `Language Selection` has one.
    pub kind: Option<String>,
    /// Full `Parent->Child` path, matching the state machine's naming.
    pub path: String,
    /// Solid-colour `Image` widgets, each with its own rect.
    pub fills: Vec<Fill>,
    /// `Image` widgets that name a texture, in document order.
    pub images: Vec<Image>,
    /// `Movie` widgets in document order.
    ///
    /// **A `Vec`, because one screen really does declare several.** Pure's
    /// `Intro Screen` carries four - `IntroMovie1`, `FMV Movie`, `ProfileMovie1`
    /// and `ProfileMovie2` - and a single slot kept whichever came last, so the
    /// three before it were invisible. That silently broke the rule that
    /// [`crate::boot`] applies to a movie's sound: a widget carrying
    /// `sound="false"` could not be found and so could not mute anything.
    pub movies: Vec<Movie>,
    /// `Text` widgets in document order.
    pub texts: Vec<Text>,
    /// `Redirect` widgets in document order.
    pub redirects: Vec<Redirect>,
    /// Whether the screen has a `DisplayLanguages` widget, which is what makes
    /// it the language picker.
    pub display_languages: bool,
    /// The screen's `Menu` widget, if it has one.
    pub menu: Option<Menu>,
    /// What the screen's `<ScreenClear>` fills the frame with, as ARGB.
    ///
    /// **Not [`Self::fills`]**, which is the `<Image>`-with-a-colour-and-no-`src`
    /// spelling. The two say the same kind of thing and are kept apart because
    /// only one of them is a *clear*: a `ScreenClear` is the frame the screen
    /// starts from, and every widget on the screen - and on the screens nested
    /// inside it - is drawn over it.
    ///
    /// Wipeout HD's `FE Screen` is what this exists for:
    /// `<ScreenClear><Values Colour="FEGlobals->HD_BG">`, white, and every menu
    /// screen on that disc is a descendant of it. Read here rather than in the
    /// menus so the value comes off the disc rather than out of a constant.
    pub clear: Option<u32>,
}

impl Screen {
    /// The redirect with this widget name.
    #[must_use]
    pub fn redirect_named(&self, name: &str) -> Option<&Redirect> {
        self.redirects
            .iter()
            .find(|r| r.name.as_deref() == Some(name))
    }

    /// Where pressing `button` goes, if anywhere.
    #[must_use]
    pub fn redirect_for(&self, button: Button) -> Option<&Redirect> {
        self.redirects
            .iter()
            .find(|r| r.forward == Some(button) || r.backward == Some(button))
    }
}

/// Every screen in a front-end XML file, plus its globals.
#[derive(Debug, Clone, Default)]
pub struct Screens {
    /// Screens in document order.
    pub screens: Vec<Screen>,
    /// `FEGlobals` variables, by name.
    pub globals: HashMap<String, String>,
    /// `LoadXML` sources, in the order the root lists them.
    pub load_xml: Vec<String>,
}

impl Screens {
    /// Reads a front-end XML file.
    #[must_use]
    pub fn from_xml(xml: &str) -> Self {
        Self::from_xml_with_fallback_globals(xml, &[])
    }

    /// [`Self::from_xml`], with `fallback` globals seeded in wherever the
    /// file itself leaves a name undeclared.
    ///
    /// **Before widget collection, not after** - a global has to exist while
    /// [`Self::text_from_node`]/[`Self::image_from_node`] resolve `color`,
    /// `x` and the rest, because those bake the resolved value into the
    /// widget once and never look the name up again. Merging `fallback` into
    /// [`Self::globals`] after `from_xml` has already returned looks
    /// plausible and does nothing - every widget that needed it has already
    /// been built with whatever `resolve` found at parse time.
    ///
    /// A real declaration always wins: `fallback` only fills a name the
    /// file's own `<Variable global="...">` list never mentions, the same
    /// contract `entry().or_insert()` gives everywhere else in this crate.
    #[must_use]
    pub fn from_xml_with_fallback_globals(xml: &str, fallback: &[(&str, &str)]) -> Self {
        let root = parse(xml);
        let mut out = Self::default();

        for node in &root.children {
            out.collect_globals(node);
        }
        for &(name, value) in fallback {
            out.globals
                .entry(name.to_string())
                .or_insert_with(|| value.to_string());
        }
        for node in &root.children {
            out.collect(node, None);
        }
        out
    }

    fn collect_globals(&mut self, node: &Node) {
        if node.name.eq_ignore_ascii_case("Variable")
            && let Some(name) = node.attr("global")
            && let Some(value) = node.value("String")
        {
            self.globals.insert(name.to_string(), value.to_string());
        }
        if node.name.eq_ignore_ascii_case("LoadXML")
            && let Some(src) = node.value("src")
        {
            self.load_xml.push(src.to_string());
        }
        for child in &node.children {
            self.collect_globals(child);
        }
    }

    fn collect(&mut self, node: &Node, parent: Option<&str>) {
        if !node.name.eq_ignore_ascii_case("Screen") {
            for child in &node.children {
                self.collect(child, parent);
            }
            return;
        }

        // Anonymous `Screen` elements are grouping containers, not navigable
        // screens. They still hold children, so they are walked through.
        let Some(name) = node.attr("name") else {
            for child in &node.children {
                self.collect(child, parent);
            }
            return;
        };

        let path = match parent {
            Some(p) => format!("{p}{}{name}", crate::state_machine::SEPARATOR),
            None => name.to_string(),
        };

        let mut screen = Screen {
            name: name.to_string(),
            kind: node.attr("type").map(str::to_string),
            path: path.clone(),
            ..Screen::default()
        };

        for child in &node.children {
            self.collect_widgets(&mut screen, child, None);
        }

        self.screens.push(screen);

        for child in &node.children {
            self.collect(child, Some(&path));
        }
    }

    /// A screen's own widgets, one node at a time - and, unlike [`Self::collect`],
    /// recursing straight through a `Viewport` or an `Animation` rather than
    /// stopping at either.
    ///
    /// **No offset applied.** A `Viewport` carries its own `x`/`y`/`width`/
    /// `height` (see `Values` on the element itself), which on the one measured
    /// case - Pulse's `Show Logo->Viewport`, `x="40" y="0" height="480"
    /// width="400"` on a 480x272 screen - do not read as plausible screen-space
    /// clip or offset numbers at all (480 exceeds the screen's own height).
    /// Nothing here has worked out what they mean, so a nested widget keeps the
    /// coordinates it was authored with, exactly as if the `Viewport` were not
    /// there. That is a **no-op** for `Data\Plugins\PI001\GUI\Skin.xml`'s
    /// `Title Screen` on `pure-psp-usa.chd`, whose own `Viewport` is `x="0"
    /// y="0" width="480" height="272"` - the whole screen, verbatim - so this
    /// gap costs nothing on the one case this build actually draws through it.
    /// Confidence **60** on "no offset" as the right answer generally; **95**
    /// that it is a correct no-op for that specific screen.
    ///
    /// **`width` is not a no-op**, though: it is the one number a
    /// `widthlimited="true"` text inside the `Viewport` needs to wrap
    /// against, so `viewport_width` carries the nearest enclosing
    /// `Viewport`'s own `width` down for [`Self::text_from_node`] to read.
    /// `None` outside any `Viewport`. See [`Text::wrap_width`].
    ///
    /// **`Animation`'s own `Key` timeline is discarded, not modelled.** An
    /// `<Animation><Key Time="0" TextureWidth="-1"/><Key Time="0.1"
    /// TextureWidth="0"/><Image>...</Image></Animation>` is a reveal - Pure's
    /// `Title Screen` wraps every one of its frame-line decorations this way -
    /// and what `TextureWidth` on a `Key` means is unread (it is not the same
    /// attribute `hud.rs`'s own `<Animation><Key>` reading covers: that one is
    /// `x`/`y` as a travel, this is a texture-space size on a widget that
    /// carries its own `TxtrWidth` already). Recursing into `Animation` at all
    /// used to drop its widgets outright - nothing drew rather than something
    /// drawing at the wrong time - and dropping the timeline while keeping the
    /// widget is the same trade this front end already makes for every other
    /// widget's `delay`/`transition` attribute, neither of which is read
    /// anywhere in this crate either. `<Key>` itself is inert here: it has no
    /// case of its own, so it falls through the same as any other tag this
    /// function does not recognise.
    ///
    /// **A colour-only `Image` inside an `Animation` is collected exactly
    /// like a top-level one.** It was not always: recursing into `Animation`
    /// used to risk turning `Demo_Definition.xml`'s two underlines into
    /// full-screen washes, back when [`Fill`] was a bare ARGB value with no
    /// rect of its own. Now that it carries one, an `Animation`-wrapped
    /// colour-only `Image` draws at its own authored position the same as any
    /// other widget - which is the point: `Title Screen`'s own frame lines are
    /// authored exactly this way.
    fn collect_widgets(&self, screen: &mut Screen, child: &Node, viewport_width: Option<f32>) {
        match child.name.to_ascii_lowercase().as_str() {
            "image" => match child.value("src") {
                // A `src` names a texture; a bare colour is a [`Fill`].
                Some(src) => screen.images.push(self.image_from_node(child, src)),
                None => {
                    if let Some(fill) = self.fill_from_node(child) {
                        screen.fills.push(fill);
                    }
                }
            },
            "screenclear" => {
                // `Colour`, the British spelling, which is what every
                // `ScreenClear` on Wipeout HD's disc uses - and `color` beside
                // it because the same file spells the `Image` attribute the
                // other way, so which one an element takes is not a thing to
                // remember.
                if let Some(value) = child.value("Colour").or_else(|| child.value("color"))
                    && let Some(argb) = self.resolve(value).and_then(parse_argb)
                {
                    screen.clear = Some(argb);
                }
            }
            "movie" => screen.movies.push(Movie::from_node(child)),
            "text" => screen
                .texts
                .push(self.text_from_node(child, viewport_width)),
            "redirect" => screen.redirects.push(redirect_from_node(child)),
            "displaylanguages" => screen.display_languages = true,
            "menu" => screen.menu = Some(self.menu_from_node(child)),
            "viewport" => {
                let width = self.number(child.value("width"));
                for grandchild in &child.children {
                    self.collect_widgets(screen, grandchild, width);
                }
            }
            // Neither carries a `width` of its own to pass down - whatever the
            // enclosing `Viewport` gave keeps applying inside. `BackgroundController`
            // is Pure's own case: its `Skin.xml` wraps `FE Screen`'s background art
            // in one (`<BackgroundController><Image name="BackgroundImage">...`),
            // and before this arm existed nothing walked through it either - the
            // same silent drop `Screen` below has, just one container earlier.
            "animation" | "backgroundcontroller" => {
                for grandchild in &child.children {
                    self.collect_widgets(screen, grandchild, viewport_width);
                }
            }
            // An anonymous `Screen` is a grouping container, not a navigable
            // screen - [`Self::collect`] already says so and walks straight
            // through one to find the named screens nested inside. This is
            // that same rule applied to widgets: an anonymous `Screen`'s own
            // `Image`/`Text`/etc. children belong to whichever named screen
            // encloses it, exactly as if the wrapper were not there. Before
            // this arm existed they were silently dropped - neither an error
            // nor a picture, just gone - because nothing else in this match
            // recurses into a `Screen` tag. Pulse's own top bar is the case
            // that found it: its `<Image>` sits three anonymous `Screen`
            // levels under `Top FE Screen->FE Screen`. A *named* child is
            // left alone here - it collects its own widgets separately, the
            // next time [`Self::collect`] reaches it, and counting them twice
            // would draw a menu's own chrome on its parent's frame too.
            "screen" if child.attr("name").is_none() => {
                for grandchild in &child.children {
                    self.collect_widgets(screen, grandchild, viewport_width);
                }
            }
            _ => {}
        }
    }

    fn fill_from_node(&self, node: &Node) -> Option<Fill> {
        let color = self
            .resolve(node.value("color").unwrap_or_default())
            .and_then(parse_argb)?;
        Some(Fill {
            x: self.number(node.value("x")).unwrap_or(0.0),
            y: self.number(node.value("y")).unwrap_or(0.0),
            width: self.number(node.value("width")),
            height: self.number(node.value("height")),
            color,
        })
    }

    fn image_from_node(&self, node: &Node, src: &str) -> Image {
        Image {
            name: node.attr("name").map(str::to_string),
            src: src.to_string(),
            x: self.number(node.value("x")).unwrap_or(0.0),
            y: self.number(node.value("y")).unwrap_or(0.0),
            width: self.number(node.value("width")),
            height: self.number(node.value("height")),
            color: self
                .resolve(node.value("color").unwrap_or_default())
                .and_then(parse_argb)
                .unwrap_or(0xffff_ffff),
            u: self.number(node.value("U")),
            v: self.number(node.value("V")),
            texture_width: self.number(node.value("TxtrWidth")),
            texture_height: self.number(node.value("TxtrHeight")),
            auto_load: node.flag("AutoLoad").unwrap_or(false),
        }
    }

    fn menu_from_node(&self, node: &Node) -> Menu {
        Menu {
            name: node.attr("name").unwrap_or("Menu").to_string(),
            x: self.number(node.value("x")).unwrap_or(0.0),
            y: self.number(node.value("y")).unwrap_or(0.0),
            scale: self.number(node.value("scale")).unwrap_or(1.0),
            color: self
                .resolve(node.value("color").unwrap_or_default())
                .and_then(parse_argb)
                .unwrap_or(0xffff_ffff),
            align: node.value("align").unwrap_or("left").to_string(),
            font: node.value("font").unwrap_or("Default").to_string(),
        }
    }

    fn text_from_node(&self, node: &Node, viewport_width: Option<f32>) -> Text {
        Text {
            name: node.attr("name").map(str::to_string),
            idstring: node.value("idstring").map(str::to_string),
            string: node.value("String").map(str::to_string),
            font: node.value("font").unwrap_or("Default").to_string(),
            x: self.number(node.value("x")).unwrap_or(0.0),
            y: self.number(node.value("y")).unwrap_or(0.0),
            scale: self.number(node.value("scale")).unwrap_or(1.0),
            color: self
                .resolve(node.value("color").unwrap_or_default())
                .and_then(parse_argb)
                .unwrap_or(0xffff_ffff),
            align: node.value("align").unwrap_or("left").to_string(),
            start_enabled: node.flag("StartEnabled").unwrap_or(true),
            pulse: node.flag("pulse").unwrap_or(false),
            delay: self.number(node.value("delay")).unwrap_or(0.0),
            wrap_width: node
                .flag("widthlimited")
                .unwrap_or(false)
                .then_some(viewport_width)
                .flatten(),
        }
    }

    /// Follows a `FEGlobals->Name` indirection, or returns the value as-is.
    #[must_use]
    pub fn resolve<'a>(&'a self, value: &'a str) -> Option<&'a str> {
        if value.is_empty() {
            return None;
        }
        match value.strip_prefix("FEGlobals->") {
            // The real XML has newlines inside these values, from being written
            // across two lines in the source file.
            Some(key) => self.globals.get(key.trim()).map(String::as_str),
            None => Some(value),
        }
    }

    fn number(&self, value: Option<&str>) -> Option<f32> {
        self.resolve(value?)?.trim().parse().ok()
    }

    /// A screen by its `name`.
    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<&Screen> {
        self.screens.iter().find(|s| s.name == name)
    }

    /// The screen whose `type` is `Language Selection`, or failing that the one
    /// with a `DisplayLanguages` widget.
    ///
    /// Two ways of finding the same screen, because either alone would be a
    /// guess and agreeing makes it a determination.
    #[must_use]
    pub fn language_selection(&self) -> Option<&Screen> {
        self.screens
            .iter()
            .find(|s| s.kind.as_deref() == Some("Language Selection"))
            .or_else(|| self.screens.iter().find(|s| s.display_languages))
    }

    /// Every screen that plays a movie, in document order.
    pub fn with_movies(&self) -> impl Iterator<Item = &Screen> {
        self.screens.iter().filter(|s| !s.movies.is_empty())
    }
}

fn redirect_from_node(node: &Node) -> Redirect {
    Redirect {
        name: node.attr("name").map(str::to_string),
        forward: node.value("forward").and_then(button_from_name),
        backward: node.value("backward").and_then(button_from_name),
        goto: node
            .children_named("Default")
            .find_map(|d| d.attr("goto"))
            .map(str::to_string),
    }
}

/// Parses `0xAARRGGBB`, as every colour in the XML is written.
#[must_use]
pub fn parse_argb(value: &str) -> Option<u32> {
    let text = value.trim();
    let hex = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    u32::from_str_radix(hex, 16).ok()
}

/// Splits ARGB into linear-ish RGBA floats for the renderer.
#[must_use]
pub fn argb_to_rgba(argb: u32) -> [f32; 4] {
    let a = ((argb >> 24) & 0xff) as f32 / 255.0;
    let r = ((argb >> 16) & 0xff) as f32 / 255.0;
    let g = ((argb >> 8) & 0xff) as f32 / 255.0;
    let b = (argb & 0xff) as f32 / 255.0;
    [r, g, b, a]
}

#[cfg(test)]
mod tests;
