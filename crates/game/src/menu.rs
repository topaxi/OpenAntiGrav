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
    /// The screens this machine has, which is a property of the desk rather
    /// than of the disc - the first source here that is.
    ///
    /// Supplied for the same reason the other two are: a definition file cannot
    /// know them, and a row offering a monitor that is not plugged in would be
    /// offering something that cannot be selected. `default` is always first,
    /// so there is a way back from a screen that has since been unplugged.
    Monitors,
    /// The race modes, labelled from the disc's own string table.
    ///
    /// The list itself is fixed - it is `oag_race::Mode::ALL` - so unlike the
    /// other sources this one is not supplied because the set is unknown. It is
    /// supplied because the *labels* are: what a player reads is the front end's
    /// own event text off their disc, in their own language, and so it cannot be
    /// spelled in a definition file that ships in this repository.
    RaceModes,
    /// The adapters this machine can present with, which is the other property
    /// of the desk rather than of the disc.
    ///
    /// Supplied and not spelled for a reason this list makes especially plain:
    /// nothing that could be written in a definition file describes a GPU. What
    /// is on it depends on the card, the driver and whether a *software* driver
    /// is installed at all - see [`crate::adapter`]. `default` is always first,
    /// so there is a way back from an adapter that has since been uninstalled.
    Renderers,
    /// Which release's soundtrack can be played, which is a property of what
    /// discs the *player owns* rather than of the one that booted.
    ///
    /// The first source whose answer is routinely **nothing**: choosing between
    /// two encodes needs both discs, and most people have one. Supplied empty
    /// there, which draws the row unusable rather than offering a swap that
    /// cannot happen - see [`Menu::supply`]. Hiding it instead would leave the
    /// AUDIO page with no way to say that the setting exists and this machine
    /// cannot reach it, which is the same argument [`Menu::is_disabled`] makes
    /// for greying a row rather than removing it.
    MusicSources,
}

impl ValueSource {
    /// The spelling used in the definition file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Languages => "languages",
            Self::Tracks => "tracks",
            Self::Monitors => "monitors",
            Self::Renderers => "renderers",
            Self::RaceModes => "race_modes",
            Self::MusicSources => "music_sources",
        }
    }

    /// Parses a definition file's spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "languages" => Some(Self::Languages),
            "tracks" => Some(Self::Tracks),
            "monitors" => Some(Self::Monitors),
            "renderers" => Some(Self::Renderers),
            "race_modes" => Some(Self::RaceModes),
            "music_sources" => Some(Self::MusicSources),
            _ => None,
        }
    }
}

/// What a race mode's row shows, out of the disc's own string table.
///
/// The front end has no short "Time Trial" string - checked across the whole
/// English table - but it does have an event *description* per mode, and each
/// one opens with the mode's name followed by a colon. So the name is the head
/// of `MSC_EVENT_*`, and taking it is a substring of shipped text rather than a
/// translation written here.
///
/// Falls back to [`oag_race::Mode::fallback_label`] when the table has no such
/// entry, and **also when the text does not look like a name**: no colon, or a
/// head long enough to be prose rather than a label. A localisation that
/// punctuates differently then reads a little plainer instead of putting a
/// paragraph in a menu row.
#[must_use]
pub fn mode_label(mode: oag_race::Mode, strings: &crate::language::StringTable) -> String {
    /// Longest a head can be and still be a name rather than a sentence.
    const MAX: usize = 24;

    strings
        .get(mode.string_id())
        .and_then(|text| text.split(':').next())
        .map(str::trim)
        .filter(|head| !head.is_empty() && head.chars().count() <= MAX)
        .unwrap_or(mode.fallback_label())
        .to_string()
}

/// Every race mode as a row, labelled from `strings`.
#[must_use]
pub fn mode_choices(strings: &crate::language::StringTable) -> Vec<Choice> {
    oag_race::Mode::ALL
        .iter()
        .map(|mode| Choice::labelled(mode.name(), mode_label(*mode, strings)))
        .collect()
}

/// One frame of the looping picture the rows are drawn on top of.
///
/// **A frame and a rectangle, not a movie.** Deciding which frame is showing is
/// timing, deciding where it goes is the source's own display aspect, and
/// neither is a menu's business - this module draws a list and knows nothing
/// about either, the same way it knows nothing about what a setting means.
/// `main.rs` owns the player; see [`draw_list`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Backdrop {
    /// Where the picture goes on the 480x272 screen: `[x, y, width, height]`.
    pub rect: [f32; 4],
    /// Which frame of it to show, counting from zero.
    pub frame: usize,
    /// How far playback has got, counting every loop.
    ///
    /// Carried through to the draw for the same reason the front end carries it
    /// (see [`Draw::Video`]), even though the menus have already taken their
    /// picture out of the feed by the time they build one. It is the *same*
    /// playback either way: the number goes on rising across the handoff from
    /// `Show Logo` rather than starting again at zero, and a draw list that said
    /// otherwise would be the one place that claim is not visible.
    pub position: u64,
}

/// One option on a `choice` row: what it stores, and what it shows.
///
/// Named for the row kind rather than for "option", which would shadow
/// `std::option::Option` everywhere this module returns one.
///
/// The two are the same thing for a list the definition spells out - `off`,
/// `4x` - and are **not** for a list that comes off a disc. A circuit stores
/// `16_Track`, which is a plugin id and stable, and shows "Talon's Junction
/// White", which is localised shipped content that only ever exists in memory.
/// Keeping them apart is what lets the menus name a track without this
/// repository containing its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// What the setting is set to.
    pub value: String,
    /// What the row draws.
    pub label: String,
}

impl Choice {
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

/// "This row is inert while *that* row holds *this* value."
///
/// Named rather than a bare setting name because the interesting cases are not
/// boolean. VSYNC has three values and only the middle one - classic `Fifo` -
/// makes a frame limit meaningless: `Immediate` and `Mailbox` both leave the
/// loop free to run ahead, so the limiter is live under either. A condition
/// that could only say "while that toggle is on" would have got that wrong in
/// the one place it is used.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// The settings key of the row that decides.
    pub setting: String,
    /// The values that row may hold for this condition to hold - **any one of
    /// them**, not all.
    ///
    /// A list and not a single value because the interesting conditions are not
    /// all one-valued either: "the upscaler does nothing" is true at every
    /// render scale of 100 % and above, which is four of the six that row
    /// offers. Written `value = "on"` for the one-valued case and
    /// `values = [...]` otherwise; the loader accepts either and stores this.
    ///
    /// The **stored** values, not the labels: what a row is set to is
    /// [`Entry::chosen`], and on a disc-supplied list those are different
    /// strings.
    pub values: Vec<Value>,
}

impl Condition {
    /// Whether `held` is one of the values this condition names.
    #[must_use]
    pub fn matches(&self, held: Option<&Value>) -> bool {
        held.is_some_and(|held| self.values.iter().any(|value| value == held))
    }
}

/// A note shown against a row whose setting is currently doing nothing, or
/// less than it says.
///
/// Distinct from `disabled_by`, and the difference is who caused it. A disabled
/// row cannot be changed *now* because another row rules it out - the frame
/// limit under classic vsync. A warned row can be changed and will be stored
/// and simply will not have the effect its label promises, because of a value
/// somewhere else. Greying it would be a lie: the setting is live, it is the
/// combination that is pointless.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    /// When to show it: **every** one of these must hold.
    ///
    /// A list because a conflict is between settings, plural. A single
    /// condition can only say "while that row holds this", which describes a
    /// row that is *always* pointless under some other value - and that is a
    /// row that should not exist rather than one that needs a warning. What is
    /// worth warning about is a *combination*, and saying so takes at least two
    /// conditions: one naming this row's own offending value, one naming the
    /// other row's. Getting that wrong is not subtle - the first version of
    /// this warned at every render scale of 100 % and above whether or not the
    /// upscaler was even selected.
    pub all: Vec<Condition>,
    /// What to tell the player. Shown under the rows, once, for the first
    /// warned row on the page.
    pub message: String,
}

