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

/// A parsed XML element.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Node {
    /// Element name, expanded through the file's own dictionary.
    pub name: String,
    /// Attributes in document order.
    pub attrs: Vec<(String, String)>,
    /// Child elements. Text content is discarded: this schema has none.
    pub children: Vec<Node>,
}

impl Node {
    /// An attribute by name, case-insensitively.
    ///
    /// The XML is inconsistent about case (`font="menu"` and `font="Menu"` both
    /// appear, as do `color` and `Color`), so matching exactly would silently
    /// drop attributes.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// An attribute of this element or of its `Values` child.
    #[must_use]
    pub fn value(&self, name: &str) -> Option<&str> {
        self.attr(name).or_else(|| {
            self.children
                .iter()
                .filter(|c| c.name.eq_ignore_ascii_case("Values"))
                .find_map(|c| c.attr(name))
        })
    }

    /// A boolean attribute. True for `"true"` or `"1"`, as
    /// `Movie_ParseAttributes` has it.
    #[must_use]
    pub fn flag(&self, name: &str) -> Option<bool> {
        self.value(name)
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
    }

    /// Children with this element name, case-insensitively.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children
            .iter()
            .filter(move |c| c.name.eq_ignore_ascii_case(name))
    }
}

/// Parses expanded front-end XML into a synthetic root node.
///
/// The files have several top-level elements, so the root is `"#document"`
/// rather than any element from the file.
#[must_use]
pub fn parse(xml: &str) -> Node {
    let mut stack = vec![Node {
        name: "#document".to_string(),
        ..Node::default()
    }];
    let bytes = xml.as_bytes();
    let mut at = 0usize;

    while at < bytes.len() {
        let Some(open) = find(bytes, at, b'<') else {
            break;
        };
        let Some(close) = tag_end(bytes, open) else {
            break;
        };
        at = close + 1;

        let inner = &xml[open + 1..close];
        // Comments, declarations and processing instructions carry no structure.
        if inner.starts_with('!') || inner.starts_with('?') {
            continue;
        }

        if let Some(name) = inner.strip_prefix('/') {
            let name = name.trim();
            // Tolerate a stray close tag rather than unwinding past the root.
            if stack.len() > 1
                && stack
                    .last()
                    .is_some_and(|n| n.name.eq_ignore_ascii_case(name))
            {
                let node = stack.pop().expect("checked above");
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                }
            }
            continue;
        }

        let self_closing = inner.ends_with('/');
        let inner = inner.trim_end_matches('/');
        let name_end = inner.find(char::is_whitespace).unwrap_or(inner.len());
        let node = Node {
            name: inner[..name_end].to_string(),
            attrs: attributes(&inner[name_end..]),
            children: Vec::new(),
        };

        if self_closing {
            if let Some(parent) = stack.last_mut() {
                parent.children.push(node);
            }
        } else {
            stack.push(node);
        }
    }

    // An unterminated document still yields everything it did open.
    while stack.len() > 1 {
        let node = stack.pop().expect("checked above");
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        }
    }
    stack.pop().unwrap_or_default()
}

/// One `Movie` widget's attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movie {
    /// The `src` attribute: a base name with **no extension**.
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
    /// `_US.PMF` otherwise. The intro's `src` is `Data\Movies\Intro`, which is
    /// why the filename never turned up in a string search of the executable.
    #[must_use]
    pub fn entry_name(&self) -> String {
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
    /// The `Movie` widget, if the screen has one.
    pub movie: Option<Movie>,
    /// `Text` widgets in document order.
    pub texts: Vec<Text>,
    /// `Redirect` widgets in document order.
    pub redirects: Vec<Redirect>,
    /// Whether the screen has a `DisplayLanguages` widget, which is what makes
    /// it the language picker.
    pub display_languages: bool,
    /// Whether the screen has a `Menu` widget, and its name.
    pub menu: Option<String>,
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
                "image" => {
                    // A solid colour with no `src` is a backdrop; one with a
                    // `src` is a sprite this build does not draw yet.
                    if child.value("src").is_none()
                        && let Some(color) = child.value("color")
                        && let Some(argb) = parse_argb(color)
                    {
                        screen.fills.push(argb);
                    }
                }
                "movie" => screen.movie = Some(Movie::from_node(child)),
                "text" => screen.texts.push(self.text_from_node(child)),
                "redirect" => screen.redirects.push(redirect_from_node(child)),
                "displaylanguages" => screen.display_languages = true,
                "menu" => screen.menu = Some(child.attr("name").unwrap_or("Menu").to_string()),
                _ => {}
            }
        }

        self.screens.push(screen);

        for child in &node.children {
            self.collect(child, Some(&path));
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

fn find(bytes: &[u8], from: usize, byte: u8) -> Option<usize> {
    bytes[from.min(bytes.len())..]
        .iter()
        .position(|&b| b == byte)
        .map(|at| at + from)
}

/// Index of the `>` closing the tag that starts at `open`, skipping quotes.
fn tag_end(bytes: &[u8], open: usize) -> Option<usize> {
    let mut quoted = false;
    let mut at = open + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'"' => quoted = !quoted,
            b'>' if !quoted => return Some(at),
            _ => {}
        }
        at += 1;
    }
    None
}

