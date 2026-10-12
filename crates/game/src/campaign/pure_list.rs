//! Wipeout Pure's campaign as the rows of a list: what the profile holds
//! against what `oag_tables::pure_campaign` reads off the disc.
//!
//! Pure's campaign is not a grid. Its events are every tournament, every
//! circuit raced and timed, and every Zone circuit, for the class being
//! played; the disc authors their circuits and gates, and the executable's
//! medal law (`docs/formats/pure-campaign.md`) turns the disc's figures into
//! the targets each event's [`oag_tables::race_campaign::Cell`] carries. A cell
//! launches through the same path a Pulse or HD cell does
//! (`Session::launch_campaign_cell`) and records its medal into the same
//! `[[campaign]]` table (`Store::record_campaign`), keyed on its
//! [`Event::name`].
//!
//! **Chosen, not measured:** the medal total `MedalCount` reads counts each
//! event's best medal once. The executable keeps gold, silver and bronze
//! counters it raises on every award (`Medal_Evaluate`, `0x088081d8`), and
//! whether a repeat award of one event raises them again was not traced; this
//! build does not let a replay count twice.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use oag_tables::pure_ai_stats::PureClass;
use oag_tables::pure_campaign::{
    self, ClassRef, Definition, Event, EventKind, MedalWanted, Progress, Stats, TrackKind, Unlock,
};
use oag_tables::race_campaign::Medal;
use oag_ui::language::StringTable;
use oag_ui_screens::pure_campaign::{Labels, Row, Status};

use crate::records::Store;

/// The disc's campaign data for one title, read once.
#[derive(Debug, Clone)]
pub struct PureCampaign {
    /// `Definition.xml`.
    pub definition: Definition,
    /// Each circuit's `stats.xml` figures by `PI_Track` name; a circuit whose
    /// file will not read has none, and so no time trial or Zone event.
    pub stats: BTreeMap<String, Stats>,
}

/// Reads the campaign off `archives`.
///
/// # Errors
///
/// A title with no list campaign, or a `Definition.xml` that will not read.
/// A single circuit's `stats.xml` that will not read is logged and skipped.
pub fn load(archives: &mut oag_assets::Archives, title: &oag_title::Title) -> Result<PureCampaign> {
    let entry = title
        .campaign
        .list_entry
        .with_context(|| format!("{} authors no list campaign", title.name))?;
    let blob = archives
        .read_name(entry)
        .with_context(|| format!("reading {entry}"))?;
    let definition = pure_campaign::parse(&oag_tables::fexml::text(&blob)?)
        .with_context(|| format!("reading {entry}"))?;
    let mut stats = BTreeMap::new();
    for track in &definition.tracks {
        let file = format!(r"{}\{}", track.location, oag_pure::campaign::STATS_FILE);
        let read = archives
            .read_name(&file)
            .map_err(anyhow::Error::from)
            .and_then(|blob| {
                Ok(pure_campaign::parse_stats(&oag_tables::fexml::text(
                    &blob,
                )?)?)
            });
        match read {
            Ok(figures) => {
                stats.insert(track.name.clone(), figures);
            }
            Err(error) => log::warn!("{file}: {error:#} - {} has no medal figures", track.name),
        }
    }
    Ok(PureCampaign { definition, stats })
}

impl PureCampaign {
    /// The archive entry a `Classic` or `Race` circuit's geometry is read from.
    #[must_use]
    pub fn track_entry(&self, name: &str) -> Option<String> {
        self.definition
            .tracks
            .iter()
            .find(|track| track.name == name && track.kind != TrackKind::Zone)
            .map(|track| format!(r"{}\track.vex", track.location))
    }

    /// Every event of one class, in list order.
    #[must_use]
    pub fn events(&self, class: PureClass) -> Vec<Event> {
        pure_campaign::events(&self.definition, &self.stats, class)
    }

    /// How many medals the profile holds: each event's best, once, over every
    /// class and Zone.
    #[must_use]
    pub fn medal_count(&self, medal_of: &dyn Fn(&str) -> Option<Medal>) -> u32 {
        let names: BTreeSet<String> = PureClass::ALL
            .into_iter()
            .flat_map(|class| self.events(class))
            .map(|event| event.name)
            .collect();
        u32::try_from(names.iter().filter(|name| medal_of(name).is_some()).count())
            .unwrap_or(u32::MAX)
    }
}

/// The words the list says that the disc's string table does not.
#[derive(Debug, Clone, Default)]
pub struct Words {
    /// A gated event's right-hand word.
    pub locked: String,
    /// The footer's lead-in for what opens a gated event.
    pub needs: String,
    /// The unit of a medal count.
    pub medals: String,
    /// A requirement of a medal, any tier.
    pub any_medal: String,
    /// The three tiers.
    pub gold: String,
    /// See [`Self::gold`].
    pub silver: String,
    /// See [`Self::gold`].
    pub bronze: String,
}

impl Words {
    fn medal(&self, medal: Medal) -> &str {
        match medal {
            Medal::Gold => &self.gold,
            Medal::Silver => &self.silver,
            Medal::Bronze => &self.bronze,
        }
    }
}

/// What the list needs to say a name: the disc's own text for a circuit,
/// tournament, class or mode, falling back to the name itself.
#[expect(
    missing_debug_implementations,
    reason = "holds lookup closures, which have no Debug"
)]
pub struct Names<'a> {
    /// A `PI_Track`, `PI_Tournament` or class name to the player's language.
    pub name_of: &'a dyn Fn(&str) -> String,
    /// The mode heading above a group of events.
    pub heading: &'a dyn Fn(EventKind) -> String,
    /// Our own words.
    pub words: &'a Words,
}

