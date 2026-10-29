//! A mod's campaign (`campaigns/<id>.toml`, ADR-0077) compiled into the grid
//! cells the title's own campaign screens already draw and launch.
//!
//! `oag_mod::campaign` reads the file's shape; this is where its names meet the
//! engine: a mode becomes an [`oag_tables::race_campaign::Mode`], a slot a
//! `gridN_x_y` cell name, a medal target the engine's own units, a pilot a
//! roster entry. What a cell has no field for (a preset craft's team, the AI,
//! the requirements) rides beside it as an [`Event`]. A name the engine does
//! not know is an error naming the mod, the campaign and the event.

use std::collections::BTreeMap;

use oag_mod::campaign::{CampaignDef, EventDef, MedalsDef, PilotDef, Reveal, Targets};
use oag_tables::race_campaign::{Cell, DifficultyTargets, Grid, Medal, MedalTargets, Mode};

/// The mode words a campaign file may use, and what each runs as.
pub const MODES: [(&str, Mode); 8] = [
    ("race", Mode::Race),
    ("time_trial", Mode::TimeTrial),
    ("speed_lap", Mode::SpeedLap),
    ("zone", Mode::Zone),
    ("elimination", Mode::Elimination),
    ("eliminator", Mode::Elimination),
    ("tournament", Mode::Tournament),
    ("head2head", Mode::Head2Head),
];

/// Whose driver flies an event's opponents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Ai {
    /// The recovered original driver (`oag_ai::original`), the game's default.
    #[default]
    Original,
    /// This project's driver (`oag_ai::Driver`).
    Ours,
}

/// What an event asks of its race beyond what a [`Cell`] carries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Event {
    /// The title a cross-disc track names (`wipeout-hd:03_Track`), by slug;
    /// `None` is the booted disc's own.
    pub track_title: Option<String>,
    /// A track file the mod carries, raced in place of a disc circuit.
    pub track_file: Option<TrackFile>,
    /// The player's preset team; `None` shows Team Selection.
    pub team: Option<String>,
    /// Whose driver flies.
    pub ai: Ai,
    /// The original driver's rung (`0` easy, `2` hard); `None` is the screen's.
    pub level: Option<usize>,
    /// Our driver's level.
    pub difficulty: Option<oag_ai::Difficulty>,
    /// Our driver's personality for every opponent.
    pub pilot: Option<Pilot>,
    /// The opponents' teams, grid slot 1 first.
    pub teams: Vec<String>,
    /// Weapons on or off; `None` is the mode's own.
    pub weapons: Option<bool>,
    /// Open from the start.
    pub open: bool,
    /// Each event that must have a medal first, by cell name, with the least
    /// medal it needs (`None`: any).
    pub requires: Vec<(String, Option<Medal>)>,
}

/// A track file of this project's own format (`<name>.track.toml` and the
/// `.glb` beside it) inside a folder mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackFile {
    /// The manifest, on disk.
    pub path: std::path::PathBuf,
    /// The manifest's `track.id`; its records and ghosts key as `foreign:<id>`.
    pub id: String,
    /// The manifest's `track.title`, what Cell Selection names the circuit.
    pub title: String,
}

/// Finds a mod's track file by its path in the mod.
pub type Files<'a> = dyn Fn(&str) -> Result<TrackFile, String> + 'a;

/// Reads `name`'s manifest in `module` far enough to name it: the race reads
/// and checks the whole file when it loads. A zip or `.oagmod` mod is refused:
/// the track reader takes a path on disk.
///
/// # Errors
///
/// Not a folder mod, no such file, or no `[track] id`.
pub fn mod_track_file(module: &oag_mod::Mod, name: &str) -> Result<TrackFile, String> {
    let path = module.local_path(name).ok_or_else(|| {
        format!("file {name:?}: a track file is raced from a folder mod, not a zip")
    })?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("file {name:?}: {e}"))?;
    let table: toml::Table = text.parse().map_err(|e| format!("file {name:?}: {e}"))?;
    let track = table.get("track").and_then(toml::Value::as_table);
    let field = |key: &str| {
        track
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    };
    let id = field("id").ok_or_else(|| format!("file {name:?}: no [track] id"))?;
    Ok(TrackFile {
        path,
        title: field("title").unwrap_or_else(|| id.clone()),
        id,
    })
}

