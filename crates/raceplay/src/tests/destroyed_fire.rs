//! A craft in its destroyed sequence cannot fire or use its pickup, and keeps what it
//! held for after the respawn (`docs/gameplay/pickups.md`, "A destroyed craft").

use super::*;
use oag_gameplay::PlayerInputs;
use oag_physics::CraftState;
use oag_tables::weapons::Weapon;

fn rockets(race: &Race, owner: u8) -> usize {
    race.sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Rocket) && p.owner == owner)
        .count()
}

fn eliminator_holding_a_rocket(state: CraftState) -> Race {
    let mut race = race_with_weapon_table(Mode::Eliminator, Vec::new(), 1.0, one_rocket_table());
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Rocket);
    race.sim.world.ships[0].physics.craft_state = state;
    race.sim.world.ships[0].physics.state_timer = 20.0;
    race
}

#[test]
fn a_destroyed_craft_holding_fire_launches_nothing_and_keeps_its_pickup() {
    for state in [CraftState::Destroyed, CraftState::Eliminated] {
        let mut race = eliminator_holding_a_rocket(state);
        let mut buttons = Buttons::new();
        buttons.tick(0);
        for _ in 0..3 {
            race.tick(&PlayerInputs::single(buttons.tick(SQUARE)));
            buttons.tick(0);
            race.tick(&PlayerInputs::single(buttons.tick(SQUARE)));
        }
        assert_eq!(rockets(&race, 0), 0, "{state:?} fired a rocket");
        assert_eq!(
            race.ship_pickup(),
            Some(Weapon::Rocket),
            "{state:?} lost its pickup"
        );
    }
}

#[test]
fn a_racing_craft_with_the_same_setup_does_fire() {
    let mut race = eliminator_holding_a_rocket(CraftState::Racing);
    let mut buttons = Buttons::new();
    buttons.tick(0);
    race.tick(&PlayerInputs::single(buttons.tick(SQUARE)));
    assert!(rockets(&race, 0) > 0, "the control did not fire");
}

#[test]
fn a_destroyed_opponent_uses_nothing() {
    let spent = |state: CraftState| {
        let mut setup = setup(hulled_handling());
        setup.mode = Mode::Eliminator;
        setup.weapons = Some(one_turbo_table());
        setup.start_position = Some(oag_vex::track::StartPosition {
            position: [0.0, 0.0, 0.0],
            left: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            forward: [1.0, 0.0, 0.0],
        });
        let mut race = Race::start(setup);
        for _ in 0..10 {
            race.tick(&PlayerInputs::none());
        }
        race.sim.world.ships[1].pickup.weapon = Some(Weapon::Turbo);
        race.sim.world.ships[1].physics.craft_state = state;
        race.sim.world.ships[1].physics.state_timer = 20.0;
        let mut ticks_down = 0;
        for _ in 0..300 {
            race.tick(&PlayerInputs::none());
            if race.sim.world.ships[1].physics.craft_state != CraftState::Racing {
                ticks_down += 1;
            } else if state != CraftState::Racing {
                break;
            }
        }
        assert!(
            state == CraftState::Racing || ticks_down > 30,
            "the wreck was up too briefly to test"
        );
        race.sim.world.ships[1].pickup.weapon.is_none()
    };
    assert!(
        spent(CraftState::Racing),
        "control: a racing opponent never used its Turbo"
    );
    for state in [CraftState::Destroyed, CraftState::Eliminated] {
        assert!(!spent(state), "a {state:?} opponent used its pickup");
    }
}
