//! `campaigns/<id>.toml`: a campaign a mod adds, as the file spells it.
//!
//! This crate knows no title, track, team or mode, so what it holds is the
//! **raw** definition: strings and numbers, checked for shape (the schema
//! header, unknown keys, ids, slots) and nothing else. The game compiles it
//! into the engine's own grid cells and refuses a name it does not know there,
//! naming the mod, the campaign and the event. The format is
//! `docs/formats/custom-campaign.md`; the decision ADR-0077.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// The folder a mod keeps its campaigns in.
pub const DIRECTORY: &str = "campaigns";
/// The schema kind a campaign file's header names.
pub const SCHEMA_KIND: &str = "campaign";
/// The one schema major this build reads.
pub const SCHEMA_MAJOR: u32 = 1;
/// The most grid tiers a campaign may have: each one is a page of the title's
/// own screen, and its cell names carry the tier as `gridN`.
pub const MAX_GRIDS: usize = 16;
/// Slot coordinates are one digit each, the original's own `gridN_x_y` rule.
pub const MAX_SLOT: u32 = 9;

/// What is wrong with a campaign file.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Not TOML, or not shaped like a campaign.
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    /// No `[toml-schema]` header, or one naming another kind or major.
    #[error("{0}")]
    Schema(#[from] oag_schema::Error),
    /// A value that cannot be right, wherever it is.
    #[error("{at}: {problem}")]
    Bad {
        /// Where: `[campaign] id`, `grid "opening" event [1, 2]`.
        at: String,
        /// What is wrong.
        problem: String,
    },
}

fn bad(at: impl Into<String>, problem: impl Into<String>) -> Error {
    Error::Bad {
        at: at.into(),
        problem: problem.into(),
    }
}

/// How a cell that is not open yet shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Reveal {
    /// Drawn with its lock, as Pulse and HD draw theirs.
    #[default]
    Locked,
    /// Not drawn at all until it opens, as 2048's events appear.
    Hidden,
}

/// A whole campaign file.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignDef {
    #[serde(rename = "toml-schema", skip_serializing_if = "Option::is_none")]
    header: Option<oag_schema::Header>,
    /// Who the campaign is.
    pub campaign: Info,
    /// The tiers, in page order.
    #[serde(rename = "grid", default)]
    pub grids: Vec<GridDef>,
}

/// `[campaign]`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Info {
    /// Unique within the mod.
    pub id: String,
    /// The picker row.
    pub name: String,
    /// One or two sentences, the picker row's second line.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// The titles it is for; narrows the mod's own list. Empty means the mod's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub titles: Vec<String>,
    /// How an unopened cell shows.
    #[serde(default)]
    pub reveal: Reveal,
}

/// `[[grid]]`: one tier.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GridDef {
    /// Unique within the campaign; what `unlock_grid` and an event reference name.
    pub id: String,
    /// The tier's name, for the report.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Medal points needed on `unlock_grid` to open this tier.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub required_points: u32,
    /// The tier whose points open this one. Absent: open from the start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlock_grid: Option<String>,
    /// How this tier's unopened cells show; the campaign's [`Info::reveal`]
    /// when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reveal: Option<Reveal>,
    /// The disc's own flyer card for this tier, by the name the title's grids
    /// give it (`01_uplift` on Wipeout HD); drawn on a title whose grid shows
    /// cards, ignored on one that does not. Absent: no card.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flyer: Option<String>,
    /// The events, one per hex slot.
    #[serde(rename = "event", default)]
    pub events: Vec<EventDef>,
}

/// `[[grid.event]]`: one cell.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventDef {
    /// The hex slot `[x, y]`, each `0..=9`.
    #[serde(deserialize_with = "pair")]
    pub slot: [u32; 2],
    /// What `unlock.requires` names it by; `<grid id>/<x>-<y>` when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The event's own name, for the report.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// One of the race modes the engine already runs.
    pub mode: String,
    /// The circuit: a disc circuit id, or `<title>:<id>` for another disc's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
    /// A tournament's legs, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<String>,
    /// A track file of this project's own format, raced instead of a disc
    /// circuit: a path inside the mod (`tracks/lagoon.track.toml`), or
    /// `<mod id>:<path>` inside another installed mod
    /// (`oag.hillside:tracks/hillside.track.toml`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// The speed class. Absent on a Zone event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// Laps; the mode's own default when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub laps: Option<u32>,
    /// Weapons on or off; the mode's own default when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weapons: Option<bool>,
    /// The player's craft. Absent: the player picks on Team Selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub craft: Option<CraftDef>,
    /// The opponents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<FieldDef>,
    /// Medal targets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medals: Option<MedalsDef>,
    /// What opens the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlock: Option<UnlockDef>,
}

