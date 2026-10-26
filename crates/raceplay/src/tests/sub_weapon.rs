//! What the HUD's `SubWeapon` line reads off the craft: the Cannon's rounds left, the
//! Autopilot's seconds left, and the icon the Autopilot keeps up while it runs.
//!
//! The law is the player's status update's (`PlayerStatus_Update`, `0x0883b3b8`):
//! `docs/ghidra/functions/psp-pulse-usa/sub-weapon.md`.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_hud::SubWeapon;
use oag_tables::weapons::Weapon;

/// A single race with [`one_cannon_table`]'s five rounds and the Cannon in the slot.
fn cannon_race() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_cannon_table());
    let mut race = Race::start(setup);
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Cannon);
    race
}

/// The pad grant arms the original's counter with `<Stats rounds>`, so the HUD reads the
/// full magazine before a round has left; every round after drops it by one, and the line
/// goes with the last of them. Dropping the readout's fill, or its fall-back to the authored
/// count for a counter this engine has not armed yet, fails the first or the second.
#[test]
fn a_held_cannon_reads_its_rounds_and_counts_down_as_they_leave() {
    let mut race = cannon_race();
    assert_eq!(race.readout().sub_weapon, Some(SubWeapon::Rounds(5)));

    let mut buttons = Buttons::new();
    let mut seen = vec![race.readout().sub_weapon];
    for _ in 0..240 {
        race.tick(&PlayerInputs::single(buttons.tick(SQUARE)));
        let line = race.readout().sub_weapon;
        if seen.last() != Some(&line) {
            seen.push(line);
        }
    }
    assert_eq!(
        seen,
        [
            Some(SubWeapon::Rounds(5)),
            Some(SubWeapon::Rounds(4)),
            Some(SubWeapon::Rounds(3)),
            Some(SubWeapon::Rounds(2)),
            Some(SubWeapon::Rounds(1)),
            None,
        ],
        "the count falls one round at a time and the spent Cannon prints nothing"
    );
}

/// A running Autopilot prints its seconds and keeps its icon up with nothing held, as the
/// original forces the icon id while the running bit is set; the Cannon's count wins the
/// line when both apply (the status update writes the Cannon's last).
#[test]
fn a_running_autopilot_prints_its_time_under_its_own_icon() {
    let mut race = cannon_race();
    race.sim.world.ships[0].pickup.weapon = None;
    assert_eq!(race.readout().sub_weapon, None);
    assert_eq!(race.readout().pickup, None);

    race.sim.world.ships[0].autopilot_timer = 3.5;
    let readout = race.readout();
    assert_eq!(readout.sub_weapon, Some(SubWeapon::Autopilot(3.5)));
    assert_eq!(readout.pickup, Some(Weapon::Autopilot));

    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Cannon);
    let readout = race.readout();
    assert_eq!(readout.sub_weapon, Some(SubWeapon::Rounds(5)));
    assert_eq!(
        readout.pickup,
        Some(Weapon::Cannon),
        "the held icon wins too"
    );
}
