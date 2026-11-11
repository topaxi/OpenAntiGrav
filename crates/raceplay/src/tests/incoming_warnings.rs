//! An opponent's fire raises the viewer's incoming-weapon warning by the recovered window
//! (`docs/ghidra/functions/psp-pulse-usa/hud-incoming-warnings.md`), through the real
//! gating and the readout; the fire paths themselves are `tests/incoming_warnings_ground_truth.rs`.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_hud::incoming::Icon;
use oag_tables::weapons::Weapon;

/// A single-race grid on Pulse's rules: the viewer in slot 0 with the warnings switched on,
/// the opponent in slot 1 holding `weapon`, and the two craft `gap` apart along the track
/// (viewer minus firer) as the standings last read them.
fn grid_with_fire(weapon: Weapon, table: oag_tables::weapons::WeaponStats, gap: f32) -> Race {
    grid_in(Mode::SingleRace, weapon, table, gap)
}

fn grid_in(mode: Mode, weapon: Weapon, table: oag_tables::weapons::WeaponStats, gap: f32) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = mode;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let mut race = Race::start(setup);
    race.sim.weapons = Some(Arc::new(table));
    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
    }
    race.view.huds[0].incoming_enabled = true;
    race.sim.world.ships[0].standing.progress = Some(1000.0);
    race.sim.world.ships[1].standing.progress = Some(1000.0 - gap);
    race.sim.world.ships[1].pickup.weapon = Some(weapon);
    race.view.incoming_raises.clear();
    race
}

#[test]
fn the_viewers_own_fire_raises_nothing() {
    let mut race = grid_with_fire(Weapon::Quake, one_quake_table(), 300.0);
    race.note_incoming_fire(0, Weapon::Quake);
    assert!(race.view.incoming_raises.is_empty());
}

#[test]
fn a_title_without_the_law_raises_nothing() {
    let mut race = grid_with_fire(Weapon::Quake, one_quake_table(), 300.0);
    race.view.huds[0].incoming_enabled = false;
    race.note_incoming_fire(1, Weapon::Quake);
    assert!(race.view.incoming_raises.is_empty());
}

#[test]
fn only_the_racing_modes_raise() {
    for (mode, raises) in [
        (Mode::SingleRace, true),
        (Mode::Tournament, true),
        (Mode::Eliminator, true),
        (Mode::Head2Head, true),
        (Mode::TimeTrial, false),
        (Mode::SpeedLap, false),
        (Mode::Zone, false),
    ] {
        let mut race = grid_in(mode, Weapon::Quake, one_quake_table(), 300.0);
        race.note_incoming_fire(1, Weapon::Quake);
        assert_eq!(!race.view.incoming_raises.is_empty(), raises, "{mode:?}");
    }
}

#[test]
fn the_raise_reaches_the_readout_unlit_then_blinks() {
    let mut race = grid_with_fire(Weapon::Quake, one_quake_table(), 300.0);
    race.note_incoming_fire(1, Weapon::Quake);
    race.advance_slot_huds();
    assert_eq!(
        race.readout().incoming.rear,
        None,
        "the raise frame is unlit"
    );
    race.advance_slot_huds();
    assert_eq!(race.readout().incoming.rear, Some(Icon::Quake));
    assert_eq!(race.readout().incoming.forward, None);
}