/// The track file an event's `file` names: `<mod id>:<path>` in another
/// installed mod, which must be for `slug` as well, or a path in `module`, the
/// campaign's own. A mod that is not installed, or that the player turned off
/// (`off`, by id), refuses the event by name.
fn track_file_in(
    mods: &[&oag_mod::Mod],
    off: &[&str],
    module: &oag_mod::Mod,
    slug: &str,
    name: &str,
) -> Result<TrackFile, String> {
    let Some((mod_id, path)) = name.split_once(':') else {
        return mod_track_file(module, name);
    };
    if off.contains(&mod_id) {
        return Err(format!(
            "file {name:?}: mod {mod_id:?} is turned off on the MODS page"
        ));
    }
    let other = mods
        .iter()
        .find(|other| other.manifest.id == mod_id)
        .ok_or_else(|| format!("file {name:?}: no mod {mod_id:?} is installed"))?;
    if !other.is_for(slug) {
        return Err(format!("file {name:?}: mod {mod_id:?} is not for {slug}"));
    }
    mod_track_file(other, path).map_err(|e| format!("{e} (in mod {mod_id:?})"))
}

/// A pilot an event names: from the roster at launch, or already built.
#[derive(Debug, Clone, PartialEq)]
pub enum Pilot {
    /// A roster pilot by name, looked up when the event launches.
    Named(String),
    /// Written out in the campaign file, or one of the mod's own
    /// `pilots/<name>.toml`: built when the campaign compiles.
    Built(oag_raceplay::pilots::Entry),
}

/// A campaign, ready for the grid screens.
#[derive(Debug, Clone, PartialEq)]
pub struct Custom {
    /// The mod's id.
    pub mod_id: String,
    /// The campaign's id within the mod.
    pub id: String,
    /// The picker row.
    pub name: String,
    /// The picker row's second line.
    pub description: String,
    /// How each tier's unopened cells show, by grid index.
    pub reveals: Vec<Reveal>,
    /// The `title` its medals are kept under in `[[campaign]]`, so they never
    /// meet the original campaign's rows: `<title>/mod:<mod id>/<id>`.
    pub records_title: String,
    /// The tiers.
    pub grids: Vec<Grid>,
    /// Each cell's extras, by cell name.
    pub events: BTreeMap<String, Event>,
}

impl Custom {
    /// The key the picker row and the session name this campaign by.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}/{}", self.mod_id, self.id)
    }

    /// The slugs of the other titles its events race on, each once.
    pub fn other_discs<'a>(
        &'a self,
        title: &oag_title::Title,
    ) -> impl Iterator<Item = &'a str> + 'a {
        let own = title.slug();
        let mut seen = std::collections::BTreeSet::new();
        self.events
            .values()
            .filter_map(|event| event.track_title.as_deref())
            .filter(move |slug| *slug != own && seen.insert(*slug))
    }

    /// How grid `which`'s unopened cells show.
    #[must_use]
    pub fn reveal(&self, which: usize) -> Reveal {
        self.reveals.get(which).copied().unwrap_or_default()
    }

    /// The extras of the cell named `cell`.
    #[must_use]
    pub fn event(&self, cell: &str) -> Option<&Event> {
        self.events.get(cell)
    }

    /// Whether every requirement of `cell` holds against `medal_of`; a cell with
    /// none holds trivially.
    #[must_use]
    pub fn requirements_met(&self, cell: &str, medal_of: &dyn Fn(&str) -> Option<Medal>) -> bool {
        self.event(cell).is_none_or(|event| {
            event.requires.iter().all(|(name, least)| {
                medal_of(name).is_some_and(|medal| least.is_none_or(|least| medal <= least))
            })
        })
    }

    /// Whether the cell's lock is decided by its own `unlock` table rather than
    /// the title's neighbour rule: `Some(locked)` for an `open` or `requires`
    /// event, `None` for one that names neither.
    #[must_use]
    pub fn fixed_lock(&self, cell: &str, medal_of: &dyn Fn(&str) -> Option<Medal>) -> Option<bool> {
        let event = self.event(cell)?;
        if event.open {
            Some(false)
        } else if event.requires.is_empty() {
            None
        } else {
            Some(!self.requirements_met(cell, medal_of))
        }
    }
}

