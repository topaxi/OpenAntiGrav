//! The shell's menus: our own definition, not the disc's.
//!
//! Like [`frontend`](crate::frontend), this holds no GPU handles, opens no
//! files and reads no clock. It takes input in and emits [`MenuEvent`]s and a
//! list of [`Draw`](crate::frontend::Draw)s out, both of which are plain data;
//! `main.rs` rasterises the one and acts on the other. It moves to `oag-ui`
//! when the HUD arrives and that crate exists - see
//! `docs/architecture/workspace-layout.md`.
//!
//! # Why this is not the front-end XML
//!
//! The disc's own `MainMenu_Definition.xml` describes the menus *Pulse* has, on
//! a PSP, for a player with a Memory Stick. This project needs the menus a PC
//! game has: input bindings, graphics settings, a window mode, things the
//! original had no reason to carry. Reproducing the disc's tree and then
//! bolting extra branches onto it would leave a structure that is neither
//! faithful nor ours, and every added entry would look like a recovered one.
//!
//! So the shape is ours and it is written down in [`assets/ui/menu.toml`],
//! which ships with the source rather than coming off a disc. What the disc's
//! XML says the original's menus were is a separate, still-open question, and
//! belongs in `docs/` when someone reads it - not in here.
//!
//! [`assets/ui/menu.toml`]: https://github.com/topaxi/OpenAntiGrav/blob/main/assets/ui/menu.toml
//!
//! # The definition is flat, and that is deliberate
//!
//! Pages are a **list keyed by id**, and a `submenu` entry names a target id
//! rather than nesting a page inside itself. Three things fall out of that and
//! all three are worth the slightly less pretty file:
//!
//! - TOML past two levels of nesting is unpleasant to author by hand, and this
//!   file is meant to be edited by hand.
//! - "every target resolves, every action is known, every page is reachable"
//!   becomes one pass over a flat map, which is
//!   [`Definition::check`] and runs in CI.
//! - the back stack is a `Vec<usize>` of page indices, which cannot hold a
//!   dangling page and is trivial to assert on.
//!
//! # What a menu here cannot do
//!
//! It never reads or writes [`Settings`](crate::settings::Settings). A `choice`
//! entry is seeded with its current value by whoever owns the setting and emits
//! [`MenuEvent::Changed`] when the player moves it; applying and persisting that
//! is the composition root's business. Keeping that seam is what lets every test
//! below run with no config directory, no disc and no window.

use std::collections::{BTreeSet, HashMap};

use crate::frontend::{Align, Draw};
use crate::input::{Input, button, button_from_name};

/// Reads a button as an **edge** and consumes it in one step.
///
/// Menus are not held down, and one press must not be seen twice - by two rows,
/// or by the menus and then by whatever a transition lands on. The front end
/// spells the same pair out at every site it needs it (`is_pressed` then
/// `consume_press`); this is that pair with a name, because a menu tick needs
/// six of them and the shape is what matters rather than each instance.
fn take(input: &mut Input, index: u8) -> bool {
    if input.is_pressed(index) {
        input.consume_press(index);
        true
    } else {
        false
    }
}

/// The menu tree this build ships with.
///
/// Embedded rather than read from disk so the binary works from anywhere, with
/// [`Definition::parse`] available for an override path - see `--menu`.
pub const BUILT_IN: &str = include_str!("../../../assets/ui/menu.toml");

/// Definition-format version this build understands.
///
/// Refused rather than best-effort parsed: a definition written against a
/// different entry vocabulary would otherwise load with entries silently
/// missing, which is the failure mode this whole module is arranged to avoid.
pub const FORMAT_VERSION: u32 = 1;

/// Every action a menu entry is allowed to name.
///
/// A closed set, checked by [`Definition::check`], so a typo in the asset is a
/// load error rather than an entry that does nothing when pressed. Adding an
/// action means adding it here and handling it in the composition root, which
/// is the point: the compiler and the check together make "defined but not
/// wired up" impossible to ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    /// Leave the menus and start a race with whatever the race pages hold.
    LaunchRace,
    /// Close the game.
    Quit,
}

impl Action {
    /// The spelling used in the definition file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::LaunchRace => "launch_race",
            Self::Quit => "quit",
        }
    }

    /// Parses a definition file's spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "launch_race" => Some(Self::LaunchRace),
            "quit" => Some(Self::Quit),
            _ => None,
        }
    }

    /// Every action, for error messages and for the integrity check.
    #[must_use]
    pub fn all() -> [Self; 2] {
        [Self::LaunchRace, Self::Quit]
    }
}

