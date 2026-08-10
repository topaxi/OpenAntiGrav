//! The languages a disc offers, read out of the disc.
//!
//! Nothing about the language list is hard-coded. Each language is a **plugin**
//! under `Data\Plugins\`, and its `Definition.xml` carries three things this
//! module needs:
//!
//! - a `<Font Language="French">` block per font slot, naming the language;
//! - `<Entry ID="French" String="Français">`, its name in itself, which is what
//!   the picker shows;
//! - `<Entry ID="Dynamic Entry File Source" String="Data\Plugins\PI008\entries.xml">`,
//!   pointing at the string table.
//!
//! The plugin id is the only stable handle: nothing in the path says which
//! language it is. On the USA disc the five that exist are `PI008` French,
//! `PI009` German, `PI010` Spanish, `PI011` Italian and `PI012` English, and
//! that mapping was recovered by reading each plugin rather than assumed.
//!
//! See `docs/architecture/frontend-boot.md`.

use std::collections::HashMap;

use crate::screen::{Node, parse};

/// One language the disc ships.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    /// The plugin directory, for example `PI012`.
    pub plugin: String,
    /// The language's English name, as the XML spells it: `French`, `German`.
    pub name: String,
    /// The language's name in itself: `Français`, `Deutsch`.
    ///
    /// Falls back to [`name`](Self::name) when the plugin does not name itself.
    pub native_name: String,
    /// Archive entry holding the string table, if the plugin names one.
    pub entries: Option<String>,
    /// Which `.fnt` each font *role* resolves to, in document order.
    ///
    /// `<Font><Values name="Menu" Src="Data\FE\Fonts\Pulse_20.fnt">` - the
    /// role is what the screens and the title's `MenuSkin` name, and the file is
    /// what has to be read to draw it. Kept as pairs rather than a map because
    /// there are five of them and the order is the disc's.
    ///
    /// Read off the plugin rather than hard-coded, so a title whose roles
    /// differ resolves its own without this build knowing either list: Pure
    /// names `Title`, `Stats` and `scroll` where Pulse names `menu`.
    pub fonts: Vec<(String, String)>,
}

impl Language {
    /// The `.fnt` this language draws `role` in, matched case-insensitively.
    ///
    /// `None` for a role this plugin does not declare, which is an ordinary
    /// answer: the caller falls back to the default face rather than failing.
    #[must_use]
    pub fn font(&self, role: &str) -> Option<&str> {
        self.fonts
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(role))
            .map(|(_, src)| src.as_str())
    }

    /// Reads a language plugin's `Definition.xml`.
    ///
    /// Returns `None` when the file names no language, which is how a non-language
    /// plugin is told apart from a language one without a hard-coded list.
    #[must_use]
    pub fn from_definition(plugin: &str, xml: &str) -> Option<Self> {
        let root = parse(xml);
        let name = find_language_attribute(&root)?;

        let mut native_name = None;
        let mut entries = None;
        for (id, value) in string_entries(&root) {
            if id == name {
                native_name = Some(value.clone());
            } else if id == "Dynamic Entry File Source" {
                entries = Some(value.clone());
            }
        }

        Some(Self {
            fonts: font_slots(&root),
            plugin: plugin.to_string(),
            native_name: native_name.unwrap_or_else(|| name.clone()),
            name,
            entries,
        })
    }
}

/// A language's string table: ids to strings.
#[derive(Debug, Clone, Default)]
pub struct StringTable {
    entries: HashMap<String, String>,
}

impl StringTable {
    /// Reads an `entries.xml`.
    #[must_use]
    pub fn from_xml(xml: &str) -> Self {
        let root = parse(xml);
        Self {
            entries: string_entries(&root).into_iter().collect(),
        }
    }

    /// Looks up an id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&str> {
        self.entries.get(id).map(String::as_str)
    }

    /// Looks up an id, falling back to the id itself.
    ///
    /// The front end references ids that are not in the table at all: the
    /// language picker's title is `idstring="Language Selection"` and no such
    /// entry exists on the USA disc. Showing the id beats showing nothing, and
    /// makes the gap visible.
    #[must_use]
    pub fn get_or_id<'a>(&'a self, id: &'a str) -> &'a str {
        self.get(id).unwrap_or(id)
    }

    /// How many entries the table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The first `Language` attribute anywhere in the tree.