/// A mod's own pilots (`pilots/<name>.toml`), by name: what an event's
/// `pilot = "<name>"` finds before the player's roster.
pub type Pilots = BTreeMap<String, oag_raceplay::pilots::Entry>;

/// Reads a mod's `pilots/*.toml`; a broken one is returned as `(file, reason)`.
#[must_use]
pub fn mod_pilots(module: &oag_mod::Mod) -> (Pilots, Vec<(String, String)>) {
    let mut pilots = Pilots::new();
    let mut failed = Vec::new();
    for file in module.files_under(PILOTS, "toml") {
        let stem = file[PILOTS.len() + 1..file.len() - ".toml".len()].to_string();
        let parsed = match module.read_text(&file) {
            Ok(Some(text)) => {
                oag_raceplay::pilots::parse(&stem, &text).map_err(|e| format!("{e:#}"))
            }
            Ok(None) => Err("listed but not readable".to_string()),
            Err(e) => Err(e.to_string()),
        };
        match parsed {
            Ok(entry) => {
                pilots.insert(stem, entry);
            }
            Err(reason) => failed.push((file, reason)),
        }
    }
    (pilots, failed)
}

/// The folder a mod keeps its pilots in.
pub const PILOTS: &str = "pilots";

/// What a mod campaign event hands its race beyond the cell: drained into the
/// race stage beside the cell, as `Session::campaign_cell` is.
#[derive(Debug, Clone, PartialEq)]
pub struct Launch {
    /// See [`Custom::records_title`].
    pub records_title: String,
    /// Whose driver flies.
    pub ai: Ai,
    /// The original driver's rung, in place of the screen's.
    pub level: Option<usize>,
    /// Every opponent's pilot, already resolved.
    pub pilot: Option<oag_raceplay::pilots::Entry>,
}

/// RACE CAMPAIGN as a page picking between the disc's campaign and each of
/// `installed`'s (ADR-0077), and the MODS page's ADDS line naming each. One left
/// out is offered nowhere, named in the log and in its mod's MODS entry
/// ([`crate::mods::Listing::problems`]). With none the tree is untouched.
pub fn add_picker(
    definition: &mut oag_ui::menu::Definition,
    listing: &mut [crate::mods::Listing],
    installed: &Installed,
    strings: &oag_ui::language::StringTable,
) {
    for (mod_id, reason) in &installed.failed {
        log::error!("mod {mod_id}: campaign left out: {reason}");
    }
    crate::mods::note_left_out(listing, installed, strings);
    for entry in listing.iter_mut() {
        for custom in installed.campaigns.iter().filter(|c| c.mod_id == entry.id) {
            entry.rows.push(format!("campaign {}", custom.name));
        }
    }
    let rows: Vec<(String, String, String)> = installed
        .campaigns
        .iter()
        .map(|custom| {
            (
                custom.key(),
                custom.name.clone(),
                custom.description.clone(),
            )
        })
        .collect();
    let (id, english) = crate::mods::words::CAMPAIGN_PICKER;
    definition.add_campaign_picker(strings.get(id).unwrap_or(english), &rows);
}

/// The records `title` a mod campaign's medals are kept under.
#[must_use]
pub fn records_title(title: &oag_title::Title, mod_id: &str, id: &str) -> String {
    format!("{}/mod:{mod_id}/{id}", title.slug())
}