/// Where a `choice`'s values come from when the definition cannot name them.
///
/// A closed set for the same reason [`Action`] is: a typo has to be a load
/// error, not a row that is empty at runtime. The definition says *which* list
/// it wants and the composition root supplies it through [`Menu::supply`], so
/// this module still knows nothing about discs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueSource {
    /// The languages this source actually carries, which is a property of the
    /// disc: the USA PSP release ships English only, and a menu that offered
    /// French on it would be offering something that cannot be selected.
    Languages,
    /// The circuits this source offers, from its own plugin definition.
    ///
    /// Not a directory listing: `16_Track` and `32_Track` are two races and one
    /// folder. And the label is the localised name out of the string table, so
    /// what a player reads never appears in this repository. See
    /// [`crate::catalogue`].
    Tracks,
}

impl ValueSource {
    /// The spelling used in the definition file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Languages => "languages",
            Self::Tracks => "tracks",
        }
    }

    /// Parses a definition file's spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "languages" => Some(Self::Languages),
            "tracks" => Some(Self::Tracks),
            _ => None,
        }
    }
}

/// One option on a `choice` row: what it stores, and what it shows.
///
/// The two are the same thing for a list the definition spells out - `off`,
/// `4x` - and are **not** for a list that comes off a disc. A circuit stores
/// `16_Track`, which is a plugin id and stable, and shows "Talon's Junction
/// White", which is localised shipped content that only ever exists in memory.
/// Keeping them apart is what lets the menus name a track without this
/// repository containing its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Option_ {
    /// What the setting is set to.
    pub value: String,
    /// What the row draws.
    pub label: String,
}

impl Option_ {
    /// An option whose stored value is also what it shows.
    #[must_use]
    pub fn plain(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            label: value.clone(),
            value,
        }
    }

    /// An option that stores one thing and shows another.
    #[must_use]
    pub fn labelled(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// What a setting a menu can move is worth.
///
/// Deliberately narrow: a menu is a list of rows, so anything it can edit has
/// to be expressible as one of a small number of choices or a switch. A number
/// that wants a slider is not representable and should not be bolted on here
/// without deciding what a slider *is* on a control pad first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// One of an entry's declared `values`.
    Text(String),
    /// A switch.
    Flag(bool),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(text) => f.write_str(text),
            Self::Flag(true) => f.write_str("ON"),
            Self::Flag(false) => f.write_str("OFF"),
        }
    }
}

/// One row of a page.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// Goes to another page.
    Submenu {
        /// Row label.
        label: String,
        /// Index into [`Definition::pages`], resolved at load.
        target: usize,
    },
    /// Fires an action and leaves the menus to deal with it.
    Run {
        /// Row label.
        label: String,
        /// What to fire.
        action: Action,
    },
    /// Cycles a setting through a fixed list of values.
    Choice {
        /// Row label.
        label: String,
        /// The settings key the composition root knows this by.
        setting: String,
        /// The values, in the order left and right move through them.
        ///
        /// Empty until [`Menu::supply`] fills it, when the entry declared a
        /// [`ValueSource`]. A row with nothing to offer draws its label and no
        /// value and does not move, which is the honest thing for a list this
        /// source turned out not to have.
        values: Vec<Option_>,
        /// Where [`Self::Choice::values`] comes from, when not the definition.
        source: Option<ValueSource>,
        /// Which one is current. Seeded by [`Menu::seed`], never read off disk.
        current: usize,
    },
    /// Flips a boolean setting.
    Toggle {
        /// Row label.
        label: String,
        /// The settings key the composition root knows this by.
        setting: String,
        /// Whether it is on. Seeded by [`Menu::seed`].
        on: bool,
    },
    /// Shows what an abstract button is currently bound to.
    ///
    /// **Read-only in this build.** Rebinding needs a binding table where
    /// `oag_input::keys::map_key` currently has a hardcoded `match`, plus
    /// persistence and conflict handling; the row exists so the shape of the
    /// definition is settled before that lands, and so the gap is visible in
    /// the game rather than only in a document.
    Binding {
        /// Row label.
        label: String,
        /// Abstract button index, from [`crate::input::button`].
        button: u8,
    },
    /// Goes back to the previous page, or leaves the menus from the root.
    Back {
        /// Row label.
        label: String,
    },
}

impl Entry {
    /// The row's label.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Submenu { label, .. }
            | Self::Run { label, .. }
            | Self::Choice { label, .. }
            | Self::Toggle { label, .. }
            | Self::Binding { label, .. }
            | Self::Back { label } => label,
        }
    }

    /// The settings key this row edits, if it edits one.
    #[must_use]
    pub fn setting(&self) -> Option<&str> {
        match self {
            Self::Choice { setting, .. } | Self::Toggle { setting, .. } => Some(setting),
            _ => None,
        }
    }

    /// What the row shows on its right-hand side, if anything.
    ///
    /// The *label*. What the setting is set to is [`Self::chosen`], and on a
    /// disc-supplied list the two are different strings.
    #[must_use]
    pub fn value(&self) -> Option<Value> {
        match self {
            Self::Choice {
                values, current, ..
            } => values
                .get(*current)
                .map(|option| Value::Text(option.label.clone())),
            Self::Toggle { on, .. } => Some(Value::Flag(*on)),
            _ => None,
        }
    }

    /// What this row's setting is currently set to, as opposed to what it
    /// shows.
    #[must_use]
    pub fn chosen(&self) -> std::option::Option<Value> {
        match self {
            Self::Choice {
                values, current, ..
            } => values
                .get(*current)
                .map(|option| Value::Text(option.value.clone())),
            Self::Toggle { on, .. } => Some(Value::Flag(*on)),
            _ => None,
        }
    }

    /// Whether left and right move this row.
    #[must_use]
    pub fn is_adjustable(&self) -> bool {
        matches!(self, Self::Choice { .. } | Self::Toggle { .. })
    }
}

