//! The campaign editor's model: a custom campaign (ADR-0077) built one hex slot
//! at a time, and written as a folder mod the next start finds.
//!
//! It knows no screen. The session draws it on the title's own `Cell Selection`
//! and feeds it the cursor, a cycle of a field, a save; the same calls build a
//! campaign in a test. What it writes is [`oag_mod::campaign::CampaignDef`], so the
//! editor can express nothing the format does not.

use std::collections::BTreeMap;

use oag_core::Rng;
use oag_mod::campaign::{
    CampaignDef, CraftDef, EventDef, FieldDef, GridDef, Info, MedalsDef, Progression, Reveal,
    UnlockDef,
};

/// One thing a field can be set to: the word the file carries, and the words a
/// player reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pick {
    /// What the campaign file writes.
    pub value: String,
    /// What the screen shows.
    pub label: String,
}

impl Pick {
    /// A pick whose label is its value.
    #[must_use]
    pub fn plain(value: &str) -> Self {
        Self {
            value: value.to_string(),
            label: value.to_ascii_uppercase(),
        }
    }
}

/// The modes the editor offers: the campaign file's word, and the string id and English
/// of its label. Tournament names legs of its own and is left to the file.
pub const MODES: [(&str, &str, &str); 6] = [
    ("race", "OAG_EDITOR_MODE_RACE", "RACE"),
    ("time_trial", "OAG_EDITOR_MODE_TIME_TRIAL", "TIME TRIAL"),
    ("speed_lap", "OAG_EDITOR_MODE_SPEED_LAP", "SPEED LAP"),
    ("zone", "OAG_EDITOR_MODE_ZONE", "ZONE"),
    ("elimination", "OAG_EDITOR_MODE_ELIMINATOR", "ELIMINATOR"),
    ("head2head", "OAG_EDITOR_MODE_HEAD_TO_HEAD", "HEAD TO HEAD"),
];

/// Whose driver flies the opponents: the file's word, then the label's id and English.
pub const AIS: [(&str, &str, &str); 2] = [
    ("original", "OAG_EDITOR_AI_ORIGINAL", "ORIGINAL"),
    ("ours", "OAG_EDITOR_AI_OURS", "OURS"),
];

/// The base difficulties the editor offers, the original driver's rungs: the file's word,
/// then the label's id and English. Our driver takes the rung at the same place on its own
/// ladder (`docs/formats/custom-campaign.md`, "Difficulty").
pub const DIFFICULTIES: [(&str, &str, &str); 3] = [
    ("easy", "OAG_EDITOR_DIFFICULTY_EASY", "EASY"),
    ("medium", "OAG_EDITOR_DIFFICULTY_MEDIUM", "MEDIUM"),
    ("hard", "OAG_EDITOR_DIFFICULTY_HARD", "HARD"),
];

/// How the difficulty moves across the cells: the file's word, then the label's id and English.
pub const PROGRESSIONS: [(&str, &str, &str); 2] = [
    ("flat", "OAG_EDITOR_PROGRESSION_FLAT", "FLAT"),
    ("ramp", "OAG_EDITOR_PROGRESSION_RAMP", "RAMP"),
];

/// The longest seed the keyboard takes: nine digits always fit a `u32`.
pub const SEED_DIGITS: usize = 9;

