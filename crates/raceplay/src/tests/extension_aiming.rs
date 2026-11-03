//! The aiming record: where each craft sits in each other craft's own frame, as a guest
//! reads it from the observation.

use super::extensions::grid;
use super::extensions::on::{STATE, module, prepared};
use oag_core::math::Vec3;
use oag_ext_abi::{AIM_AT, AIM_LEN, AIM_SLOTS, Aim};
use oag_gameplay::PlayerInputs;

/// Copies the aim row of observer `row` (every target's three bytes) into its state.
pub(super) fn copies_aim_row(id: &str, row: usize) -> crate::extensions::Source {
    let mut source = module(
        id,
        &format!(
            "(memory.copy (i32.const {STATE}) (i32.const {from}) (i32.const {len}))
             i32.const 0",
            from = 1024 + (AIM_AT + row * AIM_SLOTS * AIM_LEN) as i32,
            len = AIM_SLOTS * AIM_LEN,
        ),
    );
    source.state_bytes = (AIM_SLOTS * AIM_LEN) as u32;
    source
}

pub(super) fn row_of(state: &[u8]) -> Vec<Aim> {
    state
        .chunks_exact(AIM_LEN)
        .map(|c| Aim::decode(&[c[0], c[1], c[2]]))
        .collect()
}

#[test]
fn a_craft_ahead_reads_ahead_and_centred_one_beside_reads_across_and_the_view_is_per_craft() {
    let mut race = grid(prepared(&[copies_aim_row("test.aim", 0)]));
    assert!(race.sim.world.ships[1].active, "the fixture has a slot 1");
    for ship in &mut race.sim.world.ships {
        ship.physics.body.position = Vec3::ZERO;
        ship.physics.body.orientation = oag_core::math::Quat::IDENTITY;
    }
    // The identity body faces -Z with +X to its right and +Y above it.
    race.sim.world.ships[1].physics.body.position = Vec3::new(0.0, 0.0, -60.0);
    race.tick(&PlayerInputs::none());
    let row = row_of(&race.extension_state(0).unwrap());
    let ahead = row[1];
    assert!(ahead.forward > 0 && ahead.in_cone(200.0, 0.2), "{ahead:?}");
    assert_eq!((ahead.right, ahead.up), (0, 0), "centred");
    assert_eq!(row[0], Aim::default(), "itself is zero");

    // Across the track: 40 to the right and 4 ahead is no target in front.
    race.sim.world.ships[1].physics.body.position = Vec3::new(40.0, 0.0, -4.0);
    race.tick(&PlayerInputs::none());
    let beside = row_of(&race.extension_state(0).unwrap())[1];
    assert!(
        beside.right > 0 && !beside.in_cone(200.0, 0.2),
        "{beside:?}"
    );

    // Behind it, and turned to face the other way: the same offset is now ahead.
    race.sim.world.ships[1].physics.body.position = Vec3::new(0.0, 0.0, 60.0);
    race.tick(&PlayerInputs::none());
    assert!(row_of(&race.extension_state(0).unwrap())[1].forward < 0);
    race.sim.world.ships[0].physics.body.orientation =
        oag_core::math::Quat::from_rotation_y(std::f32::consts::PI);
    race.tick(&PlayerInputs::none());
    assert!(row_of(&race.extension_state(0).unwrap())[1].in_cone(200.0, 0.2));
}