impl EventDef {
    /// The id `unlock.requires` names this event by.
    #[must_use]
    pub fn reference(&self, grid: &GridDef) -> String {
        self.id
            .clone()
            .unwrap_or_else(|| format!("{}/{}-{}", grid.id, self.slot[0], self.slot[1]))
    }
}

/// `[grid.event.craft]`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CraftDef {
    /// The team the player flies; Team Selection is skipped.
    pub team: String,
}

/// `[grid.event.field]`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDef {
    /// `"original"` (the recovered driver, the default) or `"ours"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<String>,
    /// The original's rung (`easy`, `medium`, `hard`) or our driver's level
    /// (`novice`, `skilled`, `elite`, `ace`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    /// The original's position on the track's skill curve, as Pulse's `skill`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<f32>,
    /// Our driver's personality for every opponent: a pilot by name, or one
    /// written out here with the pilot file's own keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pilot: Option<PilotDef>,
    /// The opponents' teams, grid slot 1 first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub teams: Vec<String>,
}

/// A pilot named or written out.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PilotDef {
    /// A pilot of the roster, built-in or the player's.
    Named(String),
    /// A pilot file's keys, inline.
    Inline(toml::Table),
}

/// `[grid.event.medals]`: one triple for every rung, or one per rung.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MedalsDef {
    /// Gold for any rung that names none of its own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gold: Option<f64>,
    /// Silver, as [`Self::gold`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silver: Option<f64>,
    /// Bronze, as [`Self::gold`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bronze: Option<f64>,
    /// The easy rung.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub easy: Option<Targets>,
    /// The medium rung.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<Targets>,
    /// The hard rung.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hard: Option<Targets>,
}

impl MedalsDef {
    /// The flat triple.
    #[must_use]
    pub fn all(&self) -> Targets {
        Targets {
            gold: self.gold,
            silver: self.silver,
            bronze: self.bronze,
        }
    }
}

/// One rung's targets: a place, a time in seconds, or a count, by mode. An
/// absent tier is never awarded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Targets {
    /// Gold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gold: Option<f64>,
    /// Silver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silver: Option<f64>,
    /// Bronze.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bronze: Option<f64>,
}

impl Targets {
    /// Whether no tier is named.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.gold.is_none() && self.silver.is_none() && self.bronze.is_none()
    }
}

/// `[grid.event.unlock]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnlockDef {
    /// Open from the start.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub open: bool,
    /// Every one of these must hold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Requirement>,
}

/// One event finished with at least a medal.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    /// The event, by [`EventDef::reference`].
    pub event: String,
    /// `any` (the default), `bronze`, `silver` or `gold`.
    #[serde(default = "any")]
    pub medal: String,
}