/// Compiles `def` for `title`.
///
/// # Errors
///
/// One line naming the mod, the campaign and the event, for the first thing
/// the engine cannot run.
pub fn compile(
    def: &CampaignDef,
    mod_id: &str,
    title: &'static oag_title::Title,
    pilots: &Pilots,
    files: &Files<'_>,
) -> Result<Custom, String> {
    let info = &def.campaign;
    let prefix = format!("mod {mod_id} campaign {}", info.id);
    let mut names = BTreeMap::new();
    for (index, grid) in def.grids.iter().enumerate() {
        for event in &grid.events {
            names.insert(event.reference(grid), cell_name(index, event));
        }
    }
    let ladder = title
        .race
        .speed_classes
        .map_or(&[][..], |classes| classes.names);
    let mut grids = Vec::with_capacity(def.grids.len());
    let mut events = BTreeMap::new();
    for (index, grid_def) in def.grids.iter().enumerate() {
        let mut cells = Vec::with_capacity(grid_def.events.len());
        for event_def in &grid_def.events {
            let at = format!("{prefix} grid {:?} event {:?}", grid_def.id, event_def.slot);
            let (cell, event) = compile_event(index, event_def, &names, ladder, pilots, files)
                .map_err(|problem| format!("{at}: {problem}"))?;
            events.insert(cell.name.clone(), event);
            cells.push(cell);
        }
        let unlock_grid = grid_def.unlock_grid.as_ref().map(|target| {
            let at = def
                .grids
                .iter()
                .position(|g| &g.id == target)
                .unwrap_or_default();
            format!("grid{at}")
        });
        grids.push(Grid {
            name: format!("grid{index}"),
            required_points: grid_def.required_points,
            locked: unlock_grid.is_some(),
            group: None,
            unlock_grid,
            cells,
            campaign: None,
            title_color: None,
            text_color: None,
            flyer_name: grid_def.flyer.clone(),
            billboard_name: None,
        });
    }
    // A tier with no open event would wait on a medal it can never earn: its
    // first event without requirements opens it, chosen.
    for grid in &mut grids {
        let any_open = grid
            .cells
            .iter()
            .any(|cell| events.get(&cell.name).is_some_and(|e: &Event| e.open));
        if any_open {
            continue;
        }
        if let Some(cell) = grid.cells.iter_mut().find(|cell| {
            events
                .get(&cell.name)
                .is_some_and(|e: &Event| e.requires.is_empty())
        }) && let Some(event) = events.get_mut(&cell.name)
        {
            event.open = true;
            cell.locked = Some(false);
        }
    }
    Ok(Custom {
        mod_id: mod_id.to_string(),
        id: info.id.clone(),
        name: info.name.clone(),
        description: info.description.clone(),
        reveals: def
            .grids
            .iter()
            .map(|grid| grid.reveal.unwrap_or(info.reveal))
            .collect(),
        records_title: records_title(title, mod_id, &info.id),
        grids,
        events,
    })
}

/// The laps Pulse's own cells author for `mode` at `class`: 7 on a speed lap,
/// 3 Venom, 4 Flash or Rapier, 5 Phantom on a lap race; none on Zone or
/// Elimination, or a class outside the four.
fn default_laps(mode: &Mode, class: &str) -> Option<u32> {
    use oag_tables::handling::SpeedClass;
    match mode {
        Mode::SpeedLap => Some(7),
        Mode::Race | Mode::Tournament | Mode::TimeTrial | Mode::Head2Head => {
            match SpeedClass::census(class)? {
                SpeedClass::VENOM => Some(3),
                SpeedClass::FLASH | SpeedClass::RAPIER => Some(4),
                SpeedClass::PHANTOM => Some(5),
                _ => None,
            }
        }
        _ => None,
    }
}

/// `VENOM` as a disc cell writes it, `Venom`.
fn title_case(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
        .unwrap_or_default()
}

fn cell_name(grid: usize, event: &EventDef) -> String {
    format!("grid{grid}_{}_{}", event.slot[0], event.slot[1])
}

fn mode_named(word: &str) -> Result<Mode, String> {
    MODES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(word))
        .map(|(_, mode)| mode.clone())
        .ok_or_else(|| {
            let known: Vec<&str> = MODES.iter().map(|(name, _)| *name).collect();
            format!("mode {word:?} is not one of {}", known.join(", "))
        })
}

/// A track reference: `id` on the booted disc, `<title>:<id>` on another.
fn track_ref(reference: &str) -> (Option<String>, String) {
    match reference.split_once(':') {
        Some((title, id)) => (
            Some(title.trim().to_ascii_lowercase()),
            id.trim().to_string(),
        ),
        None => (None, reference.trim().to_string()),
    }
}

