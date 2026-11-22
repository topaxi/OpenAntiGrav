//! The Zone hull's team texture: the one thing a craft's pick changes on it. The path
//! substitution it makes ([`swap`], [`Reads`]) also repaints an HD-era model from another livery
//! folder ([`crate::LiveryFolder`]).
//!
//! The ship-model loader builds `hdships\Zone\Ship.vex` for every craft and
//! then substitutes one material key (`zoneship_zone` on 2048 v1.04,
//! `zoneship_team` on Omega) with the craft's livery name, which names
//! `hdships\Zone\<livery>\Team.<ext>` (`docs/ghidra/functions/ps4-omega-eu/zone-craft.md`,
//! confidence 75). The livery name is the craft's own `PI_TeamModel
//! name="zone"` `texturelocation` in the plugin definition, so nothing here
//! invents a name: a craft that authors none keeps the hull's default skin and
//! the loader report says so.

use crate::LiveryFolder;
use oag_title::race::ShipPaths;

/// The livery folder `team` wears on this Zone hull, or `None` with the reason in `report`.
/// `liveries` is `(team id, livery name)` read off the definition.
///
/// The one decision of which skin the Zone hull wears: a race's hull build ([`crate::load`]) and
/// the front end's ship preview both take it, so the picker shows what the race flies. The folder
/// falls back to the hull's own default skin where the archive lacks a texture of it
/// ([`LiveryFolder::fall_back`]).
pub fn swap<'a>(
    ships: ShipPaths,
    mode: oag_race::Mode,
    liveries: &'a [(String, String)],
    team: &str,
    report: &mut Vec<String>,
) -> Option<LiveryFolder<'a>> {
    let key = crate::entry::own_craft(ships, mode)?.livery_key()?;
    let own = liveries
        .iter()
        .find(|(id, _)| id.eq_ignore_ascii_case(team))
        .map(|(_, livery)| livery.as_str());
    let fallback = (mode == oag_race::Mode::Detonator)
        .then(|| ships.detonator.and_then(|craft| craft.livery_fallback))
        .flatten();
    let Some(livery) = own.or(fallback) else {
        report.push(format!(
            "{team}: the definition authors no {} livery for this craft - the hull keeps \
             its default {key} skin",
            mode.name()
        ));
        return None;
    };
    Some(LiveryFolder {
        own: key,
        worn: livery,
        fall_back: true,
    })
}

/// A hull build's texture reads through one [`LiveryFolder`], counted for the report. A race's
/// hull build and the front end's ship preview read through this one reader.
#[derive(Debug)]
pub struct Reads<'a> {
    folder: Option<LiveryFolder<'a>>,
    label: &'static str,
    swapped: usize,
    missing: Vec<String>,
}

impl<'a> Reads<'a> {
    #[must_use]
    pub fn new(folder: Option<LiveryFolder<'a>>) -> Self {
        Self {
            folder,
            label: "Zone",
            swapped: 0,
            missing: Vec::new(),
        }
    }

    /// Names the mode whose hull the folder paints in [`Self::describe`]'s line: `"Zone"` unless
    /// told otherwise.
    #[must_use]
    pub fn named(mut self, label: &'static str) -> Self {
        self.label = label;
        self
    }

    /// The texture at `path`, read through the folder: from the worn folder where `path` is
    /// under the own one, and, for a folder that [falls back](LiveryFolder::fall_back), from
    /// `path` itself where the worn folder lacks it.
    pub fn read(&mut self, archives: &mut oag_assets::Archives, path: &str) -> Option<Vec<u8>> {
        let Some(folder) = self.folder else {
            return archives.read_name(path).ok();
        };
        let Some(alt) = folder.rewrite(path) else {
            return archives.read_name(path).ok();
        };
        if let Ok(blob) = archives.read_name(&alt) {
            self.swapped += 1;
            return Some(blob);
        }
        self.missing.push(alt);
        if folder.fall_back {
            archives.read_name(path).ok()
        } else {
            None
        }
    }

    /// The report line, when a folder was asked for.
    #[must_use]
    pub fn describe(&self, team: &str) -> Option<String> {
        let folder = self.folder?;
        let (own, worn) = (
            folder.own.to_ascii_lowercase(),
            folder.worn.to_ascii_lowercase(),
        );
        Some(if folder.fall_back {
            if self.missing.is_empty() {
                format!(
                    "{team}: {} livery {worn} replaces {own} on {} texture request(s)",
                    self.label, self.swapped
                )
            } else {
                format!(
                    "{team}: {} livery {worn} named {}, which the archive does not hold - the \
                     default {own} skin stays ({} swapped)",
                    self.label,
                    self.missing.join(", "),
                    self.swapped
                )
            }
        } else if self.missing.is_empty() {
            format!(
                "{team}: livery {worn} replaces {own} on {} texture request(s)",
                self.swapped
            )
        } else {
            format!(
                "{team}: livery {worn} does not hold {} - those draw untextured ({} swapped)",
                self.missing.join(", "),
                self.swapped
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_race_and_a_preview_take_the_same_folder_for_a_zone_pick() {
        let ships = oag_title::race::ShipPaths {
            dir: r"Data\Ships",
            hull: "ship",
            zone: oag_title::ZoneCraft::OwnShipAt {
                root: r"Data\Ships",
                ship: "zone",
                livery_key: "zoneship_zone",
            },
            detonator: None,
            boost: None,
        };
        let liveries = vec![("Qirex".to_string(), "zoneship_quirex".to_string())];
        let mut report = Vec::new();
        let folder = swap(ships, oag_race::Mode::Zone, &liveries, "qirex", &mut report)
            .expect("Qirex wears a Zone livery");
        assert!(report.is_empty());
        assert_eq!(
            folder
                .rewrite("data/ships/zone/zoneship_zone/team.gtf")
                .as_deref(),
            Some("data/ships/zone/zoneship_quirex/team.gtf")
        );
        assert!(folder.fall_back);
        assert_eq!(
            folder
                .rewrite(r"Data\Ships\Zone\Zoneship_Zone\Team.gtf")
                .as_deref(),
            Some("data/ships/zone/zoneship_quirex/team.gtf")
        );
        assert_eq!(
            folder.rewrite("data/ships/zone/zoneship_quirex/team.gtf"),
            None
        );
        assert!(
            swap(
                ships,
                oag_race::Mode::TimeTrial,
                &liveries,
                "Qirex",
                &mut report
            )
            .is_none()
        );
        assert!(
            swap(
                ships,
                oag_race::Mode::Zone,
                &liveries,
                "Auricom",
                &mut report
            )
            .is_none()
        );
        assert_eq!(report.len(), 1, "an absent livery says so: {report:?}");
    }
}
