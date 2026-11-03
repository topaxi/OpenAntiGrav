//! Each grid slot's `Ship Muzzle` locator, read off its own hull for the
//! simulation: the weapon node a Plasma bolt charges on and is released from
//! (`oag_weapons::projectile::plasma::muzzle`).
//!
//! Read here, in both halves of the crate, rather than taken off the liveries:
//! the headless server links no livery and must release the bolt where the
//! drawing client does. The hull entry is the one the livery reads
//! (`oag_pulse::race::ships::hull_entry_name`), so the archive's memoised read
//! serves both.

use oag_core::math::Vec3;

/// One locator per slot of `slot_teams`, through the hull file the slot's livery is
/// built from (slot 0's own variant pick, a seated person's own, none under an
/// Eliminator's forced skin, as `oag_livery::load` resolves them): `None` where the
/// hull authors none, where the version's class id is unread (Pure), or where the
/// hull cannot be read at all (reported).
pub(super) fn per_slot(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    slot_teams: &[String],
    options: &crate::Options,
    hull_variant: Option<&str>,
    report: &mut Vec<String>,
) -> Vec<Option<Vec3>> {
    let zone = options.mode == oag_race::Mode::Zone;
    let forced = options.mode == oag_race::Mode::Eliminator && title.race.eliminator_skin.is_some();
    slot_teams
        .iter()
        .enumerate()
        .map(|(slot, team)| {
            let variant = match slot {
                _ if forced => None,
                0 => hull_variant,
                _ => options
                    .seat_picks
                    .get(slot - 1)
                    .and_then(|pick| pick.hull_variant.as_deref()),
            };
            let name = oag_pulse::race::ships::hull_entry_name(
                title.race.ships_for(team),
                team,
                zone,
                variant,
            );
            match archives.read_name(&name) {
                Ok(blob) => oag_vex::vex::ship_muzzle(&blob).map(Vec3::from),
                Err(error) => {
                    report.push(format!(
                        "{name}: not read for the Plasma's weapon node ({error}); the bolt \
                         leaves from the hold point"
                    ));
                    None
                }
            }
        })
        .collect()
}
