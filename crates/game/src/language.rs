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

/// Font role names, as a `<Font><Values name="...">` slot spells them.
///
/// **A property of the front-end format, not of either release**, which is why
/// these are here rather than in a title package: a `<Font>` slot is spelled the
/// same way whichever disc declares it, and what differs is only *which* slots a
/// disc fills in and what file each resolves to. Both of those are read off the
/// plugin at runtime by [`Language::font`], so nothing here needs a per-title
/// table of filenames - and the one place that had one is what this exists to
/// remove.
///
/// The two discs fill in **eight slots each, and share exactly one file between
/// them**, which is the whole reason to resolve rather than hard-code. Read off
/// every language plugin on `pulse-psp-eu.chd` and `pure-psp-eu.chd`,
/// 2026-08-12; confidence **94**, the files state it and nothing here has been
/// watched under an emulator.
///
/// | role | Pulse | Pure |
/// | --- | --- | --- |
/// | [`DEFAULT`] | `pulse_text.fnt` | `FX300ANG.fnt` |
/// | [`SMALL`] | `Pulse_14.fnt` | `small.fnt` |
/// | [`TITLE`] | `Pulse_14.fnt` | `FX300ANG.fnt` |
/// | [`HUD`] | `PulseHud.fnt` | `HUDFont.fnt` |
/// | [`HUD_SMALL`] | `small.fnt` | `small.fnt` |
/// | `InGame` | `Pulse_14.fnt` | `FX300ANG.fnt` |
/// | `Stats` | `Pulse_14.fnt` | `LTe50325.fnt` |
/// | `Menu` | `Pulse_20.fnt` | **no such slot** |
/// | `Scroll` | **no such slot** | `LTe50325.fnt` |
///
/// `small.fnt` is the one name in common, and it is not the body face. So the
/// build that named Pulse's files outright - which is what this replaced - drew
/// the whole of Pure's front end in the 5x7 fallback while looking entirely
/// correct on Pulse and on the PS2 port.
///
/// **`Menu` against `Scroll` is a slot difference, not a rename.** Pure's own
/// `<Menu>` widgets draw in `Default` (see `oag_pure::frontend::MENU_SKIN`);
/// what its `Scroll` face is for is the `Confirm button` / `Back button` prompt
/// rows, which say `font="scroll"` in `Skin.xml`. Nothing here maps one title's
/// slot onto the other's.
///
/// # A role is not the same file in every language
///
/// The resolver takes the first plugin that fills a slot, on the reasoning that
/// plugins differ in which *glyphs* a face carries rather than in which file a
/// role names. **That holds for every plugin either title's picker offers, and
/// is false in general**: Pure ships a Japanese plugin, `PI005`, resolving
/// `Default` and `Title` to `jap_default.fnt` and `Scroll`/`Stats` to
/// `jap_scroll.fnt` - four different files across four of the eight roles. It is
/// out of reach only because it is not one of the plugins scanned, so the
/// reasoning holds here by accident rather than by construction. A title whose
/// picker offered Japanese would have to ask the *chosen* language's plugin.
pub mod roles {
    /// Body text: the picker's rows, the loading tips, most captions.
    pub const DEFAULT: &str = "Default";
    /// The smaller face, where a screen asks for one.
    pub const SMALL: &str = "Small";
    /// Screen titles.
    pub const TITLE: &str = "Title";
    /// The in-race readouts.
    pub const HUD: &str = "HUD";
    /// The HUD's own smaller face. Both titles fill it in, both with `small.fnt`.
    pub const HUD_SMALL: &str = "HUDSmall";
}

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
        Self {
            entries: string_entries(&parse(xml)).into_iter().collect(),
        }
    }

    /// Looks up an id, **exactly**.
    ///
    /// # Why this is not case-insensitive, having briefly been
    ///
    /// Wipeout HD's front-end plugin declares `PI_Track name="01_Track"` and its
    /// string table keys the circuit `01_TRACK`, so every circuit on the RACE
    /// page draws as its own id. Folding the case resolves sixteen of the
    /// twenty-eight and **puts a confidently wrong name on eight of them**, which
    /// is worse than the id: `17_Track` loads `Data\Environments\Talons_Junction`
    /// and `17_TRACK` reads `SEBENCO CLIMB REVERSE`.
    ///
    /// The case is not the fault. **The two files come from different archives
    /// and were numbered at different times.** Measured on `hdfury-ps3-eu-dec.iso`:
    ///
    /// | Archive | `PI_Track` nodes | `17_TRACK` |
    /// | --- | ---: | --- |
    /// | `DATA00` | 28 | *no `entries.xml` at all* |
    /// | `DATA02` | 8 | `SEBENCO CLIMB REVERSE` |
    /// | `DATA03`, `DATA05` | 16 | `SEBENCO CLIMB REVERSE` |
    /// | `DATA06` | 16 | `TALON'S JUNCTION` |
    ///
    /// `DATA00` supplies the 28 circuits this build offers and `DATA06` is the
    /// only table whose numbering agrees with them - and the two are different
    /// archives, so no copy on the disc pairs the list with its own names. The
    /// same table also keeps a parallel `NN_TRACK_OLD` set (`09_TRACK_OLD` is
    /// `TALON'S JUNCTION`), which is the renumbering saying so in the data.
    ///
    /// Until which copy the runtime serves is settled - the open question
    /// `docs/formats/hd-frontend.md` already carries about `skin.xml`'s six
    /// copies - a miss here shows the id. That is an honest, visible absence;
    /// a name this build cannot vouch for is not. See
    /// [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md)'s
    /// neighbour rule in `CLAUDE.md`: never invent what the assets already
    /// author, and draw nothing rather than a legible stand-in.
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