fn find_language_attribute(node: &Node) -> Option<String> {
    if let Some(value) = node.attr("Language").or_else(|| {
        node.children
            .iter()
            .filter(|c| c.name.eq_ignore_ascii_case("Values"))
            .find_map(|c| c.attr("Language"))
    }) && !value.is_empty()
    {
        return Some(value.to_string());
    }
    node.children.iter().find_map(find_language_attribute)
}

/// Which file each `<Font name=... Src=...>` role names, in document order.
///
/// A slot with no `name` or no `Src` is skipped rather than guessed at: the
/// role is the key every caller looks up by, and a nameless one could not be
/// found again anyway.
fn font_slots(node: &Node) -> Vec<(String, String)> {
    let mut out = Vec::new();
    collect_font_slots(node, &mut out);
    out
}

fn collect_font_slots(node: &Node, out: &mut Vec<(String, String)>) {
    if node.name.eq_ignore_ascii_case("Font")
        && let Some(name) = node.value("name")
        && let Some(src) = node.value("Src")
        && !name.is_empty()
        && !src.is_empty()
    {
        out.push((name.to_string(), src.to_string()));
    }
    for child in &node.children {
        collect_font_slots(child, out);
    }
}

/// Every `<Entry ID=... String=...>` in the tree, in document order.
fn string_entries(node: &Node) -> Vec<(String, String)> {
    let mut out = Vec::new();
    collect_entries(node, &mut out);
    out
}

fn collect_entries(node: &Node, out: &mut Vec<(String, String)>) {
    if node.name.eq_ignore_ascii_case("Entry")
        && let Some(id) = node.attr("ID")
    {
        out.push((
            id.to_string(),
            node.attr("String").unwrap_or_default().to_string(),
        ));
    }
    for child in &node.children {
        collect_entries(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed to shape from `Data\Plugins\PI008\Definition.xml`.
    const FRENCH: &str = r#"
<Screen name="Top" GlobalUpdate="false">
  <Font><Values name="Default" Language="French" Src="Data\FE\Fonts\pulse_text.fnt"></Values></Font>
  <Font><Values name="Menu" Language="French" Src="Data\FE\Fonts\Pulse_20.fnt"></Values></Font>
  <StringTable>
    <Entry Language="French" ID="Wipeout Pulse" String="Wipeout Pulse"></Entry>
    <Entry ID="Dynamic Entry File Source" String="Data\Plugins\PI008\entries.xml"></Entry>
    <Entry ID="French" String="Français"></Entry>
  </StringTable>
</Screen>
"#;

    #[test]
    fn reads_a_language_plugin() {
        let language = Language::from_definition("PI008", FRENCH).unwrap();
        assert_eq!(language.name, "French");
        assert_eq!(language.native_name, "Français");
        assert_eq!(
            language.entries.as_deref(),
            Some(r"Data\Plugins\PI008\entries.xml")
        );
    }

    #[test]
    fn a_plugin_with_no_language_is_not_one() {
        let other = r#"<Screen name="Top"><PI_Track name="16_Track"></PI_Track></Screen>"#;
        assert_eq!(Language::from_definition("PI001", other), None);
    }

    #[test]
    fn a_language_that_does_not_name_itself_falls_back_to_its_english_name() {
        let sparse = r#"<Screen><Font><Values Language="Dutch"></Values></Font></Screen>"#;
        let language = Language::from_definition("PI013", sparse).unwrap();
        assert_eq!(language.name, "Dutch");
        assert_eq!(language.native_name, "Dutch");
        assert_eq!(language.entries, None);
    }

    #[test]
    fn reads_a_string_table() {
        let table = StringTable::from_xml(
            r#"<StringTable>
                 <Entry Language="English" ID="BOOT_PRESS_START" String="Press START button"></Entry>
                 <Entry ID="FE_CONFIRM" String="Confirm"></Entry>
               </StringTable>"#,
        );
        assert_eq!(table.len(), 2);
        assert_eq!(table.get("FE_CONFIRM"), Some("Confirm"));
        assert_eq!(table.get("BOOT_PRESS_START"), Some("Press START button"));
    }

    #[test]
    fn a_missing_id_falls_back_to_itself() {
        let table = StringTable::default();
        assert!(table.is_empty());
        assert_eq!(table.get("Language Selection"), None);
        assert_eq!(table.get_or_id("Language Selection"), "Language Selection");
    }
}
