//! Wipeout Pure's campaign: the events its `Definition.xml` and its per-track
//! `stats.xml` author, as the same [`Cell`]s a Pulse grid is made of.
//!
//! Pure has no `grid_NN.xml`. Its campaign is a **list**: the front end walks
//! `Class Selection` to `League Selection` or `Tournament Selection` to
//! `Track Selection` to `Team Selection` (`docs/formats/race-setup.md`), and
//! what the disc authors for it is two files.
//!
//! - `Data\Plugins\PI001\Definition.xml`: every `PI_Track` (type `Race`,
//!   `Classic` or `Zone`), `PI_League`, `PI_Tournament` and `PI_Team`, each
//!   with the `<Unlock>` rows that gate it. [`parse`].
//! - a circuit's `<location>\stats.xml` in `FEData.wad`: the time-trial gold time
//!   per class, the silver and bronze offsets, and the Zone counts. [`Stats`].
//!
//! Neither names a medal law; that is the executable's. `Medal_Evaluate`
//! (`0x088081d8`, `docs/ghidra/functions/psp-pure-usa/campaign.md`) switches on
//! the race's mode: a finishing place under 3 is gold, silver or bronze (a
//! single race, and a tournament's final rank); a time is compared against
//! the class's gold time and that time plus the silver and bronze offsets
//! (centiseconds); a Zone count is compared against the three `ZoneMedals`
//! figures, highest first. So an event is a [`Cell`] whose three targets are
//! those numbers, and [`Cell::evaluate_medal`] is the law.
//!
//! # Unlocks
//!
//! Every shipped `<Unlock>` row names one of three conditions, and a row passes
//! when every condition it names does ([`Unlock::met`]):
//!
//! | Row | Passes when |
//! | --- | --- |
//! | `MedalCount="n"` | the profile holds at least `n` medals ([`Progress::medal_count`]) |
//! | `Track="Zone 1" medal="bronze"` | the Zone event on that circuit holds a medal at least that good |
//! | `Tournament="Alpha" class="asSelected" medal="Any"` | the tournament, at the class being played, holds any medal |
//!
//! A row naming anything else is an [`Error`], so a different schema fails
//! loudly rather than reading as "unlocked".

use std::collections::BTreeMap;

use crate::fexml::{self, Node};
use crate::pure_ai_stats::PureClass;
use crate::race_campaign::{Cell, Medal, Mode};

/// Why a file did not read as a Pure campaign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The text was not front-end XML.
    Xml(String),
    /// A required element or attribute was missing, or a number did not parse.
    Missing(String),
    /// An `<Unlock>` attribute or value this reader has no branch for.
    Unmodelled(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xml(why) => write!(f, "not front-end XML: {why}"),
            Self::Missing(what) => write!(f, "missing or unreadable: {what}"),
            Self::Unmodelled(what) => write!(f, "an <Unlock> this reader does not model: {what}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<fexml::Error> for Error {
    fn from(error: fexml::Error) -> Self {
        Self::Xml(error.to_string())
    }
}

/// A `Result` with this module's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// What a `PI_Track` is for, from its `type` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    /// `Race`: the eight circuits of the two leagues and the Championship.
    Race,
    /// `Classic`: the four Wipeout 1 and 2097 circuits.
    Classic,
    /// `Zone`: the four Zone-mode circuits.
    Zone,
}

/// Which class an `<Unlock>` row is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassRef {
    /// `class="asSelected"`: whichever class the player is playing.
    AsSelected,
    /// A class by name.
    Named(String),
}

/// The medal an `<Unlock>` row asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MedalWanted {
    /// `medal="Any"`: any tier.
    Any,
    /// A tier or better: gold is the best, so `Bronze` passes on all three.
    AtLeast(Medal),
}

impl MedalWanted {
    /// Whether `held` satisfies this.
    #[must_use]
    pub fn satisfied_by(self, held: Option<Medal>) -> bool {
        match (self, held) {
            (_, None) => false,
            (Self::Any, Some(_)) => true,
            (Self::AtLeast(wanted), Some(held)) => held <= wanted,
        }
    }
}

/// One `<Unlock>` row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Unlock {
    /// `MedalCount`.
    pub medal_count: Option<u32>,
    /// `Track`, the circuit's `PI_Track` name.
    pub track: Option<String>,
    /// `Tournament`, a `PI_Tournament` name.
    pub tournament: Option<String>,
    /// `class`.
    pub class: Option<ClassRef>,
    /// `medal`.
    pub medal: Option<MedalWanted>,
}

/// What a profile holds, as an [`Unlock`] reads it.
#[expect(
    missing_debug_implementations,
    reason = "holds a lookup closure, which has no Debug"
)]
pub struct Progress<'a> {
    /// The class being played, for `class="asSelected"`.
    pub class: &'a str,
    /// The profile's total medal count.
    pub medal_count: u32,
    /// The best medal a named event holds ([`Event::name`]).
    pub medal_of: &'a dyn Fn(&str) -> Option<Medal>,
}

