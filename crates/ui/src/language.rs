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
//!   pointing at the string table - or, when a plugin names none, its strings
//!   are stated inline in `Definition.xml` instead: Pure's `PI000` (English)
//!   does this, and `boot::load_strings` falls
//!   back to re-parsing the definition itself in that case.
//!
//! The plugin id is the only stable handle: nothing in the path says which
//! language it is, **and it is not a shared convention between titles.**
//! On the Pulse USA disc the five that exist are `PI008` French, `PI009`
//! German, `PI010` Spanish, `PI011` Italian and `PI012` English; on Pure the
//! same four non-English plugins keep their ids but English is `PI000`, not
//! `PI012` - `PI012` on Pure is an unrelated American-spelling patch with no
//! `<Font>` block of its own. Each mapping was recovered by reading every
//! plugin on its own disc, never carried over from the other title. See
//! `docs/formats/pure-status.md#the-language-plugin-id-space-is-pures-own-not-pulses`.
//!
//! See `docs/architecture/frontend-boot.md`.

use std::collections::HashMap;

use crate::screen::{Node, parse};

pub mod load;

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
    /// The PlayStation button glyphs (`ps_buttons.fnt`/`PS_BUTTONS.fnt`) a
    /// `font="buttons"` widget names - Wipeout HD/Fury's
    /// `NavigationController` icon halves and `Cell Selection`'s own
    /// `DifficultyButtonIcon`. **Measured on HD only**: Omega carries HD's
    /// own `PI001` front-end plugin forward (this crate's own table), so it
    /// very likely declares the same slot, but this pass could not open
    /// `data/images/omega-ps4-eu.pkg` directly (`no ISO 9660 primary volume
    /// descriptor found` - the day-one patch's own `data09.psarc` needs
    /// extracting first, not attempted here) to check. Not one of the five
    /// roles `Language`'s own
    /// module doc table was measured over (Pulse/Pure, 2026-08-12) -
    /// recovered separately, off HD's own `definition.xml` (`docs/formats/hd-frontend.md`'s
    /// "`menu_font` is `None`, and that is a measurement" section: all 32
    /// language plugins on the disc declare a `Buttons` slot resolving to
    /// `Data\FE\Fonts\PS_BUTTONS.fnt`). Whether Pulse/Pure declare the same
    /// slot name is not yet checked; `Language::font` answers `None` for a
    /// title that does not, the same as any other role.
    pub const BUTTONS: &str = "Buttons";
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
    /// `borderExtendPixels` per font role: how many atlas pixels around a
    /// glyph's metric box hold the baked halo the quad must also draw.
    ///
    /// `<Font><Values name="HUD" ... borderExtendPixels="5">`. Only roles that
    /// author it appear; [`Self::border_extend`] answers `0` for the rest.
    pub font_borders: Vec<(String, u32)>,
    /// The per-title namespace of this project's disc-keyed translations that
    /// applies to this language, read by [`crate::strings::overlay_disc`].
    ///
    /// `Some` only on a language this build adds on top of the disc's own
    /// ([`crate::strings::PROJECT_LANGUAGES`]): its `plugin`, `entries` and
    /// `fonts` are the disc's English ones, so the table underneath it is the
    /// disc's English text, and this names what overlays it.
    pub disc_strings: Option<&'static str>,
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

    /// The `borderExtendPixels` this language authors for `role`, matched
    /// case-insensitively; `0` when it authors none.
    #[must_use]
    pub fn border_extend(&self, role: &str) -> u32 {
        self.font_borders
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(role))
            .map_or(0, |(_, px)| *px)
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
            font_borders: font_borders(&root),
            plugin: plugin.to_string(),
            native_name: native_name.unwrap_or_else(|| name.clone()),
            name,
            entries,
            disc_strings: None,
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

    /// Looks up an id, **exactly**: what this copy of the table literally says.
    ///
    /// # Exact, and why the circuits go through [`CircuitNames`] instead
    ///
    /// Wipeout HD keys its circuits `NN_TRACK` where the front-end plugin
    /// declares `PI_Track name="NN_Track"`, so nothing on the RACE page resolved
    /// here and every circuit drew as its own id. This used to argue that the
    /// case was the *only* thing wrong and folding it was still refused, because
    /// folding resolved sixteen of the twenty-eight and put a **confidently
    /// wrong** name on eight: `17_Track` loads `Data\Environments\Talons_Junction`
    /// and the served table's `17_TRACK` read `SEBENCO CLIMB REVERSE`.
    ///
    /// That was right about the fold and wrong about the conclusion, and the
    /// missing measurement was **which copy**. HD ships this file in five
    /// archives and they disagree; the served one is `DATA02`'s, which numbers
    /// the circuits the way the base game did. Read off `hdfury-ps3-eu-dec.iso`:
    ///
    /// | Archive | `NN_TRACK` keys | `17_TRACK` reads |
    /// | --- | ---: | --- |
    /// | `DATA00` | *no `entries.xml` at all* | - |
    /// | `DATA02`, `DATA03`, `DATA04`, `DATA05` | 24 | `SEBENCO CLIMB REVERSE` |
    /// | `DATA06` | 28 | `TALON'S JUNCTION` |
    ///
    /// `DATA00` supplies the 28 circuits this build offers, and `DATA06` is the
    /// only copy that names all 28 - the other four stop at 24 and have no key
    /// at all for the four Zone circuits. So the copy is chosen on coverage and
    /// the fold applies to that copy alone; see [`CircuitNames`], which also
    /// records what corroborates the choice.
    ///
    /// **The fold is unavoidable, not a shortcut.** No exact spelling works for
    /// all 28 in any copy: `DATA06` writes `01_TRACK` upper and `25_Track`
    /// mixed, against a plugin that writes `NN_Track` throughout.
    ///
    /// This accessor stays exact so "what does this table say for this key" has
    /// an answer that is not a search, and so the mismatch above stays
    /// measurable rather than papered over.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&str> {
        self.entries.get(id).map(String::as_str)
    }

    /// Overlays `other`, which wins over any id already present.
    ///
    /// The one caller is [`crate::strings::overlay`] - see its own doc for
    /// why "wins" is the right direction: a project file is meant to
    /// override a disc entry, not lose to one.
    pub fn merge(&mut self, other: HashMap<String, String>) {
        self.entries.extend(other);
    }

    /// Adds the ids `other` has that this table lacks or holds empty, leaving
    /// every id the table already says something for as it is.
    pub fn fill(&mut self, other: HashMap<String, String>) {
        for (id, text) in other {
            let entry = self.entries.entry(id).or_default();
            if entry.trim().is_empty() {
                *entry = text;
            }
        }
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

    /// Looks up an id with the case folded, walking the table.
    ///
    /// **Deliberately not the general accessor**, and not a shortcut past
    /// [`Self::get`]'s reasoning: folding is only ever right once a caller has
    /// established which copy of the table it is folding, which is a per-key
    /// question rather than a per-table one. [`CircuitNames`] is the one caller
    /// that has established it, and it tries [`Self::get`] first so an exact
    /// key always wins.
    ///
    /// **A tie would resolve arbitrarily** - this walks a `HashMap` - so it is
    /// worth knowing that none exists to resolve: all five of HD's copies were
    /// counted key by key with the case folded and none holds two keys that
    /// differ only in case. Nothing here feeds simulation state; a label is not
    /// a tick.
    #[must_use]
    pub fn find_ignoring_case(&self, id: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(id))
            .map(|(_, value)| value.as_str())
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

/// What a circuit is called, out of the copy of the string table that names
/// every circuit the source offers.
///
/// # Why the circuits need their own table when nothing else does
///
/// A `PI_Track`'s `name` is an **id** - the folder a setting stores - and the
/// name a player reads is that id looked up in the language's `entries.xml`.
/// On both PSP titles the two files are the same age and the lookup is exact,
/// which is why every other label on the front end goes straight through
/// [`StringTable::get_or_id`].
///
/// Wipeout HD breaks that in two ways at once, and fixing either alone gives a
/// wrong answer:
///
/// 1. **The copies disagree.** `entries.xml` is in five of the seven archives.
///    `oag_assets::Archives` serves `DATA02`'s, whose circuit numbering is the
///    base game's; the circuit *list* comes out of `DATA00`'s
///    `definition.xml`, whose numbering is Fury's. Fold the case with `DATA02`
///    served and eight circuits get a confident wrong name - `17_Track` loads
///    `Talons_Junction` and `DATA02` reads its `17_TRACK` as `SEBENCO CLIMB
///    REVERSE`.
/// 2. **The spelling differs.** The plugin writes `NN_Track`; every table
///    writes `NN_TRACK`, except `DATA06`'s four Zone keys, which are
///    `25_Track`..`28_Track`. **No exact spelling resolves all 28 in any
///    copy**, so the fold is required once the right copy is in hand.
///
/// So this picks the copy first and folds second.
///
/// # How the copy is chosen, and what corroborates it
///
/// **Coverage: the copy that has a key for every circuit the source offers.**
/// Measured on `hdfury-ps3-eu-dec.iso` over **all sixteen languages in all five
/// archives**, with no exception: `DATA02`, `DATA03`, `DATA04` and `DATA05`
/// carry 24 `NN_TRACK` keys and `DATA06` carries 28. The four missing everywhere else
/// are the Zone circuits - `DATA00`'s `25_Track`..`28_Track`, which load
/// `Data\Environments\Zone_1`..`Zone_4`. One copy qualifies and it is not the
/// served one.
///
/// **What corroborates it is the geometry**, and it is checked rather than
/// assumed: twelve of HD's environments are declared twice, once each way
/// round, and on the chosen copy every such pair reads the same name -
/// `17_Track` and `18_Track` both load `Talons_Junction` and both read
/// `TALON'S JUNCTION`. On the served copy they read `SEBENCO CLIMB REVERSE`
/// and `SOL 2 REVERSE`: two names for one piece of track, which is the
/// mismatch stating itself. That agreement is **not** the selector, because it
/// is false by design on Pulse, where `16_Track` and `32_Track` share an
/// environment and are named separately; it is asserted in
/// `crates/game/tests/hd_boot_ground_truth.rs`.
///
/// **Measured against the real front end, 2026-08-30**, not only inferred
/// from archive geometry: every one of Racebox's 24 `Track Creation` carousel
/// entries reads its name with no `REVERSE` suffix, `Talons_Junction`'s two
/// entries both included - see
/// `docs/formats/hd-frontend.md#a-circuits-name-is-in-a-different-archive-from-the-circuit-list`.
///
/// A source with one copy - which is every PSP title - selects that copy, folds
/// a spelling that already matched, and comes out exactly where
/// [`StringTable::get_or_id`] left it.
#[derive(Debug, Clone, Default)]
pub struct CircuitNames {
    /// Lowercased id to name.
    by_id: HashMap<String, String>,
    /// The archive the chosen copy came from, for the boot report.
    source: String,
}

impl CircuitNames {
    /// Picks the copy of the table that names every id in `ids`.
    ///
    /// `copies` is `(archive label, table)` in mount order, so a tie - every
    /// copy covering the list, which is what a single-copy source looks like -
    /// keeps the copy `oag_assets::Archives::read_name` would have served.
    ///
    /// `None` when no copy covers the list. That leaves every circuit showing
    /// its id, which is what this build did before the copies were counted: an
    /// honest, visible absence rather than a name it cannot vouch for.
    #[must_use]
    pub fn choose(copies: &[(String, StringTable)], ids: &[String]) -> Option<Self> {
        let (source, table) = copies
            .iter()
            .find(|(_, table)| ids.iter().all(|id| folded(table, id).is_some()))?;
        Some(Self {
            by_id: ids
                .iter()
                .filter_map(|id| {
                    folded(table, id).map(|name| (id.to_lowercase(), name.to_string()))
                })
                .collect(),
            source: source.clone(),
        })
    }

    /// This circuit's name, or `None` when no copy named it.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&str> {
        self.by_id.get(&id.to_lowercase()).map(String::as_str)
    }

    /// The archive the chosen copy came from.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// How many circuits it names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether it names none, which is what an unchosen copy leaves behind.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// One id looked up with the case folded, for [`CircuitNames`] alone.
///
/// A linear walk rather than a second map: a string table is read once at boot
/// and this runs over at most 28 ids, so the cost is invisible and the
/// alternative is a second copy of every key in memory for the rest of the run.
fn folded<'a>(table: &'a StringTable, id: &str) -> Option<&'a str> {
    table
        .get(id)
        .or_else(|| table.find_ignoring_case(id))
        .filter(|name| !name.is_empty())
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

