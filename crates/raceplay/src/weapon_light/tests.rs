//! Unit tests for [`super`]: the Missile's law at its ends, the Rocket's
//! literals, and that a live blast or rocket reaches the built list.

use super::*;
use crate::tests::race_with_a_grid;
use oag_core::math::Quat;

fn hd_race() -> Race {
    let mut race = race_with_a_grid();
    race.view.hd_missile_blast = true;
    race.view.hd_trail_active = true;
    race
}

/// At birth `f = 1`: `D = 150`, `w = 8.5`, the seeded colour.
#[test]
fn a_new_missile_light_is_a_hundred_and_fifty_wide_and_steep() {
    let light = missile_record(Vec3::new(1.0, 2.0, 3.0), 0.0).unwrap();
    assert_eq!(light.position, [1.0, 2.0, 3.0, 8.5]);
    assert_eq!(light.colour, [500.0, 200.0, 50.0, 150.0]);
}

/// Half way `f = 0.25`: the range and the exponent both follow `(1 - p)^2`.
#[test]
fn the_missile_light_dies_quadratically() {
    let light = missile_record(Vec3::ZERO, 0.5).unwrap();
    assert_eq!(light.colour[3], 37.5);
    assert_eq!(light.position[3], 3.25);
}

/// The entry is freed at `p = 1`, and the light with it.
#[test]
fn a_spent_missile_blast_lights_nothing() {
    assert!(missile_record(Vec3::ZERO, LIFETIME_SECONDS).is_none());
    assert!(missile_record(Vec3::ZERO, -0.1).is_none());
}

/// `D = 50`, `w = 1`, `(7, 5, 1)`.
#[test]
fn the_rocket_light_is_the_literals_the_update_loads() {
    let light = rocket_record(Vec3::new(4.0, 5.0, 6.0));
    assert_eq!(light.position, [4.0, 5.0, 6.0, 1.0]);
    assert_eq!(light.colour, [7.0, 5.0, 1.0, 50.0]);
}

/// Dropping the wiring from the built list fails this.
#[test]
fn a_live_missile_blast_reaches_the_frame_list() {
    let mut race = hd_race();
    let before = race.hd_spu_lights().len();
    race.view
        .spawn_hd_missile_blast(Vec3::new(7.0, 8.0, 9.0), Quat::IDENTITY);
    let lights = race.hd_spu_lights();
    assert_eq!(lights.len(), before + 1);
    let light = lights.last().unwrap();
    assert_eq!(light.position, [7.0, 8.0, 9.0, 8.5]);
}

/// Off HD, or with the circuit's switch off, the weapons light nothing.
#[test]
fn the_circuits_switch_silences_the_weapons_too() {
    let mut race = hd_race();
    race.view.spawn_hd_missile_blast(Vec3::ZERO, Quat::IDENTITY);
    race.view.spu_vertex_lights = false;
    assert!(race.hd_spu_lights().is_empty());
    race.view.spu_vertex_lights = true;
    race.view.hd_trail_active = false;
    assert!(race.hd_spu_lights().is_empty());
}

/// The emulated update's samples: `x` is 1 until 0.1 s, `0.857` at 0.2 s,
/// `0` at 0.8 s, `0.7311` at 1.0 s, `1` at 1.5 s, `0.64` at 1.6 s.
#[test]
fn the_bomb_envelope_matches_the_emulated_update() {
    for (age, x) in [
        (0.05, 1.0),
        (0.2, 0.8571),
        (0.8, 0.0),
        (1.0, 0.7311),
        (1.4, 0.9622),
        (1.6, 0.64),
        (2.1, 0.0),
    ] {
        let got = bomb_envelope(age).0;
        assert!((got - x).abs() < 1e-3, "age {age}: {got} vs {x}");
    }
}

/// At birth: both lights three units up, ranges 40 and 100, `w = 1.5`.
#[test]
fn a_new_bomb_blast_lights_two_balls_three_units_up() {
    let [one, two] = bomb_records(Vec3::new(1.0, 0.0, 2.0), Vec3::Y, 0.0);
    assert_eq!(one.position, [1.0, 3.0, 2.0, 1.5]);
    assert_eq!(one.colour, [500.0, 200.0, 50.0, 40.0]);
    assert_eq!(two.colour, [20.0, 5.0, 0.5, 100.0]);
}