impl Unlock {
    /// Whether every condition this row names holds.
    #[must_use]
    pub fn met(&self, progress: &Progress<'_>) -> bool {
        if let Some(needed) = self.medal_count
            && progress.medal_count < needed
        {
            return false;
        }
        let wanted = self.medal.unwrap_or(MedalWanted::Any);
        if let Some(track) = &self.track
            && !wanted.satisfied_by((progress.medal_of)(&zone_event_name(track)))
        {
            return false;
        }
        if let Some(tournament) = &self.tournament {
            let class = match &self.class {
                Some(ClassRef::Named(name)) => name.as_str(),
                Some(ClassRef::AsSelected) | None => progress.class,
            };
            if !wanted.satisfied_by((progress.medal_of)(&tournament_event_name(
                tournament, class,
            ))) {
                return false;
            }
        }
        true
    }
}

/// Whether every row passes. Pure authors no `Exclusive` row, so the rows are
/// ANDed; no rows means open.
#[must_use]
pub fn all_met(rows: &[Unlock], progress: &Progress<'_>) -> bool {
    rows.iter().all(|row| row.met(progress))
}

/// One `PI_Track`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// The `name` attribute, which is also the id a race setting stores.
    pub name: String,
    /// `type`.
    pub kind: TrackKind,
    /// `location`, the circuit's directory.
    pub location: String,
    /// Its `<Unlock>` rows.
    pub unlocks: Vec<Unlock>,
}

/// A `PI_League` or `PI_Tournament`: an ordered list of circuits and a gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    /// The `name` attribute.
    pub name: String,
    /// `<Entry track="...">`, in order: `PI_Track` names.
    pub tracks: Vec<String>,
    /// Its `<Unlock>` rows.
    pub unlocks: Vec<Unlock>,
}

/// A `PI_Team` that carries a gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    /// The `name` attribute.
    pub name: String,
    /// `location`, the ship's directory: the leaf is the id a race stores.
    pub location: String,
    /// Its `<Unlock>` rows; empty for an open team.
    pub unlocks: Vec<Unlock>,
}

/// What `Definition.xml` authors of the campaign.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Definition {
    /// Every `PI_Track`, in file order.
    pub tracks: Vec<Track>,
    /// Every `PI_League`: a league is what `League Selection` lists.
    pub leagues: Vec<Series>,
    /// Every `PI_Tournament`: what `Tournament Selection` lists.
    pub tournaments: Vec<Series>,
    /// Every `PI_Team`.
    pub teams: Vec<Team>,
}

fn attr<'a>(node: &'a Node, name: &str) -> Result<&'a str> {
    node.attr(name)
        .ok_or_else(|| Error::Missing(format!("<{}> has no {name}", node.name)))
}

fn read_unlock(node: &Node) -> Result<Unlock> {
    let mut out = Unlock::default();
    for (key, value) in &node.attrs {
        match key.to_ascii_lowercase().as_str() {
            "medalcount" => {
                out.medal_count = Some(
                    value
                        .trim()
                        .parse()
                        .map_err(|_| Error::Missing(format!("MedalCount {value:?}")))?,
                );
            }
            "track" => out.track = Some(value.clone()),
            "tournament" => out.tournament = Some(value.clone()),
            "class" => {
                out.class = Some(if value.eq_ignore_ascii_case("asSelected") {
                    ClassRef::AsSelected
                } else {
                    ClassRef::Named(value.clone())
                });
            }
            "medal" => {
                out.medal = Some(match value.to_ascii_lowercase().as_str() {
                    "any" => MedalWanted::Any,
                    "gold" => MedalWanted::AtLeast(Medal::Gold),
                    "silver" => MedalWanted::AtLeast(Medal::Silver),
                    "bronze" => MedalWanted::AtLeast(Medal::Bronze),
                    other => return Err(Error::Unmodelled(format!("medal={other:?}"))),
                });
            }
            other => return Err(Error::Unmodelled(format!("attribute {other:?}"))),
        }
    }
    Ok(out)
}

fn unlocks_of(node: &Node) -> Result<Vec<Unlock>> {
    node.children_named("Unlock").map(read_unlock).collect()
}

fn read_series(node: &Node) -> Result<Series> {
    Ok(Series {
        name: attr(node, "name")?.to_string(),
        tracks: node
            .children_named("Entry")
            .map(|entry| attr(entry, "track").map(str::to_string))
            .collect::<Result<_>>()?,
        unlocks: unlocks_of(node)?,
    })
}

