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
}

/// One `Redirect`: a button, and the screen it goes to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Redirect {
    /// Widget name, when it has one. `AutoRedirect` is fired by a finishing
    /// movie; `LanguageAutoRedirect` by the language menu.
    pub name: Option<String>,
    /// Abstract button index that fires it forward, if any.
    pub forward: Option<u8>,
    /// Abstract button index that fires it backward, if any.
    pub backward: Option<u8>,
    /// Target screen name from the `Default` child's `goto`.
    pub goto: Option<String>,
}

/// An `Image` widget that names a texture.
///
/// The solid-colour kind, which has a `color` and no `src`, is a backdrop and
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
    /// The `AutoLoad` attribute.
    ///
    /// Recorded but not acted on: this build loads every referenced texture up
    /// front, so there is nothing yet for a load-on-demand flag to change. It is
    /// parsed rather than dropped because the attribute is real.
    pub auto_load: bool,
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
    /// Solid-colour `Image` backdrops, as ARGB.
    pub fills: Vec<u32>,
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
    pub fn redirect_for(&self, button: u8) -> Option<&Redirect> {
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
            self.collect_widgets(&mut screen, child);
        }

        self.screens.push(screen);

        for child in &node.children {
            self.collect(child, Some(&path));
        }
    }

    /// A screen's own widgets, one node at a time - and, unlike [`Self::collect`],
    /// recursing straight through a `Viewport` rather than stopping at it.
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
    fn collect_widgets(&self, screen: &mut Screen, child: &Node) {
        match child.name.to_ascii_lowercase().as_str() {
            "image" => match child.value("src") {
                // A `src` names a texture; a bare colour is a backdrop.
                Some(src) => screen.images.push(self.image_from_node(child, src)),
                None => {
                    if let Some(color) = child.value("color")
                        && let Some(argb) = parse_argb(color)
                    {
                        screen.fills.push(argb);
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
            "text" => screen.texts.push(self.text_from_node(child)),
            "redirect" => screen.redirects.push(redirect_from_node(child)),
            "displaylanguages" => screen.display_languages = true,
            "menu" => screen.menu = Some(self.menu_from_node(child)),
            "viewport" => {
                for grandchild in &child.children {
                    self.collect_widgets(screen, grandchild);
                }
            }
            _ => {}
        }
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

    fn text_from_node(&self, node: &Node) -> Text {
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
