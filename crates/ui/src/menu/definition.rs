//! The file's own shape, checked and turned into the tree [`super`] runs.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change - the same
//! split `frame`/`rows`/`skin`/`strip` already are. The seam is real, not a
//! cut at a line number: everything here is `raw::File` in, a checked
//! [`Definition`] or an [`Error`] out, and nothing past this file reads
//! `raw::*` directly.
//!
//! # `string_id`
//!
//! A row's `label` and a page's `title` are each resolved once, here, rather
//! than wherever they are drawn: [`resolved`] takes the same [`StringTable`]
//! `menu::mode_label` already reads, and looks `string_id` (or a page's own
//! `title_string_id`) up in it when one is named, falling back to the
//! literal `label`/`title` otherwise. **This is only ever the caller's own
//! project-owned table, never the disc's** - `Definition::parse` runs before
//! any disc is open (`prepare::definition`, called from `main()` before
//! `source::resolve`), so a real disc-merged table is never in hand here.
//! See `crate::strings::project_table` for the one this build passes, and
//! the invented-UI-text handover thread for why that split exists.
//!
//! **`just check-strings` (`scripts/check-strings.py`) is what stops a new
//! screen shipping without one of these**, the same way this file's own
//! `check()` stops a dangling `target`: it is a hard failure at gate time,
//! not a runtime one, because nothing here can see the whole tree of pages
//! at once and neither can a player watching one screen ever tell that a
//! *different* screen forgot.

use std::collections::{BTreeSet, HashMap};

use crate::language::StringTable;
use oag_core::buttons::button_from_name;

use super::{
    Action, Choice, Condition, Entry, FORMAT_VERSION, Page, Pin, Restart, Value, ValueSource,
    Warning,
};

/// A parsed, checked menu tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    /// Every page, in file order.
    pub pages: Vec<Page>,
    /// Index of the page the menus open on.
    pub root: usize,
}

/// What went wrong loading a definition.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file is not TOML, or not shaped like a definition.
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    /// Written against a different vocabulary of entry kinds.
    #[error("menu format version {found}, but this build understands {FORMAT_VERSION}")]
    Version {
        /// What the file declared.
        found: u32,
    },
    /// Two pages share an id, so a `target` would be ambiguous.
    #[error("duplicate page id {0:?}")]
    DuplicatePage(String),
    /// A `target` or the `root` names a page that is not there.
    #[error("{context}: no page with id {id:?}")]
    NoSuchPage {
        /// Where the dangling reference is.
        context: String,
        /// What it named.
        id: String,
    },
    /// An `action` that nothing would handle.
    #[error("{context}: {name:?} is not an action ({known})")]
    NoSuchAction {
        /// Where it is.
        context: String,
        /// What it named.
        name: String,
        /// The closed set, listed so the message is actionable.
        known: String,
    },
    /// A `binding` naming a button that does not exist.
    #[error("{context}: {name:?} is not a button")]
    NoSuchButton {
        /// Where it is.
        context: String,
        /// What it named.
        name: String,
    },
    /// An entry missing a field its kind needs, or carrying one it cannot use.
    #[error("{context}: {problem}")]
    BadEntry {
        /// Where it is.
        context: String,
        /// What is wrong with it.
        problem: String,
    },
    /// A page nothing can reach, which is a menu entry somebody forgot to add.
    #[error("page {0:?} is not reachable from the root page")]
    Unreachable(String),
    /// A `disabled_by` naming a setting nothing edits, or a value no row can
    /// hold.
    #[error("{context}: disabled_by {setting:?}: {problem}")]
    BadCondition {
        /// Where it is.
        context: String,
        /// What it named.
        setting: String,
        /// Why that cannot work.
        problem: String,
    },
}

/// The file's own shape, before it is checked and resolved.
mod raw {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct File {
        pub version: u32,
        pub root: String,
        #[serde(default, rename = "page")]
        pub pages: Vec<Page>,
    }