/// One page: a title and its rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// The id a `submenu` entry names.
    pub id: String,
    /// Heading drawn above the rows.
    pub title: String,
    /// The rows, in the order they are drawn and moved through.
    pub entries: Vec<Entry>,
}

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
        /// Reserved for localisation: the id of a string in the disc's own
        /// table, for a build that wants the original's wording.
        ///
        /// **Deliberately parsed and not read.** Accepting the field now means
        /// a definition that carries it loads today rather than failing on an
        /// unknown key, so adding localisation later is not a format change.
        #[serde(default)]
        #[allow(
            dead_code,
            reason = "accepted so the format does not change when localisation lands"
        )]
        pub string_id: Option<String>,
    }
}

impl Definition {
    /// Parses and checks a definition.
    ///
    /// # Errors
    ///
    /// Every problem in [`Error`]: malformed TOML, a version this build does
    /// not know, a duplicate or dangling page id, an unknown action or button,
    /// an entry missing a field its kind needs, and a page nothing links to.
    pub fn parse(text: &str) -> Result<Self, Error> {
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
            for (row, entry) in page.entries.iter().enumerate() {
                let context = format!("page {:?} entry {row}", page.id);
                entries.push(resolve(entry, &context, &index)?);
            }
            pages.push(Page {
                id: page.id.clone(),
                title: page.title.clone(),
                entries,
            });
        }

        let definition = Self { pages, root };
        definition.check()?;
        Ok(definition)
    }

    /// The check that earns its keep: every page is reachable from the root.
    ///
    /// Dangling targets and unknown actions are already impossible by the time
    /// this runs - [`resolve`] refuses them - so what is left is the failure
    /// this cannot catch at the row level: a page that parses, resolves, and
    /// that no entry anywhere links to. That is a menu somebody wrote and
    /// forgot to hang off anything, and without this it ships silently.
    ///
    /// # Errors
    ///
    /// [`Error::Unreachable`], naming the first orphan in file order.
    pub fn check(&self) -> Result<(), Error> {
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
}

