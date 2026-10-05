//! The Repulser in a race: the press, the cue, the hit and the swept Mine.
//! The law itself is `oag_weapons::projectile::repulser`'s own tests.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_sound::sfx::Cue;

/// Two craft on the fixture's straight, the target `ahead` units down the
/// firer's nose, both held still. The button is pressed on tick 1.
fn fire_at(ahead: f32, ticks: usize) -> (Race, std::collections::BTreeSet<Cue>) {
    run(ahead, ticks, true, None)
}

/// [`fire_at`], optionally without the press, optionally with a Mine laid
/// `mine` units down the nose before the first tick.
fn run(
    ahead: f32,
    ticks: usize,
    press: bool,
    mine: Option<f32>,
) -> (Race, std::collections::BTreeSet<Cue>) {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_repulser_table());
    // The shared straight authors no AI corridor, and a wave's sweep is bounded
    // by it (`RepulserPool_SweepTargets`): give it one, 24 wide.
    for path in &mut setup.ai.paths {
        for point in &mut path.points {
            point.ai_bound_left = -12.0;
            point.ai_bound_right = 12.0;
        }
    }
    setup.course = Course::from_track(&setup.ai, None);
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    for ship in &mut race.sim.world.ships[..2] {
        ship.handling.dimensions.shield = 100.0;
        ship.physics.shield = 100.0;
    }
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Repulser);
    if let Some(distance) = mine {
        let forward = race.sim.world.ships[0].physics.body.forward();
        assert!(race.sim.world.projectiles.lay(
            oag_tables::weapons::Weapon::Mine,
            forward * distance,
            0,
            100.0,
            oag_core::math::Quat::IDENTITY,
        ));
    }
    let mut buttons = Buttons::new();
    let mut raised = std::collections::BTreeSet::new();
    for tick in 0..ticks {
        race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
        race.sim.world.ships[0].physics.body.linear_velocity = Vec3::ZERO;
        let forward = race.sim.world.ships[0].physics.body.forward();
        race.sim.world.ships[1].physics.body.position = forward * ahead;
        race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
        // Past the countdown, so `Ship_Damage`'s state gate lets a hit through.
        for ship in &mut race.sim.world.ships[..2] {
            ship.physics.craft_state = oag_physics::CraftState::Racing;
        }
        let snapshot = buttons.tick(if press && tick == 1 { SQUARE } else { 0 });
        race.tick(&PlayerInputs::single(snapshot));
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    (race, raised)
}

pub(super) fn repulser_hits_a_craft() -> std::collections::BTreeSet<Cue> {
    fire_at(15.0, 60).1
}

#[test]
fn pressing_fire_spends_the_repulser_and_raises_repulsor_on_the_firer() {
    let (race, raised) = fire_at(15.0, 3);
    assert!(race.sim.world.ships[0].pickup.weapon.is_none());
    assert!(raised.contains(&Cue::Repulsor), "{raised:?}");
    assert!(race.sim.world.repulsers.iter().flatten().count() == 1);
}

#[test]
fn a_wave_hits_the_craft_ahead_once_with_full_damage() {
    let (race, raised) = fire_at(15.0, 60);
    assert!(raised.contains(&Cue::RepulsorHit), "{raised:?}");
    let target = &race.sim.world.ships[1];
    let full = target.handling.dimensions.shield;
    assert_eq!(
        full - target.physics.shield,
        74.0,
        "one hit of the authored damage"
    );
    // It retires at blast_time + wave_time.
    assert!(race.sim.world.repulsers.iter().all(Option::is_none));
}

#[test]
fn the_firer_is_never_hit_by_its_own_waves() {
    let (race, _) = fire_at(15.0, 60);
    let firer = &race.sim.world.ships[0];
    assert_eq!(firer.physics.shield, firer.handling.dimensions.shield);
}

fn mines_left(race: &Race) -> usize {
    race.sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(oag_tables::weapons::Weapon::Mine))
        .count()
}

/// `RepulserPool_SweepTargets` raises a swept Mine's destroy bit. The control
/// run, with no press, keeps it.
#[test]
fn a_wave_sets_off_a_laid_mine_it_sweeps() {
    let (kept, _) = run(200.0, 60, false, Some(30.0));
    assert_eq!(mines_left(&kept), 1, "the control run keeps its mine");
    let (swept, _) = run(200.0, 60, true, Some(30.0));
    assert_eq!(mines_left(&swept), 0, "the wave set the mine off");
}

/// The field model draws from the first tick to the slot's retirement, at the
/// firer, fading in from `0.2`. Dropping the field leaves no draw at all.
#[test]
fn the_field_model_draws_at_the_firer_for_the_repulsers_whole_life() {
    let (race, _) = fire_at(200.0, 4);
    let draws: Vec<_> = race.repulser_field_draws().into_iter().flatten().collect();
    assert_eq!(draws.len(), 1, "one live field");
    let (matrix, alpha, _) = draws[0];
    let firer = race.sim.world.ships[0].physics.body.position;
    assert!((matrix.w_axis.truncate() - firer).length() < 1e-4);
    // Fired on tick 1, stepped on ticks 1-3: 1 - 0.8^3.
    assert!((alpha - 0.488).abs() < 1e-4, "{alpha}");
    let (gone, _) = fire_at(200.0, 120);
    assert!(gone.repulser_field_draws().iter().all(Option::is_none));
}

/// The field model's texture track plays on the Repulser's age: `Repulser_Init`
/// seeds the model with `Node_SetAnimTimeTree(0.0)` (`0x08875324`) and the mesh
/// update then integrates the clock's delta, so the time is the age at rate 1.
/// Measured live on PPSSPP: the one fresh mesh started at `0.000` on the fire
/// frame, ran at slope `1.0000` against the race clock and stopped at 1.569 s,
/// the entity's own `blast_time + wave_time`. The draw carries that age.
#[test]
fn the_field_models_texture_plays_on_the_repulsers_age() {
    let (early, _) = fire_at(200.0, 4);
    let (later, _) = fire_at(200.0, 34);
    let age = |race: &Race| {
        let draws: Vec<_> = race.repulser_field_draws().into_iter().flatten().collect();
        assert_eq!(draws.len(), 1, "one live field");
        draws[0].2
    };
    let step = age(&later) - age(&early);
    assert!(
        (step - 30.0 / 60.0).abs() < 1e-3,
        "thirty ticks later the age is half a second further on, not {step}"
    );
    assert!(age(&early) > 0.0 && age(&early) < 0.1, "{}", age(&early));
}