    #[derive(Deserialize)]
    pub struct Page {
        pub id: String,
        #[serde(default)]
        pub title: String,
        /// An id [`super::resolved`] looks up in place of `title` - the same
        /// mechanism an entry's own `string_id` uses, one level up. See this
        /// module's own `# string_id` doc.
        #[serde(default)]
        pub title_string_id: Option<String>,
        #[serde(default, rename = "entry")]
        pub entries: Vec<Entry>,
    }

    #[derive(Deserialize)]
    pub struct Entry {
        pub kind: String,
        #[serde(default)]
        pub label: String,
        pub target: Option<String>,
        pub action: Option<String>,
        pub setting: Option<String>,
        #[serde(default)]
        pub values: Vec<String>,
        pub values_from: Option<String>,
        pub button: Option<String>,
        /// `{ setting = "...", value = ... }`: what makes this row inert. Only
        /// `choice` and `toggle` rows may carry it.
        pub disabled_by: Option<Condition>,
        /// `[{ setting = "...", value = ..., shows = "..." }, ...]`: what pins the
        /// row to one of its own options. Only `choice` rows may carry it.
        pub pinned: Option<Vec<Pinned>>,
        /// `[{ setting = "...", values = [...], message = "..." }, ...]`, refused if empty.
        pub warn_when: Option<Vec<Warning>>,
        /// `restart_required = "..."`: what to say once this row has been moved
        /// off the value the game is running on. The message rather than a
        /// `true`, for the reason a `warn_when` carries one - a marker with
        /// nothing to read is a puzzle - and because the wording belongs next to
        /// the row it describes rather than in the code. Only `choice` and
        /// `toggle` rows may carry it.
        pub restart_required: Option<String>,
        /// An id [`super::resolve`] looks up in whatever `StringTable` its
        /// caller passed, in place of `label`. See this module's own `#
        /// string_id` doc for which table that actually is at parse time.
        #[serde(default)]
        pub string_id: Option<String>,
        /// A second line, shown only while this row is selected. Omitted
        /// entirely on a row that wants none - unlike `label`, this has no
        /// required literal fallback for the loader to fall back to when
        /// only `subtitle_string_id` is named.
        pub subtitle: Option<String>,
        /// An id [`super::resolved`] looks up in place of `subtitle`, the same
        /// mechanism `string_id` is for `label`.
        pub subtitle_string_id: Option<String>,
    }

    /// `disabled_by = { setting = "display.vsync", value = "on" }`,
    /// `value = true` against a toggle, or
    /// `values = ["100", "125", "150", "200"]` for a condition that is not
    /// one-valued.
    #[derive(Deserialize)]
    pub struct Condition {
        pub setting: String,
        pub value: Option<toml::Value>,
        pub values: Option<Vec<toml::Value>>,
    }

    /// One `pinned` entry: a [`Condition`] plus the option shown while it holds.
    #[derive(Deserialize)]
    pub struct Pinned {
        #[serde(flatten)]
        pub when: Condition,
        pub shows: String,
    }

    /// `warn_when = { message = "...", all = [{ ... }, { ... }] }`.
    ///
    /// The message is not optional: a marker with nothing to read is a puzzle,
    /// not a warning. Neither is the second condition - see [`super::Warning`].
    #[derive(Deserialize)]
    pub struct Warning {
        pub all: Vec<Condition>,
        pub message: String,
    }
}