/// Every other word the editor's screen and flow show: string id and English.
pub const WORDS: [(&str, &str); 33] = [
    ("OAG_EDITOR_FIELD_DIFFICULTY", "DIFFICULTY"),
    ("OAG_EDITOR_FIELD_PROGRESSION", "PROGRESSION"),
    ("OAG_EDITOR_FIELD_SEED", "SEED"),
    ("OAG_EDITOR_FIELD_CLASS_FROM", "CLASS FROM"),
    ("OAG_EDITOR_FIELD_CLASS_TO", "CLASS TO"),
    ("OAG_EDITOR_ACTION_FILL", "FILL GRID"),
    ("OAG_EDITOR_ACTION_SETTINGS", "CAMPAIGN SETTINGS"),
    ("OAG_EDITOR_ACTION_EVENT", "EVENT FIELDS"),
    ("OAG_EDITOR_SEED_TITLE", "TYPE A SEED"),
    ("OAG_EDITOR_FILLED", "GRID FILLED FROM THE SEED"),
    ("OAG_EDITOR_HINT_TO_SETTINGS", "SELECT FOR SETTINGS"),
    ("OAG_EDITOR_HINT_FILL", "TRIANGLE FILLS THE GRID"),
    ("OAG_EDITOR_LEGEND_FILL", "Fill grid"),
    ("OAG_EDITOR_TITLE", "CAMPAIGN EDITOR"),
    ("OAG_EDITOR_FIELD_MODE", "MODE"),
    ("OAG_EDITOR_FIELD_CIRCUIT", "CIRCUIT"),
    ("OAG_EDITOR_FIELD_CLASS", "SPEED CLASS"),
    ("OAG_EDITOR_FIELD_TEAM", "TEAM"),
    ("OAG_EDITOR_FIELD_AI", "OPPONENTS"),
    ("OAG_EDITOR_TEAM_CHOOSE", "PLAYER CHOOSES"),
    ("OAG_EDITOR_EMPTY", "EMPTY SLOT"),
    ("OAG_EDITOR_PLACED", "EVENTS"),
    ("OAG_EDITOR_ACTION_NAME", "NAME"),
    ("OAG_EDITOR_ACTION_SAVE", "SAVE CAMPAIGN"),
    ("OAG_EDITOR_ACTION_CLEAR", "CLEAR SLOT"),
    ("OAG_EDITOR_SAVED", "SAVED - FIND IT UNDER RACE CAMPAIGN"),
    ("OAG_EDITOR_NEEDS_EVENT", "PLACE AN EVENT FIRST"),
    ("OAG_EDITOR_NAME_TITLE", "NAME YOUR CAMPAIGN"),
    ("OAG_EDITOR_SAVE_FAILED", "COULD NOT SAVE THE CAMPAIGN"),
    ("OAG_EDITOR_HINT_PLACE", "CROSS PLACES AN EVENT"),
    ("OAG_EDITOR_LEGEND_CLEAR", "Clear slot"),
    ("OAG_EDITOR_LEGEND_NAME", "Name"),
    ("OAG_EDITOR_DEFAULT_NAME", "MY CAMPAIGN"),
];

/// The medal targets one of the disc's own cells authors for a mode, circuit and class,
/// in the campaign file's units (seconds on a timed mode).
#[derive(Debug, Clone, PartialEq)]
pub struct Authored {
    /// The campaign file's mode word.
    pub mode: String,
    /// The circuit id.
    pub track: String,
    /// The speed class, lowercase.
    pub class: String,
    /// Gold, silver, bronze.
    pub targets: [f64; 3],
}

impl Authored {
    /// What `cell` authors, when it is one of the modes the editor offers and names a
    /// circuit.
    #[must_use]
    pub fn of(cell: &oag_tables::race_campaign::Cell) -> Option<Self> {
        let mode = MODES.iter().map(|m| m.0).find(|word| {
            super::custom::MODES
                .iter()
                .any(|(w, m)| w == word && *m == cell.mode)
        })?;
        let scale = if matches!(mode, "time_trial" | "speed_lap") {
            100.0
        } else {
            1.0
        };
        Some(Self {
            mode: mode.to_string(),
            track: cell.track.clone()?,
            class: cell.class.to_ascii_lowercase(),
            targets: [cell.gold, cell.silver, cell.bronze].map(|t| t as f64 / scale),
        })
    }
}

/// What the title and its disc offer a slot to be set to.
#[derive(Debug, Clone)]
pub struct Choices {
    /// The disc's circuits.
    pub tracks: Vec<Pick>,
    /// The roster's teams.
    pub teams: Vec<Pick>,
    /// The title's speed classes, slowest first.
    pub classes: Vec<Pick>,
    /// The hexes the title's `Cell Selection` draws (`Layout::hex_slots`).
    pub slots: Vec<(u32, u32)>,
    /// The title's slug, for the mod's `titles`.
    pub title: String,
    /// The medal targets the disc's own cells author: a placed event takes those of the
    /// cell with its mode, circuit and class.
    pub authored: Vec<Authored>,
}