/// Each font role's `borderExtendPixels`, for the slots that author one.
fn font_borders(node: &Node) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    collect_font_borders(node, &mut out);
    out
}

fn collect_font_borders(node: &Node, out: &mut Vec<(String, u32)>) {
    if node.name.eq_ignore_ascii_case("Font")
        && let Some(name) = node.value("name")
        && let Some(px) = node.value("borderExtendPixels")
        && let Ok(px) = px.trim().parse::<u32>()
        && !name.is_empty()
    {
        out.push((name.to_string(), px));
    }
    for child in &node.children {
        collect_font_borders(child, out);
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

    #[test]
    fn reads_the_border_a_font_role_authors() {
        let xml = r#"<Screen name="Top">
  <Font><Values name="HUD" Language="English" Src="Data\FE\Fonts\PulseHud.fnt" borderExtendPixels="5"></Values></Font>
  <Font><Values name="HUDSmall" Language="English" Src="Data\FE\Fonts\small.fnt" borderExtendPixels="3"></Values></Font>
  <Font><Values name="Menu" Language="English" Src="Data\FE\Fonts\Pulse_20.fnt"></Values></Font>
</Screen>"#;
        let language = Language::from_definition("PI012", xml).unwrap();
        assert_eq!(language.border_extend("HUD"), 5);
        assert_eq!(language.border_extend("hudsmall"), 3);
        assert_eq!(language.border_extend("Menu"), 0, "authored none");
        assert_eq!(language.border_extend("Absent"), 0);
    }
}