/// Reads `Data\Plugins\PI001\Definition.xml`.
///
/// # Errors
///
/// A track without a location or a recognised `type`, a row without a name, or
/// an `<Unlock>` this module does not model ([`Error::Unmodelled`]).
pub fn parse(text: &str) -> Result<Definition> {
    let root = fexml::parse(text);
    let mut out = Definition::default();
    let top = root.children.first().unwrap_or(&root);
    for node in &top.children {
        match node.name.as_str() {
            "PI_Track" => {
                let values = node
                    .children_named("Values")
                    .next()
                    .ok_or_else(|| Error::Missing("a PI_Track without Values".into()))?;
                let kind = match values.attr("type").unwrap_or("Race") {
                    "Race" => TrackKind::Race,
                    "Classic" => TrackKind::Classic,
                    "Zone" => TrackKind::Zone,
                    other => return Err(Error::Unmodelled(format!("track type {other:?}"))),
                };
                out.tracks.push(Track {
                    name: attr(node, "name")?.to_string(),
                    kind,
                    location: attr(values, "location")?.to_string(),
                    unlocks: unlocks_of(node)?,
                });
            }
            "PI_League" => out.leagues.push(read_series(node)?),
            "PI_Tournament" => out.tournaments.push(read_series(node)?),
            "PI_Team" => out.teams.push(Team {
                name: attr(node, "name")?.to_string(),
                location: node
                    .children_named("Values")
                    .next()
                    .and_then(|values| values.attr("location"))
                    .unwrap_or_default()
                    .to_string(),
                unlocks: unlocks_of(node)?,
            }),
            _ => {}
        }
    }
    for track in &out.tracks {
        for row in &track.unlocks {
            if let Some(named) = &row.track
                && !out
                    .tracks
                    .iter()
                    .any(|t| &t.name == named && t.kind == TrackKind::Zone)
            {
                return Err(Error::Unmodelled(format!(
                    "Track={named:?} on {:?}: only a Zone circuit is modelled",
                    track.name
                )));
            }
        }
    }
    Ok(out)
}

/// The medal figures of one circuit's `stats.xml`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    /// `<RaceTimes>`: the gold time in seconds, per class in
    /// [`PureClass::ALL`] order.
    pub gold_seconds: [f32; 5],
    /// `Silver`: seconds added to the gold time for silver.
    pub silver_seconds: f32,
    /// `Bronze`: seconds added to the gold time for bronze.
    pub bronze_seconds: f32,
    /// `<ZoneMedals>`: the zone counts for gold, silver and bronze.
    pub zone: [i64; 3],
}

fn number(node: &Node, name: &str) -> Result<f32> {
    attr(node, name)?
        .trim()
        .parse()
        .map_err(|_| Error::Missing(format!("<{}> {name} is not a number", node.name)))
}

/// Reads a circuit's `stats.xml` for what the campaign's medals need; the rest
/// of the file is `oag_tables::pure_ai_stats`'s.
///
/// # Errors
///
/// A missing `<RaceTimes>` or `<ZoneMedals>`, or a figure that is not a number.
pub fn parse_stats(text: &str) -> Result<Stats> {
    let root = fexml::parse(text);
    let stats = root
        .children_named("Stats")
        .next()
        .ok_or_else(|| Error::Missing("<Stats>".into()))?;
    let times = stats
        .children_named("RaceTimes")
        .next()
        .ok_or_else(|| Error::Missing("<RaceTimes>".into()))?;
    let zone = stats
        .children_named("ZoneMedals")
        .next()
        .ok_or_else(|| Error::Missing("<ZoneMedals>".into()))?;
    let mut gold_seconds = [0.0; 5];
    for (slot, class) in gold_seconds.iter_mut().zip(PureClass::ALL) {
        *slot = number(times, class.title())?;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "authored whole numbers of zones"
    )]
    let count = |name: &str| number(zone, name).map(|n| n as i64);
    Ok(Stats {
        gold_seconds,
        silver_seconds: number(times, "Silver")?,
        bronze_seconds: number(times, "Bronze")?,
        zone: [count("Gold")?, count("Silver")?, count("Bronze")?],
    })
}

/// A circuit's medal figures, relative to its `location`.
pub const STATS_FILE: &str = "stats.xml";

/// What an event is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A tournament: every circuit in turn, ranked on points.
    Tournament,
    /// One race on a league's circuit.
    Race,
    /// One circuit against the class's gold time.
    TimeTrial,
    /// A Zone circuit.
    Zone,
}

/// One entry of the campaign list.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// What it is.
    pub kind: EventKind,
    /// The stable name its medal is stored under.
    pub name: String,
    /// The tournament or circuit it is about (a `PI_Tournament` or `PI_Track`
    /// name), for the list's label.
    pub subject: String,
    /// The class it is played in; `None` for Zone, which has none.
    pub class: Option<PureClass>,
    /// What it asks the race for, and the medal law.
    pub cell: Cell,
    /// Every `<Unlock>` row that gates it: its own, and its league's.
    pub unlocks: Vec<Unlock>,
}