fn compile_event(
    grid: usize,
    def: &EventDef,
    names: &BTreeMap<String, String>,
    ladder: &[&str],
    pilots: &Pilots,
    files: &Files<'_>,
) -> Result<(Cell, Event), String> {
    let mode = mode_named(&def.mode)?;
    let class = match (&mode, &def.class) {
        (Mode::Zone, None) => "Zone".to_string(),
        (_, None) => return Err("no class".to_string()),
        (_, Some(class)) => {
            // The spelling a disc cell uses (`Venom`), which the screens show;
            // a rung past the four (Pure's `Vector`) as the title's ladder has it.
            oag_tables::handling::SpeedClass::census(class)
                .map(|known| title_case(known.as_str()))
                .or_else(|| {
                    ladder
                        .iter()
                        .find(|name| name.eq_ignore_ascii_case(class))
                        .map(|name| (*name).to_string())
                })
                .ok_or_else(|| format!("class {class:?} is not a speed class of this title"))?
        }
    };
    let mut cell = Cell::authored(cell_name(grid, def), mode.clone(), class);
    let mut event = Event {
        weapons: def.weapons,
        ..Event::default()
    };

    let tournament = mode == Mode::Tournament;
    let mut track_titles = Vec::new();
    if let Some(file) = &def.file {
        if tournament || def.track.is_some() || !def.tracks.is_empty() {
            return Err(
                "`file` replaces `track`, and a tournament's legs are disc circuits".to_string(),
            );
        }
        let track_file = files(file)?;
        track_titles.push(None);
        cell.track = Some(track_file.title.clone());
        event.track_file = Some(track_file);
    } else if tournament {
        if def.tracks.is_empty() || def.track.is_some() {
            return Err("a tournament names its legs in `tracks`, and no `track`".to_string());
        }
        for leg in &def.tracks {
            let (title, id) = track_ref(leg);
            track_titles.push(title);
            cell.tournament_tracks.push(id);
        }
    } else {
        if !def.tracks.is_empty() {
            return Err("only a tournament names `tracks`".to_string());
        }
        let Some(track) = &def.track else {
            return Err("no track".to_string());
        };
        let (title, id) = track_ref(track);
        track_titles.push(title);
        cell.track = Some(id);
    }
    if track_titles.iter().any(|t| *t != track_titles[0]) {
        return Err("a tournament's legs are all on one disc".to_string());
    }
    event.track_title = track_titles[0].clone();
    // Absent: what every one of Pulse's 236 authored cells gives its mode and
    // class (`Cell::laps`'s census), so the screen never shows a lap mode as
    // endless. Chosen for a mod cell, measured on the disc's.
    cell.laps = def.laps.or_else(|| default_laps(&mode, &cell.class));
    if mode == Mode::Head2Head {
        cell.ai_count = Some(1);
    }

    if let Some(craft) = &def.craft {
        if craft.team.trim().is_empty() {
            return Err("craft team is empty".to_string());
        }
        cell.ship = Some(craft.team.clone());
        cell.ship_choice = Some(false);
        event.team = Some(craft.team.clone());
    } else {
        cell.ship_choice = Some(true);
    }

    if let Some(field) = &def.field {
        event.ai = match field.ai.as_deref() {
            None | Some("original") => Ai::Original,
            Some("ours") => Ai::Ours,
            Some(other) => return Err(format!("ai {other:?} is not `original` or `ours`")),
        };
        if let Some(word) = &field.difficulty {
            match event.ai {
                Ai::Original => {
                    event.level = Some(match word.as_str() {
                        "easy" => 0,
                        "medium" => 1,
                        "hard" => 2,
                        other => {
                            return Err(format!(
                                "difficulty {other:?}: the original driver's rungs are easy, medium, hard"
                            ));
                        }
                    });
                }
                Ai::Ours => {
                    event.difficulty = Some(oag_ai::Difficulty::from_name(word).ok_or_else(
                        || format!("difficulty {word:?}: our driver's levels are novice, skilled, elite, ace"),
                    )?);
                }
            }
        }
        if let Some(skill) = field.skill {
            if event.ai == Ai::Ours {
                return Err("skill is the original driver's; ours takes difficulty".to_string());
            }
            cell.skill = Some(skill);
        }
        if let Some(pilot) = &field.pilot {
            if event.ai != Ai::Ours {
                return Err("a pilot is our driver's: set ai = \"ours\"".to_string());
            }
            event.pilot = Some(match pilot {
                PilotDef::Named(name) => pilots.get(name).map_or_else(
                    || Pilot::Named(name.clone()),
                    |entry| Pilot::Built(entry.clone()),
                ),
                PilotDef::Inline(table) => Pilot::Built(inline_pilot(&cell.name, table)?),
            });
        }
        event.teams.clone_from(&field.teams);
    }

    let flipped = matches!(mode, Mode::Zone | Mode::Elimination);
    let timed = matches!(mode, Mode::TimeTrial | Mode::SpeedLap);
    if let Some(medals) = &def.medals {
        let (medium, rungs) = medal_targets(medals, flipped, timed)?;
        cell.gold = medium.gold;
        cell.silver = medium.silver;
        cell.bronze = medium.bronze;
        cell.difficulty_targets = rungs;
    } else {
        let never = if flipped { i64::MAX } else { 0 };
        (cell.gold, cell.silver, cell.bronze) = (never, never, never);
    }

    if let Some(unlock) = &def.unlock {
        event.open = unlock.open;
        if unlock.open && !unlock.requires.is_empty() {
            return Err("unlock is `open` or `requires`, not both".to_string());
        }
        for requirement in &unlock.requires {
            let name = names
                .get(&requirement.event)
                .ok_or_else(|| format!("requires {:?}: no such event", requirement.event))?;
            let least = match requirement.medal.as_str() {
                "gold" => Some(Medal::Gold),
                "silver" => Some(Medal::Silver),
                "bronze" => Some(Medal::Bronze),
                _ => None,
            };
            event.requires.push((name.clone(), least));
        }
    }
    cell.locked = if event.open {
        Some(false)
    } else if event.requires.is_empty() {
        None
    } else {
        Some(true)
    };
    Ok((cell, event))
}