/// A note shown against a row whose new value is stored but cannot take effect
/// until the game is relaunched.
///
/// The third of the three things a row can say about itself, and the one the
/// other two cannot: `disabled_by` is "not now, because of that row", a
/// [`Warning`] is "stored, and pointless next to that row", and this is "stored,
/// and nothing on this machine will act on it before the next launch". Only the
/// RENDERER row is like that today - the device is made once, at boot, and every
/// pipeline and uploaded mesh hangs off it.
///
/// Unlike a warning this depends on nothing in the definition, because the
/// question it answers is not about another row: it is whether the row has been
/// moved off *what the game is actually doing*. Only the composition root knows
/// that, and it says so through [`Menu::in_effect`].
#[derive(Debug, Clone, PartialEq)]
pub struct Restart {
    /// What to tell the player. Shown under the rows, the same place a
    /// [`Warning`]'s message is.
    pub message: String,
    /// Every value that describes what this setting is doing *this run*.
    ///
    /// Empty until [`Menu::in_effect`] supplies it, and until then the note is
    /// silent: a row that cannot be compared against anything has nothing
    /// truthful to say, and guessing would mean warning about a change the
    /// player has not made. It is deliberately **not** seeded from
    /// [`Menu::seed`], which reads the settings file - by the second time the
    /// menus open, the file holds the value the player just chose and comparing
    /// against it would say a change had taken effect when it had not.
    ///
    /// A list, for the same reason a [`Condition`] holds one: several rows can
    /// spell the same run. A game booted on `default` is drawing with a
    /// particular adapter *and* is on the default, so naming that adapter
    /// explicitly changes the settings file and changes nothing about the
    /// picture - and a note saying to restart for it would be false.
    in_effect: Vec<Value>,
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
        values: Vec<Choice>,
        /// Where [`Self::Choice::values`] comes from, when not the definition.
        source: Option<ValueSource>,
        /// Which one is current. Seeded by [`Menu::seed`], never read off disk.
        current: usize,
        /// What makes this row inert. See [`Menu::is_disabled`].
        disabled_by: Option<Condition>,
        /// What makes this row's setting stored but ineffective. See
        /// [`Menu::warning`].
        warning: Option<Warning>,
        /// What this row says when it has been moved off the value the game is
        /// running on. See [`Menu::restart_note`].
        restart: Option<Restart>,
    },
    /// Flips a boolean setting.
    Toggle {
        /// Row label.
        label: String,
        /// The settings key the composition root knows this by.
        setting: String,
        /// Whether it is on. Seeded by [`Menu::seed`].
        on: bool,
        /// What makes this row inert. See [`Menu::is_disabled`].
        disabled_by: Option<Condition>,
        /// What makes this row's setting stored but ineffective. See
        /// [`Menu::warning`].
        warning: Option<Warning>,
        /// What this row says when it has been moved off the value the game is
        /// running on. See [`Menu::restart_note`].
        restart: Option<Restart>,
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
    pub fn chosen(&self) -> Option<Value> {
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

    /// This row's warning, if it has one.
    #[must_use]
    pub fn warning(&self) -> Option<&Warning> {
        match self {
            Self::Choice { warning, .. } | Self::Toggle { warning, .. } => warning.as_ref(),
            _ => None,
        }
    }

    /// This row's restart note, if it declared one.
    ///
    /// Declared, not applying: whether it currently has anything to say is
    /// [`Menu::restart_note`], which needs the value in effect this run.
    #[must_use]
    pub fn restart(&self) -> Option<&Restart> {
        match self {
            Self::Choice { restart, .. } | Self::Toggle { restart, .. } => restart.as_ref(),
            _ => None,
        }
    }

    /// What makes this row inert, if it declared anything.
    #[must_use]
    pub fn disabled_by(&self) -> Option<&Condition> {
        match self {
            Self::Choice { disabled_by, .. } | Self::Toggle { disabled_by, .. } => {
                disabled_by.as_ref()
            }
            _ => None,
        }
    }

    /// Every value this row could be set to, for the loader's own checking.
    ///
    /// Empty for a row whose list comes off a disc, which is why the check that
    /// uses this treats empty as "cannot say" rather than as "holds nothing".
    fn offers(&self) -> Vec<Value> {
        match self {
            Self::Choice { values, .. } => values
                .iter()
                .map(|option| Value::Text(option.value.clone()))
                .collect(),
            Self::Toggle { .. } => vec![Value::Flag(true), Value::Flag(false)],
            _ => Vec::new(),
        }
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
        /// `{ setting = "...", values = [...], message = "..." }`: when this
        /// row's setting is stored but has no effect. Only `choice` and
        /// `toggle` rows may carry it.
        pub warn_when: Option<Warning>,
        /// `restart_required = "..."`: what to say once this row has been moved
        /// off the value the game is running on. The message rather than a
        /// `true`, for the reason a `warn_when` carries one - a marker with
        /// nothing to read is a puzzle - and because the wording belongs next to
        /// the row it describes rather than in the code. Only `choice` and
        /// `toggle` rows may carry it.
        pub restart_required: Option<String>,
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
                    .chain(entry.warning().into_iter().flat_map(|w| w.all.iter()))
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

    // Only a row that can be adjusted can be disabled, warned or deferred. On a
    // `back` or a `submenu` the field would parse and do nothing, which is the
    // failure the rest of this loader exists to make impossible.
    if (entry.disabled_by.is_some()
        || entry.warn_when.is_some()
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
    let warning = entry
        .warn_when
        .as_ref()
        .map(|warning| warning_from(warning, context))
        .transpose()?;
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
                warning,
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
                warning,
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

    /// Tells every `restart_required` row that edits `setting` what the game is
    /// **actually doing this run**.
    ///
    /// The third narrow call alongside [`Self::seed`] and [`Self::supply`], and
    /// the distinction between it and `seed` is the whole point: `seed` says
    /// what the settings file holds, this says what the running build is doing,
    /// and a restart note exists precisely for the setting where those two can
    /// disagree. Seeding both from the file would make the note vanish the
    /// second time the menus opened - by then the file holds the value the
    /// player chose, and nothing would be left to compare it against.
    ///
    /// Called after [`Self::supply`], for the same reason `seed` is: on a
    /// disc-supplied or machine-supplied list there is nothing to compare
    /// against until the list is there.
    ///
    /// Returns whether any row took it, which is how a caller notices a key
    /// nothing defers - a `restart_required` row nobody supplies is silent, and
    /// silence is what this is meant to prevent.
    pub fn in_effect(&mut self, setting: &str, values: &[Value]) -> bool {
        let mut told = false;
        for page in &mut self.definition.pages {
            for entry in &mut page.entries {
                let (key, restart) = match entry {
                    Entry::Choice {
                        setting, restart, ..
                    }
                    | Entry::Toggle {
                        setting, restart, ..
                    } => (setting, restart),
                    _ => continue,
                };
                if key == setting
                    && let Some(restart) = restart.as_mut()
                {
                    values.clone_into(&mut restart.in_effect);
                    told = true;
                }
            }
        }
        told
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
    ///
    /// **The row keeps the value it was already on**, if the new list still has
    /// it, and falls to the first option otherwise. That is what makes the order
    /// of `supply` and `seed` stop mattering: resetting to index 0 instead would
    /// leave a seeded row silently showing whatever happened to be first, and
    /// the first nudge of it would persist that as the player's choice.
    pub fn supply(&mut self, source: ValueSource, values: &[Choice]) -> usize {
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
                    let held = list.get(*current).map(|option| option.value.clone());
                    values.clone_into(list);
                    *current = held
                        .and_then(|value| list.iter().position(|option| option.value == value))
                        .unwrap_or(0);
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
        if (right || left)
            && let Some(event) = self.adjust(if right { 1 } else { -1 })
        {
            out.push(event);
        }

        if take(input, button::CROSS) || take(input, button::START) {
            out.extend(self.activate());
        }
        if take(input, button::CIRCLE) {
            out.extend(self.back());
        }

        out
    }

    /// What the row that edits `setting` is currently set to.
    fn held(&self, setting: &str) -> Option<Value> {
        self.definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .find(|entry| entry.setting() == Some(setting))
            .and_then(Entry::chosen)
    }

    /// Whether this row is inert because another row holds a particular value.
    ///
    /// The one piece of cross-row logic in the menus, and it is deliberately
    /// the *only* one: a definition names another row's setting and value, and
    /// the answer is read off **that row**, so this module still knows nothing
    /// about what any setting means. Vsync greys out the frame limit because
    /// the definition says so, not because `menu.rs` has heard of either.
    ///
    /// A disabled row is **greyed, not hidden**. Hiding it would change the row
    /// count under the cursor, and - worse - a player looking for a setting
    /// that has silently vanished has no way to find out what took it away.
    #[must_use]
    pub fn is_disabled(&self, entry: &Entry) -> bool {
        entry
            .disabled_by()
            .is_some_and(|condition| condition.matches(self.held(&condition.setting).as_ref()))
    }

    /// This row's warning, if it has one and it currently applies.
    ///
    /// A warned row is drawn normally and marked, not greyed: its setting *is*
    /// stored and *will* take effect the moment the row it conflicts with
    /// moves. Greying would say "you cannot change this", which is false.
    #[must_use]
    pub fn warning<'a>(&self, entry: &'a Entry) -> Option<&'a Warning> {
        entry.warning().filter(|warning| {
            warning
                .all
                .iter()
                .all(|condition| condition.matches(self.held(&condition.setting).as_ref()))
        })
    }

    /// This row's restart note, if it has one and the row has been moved off
    /// what the game is running on.
    ///
    /// Drawn normally like a warned row and for a stronger version of the same
    /// reason: the setting is stored, it is what the next launch will use, and
    /// greying it would say "you cannot change this" about the one row a player
    /// most needs to be able to change - the way back from an adapter that will
    /// not draw.
    ///
    /// Silent until the composition root has said what is in effect, and silent
    /// while the row still holds it: a note that is on the moment the menus open
    /// is one nobody reads by the time it means something.
    #[must_use]
    pub fn restart_note<'a>(&self, entry: &'a Entry) -> Option<&'a Restart> {
        let chosen = entry.chosen();
        entry.restart().filter(|restart| {
            !restart.in_effect.is_empty()
                && !restart
                    .in_effect
                    .iter()
                    .any(|running| Some(running) == chosen.as_ref())
        })
    }

    /// Whether the row the cursor is on is inert.
    fn selected_is_disabled(&self) -> bool {
        self.page()
            .entries
            .get(self.selected())
            .is_some_and(|entry| self.is_disabled(entry))
    }

    /// Moves the selected row's value by `step`, wrapping.
    fn adjust(&mut self, step: i32) -> Option<MenuEvent> {
        // A disabled row does not move, and does not report a change it did not
        // make. Checked here rather than in `update` so activating one is inert
        // too - `activate` steps an adjustable row forward.
        if self.selected_is_disabled() {
            return None;
        }
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
    ///
    /// Public because escape is not a game button and must not become one:
    /// mapping it onto `circle` would give the menus their back key and give a
    /// race a brake. The window layer therefore calls this directly, out of
    /// band with the tick loop, which is safe precisely because the menus hold
    /// no input state of their own - [`Self::update`] takes edges off a
    /// snapshot and this takes none at all.
    pub fn back(&mut self) -> Vec<MenuEvent> {
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
/// A setting that is stored but currently doing nothing.
///
/// Amber, and on its own channel: the other three colours already mean
/// selected, normal and inert, so a warning had to be a mark in the margin in a
/// colour none of them use rather than a fourth shade of the row itself.
const WARNING: [f32; 4] = [1.0, 0.76, 0.25, 1.0];

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
///
/// `backdrop` is the same arrangement one step further out: a frame and a
/// rectangle, already decided, rather than a movie this module would then have
/// to know how to play. `None` draws the rows on whatever the pass cleared to,
/// which is black - see [`Backdrop`].
#[must_use]
pub fn draw_list(
    menu: &Menu,
    bindings: &dyn Fn(u8) -> Vec<&'static str>,
    backdrop: Option<Backdrop>,
) -> Vec<Draw> {
    let page = menu.page();
    // First in the list, because the list is painted back to front.
    let mut out: Vec<Draw> = backdrop
        .map(|backdrop| Draw::Video {
            rect: backdrop.rect,
            frame: backdrop.frame,
            position: backdrop.position,
            // The same movie `Show Logo` sits on, still looping and still on the
            // same playhead: the menus are where the disc's own `FE Screen` was
            // going anyway.
            source: crate::frontend::Video::Backdrop,
        })
        .into_iter()
        .collect();
    out.push(Draw::Text {
        x: MARGIN_X,
        y: TITLE_Y,
        scale: 1.4,
        color: SELECTED,
        border: None,
        align: Align::Left,
        text: page.title.clone(),
    });

    // The first noted row's message, shown once under the rows however many
    // rows are marked: two lines of small text competing for the same corner
    // would be less readable than one, and the markers already say which rows.
    // A restart note and a warning share the slot and the first row in page
    // order wins, because they are the same kind of thing to a player - "this
    // row is not doing what it says" - and ranking them would mean deciding
    // which of two true sentences to hide.
    let mut noted: Option<String> = None;
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

        // A binding cannot be changed yet and a disabled row cannot be changed
        // now; both read as "this does nothing if you press it", which is what
        // the dim colour says. The highlight bar is still drawn, so a disabled
        // row can be selected and read rather than being unreachable.
        let inert = matches!(entry, Entry::Binding { .. }) || menu.is_disabled(entry);
        // Marked in the margin rather than by recolouring the row: the colour
        // already means selected, normal or inert, and a fourth meaning on the
        // same channel would collide with those three. See `WARNING`.
        let note = menu
            .warning(entry)
            .map(|warning| &warning.message)
            .or_else(|| menu.restart_note(entry).map(|restart| &restart.message));
        if let Some(message) = note {
            noted.get_or_insert(message.clone());
            out.push(Draw::Text {
                x: MARGIN_X - 18.0,
                y,
                scale: ROW_SCALE,
                color: WARNING,
                border: None,
                align: Align::Left,
                text: "!".to_string(),
            });
        }
        out.push(Draw::Text {
            x: MARGIN_X,
            y,
            scale: ROW_SCALE,
            color: if inert {
                DIMMED
            } else if selected {
                SELECTED
            } else {
                NORMAL
            },
            border: None,
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
                color: if selected && entry.is_adjustable() && !inert {
                    SELECTED
                } else {
                    DIMMED
                },
                border: None,
                align: Align::Right,
                text,
            });
        }
    }

    // Under the last row rather than at a fixed height, so it sits with the
    // page it belongs to instead of floating away from a short one.
    if let Some(message) = noted {
        out.push(Draw::Text {
            x: MARGIN_X - 18.0,
            y: FIRST_ROW_Y + page.entries.len() as f32 * ROW_HEIGHT + 6.0,
            scale: ROW_SCALE * 0.8,
            color: WARNING,
            border: None,
            align: Align::Left,
            text: format!("! {message}"),
        });
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

    /// The mode row is supplied rather than spelled, so what it must agree with
    /// is `mode_choices`, not a list in the definition file.
    #[test]
    fn the_supplied_mode_rows_are_exactly_the_modes_the_game_has() {
        let strings = crate::language::StringTable::default();
        let stored: Vec<String> = super::mode_choices(&strings)
            .into_iter()
            .map(|choice| choice.value)
            .collect();
        assert_eq!(
            stored,
            oag_race::Mode::ALL
                .iter()
                .map(|mode| mode.name().to_string())
                .collect::<Vec<_>>(),
            "the supplied rows and Mode::ALL must be one list, in the same order"
        );
    }

    #[test]
    fn a_mode_label_is_the_head_of_the_discs_event_text() {
        let strings = crate::language::StringTable::from_xml(
            r#"<StringTable>
                 <Entry ID="MSC_EVENT_ZONE" String="Zone: your ship accelerates automatically and the top speed increases."></Entry>
               </StringTable>"#,
        );
        assert_eq!(super::mode_label(oag_race::Mode::Zone, &strings), "Zone");
    }

    /// Three ways the disc can fail to yield a label, all of which have to end
    /// with a readable row rather than a paragraph or a blank.
    #[test]
    fn a_mode_label_falls_back_rather_than_printing_prose() {
        let absent = crate::language::StringTable::default();
        assert_eq!(
            super::mode_label(oag_race::Mode::TimeTrial, &absent),
            oag_race::Mode::TimeTrial.fallback_label()
        );

        let no_colon = crate::language::StringTable::from_xml(
            r#"<StringTable>
                 <Entry ID="MSC_EVENT_TT" String="Beat the clock in this solo race and make every corner count"></Entry>
               </StringTable>"#,
        );
        assert_eq!(
            super::mode_label(oag_race::Mode::TimeTrial, &no_colon),
            oag_race::Mode::TimeTrial.fallback_label(),
            "a description with no colon put its whole first clause in the row"
        );

        let empty = crate::language::StringTable::from_xml(
            r#"<StringTable>
                 <Entry ID="MSC_EVENT_SL" String=": focus all your efforts"></Entry>
               </StringTable>"#,
        );
        assert_eq!(
            super::mode_label(oag_race::Mode::SpeedLap, &empty),
            oag_race::Mode::SpeedLap.fallback_label()
        );
    }

    /// `every_settings_row_is_one_the_game_seeds` skips any page that is not
    /// `display` or `graphics`, so the race rows are not covered by it. The mode
    /// row is the one that would break silently: a `choice` whose setting nothing
    /// seeds opens on whatever the file happens to hold rather than on the saved
    /// value.
    #[test]
    fn the_mode_row_is_seeded_by_the_settings_module() {
        let settings = crate::settings::Settings::default();
        let seeds =
            crate::settings::menu_seeds(&settings, oag_render::mesh_render::Anisotropy::default());
        assert!(
            seeds.iter().any(|(key, _)| *key == "race.mode"),
            "nothing seeds race.mode, so the menu cannot open on the saved mode"
        );
    }

    /// The RACE page opens on a time trial, and that is what the row's *first*
    /// value plus the settings default together have to say. Either one alone
    /// would leave the other free to drift.
    #[test]
    fn the_mode_row_starts_on_the_time_trial() {
        let definition = built_in();
        let race = definition
            .pages
            .iter()
            .find(|page| page.id == "race")
            .expect("a race page");
        let source = race
            .entries
            .iter()
            .find_map(|entry| match entry {
                Entry::Choice {
                    setting, source, ..
                } if setting == "race.mode" => Some(*source),
                _ => None,
            })
            .expect("a mode row");
        assert_eq!(
            source,
            Some(ValueSource::RaceModes),
            "the mode row spells its values instead of taking them off the disc"
        );

        let strings = crate::language::StringTable::default();
        assert_eq!(
            super::mode_choices(&strings)
                .first()
                .map(|choice| choice.value.clone()),
            Some(oag_race::Mode::TimeTrial.name().to_string())
        );
        assert_eq!(
            crate::settings::Race::default().mode,
            oag_race::Mode::TimeTrial.name(),
            "the row opens on a time trial but the saved default is something else"
        );
    }

    /// Anisotropy's values have to be exactly what `Anisotropy` parses - a typo
    /// here would persist a setting the next run refuses to load.
    #[test]
    fn the_anisotropy_row_offers_only_levels_that_parse() {
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

    /// The same drift guard as the one above, for every row on DISPLAY and
    /// GRAPHICS. A value here that `display` cannot parse would be ignored at
    /// runtime with a message, which is the failure a player meets as "this row
    /// does nothing".
    #[test]
    fn the_two_settings_pages_offer_only_values_that_parse() {
        use crate::display::{
            Aspect, BoostFovKick, Brightness, CameraView, Fov, Gamma, Size, WindowMode,
        };
        let definition = built_in();
        let values = |setting: &str| -> Vec<String> {
            definition
                .pages
                .iter()
                .flat_map(|page| page.entries.iter())
                .find(|entry| entry.setting() == Some(setting))
                .and_then(|entry| match entry {
                    Entry::Choice { values, .. } => {
                        Some(values.iter().map(|v| v.value.clone()).collect())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("no choice edits {setting:?}"))
        };

        let aspects = values("display.aspect");
        for name in &aspects {
            name.parse::<Aspect>().unwrap_or_else(|e| panic!("{e}"));
        }
        assert_eq!(
            aspects.len(),
            Aspect::ALL.len(),
            "every aspect should be offerable"
        );

        let modes = values("display.window_mode");
        for name in &modes {
            name.parse::<WindowMode>().unwrap_or_else(|e| panic!("{e}"));
        }
        assert_eq!(modes.len(), WindowMode::ALL.len());

        let scales: Vec<crate::display::Scale> = values("graphics.render_scale")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            scales,
            crate::display::Scale::OFFERED,
            "the render-scale rows and `Scale::OFFERED` must be one list"
        );

        let upscalers: Vec<crate::display::Upscaler> = values("graphics.upscaler")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            upscalers,
            crate::display::Upscaler::ALL,
            "the upscaler rows and `Upscaler::ALL` must be one list"
        );

        let sharpness: Vec<crate::display::Sharpness> = values("graphics.upscale_sharpness")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            sharpness,
            crate::display::Sharpness::OFFERED,
            "the sharpness rows and `Sharpness::OFFERED` must be one list"
        );

        let anti_aliasing: Vec<crate::display::AntiAliasing> = values("graphics.anti_aliasing")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            anti_aliasing,
            crate::display::AntiAliasing::ALL,
            "the anti-aliasing rows and `AntiAliasing::ALL` must be one list"
        );

        let sizes: Vec<Size> = values("display.window_size")
            .iter()
            .map(|name| name.parse::<Size>().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            sizes,
            Size::OFFERED,
            "the window-size rows and `Size::OFFERED` must be one list"
        );

        let overlays: Vec<crate::perf::Overlay> = values("graphics.perf_overlay")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            overlays,
            crate::perf::Overlay::ALL,
            "the overlay rows and `Overlay::ALL` must be one list"
        );

        let limits: Vec<crate::perf::FrameLimit> = values("display.frame_limit")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            limits,
            crate::perf::FrameLimit::OFFERED,
            "the frame-limit rows and `FrameLimit::OFFERED` must be one list"
        );

        let vsync: Vec<crate::perf::Vsync> = values("display.vsync")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            vsync,
            crate::perf::Vsync::ALL,
            "the vsync rows and `Vsync::ALL` must be one list"
        );

        let brightness: Vec<Brightness> = values("display.brightness")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            brightness,
            Brightness::OFFERED,
            "the brightness rows and `Brightness::OFFERED` must be one list"
        );

        let gamma: Vec<Gamma> = values("display.gamma")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            gamma,
            Gamma::OFFERED,
            "the gamma rows and `Gamma::OFFERED` must be one list"
        );

        let music: Vec<crate::audio::Volume> = values("audio.music_volume")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            music,
            crate::audio::Volume::OFFERED,
            "the music-volume rows and `Volume::OFFERED` must be one list"
        );

        let fov: Vec<Fov> = values("graphics.fov")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            fov,
            Fov::OFFERED,
            "the field-of-view rows and `Fov::OFFERED` must be one list"
        );

        let boost_fov_kick: Vec<BoostFovKick> = values("graphics.boost_fov_kick")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            boost_fov_kick,
            BoostFovKick::OFFERED,
            "the boost-fov-kick rows and `BoostFovKick::OFFERED` must be one list"
        );

        // And this one is stronger than the others in the list: `CameraView::ALL`
        // is the *recovered cycle order*, so a row list that merely held the same
        // three values in a different order would put the menu and the in-race
        // cycle button out of step. Equality of the sequences is what rules that
        // out.
        let camera_view: Vec<CameraView> = values("graphics.camera_view")
            .iter()
            .map(|name| name.parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(
            camera_view,
            CameraView::ALL,
            "the camera-view rows and `CameraView::ALL` must be one list, in the cycle's order"
        );
    }

    /// Every row on the two settings pages has to be one `apply_setting`
    /// handles and `menu_seeds` fills in, or it is a row that moves and does
    /// nothing - which is the failure this whole file is arranged to prevent.
    ///
    /// Checked against the seeds rather than against `main.rs`, which this
    /// crate's library half cannot see: a setting the seeds do not know would
    /// also open its row on the list's first option instead of the player's
    /// own value, so the two lists have to agree anyway.
    #[test]
    fn every_settings_row_is_one_the_game_seeds() {
        let definition = built_in();
        let seeded: Vec<&str> = crate::settings::menu_seeds(
            &crate::settings::Settings::default(),
            oag_render::mesh_render::Anisotropy::default(),
        )
        .into_iter()
        .map(|(setting, _)| setting)
        .collect();

        for page in &definition.pages {
            if page.id != "display" && page.id != "graphics" {
                continue;
            }
            for entry in &page.entries {
                let Some(setting) = entry.setting() else {
                    continue;
                };
                assert!(
                    seeded.contains(&setting),
                    "{setting} is on the {} page and is not seeded",
                    page.id
                );
            }
        }

        // And both pages exist, so a rename in the definition cannot make the
        // loop above vacuous.
        for id in ["display", "graphics"] {
            assert!(
                definition.pages.iter().any(|page| page.id == id),
                "no {id} page"
            );
        }
    }

    /// The definition's own answer to "classic vsync makes the limiter
    /// meaningless, and the other two modes do not".
    ///
    /// Pinned here because the halves live in different files: the pairing is
    /// asserted in `assets/ui/menu.toml`, and the loop that honours it is in
    /// `main.rs` against `Vsync::paces_itself`. Both have to name the same
    /// value or the row greys out at the wrong time, and a row that lost its
    /// `disabled_by` altogether would still parse.
    /// The upscaler's warning must name exactly the scales it does nothing at.
    ///
    /// Two places encode "FSR 1 is a magnifier": `upscale::magnifies`, which
    /// declines to run it, and this warning, which says so. They are pinned to
    /// each other here because a drift between them is invisible either way -
    /// a missing value warns nobody at a scale where the setting is dead, and a
    /// spare value warns at a scale where it works.
    #[test]
    fn the_upscaler_warns_at_exactly_the_scales_it_does_nothing_at() {
        let definition = built_in();
        let entry = definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .find(|entry| entry.setting() == Some("graphics.upscaler"))
            .expect("nothing edits graphics.upscaler");
        let warning = entry.warning().expect("the upscaler warns");
        // One condition names this row's own offending value, the other the
        // scales. Without the first, the warning fires with the upscaler off.
        let own = warning
            .all
            .iter()
            .find(|c| c.setting == "graphics.upscaler")
            .expect("the warning must name the upscaler's own value");
        assert_eq!(
            own.values,
            vec![Value::Text(crate::display::Upscaler::Fsr1.to_string())]
        );
        let scales = warning
            .all
            .iter()
            .find(|c| c.setting == "graphics.render_scale")
            .expect("the warning must name the scales");

        let warned: Vec<crate::display::Scale> = scales
            .values
            .iter()
            .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        // Every offered scale is on exactly the side the guard puts it: warned
        // when a 1000-wide rectangle rendered at that scale is not smaller than
        // the rectangle, and unwarned when it is.
        let rect = (1000, 1000);
        for scale in crate::display::Scale::OFFERED {
            let scene = crate::upscale::target_size((0.0, 0.0, 1000.0, 1000.0), scale, 8192);
            let magnifies = crate::upscale::magnifies(scene, rect);
            assert_eq!(
                !magnifies,
                warned.contains(&scale),
                "{scale}: the guard and the warning disagree"
            );
        }
    }

    /// The anti-aliasing row warns against FXAA/SMAA at exactly the scales
    /// FSR 1 actually magnifies at - the same set the row above is pinned to
    /// above. Below that render scale a spatial post-process pass blurs the
    /// scene before EASU ever reads it, fighting the very edges it reasons
    /// about; at 100 % and above FSR 1 does not run at all (see the test
    /// above) and there is nothing to fight.
    #[test]
    fn anti_aliasing_warns_against_the_upscaler_at_exactly_the_scales_it_fights_it_at() {
        let definition = built_in();
        let entry = definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .find(|entry| entry.setting() == Some("graphics.anti_aliasing"))
            .expect("nothing edits graphics.anti_aliasing");
        let warning = entry.warning().expect("anti-aliasing warns");

        let own = warning
            .all
            .iter()
            .find(|c| c.setting == "graphics.anti_aliasing")
            .expect("the warning must name anti-aliasing's own offending values");
        let warned_modes: Vec<crate::display::AntiAliasing> = own
            .values
            .iter()
            .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        // Every value named must actually be a spatial post-process pass, and
        // every spatial post-process pass must be named - not a subset either
        // way, or the warning would mislead about MSAA or miss FXAA/SMAA.
        for mode in crate::display::AntiAliasing::ALL {
            assert_eq!(
                mode.is_spatial_post_process(),
                warned_modes.contains(&mode),
                "{mode}: the warning and `is_spatial_post_process` disagree"
            );
        }

        let upscaler = warning
            .all
            .iter()
            .find(|c| c.setting == "graphics.upscaler")
            .expect("the warning must require the upscaler to be fsr1");
        assert_eq!(
            upscaler.values,
            vec![Value::Text(crate::display::Upscaler::Fsr1.to_string())]
        );

        let scales = warning
            .all
            .iter()
            .find(|c| c.setting == "graphics.render_scale")
            .expect("the warning must name the scales fsr1 fights at");
        let warned_scales: Vec<crate::display::Scale> = scales
            .values
            .iter()
            .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        let rect = (1000, 1000);
        for scale in crate::display::Scale::OFFERED {
            let scene = crate::upscale::target_size((0.0, 0.0, 1000.0, 1000.0), scale, 8192);
            let magnifies = crate::upscale::magnifies(scene, rect);
            assert_eq!(
                magnifies,
                warned_scales.contains(&scale),
                "{scale}: the guard and the anti-aliasing warning disagree"
            );
        }
    }

    #[test]
    fn the_frame_limit_is_disabled_by_classic_vsync_alone() {
        let definition = built_in();
        let entry = definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .find(|entry| entry.setting() == Some("display.frame_limit"))
            .expect("nothing edits graphics.frame_limit");
        let condition = entry.disabled_by().expect("the limiter has a condition");
        assert_eq!(condition.setting, "display.vsync");
        assert_eq!(condition.values, vec![Value::Text("on".to_string())]);

        // The same value from the other side, so the menu and the loop cannot
        // drift into disagreeing about which mode paces itself.
        let [Value::Text(name)] = condition.values.as_slice() else {
            panic!("the vsync row stores one text value");
        };
        let mode: crate::perf::Vsync = name.parse().expect("a real vsync mode");
        assert!(mode.paces_itself());
        for other in crate::perf::Vsync::ALL {
            assert_eq!(
                other.paces_itself(),
                other == mode,
                "{other} is the wrong side of the condition"
            );
        }
    }

    /// A disabled row is selectable and readable and does not move, which is
    /// three separate things a player would notice.
    #[test]
    fn a_disabled_row_is_inert_until_the_row_that_disables_it_moves_off_the_value() {
        let mut menu = Menu::new(built_in());
        assert!(menu.open("display"), "the display page exists");
        let row = menu
            .page()
            .entries
            .iter()
            .position(|entry| entry.setting() == Some("display.frame_limit"))
            .expect("the frame limit is on the display page");

        menu.seed("display.vsync", &Value::Text("on".to_string()));
        menu.seed("display.frame_limit", &Value::Text("60".to_string()));
        for _ in 0..row {
            press(&mut menu, &[button::DOWN]);
        }
        assert_eq!(menu.selected(), row);

        // Right does nothing, and says nothing: an event here would persist a
        // change the player did not make.
        assert_eq!(press(&mut menu, &[button::RIGHT]), Vec::new());
        assert_eq!(press(&mut menu, &[button::CROSS]), Vec::new());
        assert_eq!(
            menu.page().entries[row].chosen(),
            Some(Value::Text("60".to_string()))
        );

        // And under either of the other two modes it is an ordinary row
        // again - `smooth` especially, where the limiter is the only thing
        // stopping the GPU rendering frames that get discarded.
        menu.seed("display.vsync", &Value::Text("smooth".to_string()));
        let events = press(&mut menu, &[button::RIGHT]);
        assert_eq!(events.len(), 1, "{events:?}");
        assert_ne!(
            menu.page().entries[row].chosen(),
            Some(Value::Text("60".to_string()))
        );
    }

    /// **The first source whose answer is routinely nothing.** Monitors and
    /// renderers always hold `default`; languages, circuits and race modes are
    /// never empty on a real disc. `music_sources` is empty on every machine
    /// with one Pulse disc, which is most of them, so the empty case is the
    /// *common* one here rather than a corner - and it has to draw, be walked
    /// past, and refuse to move, without panicking on a `rem_euclid(0)` or an
    /// index into an empty list.
    #[test]
    fn a_row_whose_source_has_nothing_is_inert_rather_than_a_panic() {
        let mut menu = Menu::new(built_in());
        assert_eq!(
            menu.supply(ValueSource::MusicSources, &[]),
            1,
            "the AUDIO page has exactly one such row"
        );
        assert!(menu.open("audio"));
        let row = menu
            .page()
            .entries
            .iter()
            .position(|entry| entry.setting() == Some("audio.music_source"))
            .expect("the music source is on the audio page");
        for _ in 0..row {
            press(&mut menu, &[button::DOWN]);
        }

        let entry = &menu.page().entries[row];
        assert_eq!(entry.value(), None, "an empty row shows no value");
        assert_eq!(entry.chosen(), None);

        for buttons in [&[button::LEFT], &[button::RIGHT], &[button::CROSS]] {
            assert!(
                press(&mut menu, buttons).is_empty(),
                "an empty row must report no change"
            );
        }
        // And it still draws, label and all, rather than vanishing.
        let drawn = draw_list(&menu, &|_| vec!["X"], None);
        assert!(
            drawn.iter().any(|draw| matches!(
                draw,
                Draw::Text { text, .. } if text == "MUSIC SOURCE"
            )),
            "the row is drawn"
        );
    }

    /// A machine that has both discs gets the three values, and the row keeps
    /// whatever the settings file seeded it with - which is what stops the
    /// first nudge of the row persisting `auto` over a player's `ps2`.
    #[test]
    fn a_supplied_music_source_row_keeps_the_value_it_was_seeded_with() {
        let mut menu = Menu::new(built_in());
        let offered: Vec<Choice> = crate::audio::MusicSource::ALL
            .iter()
            .map(|source| Choice::plain(source.name()))
            .collect();
        assert_eq!(menu.supply(ValueSource::MusicSources, &offered), 1);
        menu.seed("audio.music_source", &Value::Text("ps2".to_string()));

        assert!(menu.open("audio"));
        let entry = menu
            .page()
            .entries
            .iter()
            .find(|entry| entry.setting() == Some("audio.music_source"))
            .expect("the row");
        assert_eq!(entry.chosen(), Some(Value::Text("ps2".to_string())));
    }

    /// The visible half: a disabled row draws dim, so "this does nothing" is
    /// something a player can see rather than something they discover.
    #[test]
    fn a_disabled_row_is_drawn_dimmed_even_when_it_is_selected() {
        let mut menu = Menu::new(built_in());
        assert!(menu.open("display"));
        let row = menu
            .page()
            .entries
            .iter()
            .position(|entry| entry.setting() == Some("display.frame_limit"))
            .expect("the frame limit is on the display page");
        for _ in 0..row {
            press(&mut menu, &[button::DOWN]);
        }

        let label_colour = |menu: &Menu| {
            let list = draw_list(menu, &|_| vec!["X"], None);
            list.iter()
                .find_map(|draw| match draw {
                    Draw::Text { color, text, .. } if text == "FRAME LIMIT" => Some(*color),
                    _ => None,
                })
                .expect("the row is drawn")
        };

        menu.seed("display.vsync", &Value::Text("on".to_string()));
        assert_eq!(label_colour(&menu), DIMMED);
        menu.seed("display.vsync", &Value::Text("off".to_string()));
        assert_eq!(label_colour(&menu), SELECTED);
        menu.seed("display.vsync", &Value::Text("smooth".to_string()));
        assert_eq!(label_colour(&menu), SELECTED);
    }

    /// `disabled_by` naming something nothing edits is a row that is never
    /// greyed out, which looks exactly like a working one.
    #[test]
    fn a_condition_on_a_setting_nothing_edits_is_refused() {
        let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "choice"
label = "LIMIT"
setting = "a.limit"
disabled_by = { setting = "a.nothing", value = "on" }
values = ["1", "2"]
"#;
        let e = Definition::parse(text).expect_err("must not load");
        assert!(matches!(e, Error::BadCondition { .. }), "{e}");
        assert!(e.to_string().contains("a.nothing"), "{e}");
    }

    /// And a value that row can never hold is the same failure one level down:
    /// the setting exists, the condition is simply unreachable, and the row
    /// stays live forever.
    #[test]
    fn a_condition_on_a_value_no_row_can_hold_is_refused() {
        let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "choice"
label = "VSYNC"
setting = "a.vsync"
values = ["off", "on"]
[[page.entry]]
kind = "choice"
label = "LIMIT"
setting = "a.limit"
disabled_by = { setting = "a.vsync", value = "onn" }
values = ["1", "2"]
"#;
        let e = Definition::parse(text).expect_err("must not load");
        assert!(matches!(e, Error::BadCondition { .. }), "{e}");
        assert!(e.to_string().contains("onn"), "{e}");
        assert!(e.to_string().contains("off, on"), "{e}");
    }

    /// A number cannot be told apart from the text a choice row stores, so it
    /// is refused rather than guessed at.
    #[test]
    fn a_condition_value_that_is_not_text_or_a_flag_is_refused() {
        let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "choice"
label = "LIMIT"
setting = "a.limit"
disabled_by = { setting = "a.limit", value = 60 }
values = ["1", "2"]
"#;
        let e = Definition::parse(text).expect_err("must not load");
        assert!(matches!(e, Error::BadCondition { .. }), "{e}");
    }

    /// And on a kind that cannot be adjusted it would parse and do nothing,
    /// which is the whole class of mistake this loader exists to refuse.
    #[test]
    fn only_an_adjustable_row_may_be_disabled() {
        let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "toggle"
label = "VSYNC"
setting = "a.vsync"
[[page.entry]]
kind = "back"
label = "BACK"
disabled_by = { setting = "a.vsync", value = true }
"#;
        let e = Definition::parse(text).expect_err("must not load");
        assert!(matches!(e, Error::BadEntry { .. }), "{e}");
        assert!(e.to_string().contains("disabled_by"), "{e}");
    }

    /// The RENDERER row is the one setting on either page that a running game
    /// cannot act on, so it is the one row that has to say so. Pinned to the
    /// asset because losing the field is invisible: the row keeps working, keeps
    /// storing, and simply stops explaining why nothing changed.
    #[test]
    fn the_renderer_row_says_a_restart_is_needed() {
        let definition = built_in();
        let entry = definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .find(|entry| entry.setting() == Some("graphics.renderer"))
            .expect("nothing edits graphics.renderer");
        let restart = entry.restart().expect("the renderer defers to a restart");
        assert!(
            restart.message.to_uppercase().contains("RESTART"),
            "the message has to say the word: {:?}",
            restart.message
        );
        // Every other row on the two settings pages applies this run. A third
        // one appearing here is not necessarily wrong, but it is a claim about
        // what the game can do live and it should be made deliberately.
        //
        // `graphics.anti_aliasing` is the second, and deliberately: `off`,
        // `fxaa` and `smaa` are live, the same as every other row, but moving
        // to or between the two MSAA levels rebuilds every scene pipeline, so
        // that half of the row genuinely cannot apply this frame. See
        // ADR-0013 and `Session::open_menus`, which tells the two cases
        // apart by sample count rather than treating the whole row as
        // deferred.
        //
        // `graphics.boost_fov_kick` is the third, for the same shape of
        // reason as MSAA: `Race::set_boost_fov_kick` is only ever called once,
        // at `Race::start`, so a race already running keeps whatever it was
        // built with until the next one starts.
        let deferred: Vec<&str> = definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .filter(|entry| entry.restart().is_some())
            .filter_map(Entry::setting)
            .collect();
        assert_eq!(
            deferred,
            [
                "graphics.renderer",
                "graphics.anti_aliasing",
                "graphics.boost_fov_kick"
            ]
        );
    }

    /// The note is about what the *game* is doing, not about what the settings
    /// file holds - which is why it stays quiet until it has been told, and why
    /// it goes quiet again when the row comes back.
    #[test]
    fn the_restart_note_appears_only_once_the_row_leaves_what_is_running() {
        let mut menu = Menu::new(built_in());
        assert!(menu.open("graphics"), "the graphics page exists");
        let adapters = ["default", "vulkan: Card", "vulkan: Other (cpu)"]
            .map(|name| Choice::plain(name.to_string()));
        menu.supply(ValueSource::Renderers, &adapters);
        menu.seed(
            "graphics.renderer",
            &Value::Text("vulkan: Card".to_string()),
        );
        let row = |menu: &Menu| {
            menu.page()
                .entries
                .iter()
                .find(|entry| entry.setting() == Some("graphics.renderer"))
                .expect("the renderer is on the graphics page")
                .clone()
        };

        // Seeded, listed, and nothing has said what is running: silent. The
        // alternative is a note on a menu nobody has touched.
        assert!(menu.restart_note(&row(&menu)).is_none());

        assert!(menu.in_effect(
            "graphics.renderer",
            &[Value::Text("vulkan: Card".to_string())]
        ));
        assert!(
            menu.restart_note(&row(&menu)).is_none(),
            "the row still holds what the game is drawing with"
        );

        // Moving it is the whole point, and the row is *not* greyed: it is the
        // way back from an adapter that will not draw.
        let events = press(&mut menu, &[button::RIGHT]);
        assert_eq!(events.len(), 1, "{events:?}");
        assert!(menu.restart_note(&row(&menu)).is_some());

        // And back again, because a note that never clears teaches a player to
        // ignore it.
        press(&mut menu, &[button::LEFT]);
        assert!(menu.restart_note(&row(&menu)).is_none());
    }

    /// A game that let wgpu pick is on `default` *and* on whatever wgpu picked.
    /// Naming that adapter changes the settings file and nothing about the
    /// picture, so a note telling the player to restart for it would be false.
    #[test]
    fn naming_the_adapter_the_default_already_resolved_to_is_not_a_change() {
        let mut menu = Menu::new(built_in());
        assert!(menu.open("graphics"));
        let adapters = ["default", "vulkan: Card", "vulkan: Other"]
            .map(|name| Choice::plain(name.to_string()));
        menu.supply(ValueSource::Renderers, &adapters);
        menu.seed("graphics.renderer", &Value::Text("default".to_string()));
        menu.in_effect(
            "graphics.renderer",
            &[
                Value::Text("default".to_string()),
                Value::Text("vulkan: Card".to_string()),
            ],
        );
        let row = |menu: &Menu| {
            menu.page()
                .entries
                .iter()
                .find(|entry| entry.setting() == Some("graphics.renderer"))
                .expect("the renderer is on the graphics page")
                .clone()
        };

        // `default` -> `vulkan: Card`, the adapter it already resolved to.
        press(&mut menu, &[button::RIGHT]);
        assert_eq!(
            row(&menu).chosen(),
            Some(Value::Text("vulkan: Card".to_string()))
        );
        assert!(menu.restart_note(&row(&menu)).is_none());

        // One further along is a different card, and that does need a restart.
        press(&mut menu, &[button::RIGHT]);
        assert!(menu.restart_note(&row(&menu)).is_some());
    }

    /// The visible half: an amber marker in the margin and one line under the
    /// rows, the same channel a warning uses, because to a player they are the
    /// same sentence - this row is not doing what it says.
    #[test]
    fn a_restart_note_is_drawn_in_the_margin_and_under_the_rows() {
        let mut menu = Menu::new(built_in());
        assert!(menu.open("graphics"));
        let adapters = ["default", "vulkan: Card"].map(|name| Choice::plain(name.to_string()));
        menu.supply(ValueSource::Renderers, &adapters);
        menu.seed("graphics.renderer", &Value::Text("default".to_string()));
        menu.in_effect("graphics.renderer", &[Value::Text("default".to_string())]);

        let amber = |menu: &Menu| {
            draw_list(menu, &|_| vec!["X"], None)
                .into_iter()
                .filter_map(|draw| match draw {
                    Draw::Text { color, text, .. } if color == WARNING => Some(text),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(amber(&menu).is_empty(), "nothing has been changed yet");

        press(&mut menu, &[button::RIGHT]);
        let drawn = amber(&menu);
        assert_eq!(drawn.len(), 2, "a marker and a message: {drawn:?}");
        assert_eq!(drawn[0], "!");
        assert!(drawn[1].to_uppercase().contains("RESTART"), "{drawn:?}");
    }

    /// A row nothing supplies is silent, which is the one way this mechanism
    /// can fail invisibly - so `in_effect` reports whether anybody took it, and
    /// the composition root says so on stderr.
    #[test]
    fn telling_a_setting_no_row_defers_reports_it() {
        let mut menu = Menu::new(built_in());
        assert!(menu.in_effect("graphics.renderer", &[Value::Text("default".to_string())]));
        assert!(
            !menu.in_effect("display.vsync", &[Value::Text("off".to_string())]),
            "the vsync row applies live and declares no restart"
        );
        assert!(!menu.in_effect("nothing.at.all", &[Value::Flag(true)]));
    }

    /// A marker with nothing to read is a puzzle: it says something is wrong
    /// and not what to do about it.
    #[test]
    fn a_restart_required_with_no_message_is_refused() {
        let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "choice"
label = "RENDERER"
setting = "a.renderer"
values = ["one", "two"]
restart_required = ""
"#;
        let e = Definition::parse(text).expect_err("must not load");
        assert!(matches!(e, Error::BadEntry { .. }), "{e}");
        assert!(e.to_string().contains("restart_required"), "{e}");
    }

    /// And on a kind that cannot be adjusted it would parse and do nothing -
    /// the same refusal `disabled_by` gets, for the same reason.
    #[test]
    fn only_an_adjustable_row_may_need_a_restart() {
        let text = r#"
version = 1
root = "main"
[[page]]
id = "main"
[[page.entry]]
kind = "back"
label = "BACK"
restart_required = "RESTART THE GAME"
"#;
        let e = Definition::parse(text).expect_err("must not load");
        assert!(matches!(e, Error::BadEntry { .. }), "{e}");
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

    /// The bug this ordering rule exists to stop: a row seeded to the player's
    /// value, then handed its list, must not silently fall back to whatever is
    /// first. It would draw wrong *and* the next nudge would persist the wrong
    /// value as a deliberate choice.
    #[test]
    fn supplying_a_list_keeps_the_value_the_row_was_already_on() {
        let definition = Definition::parse(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            [[page.entry]]
            kind = "choice"
            label = "LANGUAGE"
            setting = "language"
            values_from = "languages"
            "#,
        )
        .expect("parse");
        let mut menu = Menu::new(definition);

        let offered =
            |names: &[&str]| -> Vec<Choice> { names.iter().map(|n| Choice::plain(*n)).collect() };
        menu.supply(ValueSource::Languages, &offered(&["French", "English"]));
        assert!(menu.seed("language", &Value::Text("English".to_string())));
        assert_eq!(
            menu.page().entries[0].chosen(),
            Some(Value::Text("English".to_string()))
        );

        // Supplied again, the seeded value survives even though it is not first.
        menu.supply(
            ValueSource::Languages,
            &offered(&["French", "German", "English"]),
        );
        assert_eq!(
            menu.page().entries[0].chosen(),
            Some(Value::Text("English".to_string())),
            "a re-supplied list must not reset the row to its first option"
        );

        // And a list that no longer has it falls to the first, which is the only
        // thing left to fall to.
        menu.supply(ValueSource::Languages, &offered(&["French", "German"]));
        assert_eq!(
            menu.page().entries[0].chosen(),
            Some(Value::Text("French".to_string()))
        );
    }

    /// A disc-supplied row stores an id and shows a name, and the event carries
    /// the id. Persisting the label would write words a player read into a
    /// settings file, where the next run would not recognise them.
    #[test]
    fn a_supplied_row_shows_its_label_and_reports_its_value() {
        let definition = Definition::parse(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            [[page.entry]]
            kind = "choice"
            label = "TRACK"
            setting = "race.track"
            values_from = "tracks"
            "#,
        )
        .expect("parse");
        let mut menu = Menu::new(definition);
        menu.supply(
            ValueSource::Tracks,
            &[
                Choice::labelled("16_Track", "A Circuit"),
                Choice::labelled("32_Track", "A Circuit Reversed"),
            ],
        );

        assert_eq!(
            menu.page().entries[0].value(),
            Some(Value::Text("A Circuit".to_string())),
            "the row draws the name"
        );
        let events = press(&mut menu, &[button::RIGHT]);
        assert_eq!(
            events,
            vec![MenuEvent::Changed {
                setting: "race.track".to_string(),
                value: Value::Text("32_Track".to_string()),
            }],
            "and reports the id"
        );
    }

    /// A row whose source turned out to have nothing draws no value and does
    /// not move, rather than panicking on an empty list.
    #[test]
    fn a_row_with_nothing_supplied_is_inert() {
        let definition = Definition::parse(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            [[page.entry]]
            kind = "choice"
            label = "TRACK"
            setting = "race.track"
            values_from = "tracks"
            "#,
        )
        .expect("parse");
        let mut menu = Menu::new(definition);
        menu.supply(ValueSource::Tracks, &[]);

        assert_eq!(menu.page().entries[0].value(), None);
        assert!(press(&mut menu, &[button::RIGHT]).is_empty());
        assert!(press(&mut menu, &[button::CROSS]).is_empty());
    }

    #[test]
    fn a_choice_cannot_declare_both_a_list_and_a_source() {
        let error = Definition::parse(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            [[page.entry]]
            kind = "choice"
            label = "TRACK"
            setting = "race.track"
            values = ["16_Track"]
            values_from = "tracks"
            "#,
        )
        .expect_err("refused");
        assert!(error.to_string().contains("alternatives"), "{error}");
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
        let list = draw_list(&menu, &no_bindings, None);

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
        for draw in draw_list(&menu, &no_bindings, None) {
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
        let list = draw_list(&menu, &no_bindings, None);
        assert!(
            list.iter()
                .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "UNBOUND")),
            "{list:?}"
        );
        let list = draw_list(&menu, &|_| vec!["ENTER", "X"], None);
        assert!(
            list.iter()
                .any(|draw| matches!(draw, Draw::Text { text, .. } if text == "ENTER / X")),
            "{list:?}"
        );
    }

    /// The backdrop has to be **first** in the list and nothing else may move.
    ///
    /// First because the list is painted back to front, and a menu drawn under
    /// its own background is a black screen with a movie on it. Nothing else
    /// moving is the other half: a source with a backdrop and one without have
    /// to lay the rows out identically, or the layout depends on which disc is
    /// in the drive.
    #[test]
    fn a_backdrop_is_drawn_behind_the_rows_and_moves_none_of_them() {
        let mut menu = Menu::new(built_in());
        assert!(menu.open("display"));

        let plain = draw_list(&menu, &no_bindings, None);
        assert!(
            !plain.iter().any(|draw| matches!(draw, Draw::Video { .. })),
            "no backdrop means no video draw at all: {plain:?}"
        );

        let backdrop = Backdrop {
            rect: [12.0, 34.0, 456.0, 78.0],
            frame: 91,
            // Past the movie's own length, because the menus are a continuation
            // of the playback `Show Logo` sat on rather than a second one - see
            // `Backdrop::position`.
            position: 631,
        };
        let with = draw_list(&menu, &no_bindings, Some(backdrop));
        assert_eq!(
            with.first(),
            Some(&Draw::Video {
                rect: backdrop.rect,
                frame: backdrop.frame,
                position: backdrop.position,
                source: crate::frontend::Video::Backdrop,
            }),
            "the backdrop has to be painted first: {with:?}"
        );
        assert_eq!(
            &with[1..],
            &plain[..],
            "a backdrop must add a draw and change no other"
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