/// Exactly two numbers: a fixed-size array alone would read the first two of a
/// longer one and drop the rest unseen.
fn pair<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[u32; 2], D::Error> {
    let given: Vec<u32> = Deserialize::deserialize(d)?;
    <[u32; 2]>::try_from(given)
        .map_err(|v| serde::de::Error::invalid_length(v.len(), &"an array of exactly two numbers"))
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

fn any() -> String {
    "any".to_string()
}

const MEDALS: [&str; 4] = ["any", "bronze", "silver", "gold"];

impl CampaignDef {
    /// A definition ready to be written, carrying the schema header
    /// [`Self::parse`] requires.
    #[must_use]
    pub fn new(campaign: Info, grids: Vec<GridDef>) -> Self {
        Self {
            header: Some(oag_schema::Header {
                location: oag_schema::location(SCHEMA_KIND, SCHEMA_MAJOR),
                version: oag_schema::LANGUAGE_VERSION.to_string(),
            }),
            campaign,
            grids,
        }
    }

    /// Checks the definition as [`Self::parse`] would and writes it as a
    /// campaign file: [`Self::parse`] of the text gives this definition back.
    ///
    /// # Errors
    ///
    /// The same [`Error`]s [`Self::parse`] gives for the shape, or the
    /// serialiser's own reason in [`Error::Bad`].
    pub fn write(&self) -> Result<String, Error> {
        self.check()?;
        let mut def = self.clone();
        if def.header.is_none() {
            def = Self::new(def.campaign, def.grids);
        }
        toml::to_string(&def).map_err(|e| bad("write", e.to_string()))
    }

    /// Reads and checks a campaign file's shape.
    ///
    /// # Errors
    ///
    /// Anything in [`Error`].
    pub fn parse(text: &str) -> Result<Self, Error> {
        let def: Self = toml::from_str(text)?;
        oag_schema::check(def.header.as_ref(), SCHEMA_KIND, SCHEMA_MAJOR)?;
        def.check()?;
        Ok(def)
    }

    fn check(&self) -> Result<(), Error> {
        let info = &self.campaign;
        if !crate::manifest::valid_id(&info.id) {
            return Err(bad(
                "[campaign] id",
                format!(
                    "{:?} is not an id: lowercase letters, digits, '.', '-' and '_', starting with a letter",
                    info.id
                ),
            ));
        }
        if info.name.trim().is_empty() {
            return Err(bad("[campaign] name", "empty"));
        }
        if self.grids.is_empty() {
            return Err(bad("[campaign]", "no [[grid]]"));
        }
        if self.grids.len() > MAX_GRIDS {
            return Err(bad(
                "[campaign]",
                format!("{} grids, at most {MAX_GRIDS}", self.grids.len()),
            ));
        }
        let mut grid_ids = BTreeSet::new();
        let mut references = BTreeSet::new();
        for grid in &self.grids {
            if grid.id.trim().is_empty() || !grid_ids.insert(grid.id.as_str()) {
                return Err(bad(format!("grid {:?}", grid.id), "empty or used twice"));
            }
            if grid.events.is_empty() {
                return Err(bad(format!("grid {:?}", grid.id), "no [[grid.event]]"));
            }
            let mut slots = BTreeSet::new();
            for event in &grid.events {
                let at = format!("grid {:?} event {:?}", grid.id, event.slot);
                if event.slot.iter().any(|&c| c > MAX_SLOT) {
                    return Err(bad(&at, format!("slot coordinates are 0..={MAX_SLOT}")));
                }
                if !slots.insert(event.slot) {
                    return Err(bad(&at, "two events in one slot"));
                }
                if let Some((mod_id, path)) = event.file.as_deref().and_then(|f| f.split_once(':'))
                    && (!crate::manifest::valid_id(mod_id) || path.is_empty())
                {
                    return Err(bad(
                        &at,
                        format!(
                            "file \"{mod_id}:{path}\": `<mod id>:<path>` names another mod's track"
                        ),
                    ));
                }
                if !references.insert(event.reference(grid)) {
                    return Err(bad(
                        &at,
                        format!("id {:?} used twice", event.reference(grid)),
                    ));
                }
            }
        }
        for grid in &self.grids {
            if let Some(target) = &grid.unlock_grid
                && !grid_ids.contains(target.as_str())
            {
                return Err(bad(
                    format!("grid {:?} unlock_grid", grid.id),
                    format!("no grid {target:?}"),
                ));
            }
            for event in &grid.events {
                let at = format!("grid {:?} event {:?}", grid.id, event.slot);
                for requirement in event.unlock.iter().flat_map(|u| &u.requires) {
                    if !references.contains(&requirement.event) {
                        return Err(bad(
                            &at,
                            format!("requires {:?}: no such event", requirement.event),
                        ));
                    }
                    if !MEDALS.contains(&requirement.medal.as_str()) {
                        return Err(bad(
                            &at,
                            format!(
                                "requires medal {:?}: one of any, bronze, silver, gold",
                                requirement.medal
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

/// The mod id a campaign saved by the game carries: `player.<campaign id>`.
#[must_use]
pub fn saved_mod_id(campaign_id: &str) -> String {
    format!("player.{campaign_id}")
}

/// Writes `def` as a fresh folder mod under `root` (the user mods directory):
/// `<root>/<campaign id>[-n]/oag-mod.toml` and `campaigns/<campaign id>.toml`, for the
/// titles named (their lowercased hyphenated names). An existing folder is never touched:
/// the next free `-n` suffix is used. Returns the folder written.
///
/// # Errors
///
/// The definition does not check, or a file cannot be written.
pub fn write_mod(
    root: &std::path::Path,
    def: &CampaignDef,
    titles: &[&str],
) -> Result<std::path::PathBuf, crate::Error> {
    #[derive(Serialize)]
    struct ModTable<'a> {
        id: String,
        name: &'a str,
        version: &'a str,
        authors: [&'a str; 1],
        description: &'a str,
        titles: &'a [&'a str],
    }
    #[derive(Serialize)]
    struct Manifest<'a> {
        #[serde(rename = "toml-schema")]
        header: oag_schema::Header,
        r#mod: ModTable<'a>,
    }
    let io = |path: &std::path::Path| {
        let origin = path.display().to_string();
        move |source| crate::Error::Io { origin, source }
    };
    let other = |path: &std::path::Path, why: String| crate::Error::Io {
        origin: path.display().to_string(),
        source: std::io::Error::other(why),
    };
    let campaign_text = def.write().map_err(|e| other(root, e.to_string()))?;
    let id = &def.campaign.id;
    let mut folder = root.join(id);
    let mut mod_id = saved_mod_id(id);
    let mut n = 1_u32;
    while folder.exists() {
        n += 1;
        folder = root.join(format!("{id}-{n}"));
        mod_id = saved_mod_id(&format!("{id}-{n}"));
    }
    let campaigns = folder.join(DIRECTORY);
    std::fs::create_dir_all(&campaigns).map_err(io(&campaigns))?;
    let manifest = Manifest {
        header: oag_schema::Header {
            location: oag_schema::location(crate::SCHEMA_KIND, crate::SCHEMA_MAJOR),
            version: oag_schema::LANGUAGE_VERSION.to_string(),
        },
        r#mod: ModTable {
            id: mod_id,
            name: &def.campaign.name,
            version: "1.0.0",
            authors: ["Player"],
            description: if def.campaign.description.trim().is_empty() {
                "A campaign made in the in-game editor."
            } else {
                &def.campaign.description
            },
            titles,
        },
    };
    let manifest_text = toml::to_string(&manifest).map_err(|e| other(&folder, e.to_string()))?;
    let manifest_path = folder.join(crate::MANIFEST);
    std::fs::write(&manifest_path, manifest_text).map_err(io(&manifest_path))?;
    let file = campaigns.join(format!("{id}.toml"));
    std::fs::write(&file, campaign_text).map_err(io(&file))?;
    Ok(folder)
}

/// A campaign found in a mod: where it came from, and what it is or why not.
#[derive(Debug, Clone)]
pub struct Found {
    /// The mod's id.
    pub mod_id: String,
    /// The file inside the mod.
    pub file: String,
    /// The definition, or the reason it is not one.
    pub def: Result<CampaignDef, String>,
}

impl crate::Mod {
    /// Every file under `prefix/` ending in `.<ext>`, sorted.
    #[must_use]
    pub fn files_under(&self, prefix: &str, ext: &str) -> Vec<String> {
        let folder = format!("{}/", prefix.trim_end_matches('/'));
        let suffix = format!(".{ext}");
        let mut names: Vec<String> = self
            .names()
            .into_iter()
            .filter(|name| {
                name.starts_with(&folder)
                    && name.to_ascii_lowercase().ends_with(&suffix)
                    && !name[folder.len()..].contains('/')
            })
            .collect();
        names.sort();
        names
    }

    /// Whether the manifest's `titles` admit `title` (lowercased, hyphenated):
    /// an empty list admits every title.
    #[must_use]
    pub fn is_for(&self, title: &str) -> bool {
        self.manifest.titles.is_empty()
            || self
                .manifest
                .titles
                .iter()
                .any(|t| t.eq_ignore_ascii_case(title))
    }

    /// Every campaign file the mod carries, read and checked; a broken one is
    /// returned with its reason rather than dropped. Titles are not filtered
    /// here: see [`Self::is_for`] and [`Info::titles`].
    #[must_use]
    pub fn campaigns(&self) -> Vec<Found> {
        self.files_under(DIRECTORY, "toml")
            .into_iter()
            .map(|file| {
                let def = match self.read_text(&file) {
                    Ok(Some(text)) => CampaignDef::parse(&text).map_err(|e| e.to_string()),
                    Ok(None) => Err("listed but not readable".to_string()),
                    Err(e) => Err(e.to_string()),
                };
                Found {
                    mod_id: self.manifest.id.clone(),
                    file,
                    def,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
