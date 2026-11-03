//! A mod's `[[pickup.odds]]`: the weapon pads draw its pickup beside the title's weapons,
//! at the most specific row's weights for the race's class, until the declaring module
//! faults.

use super::extensions::on::{module, prepared};
use super::*;
use oag_gameplay::PlayerInputs;
use oag_tables::pickup_look::OddsRow;
use oag_tables::weapons::{PickupOdds, Weapon};

/// A module that does nothing, whose mod weighs its pickup at `weight` in Venom.
fn odds_module(weight: f32) -> crate::extensions::Source {
    let mut source = module("test.odds", "i32.const 0");
    source.pickup_odds = vec![OddsRow {
        title: None,
        class: Some("venom".to_string()),
        odds: PickupOdds {
            ai: weight,
            human: weight,
            front: 0.0,
            back: 0.0,
        },
    }];
    source
}

#[test]
fn a_pad_hands_out_the_mods_pickup_at_its_odds() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    race.sim.extensions = crate::extensions::Extensions::new(prepared(&[odds_module(1.0e6)]));
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.ship_pickup(),
        Some(Weapon::Extension(oag_tables::weapons::ModKind::FIRST))
    );
}

#[test]
fn a_zero_weight_or_another_class_hands_out_the_titles_weapon() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    race.sim.extensions = crate::extensions::Extensions::new(prepared(&[odds_module(0.0)]));
    race.tick(&PlayerInputs::none());
    assert_eq!(race.ship_pickup(), Some(Weapon::Turbo));
    let extensions = crate::extensions::Extensions::new(prepared(&[odds_module(3.0)]));
    assert_eq!(
        extensions.pickup_odds("VENOM").first().map(|(_, o)| o.ai),
        Some(3.0)
    );
    assert!(extensions.pickup_odds("FLASH").is_empty());
}

/// A module that faults stops owning the pickup, so the pads stop handing it out.
#[test]
fn the_odds_end_when_their_module_faults() {
    let mut race = race_with_weapon_pads(Mode::SingleRace, enveloping_pad(), 1.0);
    race.sim.extensions = crate::extensions::Extensions::new(prepared(&[odds_module(1.0e6)]));
    assert!(!race.sim.extensions.pickup_odds("VENOM").is_empty());
    crate::extensions::fault_for_tests(&mut race, 0);
    assert!(race.sim.extensions.pickup_odds("VENOM").is_empty());
    race.tick(&PlayerInputs::none());
    assert_eq!(race.ship_pickup(), Some(Weapon::Turbo));
}