/// A field of one event, in the order the panel lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// The mode.
    Mode,
    /// The circuit.
    Circuit,
    /// The speed class.
    Class,
    /// The player's team, or `PLAYER CHOOSES`.
    Team,
    /// Whose driver flies the opponents.
    Ai,
}

impl Field {
    /// The fields, top to bottom.
    pub const ALL: [Self; 5] = [Self::Mode, Self::Circuit, Self::Class, Self::Team, Self::Ai];
}

/// Which page of the details panel is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// The selected event's fields.
    Event,
    /// The campaign's own settings: difficulty, the seed and the class bounds.
    Campaign,
}

/// A row of the campaign page, in the order the panel lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// The base difficulty of the opponents.
    Difficulty,
    /// How the difficulty moves across the cells.
    Progression,
    /// The seed [`Editor::fill`] draws from.
    Seed,
    /// The slowest speed class a filled cell may draw.
    ClassFrom,
    /// The fastest speed class a filled cell may draw.
    ClassTo,
}

impl Setting {
    /// The rows, top to bottom.
    pub const ALL: [Self; 5] = [
        Self::Difficulty,
        Self::Progression,
        Self::Seed,
        Self::ClassFrom,
        Self::ClassTo,
    ];
}

/// A row under the fields that acts on the campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Switches between the event's fields and the campaign's settings.
    Page,
    /// Fills the grid from the seed (the campaign page).
    Fill,
    /// Names the campaign (the event page).
    Name,
    /// Saves it.
    Save,
}

/// One slot's event as the editor holds it: an index into each list of
/// [`Choices`] (`team` is `0` for the player's own pick, then `1 +` a team).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draft {
    mode: usize,
    track: usize,
    class: usize,
    team: usize,
    ai: usize,
}

/// The campaign being built.
#[derive(Debug, Clone)]
pub struct Editor {
    choices: Choices,
    name: String,
    events: BTreeMap<(u32, u32), Draft>,
    cursor: (u32, u32),
    field: usize,
    editing: bool,
    words: BTreeMap<String, String>,
    page: Page,
    setting: usize,
    difficulty: usize,
    progression: usize,
    seed: u32,
    /// The lowest and highest class index a filled cell draws, inclusive.
    bounds: (usize, usize),
}

/// Where a slot's hex stands, in hex-widths and hex-heights: odd columns half a
/// hex lower, the stagger every title's grid draws.
fn position((x, y): (u32, u32)) -> (f32, f32) {
    (x as f32, y as f32 + if x % 2 == 1 { 0.5 } else { 0.0 })
}

impl Editor {
    /// An empty campaign called `name`, the cursor on the first slot.
    #[must_use]
    pub fn new(choices: Choices, name: &str) -> Self {
        let cursor = choices.slots.first().copied().unwrap_or((0, 0));
        let classes = choices.classes.len();
        Self {
            choices,
            name: name.to_string(),
            events: BTreeMap::new(),
            cursor,
            field: 0,
            editing: false,
            words: BTreeMap::new(),
            page: Page::Event,
            setting: 0,
            difficulty: 1,
            progression: 0,
            seed: 1,
            bounds: (0, classes.saturating_sub(1)),
        }
    }

    /// Takes the words the screen shows from `say` (a string id and its English).
    #[must_use]
    pub fn with_words(mut self, say: &dyn Fn(&str, &str) -> String) -> Self {
        for (id, english) in WORDS
            .iter()
            .copied()
            .chain(MODES.iter().map(|m| (m.1, m.2)))
            .chain(AIS.iter().map(|a| (a.1, a.2)))
            .chain(DIFFICULTIES.iter().map(|d| (d.1, d.2)))
            .chain(PROGRESSIONS.iter().map(|p| (p.1, p.2)))
        {
            self.words.insert(id.to_string(), say(id, english));
        }
        self
    }

    /// The words for `id`: the string table's if [`Self::with_words`] was given one,
    /// else `english`.
    #[must_use]
    pub fn say(&self, id: &str, english: &str) -> String {
        self.words
            .get(id)
            .cloned()
            .unwrap_or_else(|| english.to_string())
    }

