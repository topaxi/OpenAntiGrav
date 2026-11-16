//! An AI craft holding a mod's pickup: the engine raises `pickup-fire` for it wherever the
//! driver would fire a Turbo, unless the mod's manifest says the module drives its own AI
//! decision (`[extension] drives_ai_pickup`), when the engine raises nothing.

use super::extensions::{
    grid,
    on::{module, prepared},
};
use super::*;
use oag_gameplay::PlayerInputs;
use oag_tables::pickup_look::OddsRow;
use oag_tables::weapons::{PickupOdds, Weapon};

/// A module that owns the mod pickup from tick 0 (it declares pad odds), driving its own AI
/// or not.
fn owner(drives_ai_pickup: bool) -> crate::extensions::Source {
    let mut source = module("test.owner", "i32.const 0");
    source.drives_ai_pickup = drives_ai_pickup;
    source.pickup_odds = vec![OddsRow {
        title: None,
        class: None,
        odds: PickupOdds {
            ai: 0.0,
            human: 0.0,
            front: 0.0,
            back: 0.0,
        },
    }];
    source
}

fn turbo_table() -> oag_tables::weapons::WeaponStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Turbo"><Stats absorb="23" time="0.75"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse")
}

/// Whether the AI craft in slot 1, holding the mod pickup on a clear straight at full
/// throttle, is raised a `pickup-fire` this tick.
fn fire_raised(drives_ai_pickup: bool) -> bool {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(std::sync::Arc::new(turbo_table()));
    race.sim.extensions = crate::extensions::Extensions::new(prepared(&[owner(drives_ai_pickup)]));
    let slot = 1;
    assert!(race.sim.world.ships[slot].active, "no opponent on the grid");
    race.sim.world.ships[slot].pickup.weapon =
        Some(Weapon::Extension(oag_tables::weapons::ModKind::FIRST));
    let controls = oag_physics::ShipControls {
        thrust: 1.0,
        ..oag_physics::ShipControls::default()
    };
    race.spend_opponent_pickup(slot, &controls, &oag_ai::Field::EMPTY);
    race.sim.extensions.fire_raised(slot)
}

#[test]
fn the_engine_raises_pickup_fire_for_an_ai_craft_through_the_turbo_gate_by_default() {
    assert!(fire_raised(false));
}

#[test]
fn a_module_that_drives_its_own_ai_is_raised_nothing() {
    assert!(!fire_raised(true));
}

/// A module that copies the course length (state bytes 0..4) and slot 1's and the empty
/// slot 15's places (bytes 4 and 5) out of its observation.
fn tracker() -> crate::extensions::Source {
    let place = |slot: usize| {
        1024 + oag_ext_abi::TRACKING_AT
            + slot * oag_ext_abi::TRACKING_LEN
            + oag_ext_abi::TRACKING_PLACE
    };
    module(
        "test.tracker",
        &format!(
            "(i32.store (i32.const {state}) (i32.load (i32.const {length})))
             (i32.store8 (i32.const {s4}) (i32.load8_u (i32.const {one})))
             (i32.store8 (i32.const {s5}) (i32.load8_u (i32.const {empty})))
             i32.const 0",
            state = super::extensions::on::STATE,
            s4 = super::extensions::on::STATE + 4,
            s5 = super::extensions::on::STATE + 5,
            length = 1024 + oag_ext_abi::COURSE_LENGTH_AT,
            one = place(1),
            empty = place(15),
        ),
    )
}

#[test]
fn a_module_reads_the_course_length_and_the_places_the_hud_shows() {
    let mut race = grid(prepared(&[tracker()]));
    let before = race.places();
    race.tick(&PlayerInputs::none());
    let blob = race.sim.extensions.blob_for_tests(0);
    let length = f32::from_bits(u32::from_le_bytes(blob[..4].try_into().unwrap()));
    let expected = race
        .sim
        .course
        .as_ref()
        .map_or(0.0, |course| course.length());
    assert_eq!(length, expected, "the course the lap counter uses");
    assert_eq!(
        blob[4], before[1],
        "the HUD's place before the step, one ordering"
    );
    assert_eq!(blob[5], 0, "an empty slot has no place");
}