impl Definition {
    /// Parses and checks a definition.
    ///
    /// `strings` resolves a row's `string_id`, when it names one - see this
    /// module's own doc for which table a caller can actually pass here.
    ///
    /// # Errors
    ///
    /// Every problem in [`Error`]: malformed TOML, a version this build does
    /// not know, a duplicate or dangling page id, an unknown action or button,
    /// an entry missing a field its kind needs, and a page nothing links to.
    pub fn parse(text: &str, strings: &StringTable) -> Result<Self, Error> {
        let file: raw::File = toml::from_str(text)?;
        if file.version != FORMAT_VERSION {
            return Err(Error::Version {
                found: file.version,
            });
        }

        let mut index = HashMap::new();
        for page in &file.pages {
            if index.insert(page.id.clone(), index.len()).is_some() {
                return Err(Error::DuplicatePage(page.id.clone()));
            }
        }
        let root = *index.get(&file.root).ok_or_else(|| Error::NoSuchPage {
            context: "root".to_string(),
            id: file.root.clone(),
        })?;

        let mut pages = Vec::with_capacity(file.pages.len());
        for page in &file.pages {
            let mut entries = Vec::with_capacity(page.entries.len());
            let mut subtitles = Vec::with_capacity(page.entries.len());
            for (row, entry) in page.entries.iter().enumerate() {
                let context = format!("page {:?} entry {row}", page.id);
                entries.push(resolve(entry, &context, &index, strings)?);
                subtitles.push(resolve_subtitle(entry, strings));
            }
            pages.push(Page {
                id: page.id.clone(),
                title: resolved(strings, page.title_string_id.as_deref(), &page.title),
                entries,
                subtitles,
            });
        }

        let definition = Self { pages, root };
        definition.check()?;
        Ok(definition)
    }