/// Extracts `name="value"` pairs from a tag body.
fn attributes(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut at = 0usize;

    while at < bytes.len() {
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        let key_at = at;
        while at < bytes.len() && bytes[at] != b'=' && !bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        if key_at == at || at >= bytes.len() || bytes[at] != b'=' {
            break;
        }
        let key = body[key_at..at].to_string();

        at += 1;
        if at >= bytes.len() || bytes[at] != b'"' {
            break;
        }
        at += 1;
        let value_at = at;
        while at < bytes.len() && bytes[at] != b'"' {
            at += 1;
        }
        out.push((key, body[value_at..at].to_string()));
        at += 1;
    }

    out
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
  <Variable global="MenuXOffset"><Values String="50"></Values></Variable>
  <Screen type="Language Selection" name="Language Selection">
    <Text name="LanguageText" StartEnabled="false">
      <Values idstring="Language Selection" font="Title" x="FEGlobals->
        MenuXOffset" scale="1.0" color="FEGlobals->TitleColor"></Values>
    </Text>
    <DisplayLanguages><Values clear="true"></Values></DisplayLanguages>
    <Menu name="Language" focus="false" save="true"><Values align="left"></Values></Menu>
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
    <Screen name="Show Logo">
      <Text><Values idstring="BOOT_PRESS_START" font="Menu"></Values></Text>
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
    fn finds_the_language_selection_screen_two_ways() {
        let screens = Screens::from_xml(SAMPLE);
        let screen = screens.language_selection().unwrap();
        assert_eq!(screen.name, "Language Selection");
        assert_eq!(screen.kind.as_deref(), Some("Language Selection"));
        assert!(screen.display_languages);
        assert_eq!(screen.menu.as_deref(), Some("Language"));
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
        assert_eq!(
            screens.by_name("Show Logo").unwrap().path,
            "LogoFMV->Show Logo"
        );
        assert_eq!(screens.by_name("LogoFMV").unwrap().path, "LogoFMV");
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
    fn an_angle_bracket_in_a_value_does_not_end_the_tag() {
        let node = parse(r#"<Text x="FEGlobals->X" y="2"></Text>"#);
        let text = &node.children[0];
        assert_eq!(text.attr("x"), Some("FEGlobals->X"));
        assert_eq!(text.attr("y"), Some("2"));
    }

    #[test]
    fn attributes_match_case_insensitively() {
        let node = parse(r#"<Text Color="0xFF00FF00"></Text>"#);
        assert_eq!(node.children[0].attr("color"), Some("0xFF00FF00"));
    }

    #[test]
    fn unterminated_markup_yields_what_it_opened() {
        let root = parse("<Screen name=\"Top\"><Text");
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].attr("name"), Some("Top"));
    }

    #[test]
    fn a_stray_close_tag_is_ignored() {
        let root = parse("</Screen><Screen name=\"Top\"></Screen>");
        assert_eq!(root.children.len(), 1);
    }

    #[test]
    fn parses_argb() {
        assert_eq!(parse_argb("0xFF5FDBF6"), Some(0xff5f_dbf6));
        assert_eq!(parse_argb("nonsense"), None);
        assert_eq!(argb_to_rgba(0xff00_0000), [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn self_closing_elements_close_themselves() {
        let root = parse(r#"<Screen name="Top"><Values x="1"/></Screen>"#);
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].children.len(), 1);
        assert_eq!(root.children[0].value("x"), Some("1"));
    }
}
