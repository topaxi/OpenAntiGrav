//! A craft in its destroyed sequence cannot fire or use its pickup, and keeps what it
//! held for after the respawn (`docs/gameplay/pickups.md`, "A destroyed craft").

use super::*;
use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;
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

/// Runs `ticks` ticks of `inputs` and counts the `cue`s raised.
fn cues_over(
    race: &mut Race,
    cue: Cue,
    ticks: usize,
    mut inputs: impl FnMut() -> PlayerInputs,
) -> usize {
    let mut seen = 0;
    for _ in 0..ticks {
        race.tick(&inputs());
        seen += race.drain_cues().iter().filter(|e| e.cue == cue).count();
    }
    seen
}

fn eliminator_holding(weapon: Weapon, table: oag_tables::weapons::WeaponStats) -> Race {
    let mut race = race_with_weapon_table(Mode::Eliminator, Vec::new(), 1.0, table);
    race.sim.world.ships[0].pickup.weapon = Some(weapon);
    race
}

fn down(race: &mut Race) {
    race.sim.world.ships[0].physics.craft_state = CraftState::Destroyed;
    race.sim.world.ships[0].physics.state_timer = 20.0;
}

fn respawn(race: &mut Race) {
    race.sim.world.ships[0].physics.craft_state = CraftState::Racing;
}

#[test]
fn a_racing_craft_lays_its_whole_mine_cluster() {
    let mut race = eliminator_holding(Weapon::Mine, one_mine_table());
    race.sim.world.ships[0]
        .pickup
        .begin_drop(oag_weapons::projectile::mine::CLUSTER);
    let laid = cues_over(&mut race, Cue::MineLaunch, 120, PlayerInputs::none);
    assert_eq!(laid, usize::from(oag_weapons::projectile::mine::CLUSTER));
}

#[test]
fn a_mine_drop_cut_short_by_destruction_does_not_resume_after_the_respawn() {
    let mut race = eliminator_holding(Weapon::Mine, one_mine_table());
    race.sim.world.ships[0]
        .pickup
        .begin_drop(oag_weapons::projectile::mine::CLUSTER);
    let before = cues_over(&mut race, Cue::MineLaunch, 1, PlayerInputs::none);
    assert!(before > 0, "the drop never started");
    down(&mut race);
    let during = cues_over(&mut race, Cue::MineLaunch, 20, PlayerInputs::none);
    respawn(&mut race);
    let after = cues_over(&mut race, Cue::MineLaunch, 120, PlayerInputs::none);
    assert_eq!((during, after), (0, 0), "the drop went on");
    assert!(!race.sim.world.ships[0].pickup.is_dropping());
    assert_eq!(race.ship_pickup(), None, "the part-laid cluster survived");
}

#[test]
fn a_racing_craft_holding_fire_empties_a_cannon_magazine() {
    let mut race = eliminator_holding(Weapon::Cannon, one_cannon_table());
    let mut buttons = Buttons::new();
    let fired = cues_over(&mut race, Cue::Cannon, 30, || {
        PlayerInputs::single(buttons.tick(SQUARE))
    });
    assert!(fired > 0, "the control did not fire");
}

#[test]
fn a_cannon_burst_cut_short_by_destruction_does_not_resume_after_the_respawn() {
    let mut race = eliminator_holding(Weapon::Cannon, one_cannon_table());
    let mut buttons = Buttons::new();
    let mut held = || PlayerInputs::single(buttons.tick(SQUARE));
    let mut before = 0;
    while before == 0 {
        before = cues_over(&mut race, Cue::Cannon, 1, &mut held);
    }
    assert_eq!(before, 1);
    assert!(
        race.sim.world.ships[0].pickup.cannon_rounds > 0,
        "a spent magazine"
    );
    down(&mut race);
    let during = cues_over(&mut race, Cue::Cannon, 20, &mut held);
    respawn(&mut race);
    let after = cues_over(&mut race, Cue::Cannon, 60, &mut held);
    assert_eq!((during, after), (0, 0), "the burst went on");
    assert_eq!(race.ship_pickup(), None, "the part-fired magazine survived");
}

fn quake_holder() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_quake_table());
    let mut race = Race::start(setup);
    race.tick(&PlayerInputs::none());
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Quake);
    race.sim.world.ships[0].pickup.quake_armed = true;
    race
}

#[test]
fn an_armed_quake_fires_for_a_racing_craft() {
    let mut race = quake_holder();
    race.tick(&PlayerInputs::none());
    assert_eq!(race.ship_pickup(), None, "the control did not fire");
}

#[test]
fn an_armed_quake_cut_short_by_destruction_does_not_fire_after_the_respawn() {
    let mut race = quake_holder();
    down(&mut race);
    for _ in 0..20 {
        race.tick(&PlayerInputs::none());
    }
    respawn(&mut race);
    for _ in 0..20 {
        race.tick(&PlayerInputs::none());
    }
    assert!(race.sim.world.quake.is_none(), "the armed press fired");
    assert_eq!(
        race.ship_pickup(),
        Some(Weapon::Quake),
        "never fired, so kept"
    );
}

/// Rockets slot 1 puts in the air holding one, with the player pinned 60 ahead of
/// it; slot 1 is `state` until `resume_at`. The synthetic track never lets the
/// driver aim, so no positive control exists at this level: an opponent that
/// fires is covered by the Turbo control in `a_destroyed_opponent_uses_nothing`,
/// which goes through the same `spend_opponent_pickup` gate.
fn opponent_rockets(state: CraftState, resume_at: usize) -> usize {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(Arc::new(one_rocket_table()));
    for _ in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::none());
    }
    race.sim.world.ships[1].pickup.weapon = Some(Weapon::Rocket);
    race.sim.world.ships[1].physics.craft_state = state;
    race.sim.world.ships[1].physics.state_timer = 1000.0;
    let mut fired = 0;
    for tick in 0..600 {
        if tick == resume_at {
            race.sim.world.ships[1].physics.craft_state = CraftState::Racing;
        }
        let body = race.sim.world.ships[1].physics.body;
        race.sim.world.ships[0].physics.body.position = body.position + body.forward() * 60.0;
        race.tick(&PlayerInputs::none());
        if tick < resume_at {
            assert_eq!(rockets(&race, 1), 0, "a {state:?} opponent fired");
            assert_eq!(race.sim.world.ships[1].pickup.weapon, Some(Weapon::Rocket));
        }
        fired += rockets(&race, 1);
    }
    fired
}

#[test]
fn a_downed_opponent_keeps_its_rocket_and_fires_none() {
    for state in [CraftState::Destroyed, CraftState::Eliminated] {
        opponent_rockets(state, 100);
    }
}
