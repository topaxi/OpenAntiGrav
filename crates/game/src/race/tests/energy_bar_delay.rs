//! [`oag_hud::Readout::energy_bar_delay_fraction`], wired through a real
//! [`Race::tick`] - the same shape `race/tests/shield_flash.rs` uses for its
//! own field, rather than a synthetic call to
//! `Race::advance_energy_bar_delay` on a `Race` with no ship state to read.

use super::*;
use oag_gameplay::PlayerInputs;

fn shielded_handling() -> Handling {
    let mut handling = Handling::ZERO;
    handling.dimensions.shield = 100.0;
    handling
}

/// A fresh race starts the trail at the shield's own pre-load value, not an
/// assumed "full" - `race/start.rs`'s `energy_bar_delay_fraction: 0.0`
/// choice, on the same terms its neighbour `shield_flash_prev: 0.0` already
/// carries: the pool itself reads `0.0` until `<Misc>` loads, so seeding
/// this any higher would read the first real tick as a false drop from full.
#[test]
fn a_fresh_race_starts_the_trail_at_zero() {
    let race = Race::start(setup(shielded_handling()));
    assert_eq!(race.readout().energy_bar_delay_fraction, 0.0);
}

/// The trail chases the shield fraction at `Hud_UpdateEnergyBar`'s own
/// `0.1` rate per tick, and lands on it exactly once ticked enough times -
/// `crop_vertically` then crops `EnergyBar` and `EnergyBarDelay` to the same
/// height and the two widgets read as one.
#[test]
fn the_trail_chases_a_steady_fraction_at_one_tenth_a_tick() {
    let mut race = Race::start(setup(shielded_handling()));
    // This tick both loads the pool and runs `advance_energy_bar_delay` once
    // already (it is part of `Race::tick`), so `expected` starts from what
    // it actually left behind rather than from the pre-race `0.0`.
    race.tick(&PlayerInputs::none());
    let fraction = race.readout().shield_fraction();
    assert!(
        fraction > 0.0,
        "the pool must have loaded by the first tick"
    );
    let mut expected = race.readout().energy_bar_delay_fraction;

    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
        expected += (fraction - expected) * 0.1;
        assert!(
            (race.readout().energy_bar_delay_fraction - expected).abs() < 1e-6,
            "expected {expected}, got {}",
            race.readout().energy_bar_delay_fraction
        );
    }

    for _ in 0..200 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        (race.readout().energy_bar_delay_fraction - fraction).abs() < 1e-4,
        "the trail must converge on a fraction that stops changing"
    );
}

/// **The trail overshoots on a drop, not just on a rise.** Right after a hit
/// it reads *higher* than [`oag_hud::Readout::shield_fraction`] for a
/// handful of ticks - the decompile's `target` snaps to the new, lower
/// fraction the same tick the pool drops, and the lagging value is still
/// most of the way to the old one, so `EnergyBarDelay` (opaque white,
/// painted last - `vita_2048_hud_dump` against `data/extracted/vita/PCSF00007`)
/// stands taller than `EnergyBar` until it catches down. Pins the shape a
/// screenshot alone cannot: `36-w-15.png`'s red cap is unmistakable, but
/// only a captured tick immediately after a scripted hit would show this
/// widget trailing *above* rather than below, and no capture in this pass
/// landed on that exact tick.
#[test]
fn the_trail_reads_above_the_fraction_for_a_few_ticks_after_a_drop() {
    let mut race = Race::start(setup(shielded_handling()));
    race.tick(&PlayerInputs::none());
    // Let the trail fully catch up to the loaded, undamaged pool first -
    // `0.9^150` is far below the tolerance below.
    for _ in 0..150 {
        race.tick(&PlayerInputs::none());
    }
    let before = race.readout().energy_bar_delay_fraction;
    assert!((before - 1.0).abs() < 1e-3, "should have caught up to full");

    race.sim.world.ships[0].physics.shield = 40.0;
    race.tick(&PlayerInputs::none());
    let readout = race.readout();
    assert!(
        readout.energy_bar_delay_fraction > readout.shield_fraction(),
        "the trail ({}) must still read above the fraction ({}) the tick after a drop",
        readout.energy_bar_delay_fraction,
        readout.shield_fraction()
    );
    assert!(
        readout.energy_bar_delay_fraction < before,
        "but must already have started chasing it down"
    );
}
