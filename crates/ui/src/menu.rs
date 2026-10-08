//! The shell's menus: our own definition, not the disc's.
//!
//! Like [`frontend`](crate::frontend), this holds no GPU handles, opens no
//! files and reads no clock. It takes input in and emits [`MenuEvent`]s and a
//! list of [`Draw`](crate::frontend::Draw)s out, both of which are plain data;
//! `oag_game::render` rasterises the one and `oag_game::main` acts on the
//! other - see `docs/architecture/workspace-layout.md`.
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
//! **The shape is ours; where it is drawn is not.** That split is
//! `docs/architecture/menus.md`'s one-liner - the tree is ours, the presentation
//! is the disc's - and it reaches further than a set of coordinates: Wipeout HD
//! lays its main menu out *horizontally*, so the same page is drawn as a column
//! on a PSP title and as a strip on that one. See [`rows`] and [`strip`], which
//! [`draw_list`] picks between.
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
//! It never reads or writes `crate::settings::Settings`. A `choice`
//! entry is seeded with its current value by whoever owns the setting and emits
//! [`MenuEvent::Changed`] when the player moves it; applying and persisting that
//! is the composition root's business. Keeping that seam is what lets every test
//! below run with no config directory, no disc and no window.

use crate::frontend::Draw;
use oag_core::buttons::{Button, Input};

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