fn requirement(rows: &[Unlock], progress: &Progress<'_>, names: &Names<'_>) -> String {
    let pieces: Vec<String> = rows
        .iter()
        .filter(|row| !row.met(progress))
        .map(|row| {
            let mut parts = Vec::new();
            if let Some(count) = row.medal_count {
                parts.push(format!("{count} {}", names.words.medals));
            }
            let wanted = |medal: Option<MedalWanted>| match medal {
                Some(MedalWanted::AtLeast(tier)) => names.words.medal(tier).to_string(),
                Some(MedalWanted::Any) | None => names.words.any_medal.clone(),
            };
            if let Some(track) = &row.track {
                parts.push(format!("{} {}", (names.name_of)(track), wanted(row.medal)));
            }
            if let Some(tournament) = &row.tournament {
                let class = match &row.class {
                    Some(ClassRef::Named(name)) => format!(" {}", (names.name_of)(name)),
                    Some(ClassRef::AsSelected) | None => String::new(),
                };
                parts.push(format!(
                    "{}{class} {}",
                    (names.name_of)(tournament),
                    wanted(row.medal)
                ));
            }
            parts.join(" + ")
        })
        .collect();
    format!("{} {}", names.words.needs, pieces.join(", "))
}

/// The list for `class`, with the events behind it (a row's tag is its index
/// into the returned events).
///
/// `unlock_all` opens every gate - the developer escape `--unlock-all`, ours.
#[must_use]
pub fn rows(
    campaign: &PureCampaign,
    class: PureClass,
    medal_of: &dyn Fn(&str) -> Option<Medal>,
    unlock_all: bool,
    names: &Names<'_>,
) -> (Vec<Row>, Vec<Event>) {
    let events = campaign.events(class);
    let class_name = class.title();
    let progress = Progress {
        class: class_name,
        medal_count: campaign.medal_count(medal_of),
        medal_of,
    };
    let mut rows = Vec::new();
    let mut heading = None;
    for (tag, event) in events.iter().enumerate() {
        if heading != Some(event.kind) {
            heading = Some(event.kind);
            rows.push(Row::header((names.heading)(event.kind)));
        }
        let label = (names.name_of)(&event.subject);
        let open = unlock_all || pure_campaign::all_met(&event.unlocks, &progress);
        let held = medal_of(&event.name);
        let row = match (open, held) {
            (_, Some(medal)) => Row::event(
                label,
                tag,
                Status::Medal(medal),
                names.words.medal(medal).to_string(),
            ),
            (true, None) => Row::event(label, tag, Status::Open, String::new()),
            (false, None) => {
                let mut row = Row::event(label, tag, Status::Locked, names.words.locked.clone());
                row.requires = requirement(&event.unlocks, &progress, names);
                row
            }
        };
        rows.push(row);
    }
    (rows, events)
}

/// Our own wording, each with the id `assets/ui/strings/english.toml` holds it
/// under and the literal it falls back to.
const WORDS: [(&str, &str); 9] = [
    ("OAG_PURE_CAMPAIGN_TITLE", "CAMPAIGN"),
    ("OAG_PURE_CAMPAIGN_CLASS", "CLASS"),
    ("OAG_PURE_CAMPAIGN_LOCKED", "LOCKED"),
    ("OAG_PURE_CAMPAIGN_NEEDS", "NEEDS"),
    ("OAG_PURE_CAMPAIGN_MEDALS", "MEDALS"),
    ("OAG_PURE_CAMPAIGN_ANY_MEDAL", "ANY MEDAL"),
    ("OAG_MEDAL_GOLD", "GOLD"),
    ("OAG_MEDAL_SILVER", "SILVER"),
    ("OAG_MEDAL_BRONZE", "BRONZE"),
];

/// The screen's own text and the words its rows say, off `strings` (the disc's
/// table with the project's overlaid), so the live list and its still draw the
/// same words.
#[must_use]
pub fn wording(strings: &StringTable) -> (Labels, Words) {
    let say = |index: usize| {
        let (id, fallback) = WORDS[index];
        strings.get(id).unwrap_or(fallback).to_string()
    };
    (
        Labels {
            title: say(0),
            class: say(1),
        },
        Words {
            locked: say(2),
            needs: say(3),
            medals: say(4),
            any_medal: say(5),
            gold: say(6),
            silver: say(7),
            bronze: say(8),
        },
    )
}

/// The list for `class` as the player sees it: the rows, the events behind
/// them and the class's own name, with the disc's circuit, tournament, class and
/// mode names upper-cased as the menus' own rows are, and the profile's medals
/// read from `records`.
#[must_use]
pub fn present(
    campaign: &PureCampaign,
    strings: &StringTable,
    title_name: &str,
    class: PureClass,
    records: &Store,
) -> (Vec<Row>, Vec<Event>, String) {
    let say = |name: &str| strings.get(name).unwrap_or(name).to_uppercase();
    let (_, words) = wording(strings);
    let medal_of = crate::unlock::campaign_medal_of(records, title_name);
    let heading = |kind: EventKind| {
        say(match kind {
            EventKind::Tournament => "Tournament",
            EventKind::Race => "Single Race",
            EventKind::TimeTrial => "Time Trial",
            EventKind::Zone => "Zone",
        })
    };
    let (rows, events) = rows(
        campaign,
        class,
        &medal_of,
        crate::unlock::unlock_all(),
        &Names {
            name_of: &say,
            heading: &heading,
            words: &words,
        },
    );
    (rows, events, say(class.title()))
}

#[cfg(test)]
mod tests;
