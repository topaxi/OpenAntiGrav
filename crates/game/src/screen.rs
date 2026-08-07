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
pub const MOVIE_EXTENSIONS: [&str; 3] = [".pmf", ".pss", ".ipf"];

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
    /// The `Movie` widget, if the screen has one.
    pub movie: Option<Movie>,
    /// `Text` widgets in document order.
    pub texts: Vec<Text>,
    /// `Redirect` widgets in document order.
    pub redirects: Vec<Redirect>,
    /// Whether the screen has a `DisplayLanguages` widget, which is what makes
    /// it the language picker.
    pub display_languages: bool,
    /// The screen's `Menu` widget, if it has one.
    pub menu: Option<Menu>,
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
        let root = parse(xml);
        let mut out = Self::default();

        for node in &root.children {
            out.collect_globals(node);
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
                "movie" => screen.movie = Some(Movie::from_node(child)),
                "text" => screen.texts.push(self.text_from_node(child)),
                "redirect" => screen.redirects.push(redirect_from_node(child)),
                "displaylanguages" => screen.display_languages = true,
                "menu" => screen.menu = Some(self.menu_from_node(child)),
                _ => {}
            }
        }

        self.screens.push(screen);

        for child in &node.children {
            self.collect(child, Some(&path));
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
        self.screens.iter().filter(|s| s.movie.is_some())
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
mod tests {
    use super::*;
    use crate::input::button;

    /// The shape of the real `Language Selection` and `LogoFMV` screens, cut
    /// down to what this module reads. Attribute spellings, the `FEGlobals->`
    /// indirection and the newline inside a value are all as they appear in
    /// `Data\Plugins\PI001\GUI\Skin.xml`.
    const SAMPLE: &str = r#"
<Screen>
  <Variable global="TitleColor"><Values String="0xFF000000"></Values></Variable>
  <Variable global="TextColor"><Values String="0xFF33A6B9"></Values></Variable>
  <Variable global="MenuXOffset"><Values String="50"></Values></Variable>
  <Variable global="MenuScale"><Values String="1.0"></Values></Variable>
  <Screen type="Language Selection" name="Language Selection">
    <Text name="LanguageText" StartEnabled="false">
      <Values idstring="Language Selection" font="Title" x="FEGlobals->
        MenuXOffset" scale="1.0" color="FEGlobals->TitleColor"></Values>
    </Text>
    <DisplayLanguages><Values clear="true"></Values></DisplayLanguages>
    <Menu name="Language" focus="false" StartEnabled="false" save="true">
      <Values align="left" font="Default" x="FEGlobals->MenuXOffset" scale="FEGlobals->MenuScale" y="46" color="FEGlobals->TextColor"></Values>
    </Menu>
    <Redirect name="LanguageAutoRedirect">
      <Values backward="none"></Values>
      <Default goto="LogoFMV"></Default>
    </Redirect>
  </Screen>
  <Screen name="LogoFMV">
    <Image transition="0"><Values width="480" height="272" color="0xFF000000"></Values></Image>
    <Movie transition="0" name="BackdropMovie">
      <Values src="Data\Movies\Intro" sound="true" autostart="true" repeat="false" autoredirect="true"></Values>
    </Movie>
    <Redirect name="AutoRedirect" StartEnabled="false">
      <Values backward="none" forward="none"></Values>
      <Default goto="Show Logo"></Default>
    </Redirect>
    <Redirect>
      <Values backward="none" forward="start"></Values>
      <Default goto="LogoFMVRedirectScreen"></Default>
    </Redirect>
  </Screen>
  <Screen name="LogoFMVRedirectScreen">
    <Redirect>
      <Values backward="none" forward="none"></Values>
      <Default goto="Show Logo"></Default>
    </Redirect>
  </Screen>
  <Screen type="FEMain" name="Top FE Screen">
    <Screen name="FE Screen">
      <Movie transition="0" name="BackdropMovie">
        <Values src="Data\Movies\Backdrop" sound="false" autostart="true" repeat="true"></Values>
      </Movie>
      <Screen name="Show Logo">
        <Image transition="0"><Values y="72" AutoLoad="true" src="Data\FE\Images\pulse_logo.mip"></Values></Image>
        <Text delay="1" transition="0">
          <Values align="right" idstring="BOOT_PRESS_START" font="Menu" pulse="true" x="460" y="220" color="0x7FFFFFFF"></Values>
        </Text>
        <Viewport>
          <Values x="40" y="0" height="480" width="400"></Values>
          <Text name="USLegalText" delay="0" transition="0">
            <Values align="left" idstring="BOOT_LEGAL" font="small" scale="0.7" x="40" y="250" widthlimited="true" color="0x7FFFFFFF"></Values>
          </Text>
        </Viewport>
        <Redirect>
          <Values backward="none"></Values>
          <Default goto="RemoveMemoryStickWarning"></Default>
        </Redirect>
        <Redirect>
          <Values backward="none" forward="start"></Values>
          <Default goto="RemoveMemoryStickWarning"></Default>
        </Redirect>
      </Screen>
    </Screen>
  </Screen>
  <LoadXML><Values src="Data\Plugins\PI001\GUI\MainMenu_Definition.xml"></Values></LoadXML>
</Screen>
"#;

    #[test]
    fn parses_nested_elements() {
        let root = parse(SAMPLE);
        assert_eq!(root.name, "#document");
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].name, "Screen");
    }

    #[test]
    fn values_children_carry_their_parents_attributes() {
        let screens = Screens::from_xml(SAMPLE);
        let movie = screens.by_name("LogoFMV").unwrap().movie.clone().unwrap();
        assert_eq!(movie.src, r"Data\Movies\Intro");
        assert!(movie.sound);
        assert!(movie.autostart);
        assert!(movie.autoredirect);
        assert!(!movie.repeat);
    }

    #[test]
    fn the_movie_filename_is_built_from_src() {
        let screens = Screens::from_xml(SAMPLE);
        let movie = screens.by_name("LogoFMV").unwrap().movie.clone().unwrap();
        assert_eq!(movie.entry_name(), r"Data\Movies\Intro.PMF");

        let localised = Movie {
            localised: true,
            ..movie
        };
        assert_eq!(localised.entry_name(), r"Data\Movies\Intro_US.PMF");
    }

    #[test]
    fn a_ps2_src_already_names_its_container() {
        // Both spellings are read off the PS2 disc's own Skin.xml. Appending
        // .PMF to either asks for a file that exists nowhere, which is what
        // this build did before: `Data\Movies\Backdrop.ipf.PMF`.
        let screens = Screens::from_xml(SAMPLE);
        let template = screens.by_name("LogoFMV").unwrap().movie.clone().unwrap();

        for src in [r"Data\Movies\Intro.pss", r"Data\Movies\Backdrop.ipf"] {
            let movie = Movie {
                src: src.to_string(),
                ..template.clone()
            };
            assert_eq!(movie.entry_name(), src);
            // `localised` must not reintroduce the suffix either.
            let localised = Movie {
                localised: true,
                ..movie
            };
            assert_eq!(localised.entry_name(), src);
        }
    }

    #[test]
    fn an_extensionless_src_still_gets_the_psp_suffix() {
        // The PSP rule is settled and this is its guard: nothing about the PS2
        // divergence may change what `Data\Movies\Intro` resolves to.
        let screens = Screens::from_xml(SAMPLE);
        let movie = screens.by_name("LogoFMV").unwrap().movie.clone().unwrap();
        assert!(!has_movie_extension(&movie.src));
        assert_eq!(movie.entry_name(), r"Data\Movies\Intro.PMF");
    }

    #[test]
    fn finds_the_language_selection_screen_two_ways() {
        let screens = Screens::from_xml(SAMPLE);
        let screen = screens.language_selection().unwrap();
        assert_eq!(screen.name, "Language Selection");
        assert_eq!(screen.kind.as_deref(), Some("Language Selection"));
        assert!(screen.display_languages);
        let menu = screen.menu.as_ref().unwrap();
        assert_eq!(menu.name, "Language");
    }

    #[test]
    fn the_menu_widget_s_own_layout_resolves_through_feglobals() {
        let screens = Screens::from_xml(SAMPLE);
        let menu = screens
            .by_name("Language Selection")
            .unwrap()
            .menu
            .as_ref()
            .unwrap();
        assert_eq!(menu.x, 50.0, "x came from FEGlobals->MenuXOffset");
        assert_eq!(menu.y, 46.0);
        assert_eq!(menu.scale, 1.0, "scale came from FEGlobals->MenuScale");
        assert_eq!(
            menu.color, 0xff33_a6b9,
            "color came from FEGlobals->TextColor"
        );
        assert_eq!(menu.align, "left");
        assert_eq!(menu.font, "Default");
    }

    #[test]
    fn resolves_the_feglobals_indirection_across_a_newline() {
        let screens = Screens::from_xml(SAMPLE);
        let text = &screens.by_name("Language Selection").unwrap().texts[0];
        assert_eq!(text.x, 50.0, "x came from FEGlobals->MenuXOffset");
        assert_eq!(text.color, 0xff00_0000);
        assert_eq!(text.idstring.as_deref(), Some("Language Selection"));
        assert!(!text.start_enabled);
    }

    #[test]
    fn redirects_map_buttons_to_screens() {
        let screens = Screens::from_xml(SAMPLE);
        let logo = screens.by_name("LogoFMV").unwrap();

        let auto = logo.redirect_named("AutoRedirect").unwrap();
        assert_eq!(auto.goto.as_deref(), Some("Show Logo"));
        assert_eq!(auto.forward, None, "\"none\" is not a button");

        let start = logo.redirect_for(button::START).unwrap();
        assert_eq!(start.goto.as_deref(), Some("LogoFMVRedirectScreen"));
    }

    #[test]
    fn nested_screens_get_a_hierarchical_path() {
        let screens = Screens::from_xml(SAMPLE);
        // `Show Logo` is nested three deep and **not** under `LogoFMV`, which
        // only names it as a `goto` target. Worth asserting precisely, because
        // its parent is where the menu backdrop movie lives.
        assert_eq!(
            screens.by_name("Show Logo").unwrap().path,
            "Top FE Screen->FE Screen->Show Logo"
        );
        assert_eq!(
            screens.by_name("FE Screen").unwrap().path,
            "Top FE Screen->FE Screen"
        );
        assert_eq!(screens.by_name("LogoFMV").unwrap().path, "LogoFMV");
    }

    #[test]
    fn show_logo_advances_on_start_and_nothing_else() {
        let screens = Screens::from_xml(SAMPLE);
        let logo = screens.by_name("Show Logo").unwrap();

        let start = logo.redirect_for(button::START).unwrap();
        assert_eq!(start.goto.as_deref(), Some("RemoveMemoryStickWarning"));
        // `LogoFMV` takes all five; the screen that says "Press START button"
        // takes one. Nothing else on it is a redirect with a button, and there
        // is no timer of any kind.
        assert!(logo.redirect_for(button::CROSS).is_none());
        assert!(logo.redirect_for(button::CIRCLE).is_none());
        assert_eq!(logo.redirects.len(), 2);
    }

    #[test]
    fn a_text_inside_a_viewport_is_not_collected() {
        // `BOOT_LEGAL` sits inside a `Viewport` and only direct children are
        // read, so the USA disc's legal line is absent by construction rather
        // than by accident. This is the guard on that gap: when `Viewport` is
        // decoded, this assertion is what has to change. See
        // `docs/architecture/frontend-boot.md`.
        let screens = Screens::from_xml(SAMPLE);
        let logo = screens.by_name("Show Logo").unwrap();
        let ids: Vec<&str> = logo
            .texts
            .iter()
            .filter_map(|t| t.idstring.as_deref())
            .collect();
        assert_eq!(ids, ["BOOT_PRESS_START"]);
    }

    #[test]
    fn the_press_start_widget_keeps_its_own_layout() {
        let screens = Screens::from_xml(SAMPLE);
        let text = &screens.by_name("Show Logo").unwrap().texts[0];
        assert_eq!(text.x, 460.0);
        assert_eq!(text.y, 220.0, "230 on the EU disc, which has no BOOT_LEGAL");
        assert_eq!(text.align, "right");
        assert_eq!(text.color, 0x7fff_ffff);
        assert_eq!(text.font, "Menu");
    }

    #[test]
    fn the_logo_image_has_a_y_and_no_x() {
        // Which is what makes the centring rule in `frontend.rs` load-bearing.
        let screens = Screens::from_xml(SAMPLE);
        let image = &screens.by_name("Show Logo").unwrap().images[0];
        assert_eq!(image.src, r"Data\FE\Images\pulse_logo.mip");
        assert_eq!(image.y, 72.0);
        assert_eq!(image.x, 0.0);
        assert!(image.auto_load);
    }

    #[test]
    fn collects_solid_colour_backdrops() {
        let screens = Screens::from_xml(SAMPLE);
        assert_eq!(screens.by_name("LogoFMV").unwrap().fills, vec![0xff00_0000]);
    }

    #[test]
    fn collects_load_xml_sources() {
        let screens = Screens::from_xml(SAMPLE);
        assert_eq!(
            screens.load_xml,
            vec![r"Data\Plugins\PI001\GUI\MainMenu_Definition.xml"]
        );
    }

    #[test]
    fn parses_argb() {
        assert_eq!(parse_argb("0xFF5FDBF6"), Some(0xff5f_dbf6));
        assert_eq!(parse_argb("nonsense"), None);
        assert_eq!(argb_to_rgba(0xff00_0000), [0.0, 0.0, 0.0, 1.0]);
    }
}