/// Dropping the wiring from the built list fails this.
#[test]
fn a_live_bomb_blast_reaches_the_frame_list() {
    let mut race = hd_race();
    race.view.hd_bomb_blast = true;
    let before = race.hd_spu_lights().len();
    race.view.hd_bomb_blasts[0] = Some(crate::bomb_blast::hd::HdBlast::new(Vec3::ZERO, Vec3::Y));
    assert_eq!(race.hd_spu_lights().len(), before + 2);
}

/// `--no-weapon-lights` leaves the engines' lights and drops the weapons'.
#[test]
fn the_verification_switch_drops_the_weapons_alone() {
    let mut race = hd_race();
    let engines = race.hd_engine_lights().len();
    race.view.spawn_hd_missile_blast(Vec3::ZERO, Quat::IDENTITY);
    race.set_weapon_lights(false);
    assert_eq!(race.hd_spu_lights().len(), engines);
    race.set_weapon_lights(true);
    assert_eq!(race.hd_spu_lights().len(), engines + 1);
}

/// The circuit's `Enable spu vertex lights = 0` (Sol 2, Metropia) silences the track's
/// lights and not the hull's: the original's hull reads a blue light buffer there. Dropping
/// `hd_craft_engine_lights` from the craft's scene fails this.
#[test]
fn the_circuits_switch_does_not_reach_the_hull() {
    use oag_livery::engine_light::EngineLight;
    use oag_tables::enginelight::EngineLightData;
    let oag_title::flare::Flare::PerTeam(authored) = oag_hd::TITLE.flare else {
        panic!("HD authors a per-team flare");
    };
    let mut race = hd_race();
    race.view.engine_lights = vec![Some(EngineLight {
        law: *authored.engine_light.expect("HD's engine light"),
        era_2048: false,
        strip_circuit: false,
        data: EngineLightData {
            distance: 0.0,
            radius: 1.0,
        },
        axis: Vec3::Z,
    })];
    race.view.nozzles = vec![Some(Vec3::ZERO)];
    race.view.spu_vertex_lights = false;
    assert!(race.hd_engine_lights().is_empty());
    assert!(race.hd_spu_lights().is_empty());
    let craft = race.hd_craft_spu_lights();
    assert_eq!(craft.len(), 1, "the player's craft keeps its engine light");
    race.view.hd_trail_active = false;
    assert!(
        race.hd_craft_spu_lights().is_empty(),
        "off HD there is none"
    );
}

/// A craft over a mag strip on a gated circuit takes the strip's colour; leaving the strip
/// returns the skin's. Reading `mag_floor_fx[slot].on` is the wiring: a craft that never
/// reads it keeps its skin colour on the strip and fails this. Radius 2 keeps Omega's wander
/// of one unit off the range floor.
#[test]
fn a_craft_over_a_mag_strip_takes_the_strips_colour() {
    use oag_livery::engine_light::EngineLight;
    use oag_tables::enginelight::EngineLightData;
    let oag_title::flare::Flare::PerTeam(authored) = oag_omega::TITLE.flare else {
        panic!("Omega authors a per-team flare");
    };
    let mut race = hd_race();
    race.view.engine_lights = vec![Some(EngineLight {
        law: *authored.engine_light.expect("Omega's engine light"),
        era_2048: false,
        strip_circuit: true,
        data: EngineLightData {
            distance: 0.0,
            radius: 2.0,
        },
        axis: Vec3::Z,
    })];
    race.view.nozzles = vec![Some(Vec3::ZERO)];
    let rgb = |race: &Race| {
        let lights = race.hd_craft_engine_lights();
        let light = lights.first().expect("the player's craft has a light");
        [light.colour[0], light.colour[1], light.colour[2]]
    };
    assert_eq!(
        rgb(&race),
        [4.0, 10.0, 40.0],
        "off the strip: the classic skin"
    );
    race.view.mag_floor_fx[0].on = true;
    assert_eq!(
        rgb(&race),
        [1.5, 0.1, 3.0],
        "over the strip: the strip's colour"
    );
    race.view.mag_floor_fx[0].on = false;
    assert_eq!(
        rgb(&race),
        [4.0, 10.0, 40.0],
        "leaving it restores the skin's"
    );
}