    /// The checks that earn their keep: the ones a row cannot make about
    /// itself.
    ///
    /// Dangling targets and unknown actions are already impossible by the time
    /// this runs - [`resolve`] refuses them - so what is left are the two
    /// failures that need the whole file in view: a page that parses,
    /// resolves, and that no entry anywhere links to, and a `disabled_by`
    /// whose condition can never come true. Both ship silently otherwise - the
    /// first as a menu nobody can reach, the second as a row that is never
    /// greyed out, which looks exactly like a working one.
    ///
    /// The condition is checked against the *values the deciding row offers*,
    /// not merely against its existence, so `value = "onn"` is a startup error
    /// rather than a setting that quietly never applies. A row whose list comes
    /// off a disc has no values yet at load, and there the check stops at
    /// existence rather than guessing.
    ///
    /// # Errors
    ///
    /// [`Error::Unreachable`], naming the first orphan in file order, and
    /// [`Error::BadCondition`].
    /// Asserts a condition names a row that exists and values it can hold.
    fn check_condition(&self, condition: &Condition, context: &str) -> Result<(), Error> {
        let bad = |problem: String| Error::BadCondition {
            context: context.to_string(),
            setting: condition.setting.clone(),
            problem,
        };
        let Some(decides) = self
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .find(|other| other.setting() == Some(condition.setting.as_str()))
        else {
            return Err(bad("no row edits it".to_string()));
        };
        let offered = decides.offers();
        if offered.is_empty() {
            // A disc-supplied list: what it will hold is not knowable here.
            return Ok(());
        }
        for value in &condition.values {
            if !offered.contains(value) {
                return Err(bad(format!(
                    "that row cannot hold {value}; it offers {}",
                    offered
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        }
        Ok(())
    }

    pub fn check(&self) -> Result<(), Error> {
        for page in &self.pages {
            for (row, entry) in page.entries.iter().enumerate() {
                let context = format!("page {:?} entry {row}", page.id);
                // Both kinds of condition get the same check, because both fail
                // the same silent way: a condition naming a value no row can
                // hold never fires, and a row that never greys or never warns
                // looks exactly like one that had nothing to say.
                for condition in entry
                    .disabled_by()
                    .into_iter()
                    .chain(entry.pins().iter().map(|pin| &pin.when))
                    .chain(entry.warnings().iter().flat_map(|w| w.all.iter()))
                {
                    self.check_condition(condition, &context)?;
                }
            }
        }

        let mut seen = BTreeSet::new();
        let mut stack = vec![self.root];
        while let Some(page) = stack.pop() {
            if !seen.insert(page) {
                continue;
            }
            for entry in &self.pages[page].entries {
                if let Entry::Submenu { target, .. } = entry {
                    stack.push(*target);
                }
            }
        }

        for (index, page) in self.pages.iter().enumerate() {
            if !seen.contains(&index) {
                return Err(Error::Unreachable(page.id.clone()));
            }
        }
        Ok(())
    }

    /// The page with this id.
    #[must_use]
    pub fn page(&self, id: &str) -> Option<&Page> {
        self.pages.iter().find(|page| page.id == id)
    }

    /// Drops the RACE page's VARIANT row when `title` has no variant axis at
    /// all - `oag_title::race::RaceDefaults::has_team_variants` - rather than
    /// leaving it to draw permanently empty and unusable.
    ///
    /// **Dropped from the definition itself, not greyed and not skipped at
    /// render time.** [`Menu::is_disabled`](super::Menu::is_disabled)'s own
    /// docs are deliberate about never hiding a row: greying beats hiding
    /// because a row that vanishes mid-session leaves the player unable to
    /// find what took it away. Neither concern applies here - a title is
    /// fixed for the life of a boot, so this runs once before the row is ever
    /// drawn, and the row does not exist on Wipeout Pure on *any* team, not
    /// merely the one currently held, the way the `disabled_by` machinery's
    /// "not now" is scoped to answer. **Wipeout Pulse was believed to be the
    /// same and is not** - see `oag_title::race::HullVariant`'s own doc
    /// comment - so this only drops the row on Pure now. Row count, the
    /// scroll window and `Menu::held`'s page scan all read [`Page::entries`]
    /// directly, so removing the entry here - once, before any of the three
    /// ever sees it - is what keeps all three in agreement; skipping it only
    /// at draw time would leave a gap in the strip with the cursor still
    /// able to land on it.
    ///
    /// RACE REMIX's own VARIANT row is untouched: `remix.team` can name any
    /// title's team regardless of which one booted, so it keeps offering
    /// real variants even in a Pure-booted session.
    pub fn drop_unavailable_race_variant(&mut self, title: &oag_title::Title) {
        if title.race.has_team_variants() {
            return;
        }
        if let Some(page) = self.pages.iter_mut().find(|page| page.id == "race") {
            page.retain_rows(|entry| entry.setting() != Some("race.variant"));
        }
    }

    /// Drops every QUIT row, for a platform whose apps are not quit from
    /// inside (Android: the player leaves with the system's home or app
    /// switcher, and the app is suspended, not ended). Same once-before-read
    /// timing as [`Self::drop_unavailable_race_variant`].
    pub fn drop_quit(&mut self) {
        for page in &mut self.pages {
            page.retain_rows(|entry| {
                !matches!(
                    entry,
                    Entry::Run {
                        action: Action::Quit,
                        ..
                    }
                )
            });
        }
    }

    /// Drops every row that edits one of `settings`, for a platform that has
    /// no such thing (the web build has no monitor to pick and one present
    /// mode). Same once-before-read timing as [`Self::drop_quit`].
    pub fn drop_settings(&mut self, settings: &[&str]) {
        for page in &mut self.pages {
            page.retain_rows(|entry| entry.setting().is_none_or(|key| !settings.contains(&key)));
        }
    }

    /// Drops the CONTROLS page's TOUCH CONTROLS row, for a machine with no
    /// touchscreen to set up. The page stays defined but unreachable. Same
    /// once-before-read timing as [`Self::drop_quit`].
    pub fn drop_touch_controls(&mut self) {
        let Some(touch) = self
            .pages
            .iter()
            .position(|page| page.id == "touch_controls")
        else {
            return;
        };
        for page in &mut self.pages {
            page.retain_rows(
                |entry| !matches!(entry, Entry::Submenu { target, .. } if *target == touch),
            );
        }
    }

    /// Drops the RACE page's TEAM, VARIANT and TRACK rows on a title whose
    /// front end authors its own selection screens
    /// ([`oag_title::FrontEnd::race_box`]) - Wipeout Pulse, on both
    /// pressings - where START opens Track Select and Ship Select and every
    /// one of the three is picked there, with its preview, its ratings and
    /// its livery row. The rows would be the same choice twice, and the
    /// original's own `Single Player` page carries none of them
    /// (`docs/formats/race-setup.md`). Dropped from the definition for the
    /// reason [`Self::drop_unavailable_race_variant`] drops its row: once,
    /// before anything reads the page, so the row count, the scroll window
    /// and the cursor all agree.
    ///
    /// A title with no such screens - Pure, 2048 today - keeps all
    /// three, and START launches off the rows as before. RACE REMIX and
    /// RECORDS keep their own `race.track`/`remix.*` rows either way: the
    /// first mixes titles and the second browses records, neither of which
    /// the selection screens do.
    pub fn drop_rows_picked_on_screen(&mut self, title: &oag_title::Title) {
        // The KILLS and WEAPONS rows offer the lists of the race box's
        // `Single Player` screen, which only a title with a
        // `FrontEnd::race_setup` file has: elsewhere the row would draw
        // permanently unusable, the reason the VARIANT row is dropped too.
        let reads_setup = title
            .front_end
            .is_some_and(|front_end| front_end.race_setup.is_some());
        if !reads_setup && let Some(page) = self.pages.iter_mut().find(|page| page.id == "race") {
            page.retain_rows(|entry| {
                !matches!(entry.setting(), Some("race.kill_target" | "race.weapons"))
            });
        }
        // Both screens, for a title that authors them in files of its own
        // (Wipeout HD/Fury): with only one of the two, the other pick would
        // have no screen to be made on and its row stays.
        let picks_on_screen = title.front_end.is_some_and(|front_end| {
            front_end.race_box.is_some()
                || (front_end.track_select.is_some() && front_end.team_select.is_some())
        });
        if !picks_on_screen {
            return;
        }
        if let Some(page) = self.pages.iter_mut().find(|page| page.id == "race") {
            page.retain_rows(|entry| {
                !matches!(
                    entry.setting(),
                    Some("race.team" | "race.variant" | "race.track")
                )
            });
        }
    }
}

/// Looks `string_id` up in `strings`, falling back to `literal` when it names
/// none or the table has nothing for it - not to the id itself.
///
/// **This used to fall back to the id** (`StringTable::get_or_id`), on the
/// argument that the id space is the disc's own, so showing the id beat
/// silently drawing a literal the row had moved away from. That argument
/// held while nothing in `assets/ui/menu.toml` named an id. It stopped
/// holding on 2026-09-06, when the pilot editor's rows became the first
/// that do: those ids are **ours**, `assets/ui/strings/english.toml` is
/// the only file that carries them, and every other language overlays
/// nothing - so a French player was shown `OAG_PILOT_RENAME` where the row
/// says `RENAME`. Caught in a `--menu-page` capture on the EU disc, which
/// is exactly the machine a developer working in English would not have.
///
/// Shared between a row's own `label`/`string_id` and a page's `title`/
/// `title_string_id` - one lookup, two callers, the same fallback rule.
fn resolved(strings: &StringTable, string_id: Option<&str>, literal: &str) -> String {
    string_id
        .and_then(|id| strings.get(id))
        .map_or_else(|| literal.to_string(), ToString::to_string)
}

/// A row's `subtitle`, resolved the same way [`resolved`] resolves `label` -
/// except a subtitle is optional in the first place, so no literal is
/// required for `subtitle_string_id` to fall back to. A row naming neither
/// carries `None`, not an empty string, so [`super::rows`] can tell "no
/// subtitle" apart from "resolved to nothing".
fn resolve_subtitle(entry: &raw::Entry, strings: &StringTable) -> Option<String> {
    entry
        .subtitle
        .as_deref()
        .map(|literal| resolved(strings, entry.subtitle_string_id.as_deref(), literal))
}

/// Turns one raw entry into a resolved one, or says exactly what is wrong.
fn resolve(
    entry: &raw::Entry,
    context: &str,
    index: &HashMap<String, usize>,
    strings: &StringTable,
) -> Result<Entry, Error> {
    // A `label` is required on every entry the loader accepts, so there is
    // always one to fall back to, and for a project-owned id it *is* the
    // English source text rather than a guess at the disc's.
    let label = resolved(strings, entry.string_id.as_deref(), &entry.label);
    let missing = |field: &str| Error::BadEntry {
        context: context.to_string(),
        problem: format!("a {:?} entry needs {field}", entry.kind),
    };

    // Only a row that can be adjusted can be disabled, warned or deferred. On a
    // `back` or a `submenu` the field would parse and do nothing, which is the
    // failure the rest of this loader exists to make impossible.
    if (entry.disabled_by.is_some()
        || entry.warn_when.is_some()
        || entry.pinned.is_some()
        || entry.restart_required.is_some())
        && !matches!(entry.kind.as_str(), "choice" | "toggle")
    {
        return Err(Error::BadEntry {
            context: context.to_string(),
            problem: format!(
                "a {:?} entry cannot be disabled_by anything; only choice and toggle can",
                entry.kind
            ),
        });
    }
    let disabled_by = entry
        .disabled_by
        .as_ref()
        .map(|condition| condition_from(condition, context))
        .transpose()?;
    let pins = entry
        .pinned
        .iter()
        .flatten()
        .map(|pin| {
            Ok(Pin {
                when: condition_from(&pin.when, context)?,
                shows: pin.shows.clone(),
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    if !pins.is_empty() && entry.kind != "choice" {
        return Err(Error::BadEntry {
            context: context.to_string(),
            problem: "only a choice can be pinned to one of its options".to_string(),
        });
    }
    // `warn_when = []` is refused, the same shape as `restart_required = ""` below.
    if entry.warn_when.as_deref().is_some_and(<[_]>::is_empty) {
        return Err(Error::BadEntry {
            context: context.to_string(),
            problem: "a warn_when needs at least one warning".to_string(),
        });
    }
    let warnings = entry
        .warn_when
        .iter()
        .flatten()
        .map(|warning| warning_from(warning, context))
        .collect::<Result<Vec<_>, _>>()?;
    // An empty message is the `restart_required = true` this field deliberately
    // is not: a marker with nothing under it says something is wrong and does
    // not say what to do about it.
    let restart = match entry.restart_required.as_deref() {
        Some("") => {
            return Err(Error::BadEntry {
                context: context.to_string(),
                problem: "a restart_required needs a message: it is what the player is told"
                    .to_string(),
            });
        }
        Some(message) => Some(Restart {
            message: message.to_string(),
            in_effect: Vec::new(),
        }),
        None => None,
    };

    match entry.kind.as_str() {
        "submenu" => {
            let id = entry.target.as_ref().ok_or_else(|| missing("target"))?;
            let target = *index.get(id).ok_or_else(|| Error::NoSuchPage {
                context: context.to_string(),
                id: id.clone(),
            })?;
            Ok(Entry::Submenu { label, target })
        }
        "action" => {
            let name = entry.action.as_ref().ok_or_else(|| missing("action"))?;
            let action = Action::parse(name).ok_or_else(|| Error::NoSuchAction {
                context: context.to_string(),
                name: name.clone(),
                known: Action::all()
                    .iter()
                    .map(|a| a.name())
                    .collect::<Vec<_>>()
                    .join(", "),
            })?;
            Ok(Entry::Run { label, action })
        }
        "choice" => {
            let setting = entry.setting.as_ref().ok_or_else(|| missing("setting"))?;
            let source = match entry.values_from.as_deref() {
                Some(name) => Some(ValueSource::parse(name).ok_or_else(|| Error::BadEntry {
                    context: context.to_string(),
                    problem: format!("{name:?} is not a value source (languages, tracks)"),
                })?),
                None => None,
            };
            // A list has to come from exactly one place. Both would leave it
            // ambiguous which wins when the source turns out to be empty, and
            // neither is a row with nothing on it.
            match (source, entry.values.is_empty()) {
                (None, true) => return Err(missing("values, or a values_from")),
                (Some(_), false) => {
                    return Err(Error::BadEntry {
                        context: context.to_string(),
                        problem: "values and values_from are alternatives, not both".to_string(),
                    });
                }
                _ => {}
            }
            Ok(Entry::Choice {
                label,
                setting: setting.clone(),
                values: entry.values.iter().map(Choice::plain).collect(),
                source,
                current: 0,
                disabled_by,
                pins,
                warnings,
                restart,
            })
        }
        "toggle" => {
            let setting = entry.setting.as_ref().ok_or_else(|| missing("setting"))?;
            Ok(Entry::Toggle {
                label,
                setting: setting.clone(),
                on: false,
                disabled_by,
                warnings,
                restart,
            })
        }
        "binding" => {
            let name = entry.button.as_ref().ok_or_else(|| missing("button"))?;
            let button = button_from_name(name).ok_or_else(|| Error::NoSuchButton {
                context: context.to_string(),
                name: name.clone(),
            })?;
            Ok(Entry::Binding { label, button })
        }
        "back" => Ok(Entry::Back { label }),
        other => Err(Error::BadEntry {
            context: context.to_string(),
            problem: format!("{other:?} is not an entry kind"),
        }),
    }
}

/// Turns one raw condition into a resolved one.
///
/// A string becomes the value a `choice` row stores and a boolean becomes what
/// a `toggle` row holds. Nothing else is accepted: a number would have to
/// decide silently whether `60` is the text `"60"` a frame-limit row stores or
/// something else, and guessing is how a condition that never fires ships.
fn condition_from(raw: &raw::Condition, context: &str) -> Result<Condition, Error> {
    let bad = |problem: String| Error::BadCondition {
        context: context.to_string(),
        setting: raw.setting.clone(),
        problem,
    };
    // `value` and `values` are alternatives. Both would leave it ambiguous
    // which wins, and neither is a condition that can ever hold - the same
    // rule, for the same reason, as `values` and `values_from` on an entry.
    let raws: Vec<&toml::Value> = match (&raw.value, &raw.values) {
        (Some(_), Some(_)) => {
            return Err(bad(
                "value and values are alternatives, not both".to_string()
            ));
        }
        (None, None) => return Err(bad("needs a value, or a values".to_string())),
        (Some(one), None) => vec![one],
        (None, Some(many)) if many.is_empty() => {
            return Err(bad("an empty values can never hold".to_string()));
        }
        (None, Some(many)) => many.iter().collect(),
    };
    let values = raws
        .into_iter()
        .map(|value| match value {
            toml::Value::String(text) => Ok(Value::Text(text.clone())),
            toml::Value::Boolean(flag) => Ok(Value::Flag(*flag)),
            other => Err(bad(format!(
                "a condition's value is a string or a boolean, not {}",
                other.type_str()
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Condition {
        setting: raw.setting.clone(),
        values,
    })
}

fn warning_from(raw: &raw::Warning, context: &str) -> Result<Warning, Error> {
    if raw.all.len() < 2 {
        return Err(Error::BadEntry {
            context: context.to_string(),
            problem: "a warn_when needs at least two conditions: a warning about                       one setting alone describes a row that should not exist"
                .to_string(),
        });
    }
    Ok(Warning {
        all: raw
            .all
            .iter()
            .map(|condition| condition_from(condition, context))
            .collect::<Result<Vec<_>, _>>()?,
        message: raw.message.clone(),
    })
}
