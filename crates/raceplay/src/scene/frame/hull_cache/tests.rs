//! The blend of two ticks' hulls, against a craft that really moves.

use super::*;
use crate::tests::race_with_a_grid;
use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;

/// A grid gliding down the straight fixture past the countdown, so every craft
/// moves on every tick.
fn gliding_race() -> Race {
    let mut race = race_with_a_grid();
    for _ in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::none());
    }
    for ship in &mut race.sim.world.ships {
        ship.physics.body.linear_velocity = Vec3::new(30.0, 0.0, 3.0);
    }
    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
    }
    race
}

fn vertex(at: [f32; 3], strength: f32) -> GpuVertex {
    GpuVertex {
        position: at,
        normal: [0.0, 1.0, 0.0],
        colour: [0.0, 0.0, 0.0, strength],
        ..bytemuck::Zeroable::zeroed()
    }
}

/// One slot's gather with `vertices`, taken at `race`'s poses.
fn gather(race: &Race, slot: usize, vertices: Vec<GpuVertex>) -> Gather {
    Gather {
        generation: 0,
        models: (0..usize::from(race.ship_count()))
            .map(|slot| race.ship_tick_model(slot))
            .collect(),
        slots: vec![(slot, 0..vertices.len())],
        vertices,
    }
}

/// **A draw between two ticks lerps the hull's vertices and a draw at alpha one
/// is the current gather untouched.** Fails if the blend stops reading the draw
/// alpha, or if alpha one stops short-circuiting to the current tick (the
/// byte-identical capture guarantee).
#[test]
fn a_blend_lerps_the_vertices_and_alpha_one_is_the_current_gather() {
    let mut race = gliding_race();
    let before = gather(
        &race,
        0,
        vec![vertex([0.0, 0.0, 0.0], 0.4), vertex([2.0, 0.0, 4.0], 0.4)],
    );
    race.tick(&PlayerInputs::none());
    let now = gather(
        &race,
        0,
        vec![vertex([1.0, 0.0, 0.0], 0.8), vertex([3.0, 0.0, 8.0], 0.8)],
    );

    race.set_draw_alpha(0.25);
    let drawn = blend(&race, &before, &now, 0.25).expect("both ticks hold the hull");
    assert_eq!(drawn.len(), 2);
    assert_eq!(drawn[0].position, [0.25, 0.0, 0.0]);
    assert_eq!(drawn[1].position, [2.25, 0.0, 5.0]);
    assert!(
        (drawn[0].colour[3] - 0.5).abs() < 1e-6,
        "the strength fades with the blend"
    );
}

/// A craft that gained a hull over the tick, or whose previous gather is not
/// this tick's predecessor, cannot be blended at all.
#[test]
fn a_craft_with_no_matching_previous_hull_is_gathered_live() {
    let mut race = gliding_race();
    let before = gather(&race, 0, vec![vertex([0.0; 3], 0.4)]);
    race.tick(&PlayerInputs::none());
    let now = gather(
        &race,
        0,
        vec![vertex([8.0, 0.0, 0.0], 0.4), vertex([8.0, 0.0, 1.0], 0.4)],
    );
    assert!(
        blend(&race, &before, &now, 0.5).is_none(),
        "a different vertex count is a different hull: gathered live"
    );

    let empty = gather(&race, 1, Vec::new());
    assert!(
        blend(&race, &empty, &now, 0.5).is_none(),
        "the previous gather holds no hull for this slot: gathered live"
    );

    // A previous gather taken on some other tick is not this tick's predecessor.
    let mut stale = gather(&race, 0, vec![vertex([0.0; 3], 0.4), vertex([0.0; 3], 0.4)]);
    stale.models[0] = stale.models[0] * Mat4::from_translation(Vec3::X);
    assert!(blend(&race, &stale, &now, 0.5).is_none());
}

/// A craft that jumps past the snap distance in one tick is drawn where it lands,
/// as `Race::ship_model_matrix_of` does, not swept across the jump.
#[test]
fn a_teleporting_craft_draws_its_current_gather() {
    let mut race = gliding_race();
    let before = gather(&race, 0, vec![vertex([0.0; 3], 0.4)]);
    // A hundred units in one tick, past `SNAP_DISTANCE`.
    race.sim.world.ships[0].physics.body.linear_velocity = Vec3::new(6000.0, 0.0, 0.0);
    race.tick(&PlayerInputs::none());
    assert!(race.ship_swept_from(0).is_none());
    let now = gather(&race, 0, vec![vertex([100.0, 0.0, 0.0], 0.8)]);
    let drawn = blend(&race, &before, &now, 0.5).expect("nothing to sweep");
    assert_eq!(drawn[0].position, [100.0, 0.0, 0.0]);
    assert_eq!(drawn[0].colour[3], 0.8);
}
