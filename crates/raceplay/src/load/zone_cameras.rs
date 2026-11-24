//! The camera blocks a Zone race reads out of the Zone craft's own `handlingstats.xml`.
//!
//! Pure's Zone craft is a team of its own (`oag_title::ZoneCraft::OwnShip`, `Data\Ships\Zone_01`),
//! and that file authors the five camera elements the racing teams author, with the Zone
//! craft's own numbers (`ExternalCameraClose` `pos_height 3.0`, `pos_length -9.5`, fov 70).
//! The rest of the file is classless and is not read by this engine's per-class conversion, so
//! the race keeps the player's team for physics (`docs/gameplay/race-modes.md`, "Pure's Zone
//! cameras").

use super::*;

/// `stats` with the five camera blocks of the Zone craft's own file, when the title gives its
/// Zone craft a ship directory of its own and the race is on the Zone mask.
///
/// A missing or unparsable file leaves `stats` as it was and says why in `report`: a Zone race
/// that cannot read the file is a race on the team's cameras, not one that fails to load.
pub(super) fn overlay(
    stats: oag_tables::handling::Stats,
    craft_title: &oag_title::Title,
    mode: Mode,
    team: &str,
    archives: &mut oag_assets::Archives,
    report: &mut Vec<String>,
) -> oag_tables::handling::Stats {
    let race = craft_title.race;
    let oag_title::ZoneCraft::OwnShip(zone_team) = race.zone_craft else {
        return stats;
    };
    if !mode.on_zone_mask() || zone_team == team {
        return stats;
    }
    let entry = handling::entry_name_in(race.handling_dir_for(zone_team), zone_team);
    let zone = match archives
        .read_name(&entry)
        .map_err(|e| e.to_string())
        .and_then(|blob| handling::from_blob(&blob).map_err(|e| e.to_string()))
    {
        Ok(zone) => zone,
        Err(why) => {
            report.push(format!(
                "{}: cameras stay {team}'s, {entry} did not read: {why}",
                mode.name()
            ));
            return stats;
        }
    };
    report.push(format!(
        "{}: cameras from {entry}, physics from {team}'s file",
        mode.name()
    ));
    oag_tables::handling::Stats {
        internal_camera: zone.internal_camera,
        backward_camera: zone.backward_camera,
        bonnet_camera: zone.bonnet_camera,
        external_camera_far: zone.external_camera_far,
        external_camera_close: zone.external_camera_close,
        ..stats
    }
}
