//! [`crate::hud::Readout::shield_flashing`], wired through a real
//! [`Race::tick`] rather than the pure `shield_flash_step` helper
//! `race/telemetry.rs`'s own `shield_flash_tests` exercises.
//!
//! `setup(Handling::ZERO)` alone gives every ship a zero shield pool, which
//! never arms the flash at all - `oag_physics::damage::percent` returns `0.0`
//! for a zero maximum on both sides of every comparison. These fixtures
//! override [`Handling::ZERO`]'s `dimensions.shield` so there is a real pool
//! to drop.

use super::*;

fn shielded_handling() -> Handling {
    let mut handling = Handling::ZERO;
    handling.dimensions.shield = 100.0;
    handling
}

/// A full, untouched pool must never flash - the failure mode a screenshot
/// would not catch, since nothing on screen looks wrong about a bar that is
/// merely the wrong shade of never-red. Pins [`crate::race::start`]'s
/// `shield_flash_prev: 0.0` choice: seeding it at `1.0` instead would read
/// tick zero's real percentage as a drop from "full" and flash on a race that
/// never took a hit.
#[test]
fn a_fresh_race_is_not_flashing_at_the_start() {
    let mut race = Race::start(setup(shielded_handling()));

    assert!(!race.readout().shield_flashing, "before the first tick");

    for _ in 0..10 {
        race.tick(&InputSnapshot::default());
        assert!(
            !race.readout().shield_flashing,
            "a full, untouched pool must never flash"
        );
    }
}

/// A drop flashes on the tick it is read on, not one tick late.
///
/// The pool is set directly rather than damaged through a real wall contact,
/// the same fixture shape `race/tests/weapons.rs` already uses for the
/// Shield pickup - what [`Race::advance_shield_flash`] reads is the
/// percentage itself, not any one writer's own edge, so a manual drop
/// exercises exactly the same comparison a weapon hit would.
#[test]
fn a_drop_flashes_on_the_same_tick_it_lands() {
    let mut race = Race::start(setup(shielded_handling()));
    race.tick(&InputSnapshot::default());
    assert!(!race.readout().shield_flashing, "no drop yet");

    race.sim.world.ships[0].physics.shield = 40.0;
    race.tick(&InputSnapshot::default());
    assert!(
        race.readout().shield_flashing,
        "the tick that reads the drop must flash immediately"
    );
}