/// The name a Zone event's medal is stored under.
#[must_use]
pub fn zone_event_name(track: &str) -> String {
    format!("zone/{track}")
}

/// The name a tournament event's medal is stored under.
#[must_use]
pub fn tournament_event_name(tournament: &str, class: &str) -> String {
    format!("tournament/{tournament}/{class}")
}

fn blank_cell(name: String, mode: Mode, class: &str) -> Cell {
    Cell {
        name,
        track: None,
        mode,
        class: class.to_string(),
        weapons: true,
        damage: true,
        locked: None,
        status: None,
        ai_count: None,
        skill: None,
        skill_easy: None,
        skill_hard: None,
        laps: None,
        ship: None,
        ship_choice: None,
        gold: 1,
        silver: 2,
        bronze: 3,
        tournament_tracks: Vec::new(),
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

/// Seconds to the centiseconds a time trial's value is in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "authored whole seconds, well inside i64"
)]
fn centiseconds(seconds: f32) -> i64 {
    (seconds * 100.0).round() as i64
}

/// The campaign list for one class, in the order the front end offers the
/// modes: tournaments, single races, time trials, then Zone.
///
/// `stats` answers a circuit's `stats.xml` figures by `PI_Track` name; a
/// circuit without them gets no time-trial or Zone event rather than an
/// invented target.
#[must_use]
pub fn events(
    definition: &Definition,
    stats: &BTreeMap<String, Stats>,
    class: PureClass,
) -> Vec<Event> {
    let mut out = Vec::new();
    for tournament in &definition.tournaments {
        let name = tournament_event_name(&tournament.name, class.title());
        let mut cell = blank_cell(name.clone(), Mode::Tournament, class.title());
        cell.tournament_tracks.clone_from(&tournament.tracks);
        cell.ai_count = Some(7);
        out.push(Event {
            kind: EventKind::Tournament,
            name,
            subject: tournament.name.clone(),
            class: Some(class),
            cell,
            unlocks: tournament.unlocks.clone(),
        });
    }
    let league_gate = |track: &str| -> Vec<Unlock> {
        definition
            .leagues
            .iter()
            .filter(|league| league.tracks.iter().any(|t| t == track))
            .flat_map(|league| league.unlocks.clone())
            .collect()
    };
    let raceable = || {
        definition
            .tracks
            .iter()
            .filter(|track| track.kind != TrackKind::Zone)
    };
    for track in raceable() {
        let name = format!("race/{}/{}", track.name, class.title());
        let mut cell = blank_cell(name.clone(), Mode::Race, class.title());
        cell.track = Some(track.name.clone());
        cell.ai_count = Some(7);
        out.push(Event {
            kind: EventKind::Race,
            name,
            subject: track.name.clone(),
            class: Some(class),
            cell,
            unlocks: [league_gate(&track.name), track.unlocks.clone()].concat(),
        });
    }
    for track in raceable() {
        let Some(figures) = stats.get(&track.name) else {
            continue;
        };
        let name = format!("timetrial/{}/{}", track.name, class.title());
        let mut cell = blank_cell(name.clone(), Mode::TimeTrial, class.title());
        cell.track = Some(track.name.clone());
        let gold = figures.gold_seconds[PureClass::ALL
            .iter()
            .position(|c| *c == class)
            .unwrap_or_default()];
        cell.gold = centiseconds(gold);
        cell.silver = centiseconds(gold + figures.silver_seconds);
        cell.bronze = centiseconds(gold + figures.bronze_seconds);
        out.push(Event {
            kind: EventKind::TimeTrial,
            name,
            subject: track.name.clone(),
            class: Some(class),
            cell,
            unlocks: [league_gate(&track.name), track.unlocks.clone()].concat(),
        });
    }
    out.extend(zone_events(definition, stats));
    out
}

/// The Zone events: one per `type="Zone"` circuit, with no class.
#[must_use]
pub fn zone_events(definition: &Definition, stats: &BTreeMap<String, Stats>) -> Vec<Event> {
    definition
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Zone)
        .filter_map(|track| {
            let figures = stats.get(&track.name)?;
            let name = zone_event_name(&track.name);
            let mut cell = blank_cell(name.clone(), Mode::Zone, "Zone");
            cell.track = Some(track.name.clone());
            cell.laps = Some(0);
            [cell.gold, cell.silver, cell.bronze] = figures.zone;
            Some(Event {
                kind: EventKind::Zone,
                name,
                subject: track.name.clone(),
                class: None,
                cell,
                unlocks: track.unlocks.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
