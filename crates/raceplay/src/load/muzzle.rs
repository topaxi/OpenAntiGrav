//! Each grid slot's `Ship Muzzle` locator, read off its own hull (or, on Wipeout HD, the
//! `Locators.vex` beside it) for the simulation: the weapon node a Plasma bolt charges on and is released from
//! (`oag_weapons::projectile::plasma::muzzle`).
//!
//! Read here, in both halves of the crate, rather than taken off the liveries:
//! the headless server links no livery and must release the bolt where the
//! drawing client does. The hull entry is the one the livery reads
//! (`oag_title::race::ShipPaths::hull_entry_name`), so the archive's memoised read
//! serves both.

use oag_core::math::Vec3;

/// One locator per slot of `slot_teams`, through the hull file the slot's livery is
/// built from (slot 0's own variant pick, a seated person's own, none under an
/// Eliminator's forced skin, as `oag_livery::load` resolves them): `None` where the
/// hull authors none, where the version's class id is unread, or where the
/// hull cannot be read at all (reported).
pub(super) fn per_slot(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    slot_teams: &[String],
    options: &crate::Options,
    (hull_variant, circuit_hull): (Option<&str>, Option<&'static str>),
    report: &mut Vec<String>,
) -> Vec<Option<Vec3>> {
    slot_teams
        .iter()
        .enumerate()
        .map(|(slot, team)| {
            let name = hull_entry(title, team, slot, options, (hull_variant, circuit_hull));
            match archives.read_name(&name) {
                Ok(blob) => oag_vex::vex::ship_muzzle(&blob)
                    .or_else(|| beside_the_hull(archives, &name))
                    .map(Vec3::from),
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

/// The hull entry `slot`'s livery is built from.
fn hull_entry(
    title: &oag_title::Title,
    team: &str,
    slot: usize,
    options: &crate::Options,
    (hull_variant, circuit_hull): (Option<&str>, Option<&'static str>),
) -> String {
    let forced = options.mode == oag_race::Mode::Eliminator && title.race.eliminator_skin.is_some();
    let variant = match slot {
        _ if forced => None,
        0 => hull_variant,
        _ => options
            .seat_picks
            .get(slot - 1)
            .and_then(|pick| pick.hull_variant.as_deref()),
    };
    let ships = title.race.ships_for(team);
    ships.hull_entry_name(
        team,
        ships.own_craft(
            options.mode == oag_race::Mode::Zone,
            options.mode == oag_race::Mode::Detonator,
        ),
        circuit_hull.or(variant),
    )
}

/// Slot 0's `cannon_flash_left` and `cannon_flash_right` locators in its hull's model space,
/// `[left, right]`, in a Detonator race: where the pulse cannon's rounds leave the craft
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "Muzzles"). The file is the one
/// the livery reads the same pair from, in the same order (the hull, then the `Locators.vex`
/// beside it), so the simulation and the drawn flash agree. `[None, None]` off Detonator, and a
/// side the hull authors none for is `None`.
pub(super) fn cannon_flash(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    team: &str,
    options: &crate::Options,
    (hull_variant, circuit_hull): (Option<&str>, Option<&'static str>),
    report: &mut Vec<String>,
) -> [Option<Vec3>; 2] {
    if options.mode != oag_race::Mode::Detonator {
        return [None, None];
    }
    let name = hull_entry(title, team, 0, options, (hull_variant, circuit_hull));
    let class = oag_vex::vex::classes::CLASS_CANNON_FLASH;
    let pair = |blob: &[u8]| oag_vex::vex::matrix::left_right_world_transforms(blob, class);
    let mut found = archives.read_name(&name).map_or([None, None], |b| pair(&b));
    if found == [None, None]
        && let Some(sibling) = sibling_locators(&name)
        && let Ok(blob) = archives.read_name(&sibling)
    {
        found = pair(&blob);
    }
    if found == [None, None] {
        report.push(format!(
            "{name}: no cannon_flash locators; the pulse cannon's rounds leave from the \
             hull's dimensions"
        ));
    }
    found.map(|m| m.map(|m| Vec3::new(m[12], m[13], m[14])))
}

/// The team's locator file, where Wipeout HD keeps the node (`ship_muzzle`, class
/// `0x3e2`, which `ShipMuzzle_AttachToShip` `0x002dae70` stores at `ship+0x5f34`).
const LOCATORS_ENTRY: &str = "Locators.vex";

/// The locator out of the `Locators.vex` beside the hull `name`, for a hull file that
/// authors none itself (`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, "The muzzle").
fn beside_the_hull(archives: &mut oag_assets::Archives, name: &str) -> Option<[f32; 3]> {
    let blob = archives.read_name(&sibling_locators(name)?).ok()?;
    oag_vex::vex::ship_muzzle(&blob)
}

/// The `Locators.vex` entry in the folder of the hull `name`.
fn sibling_locators(name: &str) -> Option<String> {
    let at = name.rfind(['\\', '/'])?;
    Some(format!("{}{LOCATORS_ENTRY}", &name[..=at]))
}