/// Turns one raw entry into a resolved one, or says exactly what is wrong.
fn resolve(
    entry: &raw::Entry,
    context: &str,
    index: &HashMap<String, usize>,
) -> Result<Entry, Error> {
    let label = entry.label.clone();
    let missing = |field: &str| Error::BadEntry {
        context: context.to_string(),
        problem: format!("a {:?} entry needs {field}", entry.kind),
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
                values: entry.values.iter().map(Option_::plain).collect(),
                source,
                current: 0,
            })
        }
        "toggle" => {
            let setting = entry.setting.as_ref().ok_or_else(|| missing("setting"))?;
            Ok(Entry::Toggle {
                label,
                setting: setting.clone(),
                on: false,
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

/// What the menus did on a tick, for the composition root to act on.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuEvent {
    /// An `action` row was activated.
    Fired(Action),
    /// A setting was moved. Apply it and persist it.
    Changed {
        /// The settings key from the definition.
        setting: String,
        /// Its new value.
        value: Value,
    },
    /// Back was pressed on the root page, so the menus are done.
    ///
    /// What that means is the caller's: quitting, or handing back to whatever
    /// was on screen before. The menus have no opinion.
    Closed,
}

/// A menu tree with a cursor in it.
#[derive(Debug, Clone)]
pub struct Menu {
    definition: Definition,
    /// The page being drawn, and everything it was reached through, oldest
    /// first. Never empty.
    stack: Vec<usize>,
    /// Selected row, **per page** rather than per stack level, parallel to
    /// [`Definition::pages`].
    ///
    /// Per page and not per visit on purpose: a player who backs out of Options
    /// lands on the row that took them there, *and* a player who goes back into
    /// it lands on the row they left. Keying it to the stack gives the first and
    /// not the second, which is the difference between a menu that is pleasant
    /// to use and one that is merely correct.
    cursor: Vec<usize>,
}

impl Menu {
    /// Opens a definition at its root page.
    #[must_use]
    pub fn new(definition: Definition) -> Self {
        let root = definition.root;
        let cursor = vec![0; definition.pages.len()];
        Self {
            definition,
            stack: vec![root],
            cursor,
        }
    }

    /// Seeds every row that edits `setting` with its current value.
    ///
    /// Called once by whoever owns the setting, before the menus are shown. A
    /// value that is not one of the row's declared choices is **ignored rather
    /// than added**: the definition decides what is offerable, so a stale
    /// config file moves the row to a value the player can reach instead of
    /// smuggling an unreachable one onto the list.
    ///
    /// Returns whether anything was seeded, which is how a caller notices a
    /// settings key nothing in the menus edits.
    pub fn seed(&mut self, setting: &str, value: &Value) -> bool {
        let mut seeded = false;
        for page in &mut self.definition.pages {
            for entry in &mut page.entries {
                match (entry, value) {
                    (
                        Entry::Choice {
                            setting: key,
                            values,
                            current,
                            ..
                        },
                        Value::Text(text),
                    ) if key == setting => {
                        if let Some(at) = values.iter().position(|v| &v.value == text) {
                            *current = at;
                            seeded = true;
                        }
                    }
                    (
                        Entry::Toggle {
                            setting: key, on, ..
                        },
                        Value::Flag(flag),
                    ) if key == setting => {
                        *on = *flag;
                        seeded = true;
                    }
                    _ => {}
                }
            }
        }
        seeded
    }

    /// Fills in every row whose values come from `source`.
    ///
    /// The mirror of [`Self::seed`]: that one says what a row is *set* to, this
    /// one says what it may be set to at all. Called before seeding, because a
    /// value cannot be seeded onto a list that is not there yet.
    ///
    /// Returns how many rows were filled. Supplying an empty list is allowed and
    /// is not the same as not calling this: it means the source really has
    /// nothing, and the row draws as an unusable one rather than as a lie.
    pub fn supply(&mut self, source: ValueSource, values: &[Option_]) -> usize {
        let mut filled = 0;
        for page in &mut self.definition.pages {
            for entry in &mut page.entries {
                if let Entry::Choice {
                    values: list,
                    source: from,
                    current,
                    ..
                } = entry
                    && *from == Some(source)
                {
                    values.clone_into(list);
                    *current = 0;
                    filled += 1;
                }
            }
        }
        filled
    }

    /// Jumps straight to a page, discarding the stack.
    ///
    /// For looking at one page without walking to it - `--menu-page` - rather
    /// than for navigation, which is why the stack is *replaced*: a page reached
    /// this way was not reached through anything, and pretending otherwise would
    /// give it a back destination it never had.
    ///
    /// Returns whether the page exists.
    pub fn open(&mut self, id: &str) -> bool {
        let Some(at) = self.definition.pages.iter().position(|page| page.id == id) else {
            return false;
        };
        self.stack = vec![at];
        true
    }

    /// The definition, for reporting and for drawing.
    #[must_use]
    pub fn definition(&self) -> &Definition {
        &self.definition
    }

    /// The page on screen.
    #[must_use]
    pub fn page(&self) -> &Page {
        &self.definition.pages[*self.stack.last().expect("the stack is never empty")]
    }

    /// Which row is highlighted on the page being drawn.
    #[must_use]
    pub fn selected(&self) -> usize {
        self.cursor[self.current()]
    }

    /// Index of the page being drawn.
    fn current(&self) -> usize {
        *self.stack.last().expect("the stack is never empty")
    }

    /// How deep in the tree the cursor is. `1` on the root page.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Consumes a tick of input and says what happened.
    ///
    /// Every button is taken as an **edge**, not a level: menus are not held
    /// down. `Input::take` is what the language picker uses for the same
    /// reason, and going through it here keeps one answer to "did the player
    /// press this" rather than two.
    pub fn update(&mut self, input: &mut Input) -> Vec<MenuEvent> {
        let mut out = Vec::new();
        let rows = self.page().entries.len();

        let page = self.current();
        if take(input, button::DOWN) && rows > 0 {
            self.cursor[page] = (self.cursor[page] + 1) % rows;
        }
        if take(input, button::UP) && rows > 0 {
            self.cursor[page] = (self.cursor[page] + rows - 1) % rows;
        }

        let right = take(input, button::RIGHT);
        let left = take(input, button::LEFT);
        if right || left {
            if let Some(event) = self.adjust(if right { 1 } else { -1 }) {
                out.push(event);
            }
        }

        if take(input, button::CROSS) || take(input, button::START) {
            out.extend(self.activate());
        }
        if take(input, button::CIRCLE) {
            out.extend(self.back());
        }

        out
    }

    /// Moves the selected row's value by `step`, wrapping.
    fn adjust(&mut self, step: i32) -> Option<MenuEvent> {
        let page = self.current();
        let row = self.cursor[page];
        let entry = self.definition.pages[page].entries.get_mut(row)?;

        match entry {
            Entry::Choice {
                setting,
                values,
                current,
                ..
            } => {
                let count = values.len() as i32;
                if count == 0 {
                    return None;
                }
                *current = (*current as i32 + step).rem_euclid(count) as usize;
                Some(MenuEvent::Changed {
                    setting: setting.clone(),
                    // The stored value, not the label: a settings file holds
                    // `16_Track`, never the words a player read.
                    value: Value::Text(values[*current].value.clone()),
                })
            }
            Entry::Toggle { setting, on, .. } => {
                *on = !*on;
                Some(MenuEvent::Changed {
                    setting: setting.clone(),
                    value: Value::Flag(*on),
                })
            }
            _ => None,
        }
    }

    /// Activates the selected row.
    fn activate(&mut self) -> Vec<MenuEvent> {
        let page = self.current();
        let row = self.cursor[page];
        let Some(entry) = self.definition.pages[page].entries.get(row) else {
            return Vec::new();
        };

        match entry {
            Entry::Submenu { target, .. } => {
                self.stack.push(*target);
                Vec::new()
            }
            Entry::Run { action, .. } => vec![MenuEvent::Fired(*action)],
            Entry::Back { .. } => self.back(),
            // Activating an adjustable row steps it forward, so a player who
            // only ever presses one button can still change everything. Left
            // and right are the discoverable way; this is the forgiving one.
            Entry::Choice { .. } | Entry::Toggle { .. } => self.adjust(1).into_iter().collect(),
            Entry::Binding { .. } => Vec::new(),
        }
    }

    /// Pops a page, or reports that the root was backed out of.
    fn back(&mut self) -> Vec<MenuEvent> {
        if self.stack.len() > 1 {
            self.stack.pop();
            Vec::new()
        } else {
            vec![MenuEvent::Closed]
        }
    }
}

/// Where the rows start and how they are spaced, in the 480x272 space the rest
/// of the front end draws in.
///
/// Round numbers rather than measured ones. Nothing here is recovered - the
/// disc's own menu layout has not been read and this is not trying to look like
/// it - so these are picked to be legible at the smallest window the game
/// opens, and the value column is right-aligned against [`VALUE_RIGHT`] so a
/// long label and a long value cannot collide.
const MARGIN_X: f32 = 40.0;
const VALUE_RIGHT: f32 = 440.0;
const TITLE_Y: f32 = 28.0;
const FIRST_ROW_Y: f32 = 72.0;
const ROW_HEIGHT: f32 = 20.0;
const ROW_SCALE: f32 = 1.0;

/// The highlight behind the selected row, and the two text colours.
const HIGHLIGHT: [f32; 4] = [0.37, 0.86, 0.96, 0.35];
const SELECTED: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const NORMAL: [f32; 4] = [0.72, 0.78, 0.84, 1.0];
/// Rows that show something the player cannot change yet.
const DIMMED: [f32; 4] = [0.45, 0.5, 0.56, 1.0];

/// What one page looks like, as plain data.
///
/// The same [`Draw`] vocabulary the language picker emits, so `main.rs`
/// rasterises menus with the renderer it already has and there is one text
/// path rather than two.
///
/// `bindings` answers "what is this button bound to" and is passed in rather
/// than looked up, because a keyboard is a device concern: `oag_input::keys`
/// owns the mapping, this crate owns the layout, and neither has to know how
/// the other works. `main.rs` hands over [`oag_input::keys::bound_keys`].
#[must_use]
pub fn draw_list(menu: &Menu, bindings: &dyn Fn(u8) -> Vec<&'static str>) -> Vec<Draw> {
    let page = menu.page();
    let mut out = vec![Draw::Text {
        x: MARGIN_X,
        y: TITLE_Y,
        scale: 1.4,
        color: SELECTED,
        align: Align::Left,
        text: page.title.clone(),
    }];

    for (row, entry) in page.entries.iter().enumerate() {
        let y = FIRST_ROW_Y + row as f32 * ROW_HEIGHT;
        let selected = row == menu.selected();
        if selected {
            out.push(Draw::Fill {
                rect: [
                    MARGIN_X - 8.0,
                    y - 3.0,
                    VALUE_RIGHT - MARGIN_X + 16.0,
                    ROW_HEIGHT - 2.0,
                ],
                color: HIGHLIGHT,
            });
        }

        let readable = !matches!(entry, Entry::Binding { .. });
        out.push(Draw::Text {
            x: MARGIN_X,
            y,
            scale: ROW_SCALE,
            color: if selected {
                SELECTED
            } else if readable {
                NORMAL
            } else {
                DIMMED
            },
            align: Align::Left,
            text: entry.label().to_string(),
        });

        // The right-hand column: a setting's value, or what a button is bound
        // to. `Align::Right` anchors at `x - width`, so both kinds land on the
        // same edge whatever they say.
        let value = match entry {
            Entry::Binding { button, .. } => {
                let keys = bindings(*button);
                if keys.is_empty() {
                    Some("UNBOUND".to_string())
                } else {
                    Some(keys.join(" / "))
                }
            }
            other => other.value().map(|value| value.to_string()),
        };
        if let Some(text) = value {
            out.push(Draw::Text {
                x: VALUE_RIGHT,
                y,
                scale: ROW_SCALE,
                color: if selected && entry.is_adjustable() {
                    SELECTED
                } else {
                    DIMMED
                },
                align: Align::Right,
                text,
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The definition this build actually ships. Every test that can use it
    /// does, so the file is exercised rather than a fixture standing in for it.
    fn built_in() -> Definition {
        Definition::parse(BUILT_IN).expect("the built-in menu must parse")
    }

    /// One tick with `buttons` newly down, which is what the edges above read.
    fn press(menu: &mut Menu, buttons: &[u8]) -> Vec<MenuEvent> {
        let mut input = Input::new();
        let mask = buttons.iter().fold(0u32, |mask, &b| mask | 1 << b);
        input.begin_frame(mask);
        menu.update(&mut input)
    }

    /// The check that runs in CI. Unlike most of this repository's interesting
    /// tests it needs no disc image, because the thing under test is ours.
    #[test]
    fn the_shipped_definition_is_internally_consistent() {
        let definition = built_in();
        assert!(!definition.pages.is_empty(), "a menu with no pages");
        definition.check().expect("reachability");

        for page in &definition.pages {
            assert!(!page.entries.is_empty(), "page {:?} has no rows", page.id);
            for entry in &page.entries {
                assert!(
                    !entry.label().is_empty(),
                    "page {:?} has an unlabelled row",
                    page.id
                );
            }
        }
    }

    /// Every page except the root must offer a way out, or a player who opens
    /// it with a keyboard that has no cancel key is stuck in it. The root's way
    /// out is `quit`, which is an action rather than a `back`.
    #[test]
    fn every_page_can_be_left_from_its_own_rows() {
        let definition = built_in();
        for (index, page) in definition.pages.iter().enumerate() {
            let leaves = page.entries.iter().any(|entry| {
                matches!(entry, Entry::Back { .. })
                    || matches!(entry, Entry::Run { action, .. } if *action == Action::Quit)
            });
            assert!(
                leaves || index == definition.root,
                "page {:?} has no way back",
                page.id
            );
        }
    }

    /// The one class of mistake the format check cannot see: a `choice` whose
    /// values are spelled by hand and drift from the list the game will accept.
    /// Both of these fail at *race load*, deep inside an archive lookup, with a
    /// message about a missing WAD entry - so they are pinned here, where the
    /// message names the menu.
    #[test]
    fn the_race_page_offers_only_teams_and_classes_the_game_accepts() {
        let definition = built_in();
        let values = |setting: &str| -> Vec<String> {
            #[allow(clippy::redundant_closure_for_method_calls)]
            definition
                .pages
                .iter()
                .flat_map(|page| page.entries.iter())
                .find_map(|entry| match entry {
                    Entry::Choice {
                        setting: key,
                        values,
                        ..
                    } if key == setting => Some(values.iter().map(|v| v.value.clone()).collect()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("no choice edits {setting:?}"))
        };

        assert_eq!(
            values("race.team"),
            oag_formats::handling::TEAMS,
            "the team rows and the eight shipped handlingstats.xml files must be one list"
        );
        for name in values("race.class") {
            assert!(
                oag_physics::SpeedClass::from_name(&name).is_some(),
                "{name:?} is not a speed class"
            );
        }
        assert_eq!(
            values("race.class").len(),
            oag_physics::SpeedClass::ALL.len(),
            "every speed class should be offerable"
        );
    }

    /// Anisotropy is the one graphics row that is live, so its values have to be
    /// exactly what `Anisotropy` parses - a typo here would persist a setting
    /// the next run refuses to load.
    #[test]
    fn the_graphics_page_offers_only_anisotropy_levels_that_parse() {
        let definition = built_in();
        let Some(Entry::Choice { values, .. }) = definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .find(|entry| entry.setting() == Some("graphics.anisotropy"))
        else {
            panic!("nothing edits graphics.anisotropy");
        };
        for option in values {
            option
                .value
                .parse::<oag_render::mesh_render::Anisotropy>()
                .unwrap_or_else(|e| panic!("{:?}: {e}", option.value));
        }
    }

    #[test]
    fn a_version_this_build_does_not_know_is_refused() {
        let error = Definition::parse("version = 99\nroot = \"main\"").expect_err("refused");
        assert!(matches!(error, Error::Version { found: 99 }), "{error}");
    }

    #[test]
    fn a_dangling_target_is_refused() {
        let error = Definition::parse(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            [[page.entry]]
            kind = "submenu"
            label = "NOWHERE"
            target = "does_not_exist"
            "#,
        )
        .expect_err("refused");
        assert!(matches!(error, Error::NoSuchPage { .. }), "{error}");
    }

    #[test]
    fn an_unknown_action_is_refused_and_says_what_is_known() {
        let error = Definition::parse(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            [[page.entry]]
            kind = "action"
            label = "DO IT"
            action = "make_tea"
            "#,
        )
        .expect_err("refused");
        let message = error.to_string();
        assert!(message.contains("make_tea"), "{message}");
        assert!(message.contains("launch_race"), "{message}");
    }

    /// The orphan case: a page that parses and resolves, and that nothing links
    /// to. This is the only one `resolve` cannot catch a row at a time.
    #[test]
    fn a_page_nothing_links_to_is_refused() {
        let error = Definition::parse(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            [[page.entry]]
            kind = "action"
            label = "QUIT"
            action = "quit"
            [[page]]
            id = "orphan"
            [[page.entry]]
            kind = "back"
            label = "BACK"
            "#,
        )
        .expect_err("refused");
        assert!(
            matches!(&error, Error::Unreachable(id) if id == "orphan"),
            "{error}"
        );
    }

    fn fixture() -> Definition {
        Definition::parse(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            title = "MAIN"
            [[page.entry]]
            kind = "submenu"
            label = "OPTIONS"
            target = "options"
            [[page.entry]]
            kind = "action"
            label = "QUIT"
            action = "quit"
            [[page]]
            id = "options"
            title = "OPTIONS"
            [[page.entry]]
            kind = "choice"
            label = "FILTERING"
            setting = "graphics.anisotropy"
            values = ["off", "4x", "16x"]
            [[page.entry]]
            kind = "toggle"
            label = "SWITCH"
            setting = "graphics.switch"
            [[page.entry]]
            kind = "back"
            label = "BACK"
            "#,
        )
        .expect("the fixture must parse")
    }

    #[test]
    fn the_cursor_wraps_both_ways() {
        let mut menu = Menu::new(fixture());
        assert_eq!(menu.selected(), 0);
        press(&mut menu, &[button::DOWN]);
        assert_eq!(menu.selected(), 1);
        press(&mut menu, &[button::DOWN]);
        assert_eq!(menu.selected(), 0, "past the last row wraps to the first");
        press(&mut menu, &[button::UP]);
        assert_eq!(menu.selected(), 1, "and back off the top wraps to the last");
    }

    /// Going into a page and back out again must land on the row that was
    /// selected, not on the first one.
    #[test]
    fn the_back_stack_restores_the_cursor() {
        let mut menu = Menu::new(fixture());
        press(&mut menu, &[button::DOWN]);
        press(&mut menu, &[button::UP]);
        press(&mut menu, &[button::CROSS]);
        assert_eq!(menu.page().id, "options");
        assert_eq!(menu.depth(), 2);

        press(&mut menu, &[button::DOWN]);
        assert_eq!(menu.selected(), 1);
        press(&mut menu, &[button::CIRCLE]);
        assert_eq!(menu.page().id, "main");
        assert_eq!(menu.selected(), 0, "the row OPTIONS was entered from");

        press(&mut menu, &[button::CROSS]);
        assert_eq!(
            menu.selected(),
            1,
            "and re-entering restores the row inside it"
        );
    }

    #[test]
    fn a_choice_cycles_and_reports_every_step() {
        let mut menu = Menu::new(fixture());
        press(&mut menu, &[button::CROSS]);

        let events = press(&mut menu, &[button::RIGHT]);
        assert_eq!(
            events,
            vec![MenuEvent::Changed {
                setting: "graphics.anisotropy".to_string(),
                value: Value::Text("4x".to_string()),
            }]
        );
        // And wraps backwards off the start rather than sticking.
        press(&mut menu, &[button::LEFT]);
        let events = press(&mut menu, &[button::LEFT]);
        assert_eq!(
            events,
            vec![MenuEvent::Changed {
                setting: "graphics.anisotropy".to_string(),
                value: Value::Text("16x".to_string()),
            }]
        );
    }

    /// One button has to be enough to change everything, for a player on a pad
    /// who never finds left and right.
    #[test]
    fn activating_a_choice_steps_it_forward() {
        let mut menu = Menu::new(fixture());
        press(&mut menu, &[button::CROSS]);
        let events = press(&mut menu, &[button::CROSS]);
        assert_eq!(
            events,
            vec![MenuEvent::Changed {
                setting: "graphics.anisotropy".to_string(),
                value: Value::Text("4x".to_string()),
            }]
        );
    }

    #[test]
    fn a_toggle_flips_and_reads_out_as_on_or_off() {
        let mut menu = Menu::new(fixture());
        press(&mut menu, &[button::CROSS]);
        press(&mut menu, &[button::DOWN]);

        assert_eq!(
            menu.page().entries[1].value(),
            Some(Value::Flag(false)),
            "a toggle starts off unless it is seeded"
        );
        let events = press(&mut menu, &[button::RIGHT]);
        assert_eq!(
            events,
            vec![MenuEvent::Changed {
                setting: "graphics.switch".to_string(),
                value: Value::Flag(true),
            }]
        );
        assert_eq!(Value::Flag(true).to_string(), "ON");
    }

    #[test]
    fn seeding_moves_a_row_to_the_value_the_caller_holds() {
        let mut menu = Menu::new(fixture());
        assert!(menu.seed("graphics.anisotropy", &Value::Text("16x".to_string())));
        press(&mut menu, &[button::CROSS]);
        assert_eq!(
            menu.page().entries[0].value(),
            Some(Value::Text("16x".to_string()))
        );
    }

    /// A config file naming a value the menus do not offer must not be able to
    /// put an unreachable option on the list.
    #[test]
    fn seeding_a_value_the_row_does_not_offer_changes_nothing() {
        let mut menu = Menu::new(fixture());
        assert!(!menu.seed("graphics.anisotropy", &Value::Text("64x".to_string())));
        press(&mut menu, &[button::CROSS]);
        assert_eq!(
            menu.page().entries[0].value(),
            Some(Value::Text("off".to_string())),
            "the row stays on a value the player can actually reach"
        );
    }

    #[test]
    fn seeding_a_key_nothing_edits_says_so() {
        let mut menu = Menu::new(fixture());
        assert!(!menu.seed("graphics.nothing", &Value::Flag(true)));
    }

    #[test]
    fn an_action_row_fires_rather_than_navigating() {
        let mut menu = Menu::new(fixture());
        press(&mut menu, &[button::DOWN]);
        let events = press(&mut menu, &[button::CROSS]);
        assert_eq!(events, vec![MenuEvent::Fired(Action::Quit)]);
        assert_eq!(menu.page().id, "main", "firing does not move the cursor");
    }

    /// Backing out of the root is the menus saying they are done, not a no-op
    /// and not a crash.
    #[test]
    fn backing_out_of_the_root_closes_the_menus() {
        let mut menu = Menu::new(fixture());
        let events = press(&mut menu, &[button::CIRCLE]);
        assert_eq!(events, vec![MenuEvent::Closed]);
        assert_eq!(menu.depth(), 1, "and leaves the stack alone");
    }

    /// No keys bound to anything, for the layout tests: what a binding row
    /// shows is `oag-input`'s business and is asserted there.
    fn no_bindings(_: u8) -> Vec<&'static str> {
        Vec::new()
    }

    #[test]
    fn a_drawn_page_has_a_title_a_row_each_and_one_highlight() {
        let mut menu = Menu::new(fixture());
        press(&mut menu, &[button::CROSS]);
        let list = draw_list(&menu, &no_bindings);

        let texts: Vec<&String> = list
            .iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&&"OPTIONS".to_string()), "{texts:?}");
        assert!(texts.contains(&&"FILTERING".to_string()), "{texts:?}");
        assert!(
            texts.contains(&&"off".to_string()),
            "a choice draws its value: {texts:?}"
        );
        assert!(
            texts.contains(&&"OFF".to_string()),
            "a toggle draws ON or OFF: {texts:?}"
        );

        let highlights = list
            .iter()
            .filter(|draw| matches!(draw, Draw::Fill { .. }))
            .count();
        assert_eq!(highlights, 1, "exactly one row is highlighted");
    }

    /// The value column is right-aligned against a fixed edge, which is the
    /// whole reason a long label and a long value cannot overlap. Asserted
    /// because it is invisible in a headless test otherwise.
    #[test]
    fn values_are_right_aligned_on_one_edge() {
        let mut menu = Menu::new(fixture());
        press(&mut menu, &[button::CROSS]);
        for draw in draw_list(&menu, &no_bindings) {
            if let Draw::Text { align, x, .. } = draw
                && align == Align::Right
            {
                assert!((x - VALUE_RIGHT).abs() < f32::EPSILON, "got {x}");
            }
        }
    }

    /// A row for a button nothing is bound to says so rather than drawing an
    /// empty column that reads as a layout bug.
    #[test]
    fn an_unbound_button_draws_the_word_rather_than_nothing() {
        let definition = Definition::parse(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            [[page.entry]]
            kind = "binding"
            label = "THRUST"
            button = "cross"
            "#,
        )
        .expect("parse");
        let menu = Menu::new(definition);
        let list = draw_list(&menu, &no_bindings);
        assert!(
            list.iter()
                .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "UNBOUND")),
            "{list:?}"
        );
        let list = draw_list(&menu, &|_| vec!["ENTER", "X"]);
        assert!(
            list.iter()
                .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "ENTER / X")),
            "{list:?}"
        );
    }

    #[test]
    fn the_built_in_menu_opens_on_its_root_and_can_reach_a_race() {
        let definition = built_in();
        let root = definition.pages[definition.root].id.clone();
        let menu = Menu::new(definition);
        assert_eq!(menu.page().id, root);

        // Walk every page breadth-first and assert `launch_race` is somewhere
        // in the tree: a menu that cannot start a game is not a menu for this
        // game, and this catches an asset edit that drops the row.
        let reachable = menu
            .definition()
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .any(
                |entry| matches!(entry, Entry::Run { action, .. } if *action == Action::LaunchRace),
            );
        assert!(reachable, "nothing in the menus starts a race");
    }
}