mod vocabulary;
pub use vocabulary::{Action, ValueSource};

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
        /// What pins it to one of its own options. See [`Pin`].
        pins: Vec<Pin>,
        /// What makes this row's setting stored but ineffective, one entry
        /// per independently-triggered combination. See [`Menu::warning`].
        warnings: Vec<Warning>,
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
        /// What makes this row's setting stored but ineffective. See [`Menu::warning`].
        warnings: Vec<Warning>,
        /// What this row says when it has been moved off the value the game is
        /// running on. See [`Menu::restart_note`].
        restart: Option<Restart>,
    },
    /// Shows what an abstract button is currently bound to.
    ///
    /// **`Menu::activate` still treats confirming this as a no-op.** A rebind
    /// needs the raw key just pressed, gone by the time an event reaches this
    /// crate's abstract buttons - `Session::maybe_begin_binding` and
    /// `oag_input::bindings::Bindings` do the rest. See
    /// `docs/architecture/menus.md`.
    Binding {
        /// Row label.
        label: String,
        /// Abstract button.
        button: Button,
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

    /// This row's declared warnings; [`Menu::warning`] says which applies.
    #[must_use]
    pub fn warnings(&self) -> &[Warning] {
        match self {
            Self::Choice { warnings, .. } | Self::Toggle { warnings, .. } => warnings,
            _ => &[],
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
    /// One row's second line, shown only while it is selected - `menu.toml`'s
    /// own `subtitle`. Same length and order as [`Self::entries`], `None`
    /// where a row named none. Kept parallel rather than folded into
    /// [`Entry`] itself so adding it did not touch that enum's six variants
    /// or their construction sites elsewhere in this crate and `oag_game`.
    /// See `docs/architecture/menus.md#a-per-row-subtitle`.
    pub subtitles: Vec<Option<String>>,
}

impl Page {
    /// Drops rows from [`Self::entries`] and their [`Self::subtitles`]
    /// together, so the two never drift out of the lockstep that field's own
    /// doc promises. `definition::Definition`'s two row-dropping passes both
    /// go through this rather than calling `entries.retain` directly, which
    /// is what a plain `retain` here would silently get wrong: it would drop
    /// a row from one vector and leave its subtitle attached to whatever row
    /// slid into its old index.
    pub(super) fn retain_rows(&mut self, mut keep: impl FnMut(&Entry) -> bool) {
        let mut kept = Vec::with_capacity(self.entries.len());
        self.entries.retain(|entry| {
            let k = keep(entry);
            kept.push(k);
            k
        });
        let mut kept = kept.into_iter();
        self.subtitles.retain(|_| kept.next().unwrap_or(false));
    }
}

mod definition;
pub use definition::{Definition, Error};

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
    /// How far the visible window has been pushed down, **per page** and
    /// parallel to [`Definition::pages`] for the reason [`Self::cursor`] is.
    ///
    /// A hint rather than the answer: [`Self::scroll`] clamps it against the
    /// page it is read for, so a window left somewhere that no longer holds the
    /// cursor - because [`Self::open`] jumped, or because a page was left near
    /// its end and re-entered - draws correctly without every caller having to
    /// remember to reset it. Only [`Self::update`] writes it, and only to say
    /// "the window really is here now", which is what stops a cursor moving
    /// back up from snapping the list to the top.
    scroll: Vec<usize>,
    /// How many rows fit on screen. See [`Self::set_visible_rows`].
    visible: usize,
    /// Whether this title draws navigation pages as a strip. See
    /// [`Self::set_strip_layout`].
    strip_layout: bool,
    /// How far each row's block has grown toward its selected width, per
    /// page and parallel to [`Self::cursor`]. See `focus.rs`.
    focus: Vec<Vec<f32>>,
    /// Ticks since the page on screen arrived, or `None` once settled. See
    /// `focus.rs`.
    arrival: Option<u32>,
    /// A finger drag's travel not yet worth a whole row, in rows. See `pointer.rs`.
    drag_rows: f32,
    /// The navigation sounds this menu has called for since the caller last
    /// drained them with [`Self::take_nav`].
    nav: nav::Log,
}

/// How many rows a page shows before the caller says otherwise.
///
/// **Ours**, and picked to match what a title-supplied skin will usually
/// produce rather than to be round: seven is what the 480x272 screen fits at
/// the 28-pixel pitch measured off the original, and is what the original's own
/// main menu shows. [`visible_rows`] recomputes it from the live skin, so this
/// is only ever what a `Menu` built without one uses.
pub const DEFAULT_VISIBLE_ROWS: usize = 7;

impl Menu {
    /// Opens a definition at its root page.
    #[must_use]
    pub fn new(definition: Definition) -> Self {
        let root = definition.root;
        let cursor = vec![0; definition.pages.len()];
        let scroll = vec![0; definition.pages.len()];
        let focus = definition
            .pages
            .iter()
            .map(|page| focus::snapped(page.entries.len(), 0))
            .collect();
        Self {
            definition,
            stack: vec![root],
            cursor,
            scroll,
            visible: DEFAULT_VISIBLE_ROWS,
            // A column until a title says otherwise, which is what both PSP
            // discs measurably say.
            strip_layout: false,
            focus,
            arrival: Some(0),
            drag_rows: 0.0,
            nav: nav::Log::default(),
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

    /// First row of the page being drawn that is actually on screen.
    ///
    /// Zero on every page that fits, which is most of them; the graphics and
    /// controls pages do not, and a row a player cannot see is a row they
    /// cannot set. Derived from the stored push rather than returned raw, so
    /// this is always a window that holds the cursor - see [`window_start`] for
    /// the rule and for why the list moves on the second-last visible row.
    #[must_use]
    pub fn scroll(&self) -> usize {
        window_start(
            self.scroll[self.current()],
            self.selected(),
            self.page().entries.len(),
            self.visible,
        )
    }

    /// How many rows fit on screen, which the row pitch decides.
    ///
    /// Set by whoever is drawing, from the live [`Skin`], because the pitch is
    /// the title's and this type has no font: see [`visible_rows`]. Left at
    /// [`DEFAULT_VISIBLE_ROWS`] by a caller that never sets it, so a menu built
    /// in a test scrolls the same way one on screen does.
    pub fn set_visible_rows(&mut self, rows: usize) {
        self.visible = rows.max(3);
    }

    /// What [`Self::set_visible_rows`] last said.
    #[must_use]
    pub fn visible_rows(&self) -> usize {
        self.visible
    }

    /// Whether this title's navigation pages are drawn as a horizontal strip.
    ///
    /// Set by whoever is drawing, from the live [`Skin`], for the same reason
    /// [`Self::set_visible_rows`] is: whether a strip exists at all is the
    /// *title's* answer - [`oag_title::MenuStrip`] - and this type holds no
    /// skin. Which pages it then applies to is this type's own page list, so the
    /// flag says only "this disc draws strips" and [`strip::suits`] decides the
    /// rest.
    ///
    /// It reaches [`Self::update`] and nothing else. What is drawn is decided
    /// from the skin directly in [`draw_list`], so a caller that forgets this
    /// gets a strip that reads left-to-right and steps with up and down - wrong,
    /// but not a menu drawn one way and navigated another.
    ///
    /// **`crate::capture` deliberately does not set it**, which is not an
    /// oversight to fix: `--menu-page` draws one frame and never calls
    /// [`Self::update`], so the only field that would read this is never
    /// consulted. Its picture is the strip either way, because that comes off
    /// the skin.
    pub fn set_strip_layout(&mut self, strip: bool) {
        self.strip_layout = strip;
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
        let mut moved = false;
        if input.take(Button::Down) && rows > 0 {
            self.cursor[page] = (self.cursor[page] + 1) % rows;
            moved = true;
        }
        if input.take(Button::Up) && rows > 0 {
            self.cursor[page] = (self.cursor[page] + rows - 1) % rows;
            moved = true;
        }
        // The window follows the cursor and then **stays there**. Storing it is
        // the whole difference between a list that scrolls and one that snaps:
        // without this the window is only ever the tightest one holding the
        // cursor, so stepping back up off the last row would jump the page to
        // the top instead of revealing the row above.
        if moved {
            self.scroll[page] = self.scroll();
        }
        self.nav.when(moved && rows > 1, nav::Nav::UpDown);

        let right = input.take(Button::Right);
        let left = input.take(Button::Left);
        // On a page drawn as a strip, left and right are what *step* it: the
        // entries run that way on screen, so a cursor that only answered up and
        // down would be moving across an axis the page does not have. There is
        // nothing for them to adjust on such a page in any case - [`strip::suits`]
        // admits only navigation entries, and none of those carries a value - so
        // this is a choice between "step" and "do nothing", not between two
        // meanings.
        //
        // **Up and down keep working there, and that part is ours.** The
        // entries are one list however they are laid out, and a player whose
        // thumb goes down on a menu has not asked for nothing to happen. The
        // original is presumably left/right only; nothing here claims otherwise.
        if self.strip_layout && strip::suits(self.page()) && rows > 0 {
            if right {
                self.cursor[page] = (self.cursor[page] + 1) % rows;
            }
            if left {
                self.cursor[page] = (self.cursor[page] + rows - 1) % rows;
            }
            self.nav.when((left || right) && rows > 1, nav::Nav::UpDown);
        } else if (right || left)
            && let Some(event) = self.adjust(if right { 1 } else { -1 })
        {
            out.push(event);
        }

        if input.take(Button::Cross) || input.take(Button::Start) {
            out.extend(self.activate());
        }
        if input.take(Button::Circle) {
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
        self.pin(entry).is_some()
            || entry
                .disabled_by()
                .is_some_and(|condition| condition.matches(self.held(&condition.setting).as_ref()))
    }

    /// This row's warning: the first declared one that applies. See [`Entry::warnings`].
    ///
    /// A warned row is drawn normally and marked, not greyed: its setting *is*
    /// stored and *will* take effect the moment the row it conflicts with
    /// moves. Greying would say "you cannot change this", which is false.
    #[must_use]
    pub fn warning<'a>(&self, entry: &'a Entry) -> Option<&'a Warning> {
        entry.warnings().iter().find(|warning| {
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
}

pub mod block;
mod picture;
pub use picture::{Backdrop, Picture};
mod focus;
mod frame;
mod jump;
pub mod nav;
mod pin;
pub use pin::Pin;
mod layers;
pub mod pointer;
mod rows;
mod skin;
mod strip;

pub use frame::{Frame, read as read_frame, settings::Layout as SettingsLayout};
pub use layers::{Layers, Transition};
pub use skin::{List, Skin, Strip, visible_rows};

/// Where the visible window starts, given where it was pushed to and where the
/// cursor is.
///
/// The rule is one row of lookahead at each end: the row after the selected one
/// is always drawn, so a player moving down sees where they are going before
/// they get there, and the list starts moving when the cursor reaches the
/// **second-last** visible row rather than the last. The same on the way up.
/// That is the `floor`/`ceiling` pair below, and it is why a menu never scrolls
/// with the cursor pinned to the bottom edge.
///
/// `pushed` is a hint and this is a pure function of it, which is what lets
/// [`Menu::scroll`] be correct on a page whose stored window predates a jump:
/// anything out of range is clamped back into one rather than trusted. The two
/// ends cannot fight - `floor <= ceiling` needs only `visible >= 3` - so
/// the `max` last is safe.
///
/// `visible` is passed in rather than a constant because the row pitch is now
/// the title's, and a skin with a taller face fits fewer rows.
fn window_start(pushed: usize, selected: usize, rows: usize, visible: usize) -> usize {
    let visible = visible.max(3);
    // A page that fits never scrolls, whatever it was left holding.
    if rows <= visible {
        return 0;
    }
    let last = rows - visible;
    // Far enough down that the row under the cursor is drawn - unless the
    // cursor is on the final row, where `min(rows)` says there is nothing left
    // to keep visible and the window stops at the end of the list.
    let floor = (selected + 2).min(rows).saturating_sub(visible);
    // Not so far down that the row above the cursor is cut off, and not past
    // the top of the list when the cursor is on the first row.
    let ceiling = selected.saturating_sub(1);
    pushed.min(last).min(ceiling).max(floor)
}

/// Rows that show something the player cannot change yet.
///
/// **Ours, and it has no counterpart on the disc.** The original has no
/// disabled rows: a Wipeout menu either offers a thing or does not list it.
/// This exists for `disabled_by`, which is a PC concern.
pub const DIMMED: [f32; 4] = [0.45, 0.5, 0.56, 1.0];
/// A setting that is stored but currently doing nothing.
///
/// Amber, and on its own channel: the other three colours already mean
/// selected, normal and inert, so a warning had to be a mark in the margin in a
/// colour none of them use rather than a fourth shade of the row itself.
pub const WARNING: [f32; 4] = [1.0, 0.76, 0.25, 1.0];

/// What one page looks like, as plain data.
///
/// The same [`Draw`] vocabulary the language picker emits, so `main.rs`
/// rasterises menus with the renderer it already has and there is one text
/// path rather than two.
///
/// `skin` is where every position, size and colour comes from - see [`Skin`],
/// which is also the one place this build's own fallbacks are applied. Nothing
/// below reads a title package directly.
///
/// `bindings` answers "what is this button bound to" and is passed in rather
/// than looked up, because a keyboard is a device concern: `oag_input::keys`
/// owns the mapping, this crate owns the layout, and neither has to know how
/// the other works. `main.rs` hands over `oag_input::keys::bound_keys`.
///
/// `measure` is the width of a string in the face the entries will be drawn in,
/// on the same seam and for the same reason: the atlas is the renderer's and the
/// layout is this crate's. Only a [`strip`] reads it - a column starts every row
/// at one x - which is why it arrived with the second idiom rather than with the
/// first.
///
/// `backdrop` is the same arrangement one step further out: a frame and a
/// rectangle, already decided, rather than a movie this module would then have
/// to know how to play - or the Fury backdrop's frame, already computed, rather
/// than its clock. `None` draws the rows on whatever the frame put down, or
/// black if it put down nothing - see [`Picture`].
///
/// # Two idioms, and the disc picks
///
/// A page is drawn as a column of rows ([`rows`]) unless the title authors a
/// horizontal strip and the page has nothing to put in a value column, in which
/// case it is drawn as one ([`strip`]). Wipeout HD is the title that does; both
/// PSP titles measurably do not. See [`oag_title::MenuStrip`].
#[must_use]
pub fn draw_list(
    menu: &Menu,
    skin: &Skin,
    bindings: &dyn Fn(Button) -> Vec<&'static str>,
    measure: &dyn Fn(&str) -> f32,
    backdrop: Option<Picture>,
    frame: &Frame,
    // A parked race showing through instead of this frame's own
    // clear/background/marks - see `Frame::backdrops`. Needed alongside
    // `backdrop: None`, which an ordinary no-movie title reaches too.
    race_behind: bool,
) -> Layers {
    let page = menu.page();
    let (title_x, title_y, title_scale) = skin.title_at();

    // Frame::backdrops's own clear/picture/marks order - see its doc for why
    // a race behind the menus drops anything covering the whole screen.
    let picture = backdrop.map(Picture::draw);
    let backdrops = frame.backdrops(skin.space(), skin.background(), picture, race_behind);

    let mut layers = Layers {
        backdrop: backdrops,
        ..Layers::default()
    };
    // The disc's own declared colour, the frame's own ink, or this build's
    // substitute (`Skin::title_color`), in whichever face `skin.title_font()`
    // names - `None` on both PSP titles today, which is the plain `Draw::Text`
    // this build has always emitted; see `oag_title::MenuSkin::title_font`.
    layers.chrome.push(Draw::title(
        skin.title_font(),
        title_x,
        title_y,
        title_scale,
        skin.title_color(frame.ink),
        page.title.clone(),
    ));

    // Which idiom a page is drawn in is the *title's* answer first and this
    // page's second: a strip needs a disc that authors one, and a page with
    // nothing to put in a value column. Everything else is a row list, which is
    // every page on both PSP titles and all but the root on Wipeout HD.
    layers.body = match skin.strip().filter(|_| strip::suits(page)) {
        Some(strip) => strip::draw(menu, skin, strip, measure, frame),
        None => rows::draw(menu, skin, bindings, frame),
    };

    layers
}

#[cfg(test)]
mod tests;