fn firing_race() -> (Race, crate::CannonDraw) {
    let mut race = hd_race();
    race.view.spu_vertex_lights = true;
    let rounds = &mut race.core.sim.world.detonator.cannon.rounds;
    for (index, round) in rounds.iter_mut().take(2).enumerate() {
        round.live = true;
        round.age = 0.0;
        round.slot = index as u8;
        round.pos = Vec3::new(10.0 * (index as f32 + 1.0), 0.0, 0.0);
    }
    let muzzles = vec![[
        Some(oag_core::math::Mat4::from_translation(Vec3::new(
            -1.0, 0.0, 2.0,
        ))),
        Some(oag_core::math::Mat4::from_translation(Vec3::new(
            1.0, 0.0, 2.0,
        ))),
    ]];
    let draw = crate::CannonDraw {
        look: None,
        muzzles,
    };
    (race, draw)
}

/// `(0, 30, 40)`, `D = 2.7`, `w = 1` at the muzzle; `(0, 0.003, 0.03)`, `D = 70`, `w = 1` on the
/// round: the colour table's and the constants' own values.
#[test]
fn the_detonator_lights_are_the_values_the_render_queue_loads() {
    let flash = detonator_flash_record(Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(flash.position, [1.0, 2.0, 3.0, 1.0]);
    assert_eq!(flash.colour, [0.0, 30.0, 40.0, 2.7]);
    let bolt = detonator_bolt_record(Vec3::new(4.0, 5.0, 6.0));
    assert_eq!(bolt.position, [4.0, 5.0, 6.0, 1.0]);
    assert_eq!(bolt.colour, [0.0, 0.003, 0.03, 70.0]);
}

/// A young round lights its muzzle and its flight, an old one only its flight; dropping the wiring
/// or the age gate fails this.
#[test]
fn a_detonator_round_lights_its_muzzle_while_young() {
    let (mut race, draw) = firing_race();
    assert_eq!(race.detonator_cannon_lights(&draw).len(), 4);
    for round in race
        .core
        .sim
        .world
        .detonator
        .cannon
        .rounds
        .iter_mut()
        .take(2)
    {
        round.age = 0.2;
    }
    let lights = race.detonator_cannon_lights(&draw);
    assert_eq!(lights.len(), 2);
    assert!(lights.iter().all(|light| light.colour[3] == 70.0));
}

/// The weapons' switches silence the cannon's lights as they do every weapon light.
#[test]
fn the_detonator_lights_follow_the_weapon_light_switches() {
    let (mut race, draw) = firing_race();
    race.set_weapon_lights(false);
    assert!(race.detonator_cannon_lights(&draw).is_empty());
    race.set_weapon_lights(true);
    race.view.spu_vertex_lights = false;
    assert!(race.detonator_cannon_lights(&draw).is_empty());
}

/// `PulseBullet_Update` rolls the flash every tick: its size over `[0.25, 1]` on all three axes,
/// its third axis a further `[0.7, 1.3]`, its facing a turn about that axis. The two rounds roll
/// apart, and a tick later each rolls again.
#[test]
fn the_detonator_flash_is_rolled_every_tick_and_apart_per_round() {
    let (mut race, draw) = firing_race();
    let first = race.detonator_flash_matrices(&draw);
    assert_eq!(first.len(), 2);
    assert_ne!(
        first[0].x_axis, first[1].x_axis,
        "left and right roll apart"
    );
    let mut sizes = Vec::new();
    let mut stretches = Vec::new();
    for tick in 0..64 {
        race.core.sim.world.tick = tick;
        for matrix in race.detonator_flash_matrices(&draw) {
            let (x, y, z) = (
                matrix.x_axis.truncate().length(),
                matrix.y_axis.truncate().length(),
                matrix.z_axis.truncate().length(),
            );
            assert!((x - y).abs() < 1e-5, "one size on the first two axes");
            assert!(
                matrix.x_axis.truncate().dot(matrix.z_axis.truncate()).abs() < 1e-4,
                "the turn is about the third axis"
            );
            assert!((0.25..=1.0).contains(&x), "size {x}");
            let stretch = z / x;
            assert!((0.7..=1.3).contains(&stretch), "stretch {stretch}");
            sizes.push(x);
            stretches.push(stretch);
        }
    }
    let spread = |v: &[f32]| {
        v.iter().copied().fold(f32::MIN, f32::max) - v.iter().copied().fold(f32::MAX, f32::min)
    };
    assert!(spread(&sizes) > 0.5, "the size covers its range");
    assert!(spread(&stretches) > 0.4, "the stretch covers its range");
}
