//! A ship picker's preview follows the selected variant: which directory the
//! hull is read from, and what draws when a variant ships none.

use anyhow::Result;
use oag_mesh::mesh::Model;

/// The directory a team's `variant` races out of: `location` joined with the
/// variant's suffix by the title's own [`oag_title::VariantJoin`] (HD's
/// `Feisar` + `_c1` is `Feisar_c1`). `location` itself for the classic hull's
/// empty suffix or a team that carries no variant table.
#[must_use]
pub fn variant_location(
    join: Option<oag_title::VariantJoin>,
    location: &str,
    variant: &str,
) -> String {
    // A model in another livery (`"@livery2"`) is read out of its suffix's directory.
    let (variant, _) = oag_title::race::split_livery(variant);
    match join {
        Some(join) if !variant.is_empty() => join.combine(location, variant),
        _ => location.to_string(),
    }
}

/// The livery folders a preview paints a hull from, owned so a request can carry them to the
/// thread that reads the disc: [`oag_livery::LiveryFolder`] without the borrow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folders {
    /// The folder the hull's materials name.
    pub own: String,
    /// The folder the hull is painted from.
    pub worn: String,
    /// See [`oag_livery::LiveryFolder::fall_back`].
    pub fall_back: bool,
}

impl Folders {
    /// The borrowed form every reader takes.
    #[must_use]
    pub fn folder(&self) -> oag_livery::LiveryFolder<'_> {
        oag_livery::LiveryFolder {
            own: &self.own,
            worn: &self.worn,
            fall_back: self.fall_back,
        }
    }
}

/// The livery folders a stored variant id paints `team`'s hull from: `None` for
/// an id naming no livery ([`oag_title::race::split_livery`]) or a directory whose own model
/// authors no `texturelocation`. The own folder is the directory's own model's
/// ([`oag_raceplay::catalogue::Team::model_for_directory`]), the one the hull's materials name.
#[must_use]
pub fn livery_folders(
    team: &oag_raceplay::catalogue::Team,
    join: Option<oag_title::VariantJoin>,
    variant: &str,
) -> Option<Folders> {
    let (_, worn) = oag_title::race::split_livery(variant);
    let worn = worn?;
    let directory = variant_location(join, &team.location, variant);
    let own = team
        .model_for_directory(&directory)?
        .texture_location
        .clone()?;
    Some(Folders {
        own,
        worn: worn.to_string(),
        fall_back: false,
    })
}

/// The livery folders a Zone race paints `team`'s hull from, by the decision the race's hull
/// build takes ([`oag_livery::zone_livery::swap`]) over the same rows
/// ([`oag_raceplay::catalogue::Team::zone_liveries`]). `None` outside Zone, on a title whose Zone
/// hull has no livery key, and for a team the definition gives no Zone livery.
#[must_use]
pub fn zone_folders(
    title: &oag_title::Title,
    team: &oag_raceplay::catalogue::Team,
    mode: oag_race::Mode,
) -> Option<Folders> {
    let liveries = team.zone_liveries(title.race);
    let mut report = Vec::new();
    let folder = oag_livery::zone_livery::swap(
        title.race.ships_for(&team.id),
        mode,
        &liveries,
        &team.id,
        &mut report,
    )?;
    Some(Folders {
        own: folder.own.to_string(),
        worn: folder.worn.to_string(),
        fall_back: folder.fall_back,
    })
}

/// [`super::psp2_scene::model`] for a variant's own entry, falling back to `fallback` (the
/// team's default hull) when the variant ships no such asset, and saying so
/// in the log rather than inventing one. Returns the entry name that loaded.
///
/// `race_hull` is the race hull a front-end hull's airbrake flaps are seated from
/// ([`super::race_hull_entry`], [`super::seat_flaps`]); `None` where the title draws the race
/// hull itself.
///
/// # Errors
///
/// Neither entry loads.
pub fn model_or_default(
    archives: &mut oag_assets::Archives,
    entry: &str,
    fallback: Option<&str>,
    race_hull: Option<&str>,
    livery: Option<oag_livery::LiveryFolder<'_>>,
) -> Result<(Model, String)> {
    let (mut model, loaded) = load(archives, entry, fallback, livery)?;
    if let Some(race_hull) = race_hull {
        super::seat_flaps(archives, race_hull, &mut model);
    }
    Ok((model, loaded))
}

fn load(
    archives: &mut oag_assets::Archives,
    entry: &str,
    fallback: Option<&str>,
    livery: Option<oag_livery::LiveryFolder<'_>>,
) -> Result<(Model, String)> {
    match super::psp2_scene::model_in(archives, entry, livery) {
        Ok(model) => Ok((model, entry.to_string())),
        Err(error) => {
            let Some(fallback) = fallback.filter(|fallback| *fallback != entry) else {
                return Err(error);
            };
            log::warn!(
                "{entry}: {error:#} - the variant has no hull of its own, drawing {fallback}"
            );
            super::psp2_scene::model(archives, fallback).map(|model| (model, fallback.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::variant_location;
    use oag_title::VariantJoin;

    #[test]
    fn a_suffix_joins_the_teams_own_directory() {
        let join = Some(VariantJoin::Suffix);
        assert_eq!(
            variant_location(join, r"Data\Ships\Feisar", "_c1"),
            r"Data\Ships\Feisar_c1"
        );
        assert_eq!(
            variant_location(join, r"Data\Ships\Feisar", ""),
            r"Data\Ships\Feisar"
        );
        assert_eq!(
            variant_location(None, r"Data\Ships\Feisar", "_c1"),
            r"Data\Ships\Feisar"
        );
    }
}