    /// The name of `field`.
    #[must_use]
    pub fn field_name(&self, field: Field) -> String {
        let (id, english) = match field {
            Field::Mode => ("OAG_EDITOR_FIELD_MODE", "MODE"),
            Field::Circuit => ("OAG_EDITOR_FIELD_CIRCUIT", "CIRCUIT"),
            Field::Class => ("OAG_EDITOR_FIELD_CLASS", "SPEED CLASS"),
            Field::Team => ("OAG_EDITOR_FIELD_TEAM", "TEAM"),
            Field::Ai => ("OAG_EDITOR_FIELD_AI", "OPPONENTS"),
        };
        self.say(id, english)
    }

    /// The campaign's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Renames it.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.trim().to_string();
    }

    /// The lists a field cycles through.
    #[must_use]
    pub fn choices(&self) -> &Choices {
        &self.choices
    }

    /// The selected slot.
    #[must_use]
    pub fn cursor(&self) -> (u32, u32) {
        self.cursor
    }

    /// Whether the field panel is open on the selected slot.
    #[must_use]
    pub fn editing(&self) -> bool {
        self.editing
    }

    /// The selected field.
    #[must_use]
    pub fn field(&self) -> Field {
        Field::ALL[self.field]
    }

    /// Whether `slot` holds an event.
    #[must_use]
    pub fn has(&self, slot: (u32, u32)) -> bool {
        self.events.contains_key(&slot)
    }

    /// How many events are placed.
    #[must_use]
    pub fn count(&self) -> usize {
        self.events.len()
    }

    /// The slots holding an event.
    pub fn placed(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.events.keys().copied()
    }

    /// Selects `slot`, closing the field panel. A slot the title draws no hex at is
    /// refused.
    pub fn select(&mut self, slot: (u32, u32)) -> bool {
        if !self.choices.slots.contains(&slot) {
            return false;
        }
        self.cursor = slot;
        self.editing = false;
        self.page = Page::Event;
        true
    }

    /// Moves the cursor to the hex nearest the direction `(dx, dy)` (each `-1`, `0`
    /// or `1`; screen down is `+1`). Stays put at an edge.
    pub fn step(&mut self, dx: i32, dy: i32) {
        let (cx, cy) = position(self.cursor);
        let best = self
            .choices
            .slots
            .iter()
            .copied()
            .filter(|&slot| slot != self.cursor)
            .filter_map(|slot| {
                let (x, y) = position(slot);
                let (ax, ay) = (x - cx, y - cy);
                let along = ax * dx as f32 + ay * dy as f32;
                let across = ax * dy as f32 - ay * dx as f32;
                (along > 0.25).then_some((slot, across.abs() * 2.0 + along))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(slot, _)| slot);
        if let Some(slot) = best {
            self.cursor = slot;
        }
    }

    /// Confirm on the selected slot: an empty one gets a default event and its panel
    /// opens; an occupied one opens its panel.
    pub fn confirm(&mut self) {
        let draft = self.default_draft();
        self.events.entry(self.cursor).or_insert(draft);
        self.editing = true;
        self.field = 0;
    }

    /// Opens the field panel on the selected slot when it holds an event.
    pub fn begin_edit(&mut self) {
        if self.has(self.cursor) {
            self.editing = true;
        }
    }

    /// Closes the panel; with it closed, `false` (nothing to back out of here).
    pub fn back(&mut self) -> bool {
        std::mem::take(&mut self.editing)
    }

    /// Empties the selected slot.
    pub fn clear(&mut self) {
        self.events.remove(&self.cursor);
        self.editing = false;
    }

    /// Selects the field `delta` rows on.
    pub fn move_field(&mut self, delta: i32) {
        let n = Field::ALL.len() as i32;
        self.field = (self.field as i32 + delta).rem_euclid(n) as usize;
    }

    /// Selects `field`.
    pub fn select_field(&mut self, field: Field) {
        self.field = Field::ALL.iter().position(|&f| f == field).unwrap_or(0);
    }

    /// Steps the selected field of the selected slot's event `delta` values on,
    /// wrapping.
    pub fn cycle(&mut self, delta: i32) {
        let field = self.field();
        let lens = [
            MODES.len(),
            self.choices.tracks.len(),
            self.choices.classes.len(),
            self.choices.teams.len() + 1,
            AIS.len(),
        ];
        let Some(draft) = self.events.get_mut(&self.cursor) else {
            return;
        };
        let (slot, n) = match field {
            Field::Mode => (&mut draft.mode, lens[0]),
            Field::Circuit => (&mut draft.track, lens[1]),
            Field::Class => (&mut draft.class, lens[2]),
            Field::Team => (&mut draft.team, lens[3]),
            Field::Ai => (&mut draft.ai, lens[4]),
        };
        if n > 0 {
            *slot = (*slot as i32 + delta).rem_euclid(n as i32) as usize;
        }
    }

    /// The label of `field` on the event at `slot`, or `None` when it is empty. A Zone
    /// event has no class, which reads `-`.
    #[must_use]
    pub fn label(&self, slot: (u32, u32), field: Field) -> Option<String> {
        let draft = self.events.get(&slot)?;
        let pick =
            |list: &[Pick], at: usize| list.get(at).map_or("-".to_string(), |p| p.label.clone());
        Some(match field {
            Field::Mode => self.say(MODES[draft.mode].1, MODES[draft.mode].2),
            Field::Circuit => pick(&self.choices.tracks, draft.track),
            Field::Class if MODES[draft.mode].0 == "zone" => "-".to_string(),
            Field::Class => pick(&self.choices.classes, draft.class),
            Field::Team => match draft.team {
                0 => self.say("OAG_EDITOR_TEAM_CHOOSE", "PLAYER CHOOSES"),
                n => pick(&self.choices.teams, n - 1),
            },
            Field::Ai => self.say(AIS[draft.ai].1, AIS[draft.ai].2),
        })
    }

    /// The page of the panel that is open.
    #[must_use]
    pub fn page(&self) -> Page {
        self.page
    }

    /// Opens `page`; the event's field panel closes with it.
    pub fn set_page(&mut self, page: Page) {
        self.page = page;
        self.editing = false;
    }

    /// Switches between the event page and the campaign page.
    pub fn toggle_page(&mut self) {
        self.set_page(match self.page {
            Page::Event => Page::Campaign,
            Page::Campaign => Page::Event,
        });
    }

    /// The rows under the fields on the open page.
    #[must_use]
    pub fn actions(&self) -> [Action; 3] {
        match self.page {
            Page::Event => [Action::Page, Action::Name, Action::Save],
            Page::Campaign => [Action::Page, Action::Fill, Action::Save],
        }
    }

    /// The selected row of the campaign page.
    #[must_use]
    pub fn setting(&self) -> Setting {
        Setting::ALL[self.setting]
    }

    /// Selects the campaign page's row `delta` on.
    pub fn move_setting(&mut self, delta: i32) {
        let n = Setting::ALL.len() as i32;
        self.setting = (self.setting as i32 + delta).rem_euclid(n) as usize;
    }

    /// Selects `setting`.
    pub fn select_setting(&mut self, setting: Setting) {
        self.setting = Setting::ALL.iter().position(|&s| s == setting).unwrap_or(0);
    }

    /// Steps the selected setting `delta` values on, wrapping. The seed steps by one; a
    /// class bound that passes the other drags it along, so the range is never empty.
    pub fn cycle_setting(&mut self, delta: i32) {
        let classes = self.choices.classes.len().max(1) as i32;
        let wrap = |at: usize, n: i32| (at as i32 + delta).rem_euclid(n) as usize;
        match self.setting() {
            Setting::Difficulty => {
                self.difficulty = wrap(self.difficulty, DIFFICULTIES.len() as i32)
            }
            Setting::Progression => {
                self.progression = wrap(self.progression, PROGRESSIONS.len() as i32);
            }
            Setting::Seed => self.seed = self.seed.wrapping_add_signed(delta),
            Setting::ClassFrom => {
                self.bounds.0 = wrap(self.bounds.0, classes);
                self.bounds.1 = self.bounds.1.max(self.bounds.0);
            }
            Setting::ClassTo => {
                self.bounds.1 = wrap(self.bounds.1, classes);
                self.bounds.0 = self.bounds.0.min(self.bounds.1);
            }
        }
    }

    /// The name of `setting`.
    #[must_use]
    pub fn setting_name(&self, setting: Setting) -> String {
        let (id, english) = match setting {
            Setting::Difficulty => ("OAG_EDITOR_FIELD_DIFFICULTY", "DIFFICULTY"),
            Setting::Progression => ("OAG_EDITOR_FIELD_PROGRESSION", "PROGRESSION"),
            Setting::Seed => ("OAG_EDITOR_FIELD_SEED", "SEED"),
            Setting::ClassFrom => ("OAG_EDITOR_FIELD_CLASS_FROM", "CLASS FROM"),
            Setting::ClassTo => ("OAG_EDITOR_FIELD_CLASS_TO", "CLASS TO"),
        };
        self.say(id, english)
    }

    /// What `setting` is set to.
    #[must_use]
    pub fn setting_label(&self, setting: Setting) -> String {
        let class = |at: usize| {
            self.choices
                .classes
                .get(at)
                .map_or("-".to_string(), |pick| pick.label.clone())
        };
        match setting {
            Setting::Difficulty => {
                let (_, id, english) = DIFFICULTIES[self.difficulty];
                self.say(id, english)
            }
            Setting::Progression => {
                let (_, id, english) = PROGRESSIONS[self.progression];
                self.say(id, english)
            }
            Setting::Seed => self.seed.to_string(),
            Setting::ClassFrom => class(self.bounds.0),
            Setting::ClassTo => class(self.bounds.1),
        }
    }

    /// The seed [`Self::fill`] draws from.
    #[must_use]
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// Sets the seed.
    pub fn set_seed(&mut self, seed: u32) {
        self.seed = seed;
    }

    /// The slowest and fastest class a filled cell draws, as places on the title's ladder.
    #[must_use]
    pub fn class_bounds(&self) -> (usize, usize) {
        self.bounds
    }

    /// Sets the class bounds, as places on the title's ladder, in either order.
    pub fn set_class_bounds(&mut self, a: usize, b: usize) {
        let top = self.choices.classes.len().saturating_sub(1);
        self.bounds = (a.min(b).min(top), a.max(b).min(top));
    }

    /// Fills every slot the title draws from the seed, replacing what was placed. Each
    /// cell draws its mode, circuit and speed class (inside the bounds) in that order from
    /// `oag_core::Rng` seeded with [`Self::seed`], so the same seed and choices give the
    /// same grid whatever was placed before. A cell is an event like any other afterwards.
    pub fn fill(&mut self) {
        let mut rng = Rng::new(u64::from(self.seed));
        let top = self.choices.classes.len().saturating_sub(1);
        let (low, high) = (self.bounds.0.min(top), self.bounds.1.min(top));
        let below = |rng: &mut Rng, n: usize| rng.below(n.max(1) as u32) as usize;
        self.events.clear();
        for &slot in &self.choices.slots {
            let mode = below(&mut rng, MODES.len());
            let track = below(&mut rng, self.choices.tracks.len());
            let class = low + below(&mut rng, high - low + 1);
            self.events.insert(
                slot,
                Draft {
                    mode,
                    track,
                    class,
                    team: 0,
                    ai: 0,
                },
            );
        }
        self.editing = false;
    }

    fn default_draft(&self) -> Draft {
        let class = self
            .choices
            .classes
            .iter()
            .position(|pick| pick.value.eq_ignore_ascii_case("venom"))
            .unwrap_or(0);
        Draft {
            mode: 0,
            track: 0,
            class,
            team: 0,
            ai: 0,
        }
    }

    /// The campaign id the name makes: lowercase letters and digits joined by `-`,
    /// starting with a letter.
    #[must_use]
    pub fn id(&self) -> String {
        let mut id = String::new();
        for c in self.name.chars() {
            if c.is_ascii_alphanumeric() {
                id.push(c.to_ascii_lowercase());
            } else if !id.ends_with('-') && !id.is_empty() {
                id.push('-');
            }
        }
        let id = id.trim_end_matches('-').to_string();
        if id.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
            id
        } else {
            format!("campaign-{id}").trim_end_matches('-').to_string()
        }
    }

    /// The campaign as the format spells it: one tier `grid0`, every event open from
    /// the start (the editor sets no medals to earn them with except a finishing place
    /// on the modes that have one).
    ///
    /// # Errors
    ///
    /// No event is placed, or the name is empty.
    pub fn to_def(&self) -> Result<CampaignDef, String> {
        if self.name.trim().is_empty() {
            return Err("the campaign has no name".to_string());
        }
        if self.events.is_empty() {
            return Err("no event is placed".to_string());
        }
        let events = self
            .events
            .iter()
            .map(|(&slot, draft)| self.event(slot, draft))
            .collect();
        let info = Info {
            id: self.id(),
            name: self.name.trim().to_string(),
            description: String::new(),
            titles: vec![self.choices.title.clone()],
            reveal: Reveal::Locked,
            difficulty: Some(DIFFICULTIES[self.difficulty].0.to_string()),
            progression: match PROGRESSIONS[self.progression].0 {
                "ramp" => Progression::Ramp,
                _ => Progression::Flat,
            },
        };
        let grid = GridDef {
            id: "grid0".to_string(),
            name: String::new(),
            required_points: 0,
            unlock_grid: None,
            reveal: None,
            flyer: None,
            events,
        };
        Ok(CampaignDef::new(info, vec![grid]))
    }

    /// The medal targets of an event: the disc's own cell for the same mode, circuit and
    /// class; for a mode whose targets do not depend on the circuit (a finishing place, a
    /// zone or kill count), that of the same mode and class on any circuit. A timed mode
    /// with no cell of its own on the disc gets none: a time can only be authored per
    /// circuit, and one invented would be a guess.
    fn medals(&self, mode: &str, draft: &Draft) -> Option<MedalsDef> {
        let track = self
            .choices
            .tracks
            .get(draft.track)
            .map(|p| p.value.as_str());
        let class = if mode == "zone" {
            "zone".to_string()
        } else {
            self.choices
                .classes
                .get(draft.class)
                .map(|p| p.value.to_ascii_lowercase())?
        };
        let timed = matches!(mode, "time_trial" | "speed_lap");
        let found = self
            .choices
            .authored
            .iter()
            .find(|a| a.mode == mode && a.class == class && Some(a.track.as_str()) == track)
            .or_else(|| {
                self.choices
                    .authored
                    .iter()
                    .find(|a| !timed && a.mode == mode && a.class == class)
            });
        let [gold, silver, bronze] = match found {
            Some(a) => a.targets,
            None if matches!(mode, "race" | "head2head") => [1.0, 2.0, 3.0],
            None => return None,
        };
        // A disc cell may leave a tier at 0, which the format refuses as a target: that
        // tier is left out, and is never awarded.
        let tier = |target: f64| (target > 0.0).then_some(target);
        let medals = MedalsDef {
            gold: tier(gold),
            silver: tier(silver),
            bronze: tier(bronze),
            ..MedalsDef::default()
        };
        (!medals.all().is_empty()).then_some(medals)
    }

    fn event(&self, slot: (u32, u32), draft: &Draft) -> EventDef {
        let mode = MODES[draft.mode].0;
        let ours = AIS[draft.ai].0 == "ours";
        EventDef {
            slot: [slot.0, slot.1],
            id: None,
            name: String::new(),
            mode: mode.to_string(),
            track: self
                .choices
                .tracks
                .get(draft.track)
                .map(|p| p.value.clone()),
            tracks: Vec::new(),
            file: None,
            class: (mode != "zone")
                .then(|| {
                    self.choices
                        .classes
                        .get(draft.class)
                        .map(|p| p.value.clone())
                })
                .flatten(),
            laps: None,
            weapons: None,
            craft: draft
                .team
                .checked_sub(1)
                .and_then(|n| self.choices.teams.get(n))
                .map(|team| CraftDef {
                    team: team.value.clone(),
                }),
            field: ours.then(|| FieldDef {
                ai: Some("ours".to_string()),
                difficulty: None,
                skill: None,
                pilot: None,
                teams: Vec::new(),
            }),
            medals: self.medals(mode, draft),
            unlock: Some(UnlockDef {
                open: true,
                requires: Vec::new(),
            }),
        }
    }

    /// Writes the campaign as a fresh folder mod under `root` (the user mods
    /// directory), returning the folder.
    ///
    /// # Errors
    ///
    /// [`Self::to_def`]'s, or the file system's.
    pub fn save(&self, root: &std::path::Path) -> Result<std::path::PathBuf, String> {
        let def = self.to_def()?;
        oag_mod::campaign::write_mod(root, &def, &[self.choices.title.as_str()])
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests;