fn inline_pilot(cell: &str, table: &toml::Table) -> Result<oag_raceplay::pilots::Entry, String> {
    let mut table = table.clone();
    table.insert(
        "toml-schema".to_string(),
        toml::Value::Table(toml::toml! {
            location = "https://oag.topaxi.com/mods/schemas/v1/pilot.tosd"
            version = "1.0.0"
        }),
    );
    let text = toml::to_string(&table).map_err(|e| format!("pilot: {e}"))?;
    oag_raceplay::pilots::parse(cell, &text).map_err(|e| format!("{e:#}"))
}

/// The medium rung (the flat targets) and, when any rung is named, all three.
/// A tier left out is never awarded: `0` on a lower-is-better mode, which no
/// positive result meets, and `i64::MAX` on a counting one.
fn medal_targets(
    medals: &MedalsDef,
    flipped: bool,
    timed: bool,
) -> Result<(MedalTargets, Option<DifficultyTargets>), String> {
    let never = if flipped { i64::MAX } else { 0 };
    let convert = |value: Option<f64>| -> Result<i64, String> {
        let Some(value) = value else {
            return Ok(never);
        };
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("medal target {value} is not a positive number"));
        }
        let scaled = if timed { value * 100.0 } else { value };
        #[allow(
            clippy::cast_possible_truncation,
            reason = "checked finite and positive"
        )]
        Ok(scaled.round() as i64)
    };
    let triple = |targets: Targets| -> Result<MedalTargets, String> {
        Ok(MedalTargets {
            gold: convert(targets.gold)?,
            silver: convert(targets.silver)?,
            bronze: convert(targets.bronze)?,
        })
    };
    let flat = medals.all();
    let rung = |own: Option<Targets>| own.filter(|t| !t.is_empty()).unwrap_or(flat);
    let medium = triple(rung(medals.medium))?;
    let per_rung = medals.easy.is_some() || medals.medium.is_some() || medals.hard.is_some();
    let rungs = if per_rung {
        Some(DifficultyTargets {
            easy: triple(rung(medals.easy))?,
            medium,
            hard: triple(rung(medals.hard))?,
        })
    } else {
        None
    };
    Ok((medium, rungs))
}

