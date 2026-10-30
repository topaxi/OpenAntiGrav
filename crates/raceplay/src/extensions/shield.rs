//! The Shield and energy commands' sinks: `shield-delta`, `give-shield`, `hold-shield`.

use crate::sim::RaceSim;

/// `shield-delta`'s sink: a loss through the weapon-damage path (so the destroyed
/// edge, the shield pickup's immunity and the critical crossing behave as a hit's),
/// at its full amount; a gain through the pool's clamp. A race whose rules take no
/// damage (Time Trial, Speed Lap) refuses a loss: the mod cannot wreck a craft where
/// nothing else can.
pub(super) fn delta(sim: &mut RaceSim, slot: usize, amount: f32) {
    let rules = sim.damage_rules();
    let ship = &mut sim.world.ships[slot];
    let dimensions = ship.handling.dimensions;
    if amount >= 0.0 {
        oag_physics::damage::add(&mut ship.physics, &dimensions, amount);
    } else if rules.damage {
        oag_physics::damage::apply_weapon(
            &mut ship.physics,
            &dimensions,
            -amount,
            oag_physics::damage::DamageRules {
                weapons: true,
                ..rules
            },
        );
    }
}

/// `give-shield`: the title's Shield on the slot for its authored time, as firing a
/// held Shield pickup does, and under the same rule: one already running is not
/// refreshed. A race without weapons, or a title table with no Shield, gives nothing.
pub(super) fn give(sim: &mut RaceSim, slot: usize) {
    let Some(simple) = sim
        .weapons
        .as_ref()
        .filter(|_| sim.weapons_on)
        .and_then(|weapons| weapons.simple(oag_tables::weapons::Weapon::Shield))
    else {
        return;
    };
    let timer = &mut sim.world.ships[slot].physics.shield_pickup_timer;
    if *timer <= 0.0 {
        *timer = simple.time;
        sim.extensions.on.gave |= 1 << slot;
    }
}

/// `hold-shield`: floors a running Shield's timer at two ticks, so this tick's step
/// leaves it running and a module that stops sending sees it end within two ticks.
/// It never starts one.
///
/// **Chosen, not measured:** nothing in the originals holds a Shield.
pub(super) fn hold(sim: &mut RaceSim, slot: usize) {
    let floor = 2.0 * sim.dt;
    let timer = &mut sim.world.ships[slot].physics.shield_pickup_timer;
    if *timer > 0.0 {
        *timer = timer.max(floor);
    }
}
