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
use oag_tables::pure_campaign::{self, Definition, Event, Progress, Stats, TrackKind};
use oag_tables::race_campaign::Medal;
use oag_ui::language::StringTable;

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
        let file = format!(r"{}\{}", track.location, pure_campaign::STATS_FILE);
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

/// Reads the SINGLE PLAYER chain's screens off `archives`.
///
/// # Errors
///
/// A title with no chain, or a file that will not read as one.
pub fn load_flow(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
) -> Result<oag_tables::pure_flow::Flow> {
    let files = title
        .campaign
        .flow
        .with_context(|| format!("{} authors no SINGLE PLAYER chain", title.name))?;
    let mut text = |entry: &str| -> Result<String> {
        let blob = archives
            .read_name(entry)
            .with_context(|| format!("reading {entry}"))?;
        Ok(oag_tables::fexml::text(&blob)?)
    };
    let skin = text(files.skin)?;
    let selection = text(files.selection)?;
    oag_tables::pure_flow::parse(&skin, &selection)
        .with_context(|| format!("reading {} and {}", files.skin, files.selection))
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

    /// The event a stored name stands for, in any class: a medal event off the
    /// disc, or a Free Play race.
    #[must_use]
    pub fn event_named(&self, name: &str) -> Option<Event> {
        if let Some((track, class)) = pure_campaign::free_play_parts(name) {
            return Some(pure_campaign::free_play_event(track, class));
        }
        PureClass::ALL
            .iter()
            .flat_map(|class| self.events(*class))
            .find(|event| event.name == name)
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
    /// The three tiers.
    pub gold: String,
    /// See [`Self::gold`].
    pub silver: String,
    /// See [`Self::gold`].
    pub bronze: String,
}

impl Words {
    pub(super) fn medal(&self, medal: Medal) -> &str {
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
    /// Our own words.
    pub words: &'a Words,
}

/// The ids (the leaf of each ship directory) of the teams the profile has not
/// opened yet. Teams, like events, are gated by the disc's own `<Unlock>` rows
/// (`Zone` on a gold in each Zone circuit, `Medievil` on a medal total).
#[must_use]
pub fn locked_teams(
    campaign: &PureCampaign,
    medal_of: &dyn Fn(&str) -> Option<Medal>,
    class: &str,
) -> Vec<String> {
    let progress = Progress {
        class,
        medal_count: campaign.medal_count(medal_of),
        medal_of,
    };
    campaign
        .definition
        .teams
        .iter()
        .filter(|team| !pure_campaign::all_met(&team.unlocks, &progress))
        .filter_map(|team| team.location.rsplit('\\').next().map(str::to_string))
        .collect()
}

/// Our own wording, each with the id `assets/ui/strings/english.toml` holds it
/// under and the literal it falls back to.
const WORDS: [(&str, &str); 3] = [
    ("OAG_MEDAL_GOLD", "GOLD"),
    ("OAG_MEDAL_SILVER", "SILVER"),
    ("OAG_MEDAL_BRONZE", "BRONZE"),
];

/// The words the chain says that the disc's string table does not, off
/// `strings` (the disc's table with the project's overlaid), so the live screens
/// and their stills draw the same words.
#[must_use]
pub fn wording(strings: &StringTable) -> Words {
    let say = |index: usize| {
        let (id, fallback) = WORDS[index];
        strings.get(id).unwrap_or(fallback).to_string()
    };
    Words {
        gold: say(0),
        silver: say(1),
        bronze: say(2),
    }
}

#[cfg(test)]
mod tests;
