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

/// The livery folders `mode`'s own craft is painted from for `team`, by the decision the race's
/// hull build takes ([`oag_livery::zone_livery::swap`]) over the same rows
/// ([`oag_raceplay::catalogue::Team::craft_liveries`]). `None` for a mode that flies the team's
/// hull, on a craft with no livery key, and for a team the definition gives no livery.
#[must_use]
pub fn own_craft_folders(
    title: &oag_title::Title,
    team: &oag_raceplay::catalogue::Team,
    mode: oag_race::Mode,
) -> Option<Folders> {
    let liveries = team.craft_liveries(title.race, mode);
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

/// The livery folders the picker's selected team paints its preview hull from: a Zone race's
/// ([`own_craft_folders`]) where the row is fixed (`fixed_livery`, the mode's one hull), else the selected
/// variant's ([`livery_folders`]). The `--menu-page` still and the live screen's read take it, so
/// neither can drop the Zone paint alone.
#[must_use]
pub fn picked_folders(
    title: &oag_title::Title,
    team: &oag_raceplay::catalogue::Team,
    mode: oag_race::Mode,
    fixed_livery: bool,
    variant: &str,
) -> Option<Folders> {
    if fixed_livery {
        own_craft_folders(title, team, mode)
    } else {
        livery_folders(
            team,
            title.race.team_variants_for(&team.id).map(|v| v.join),
            variant,
        )
    }
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
    race_hull: Option<&super::RaceHull>,
    livery: Option<oag_livery::LiveryFolder<'_>>,
) -> Result<(Model, String)> {
    let (mut model, loaded) = load(archives, entry, fallback, livery)?;
    if let Some(race_hull) = race_hull
        && !super::own_hull_on(archives, &loaded, race_hull.own_hull, &mut model)
    {
        super::seat_flaps(archives, &race_hull.entry, &mut model);
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
    use super::{Folders, own_craft_folders, picked_folders, variant_location};
    use oag_title::VariantJoin;

    fn qirex() -> oag_raceplay::catalogue::Team {
        let xml = r#"<Plugin>
          <PI_Team name="Qirex"><Values type="Race" location="Data\Ships\Qirex"/>
            <PI_TeamModel name="zone"><Values texturelocation="zoneship_quirex"/></PI_TeamModel></PI_Team>
        </Plugin>"#;
        oag_raceplay::catalogue::teams(xml).remove(0)
    }

    /// The picker's Zone paint: the key the hull's materials name to the definition's livery,
    /// falling back to the default skin, in Zone alone.
    #[test]
    fn a_zone_pick_paints_the_shared_hull_in_the_teams_livery() {
        let want = Folders {
            own: "zoneship_zone".into(),
            worn: "zoneship_quirex".into(),
            fall_back: true,
        };
        for title in [oag_hd::TITLE, oag_2048::TITLE] {
            let team = qirex();
            assert_eq!(
                own_craft_folders(title, &team, oag_race::Mode::Zone),
                Some(want.clone()),
                "{}",
                title.name
            );
            assert_eq!(
                own_craft_folders(title, &team, oag_race::Mode::TimeTrial),
                None
            );
            // The picker's fixed row (Zone) takes the Zone paint, whatever variant id it holds.
            assert_eq!(
                picked_folders(title, &team, oag_race::Mode::Zone, true, ""),
                Some(want.clone())
            );
            assert_eq!(
                picked_folders(title, &team, oag_race::Mode::TimeTrial, false, ""),
                None
            );
        }
    }

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