/// What a session's mods add for one title: the campaigns that compiled, a
/// line for each that did not, and the track files they carry.
#[derive(Debug, Clone, Default)]
pub struct Installed {
    /// In mod order, then file order.
    pub campaigns: Vec<Custom>,
    /// `<mod id> <file>: <reason>` for each campaign left out.
    pub failed: Vec<(String, String)>,
    /// For a campaign left out because an event races another mod's track: the
    /// campaign's mod id, the other mod's id, and whether that mod is turned off
    /// (`true`) rather than not installed. What the MODS page's NEEDS line says.
    pub needs: Vec<(String, String, bool)>,
    /// Every `tracks/*.track.toml` of a folder mod for this title, raced from
    /// RACE REMIX's CUSTOM title as well as from a campaign.
    pub tracks: Vec<std::path::PathBuf>,
}

impl Installed {
    /// Reads and compiles every campaign the mods carry for `title`: a mod or
    /// campaign whose `titles` leave this title out is skipped quietly, a broken
    /// one, or one with an event on a disc `has_disc` (by title slug) says this
    /// machine cannot open, is named in [`Self::failed`] and offered nowhere. So is
    /// one racing a track of a mod in `off` (turned off by the player:
    /// [`crate::mods::Mods::disabled`]), whose reason says so rather than "not
    /// installed". `mods` is the mods that are on ([`crate::mods::Mods::player`]).
    /// Empty on a title whose campaign does not draw the hex grid
    /// ([`oag_title::Campaign::custom_campaigns`]).
    #[must_use]
    pub fn for_title<'a>(
        mods: impl IntoIterator<Item = &'a oag_mod::Mod>,
        off: &[&str],
        title: &'static oag_title::Title,
        has_disc: &dyn Fn(&str) -> bool,
    ) -> Self {
        let mut installed = Self::default();
        let slug = title.slug();
        let mods: Vec<&oag_mod::Mod> = mods.into_iter().collect();
        // Tracks race on every title (RACE REMIX's CUSTOM); campaigns only
        // where the hex grid draws.
        for module in mods.iter().filter(|module| module.is_for(&slug)) {
            installed.tracks.extend(
                module
                    .files_under("tracks", "toml")
                    .iter()
                    .filter(|name| name.ends_with(".track.toml"))
                    .filter_map(|name| module.local_path(name)),
            );
        }
        if !title.campaign.custom_campaigns {
            return installed;
        }
        for &module in &mods {
            if !module.is_for(&slug) {
                continue;
            }
            let (pilots, broken) = mod_pilots(module);
            for (file, reason) in broken {
                installed
                    .failed
                    .push((module.manifest.id.clone(), format!("{file}: {reason}")));
            }
            for found in module.campaigns() {
                let needed = std::cell::RefCell::new(None);
                let compiled = found.def.and_then(|def| {
                    let titles = &def.campaign.titles;
                    if !titles.is_empty() && !titles.iter().any(|t| t.eq_ignore_ascii_case(&slug)) {
                        return Ok(None);
                    }
                    let custom = compile(&def, &found.mod_id, title, &pilots, &|name| {
                        let found = track_file_in(&mods, off, module, &slug, name);
                        if found.is_err()
                            && let Some((other, _)) = name.split_once(':')
                            && (off.contains(&other)
                                || !mods.iter().any(|m| m.manifest.id == other))
                        {
                            *needed.borrow_mut() = Some((other.to_string(), off.contains(&other)));
                        }
                        found
                    })?;
                    if let Some(missing) = custom.other_discs(title).find(|slug| !has_disc(slug)) {
                        return Err(format!(
                            "{} names a circuit on {missing}, and no disc of it is here",
                            custom.name
                        ));
                    }
                    Ok(Some(custom))
                });
                match compiled {
                    Ok(Some(custom)) => installed.campaigns.push(custom),
                    Ok(None) => {}
                    Err(reason) => {
                        if let Some((other, is_off)) = needed.into_inner() {
                            installed.needs.push((found.mod_id.clone(), other, is_off));
                        }
                        installed
                            .failed
                            .push((found.mod_id.clone(), format!("{}: {reason}", found.file)));
                    }
                }
            }
        }
        installed
    }

    /// The campaign a picker row names by [`Custom::key`].
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Custom> {
        self.campaigns.iter().find(|custom| custom.key() == key)
    }
}

#[cfg(test)]
mod tests;
