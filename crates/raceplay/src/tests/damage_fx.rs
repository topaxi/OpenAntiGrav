//! HD's damage smoke - see `race::damage_fx`.
//!
//! Hand-laid one-emitter effects, no disc: what is asserted is the state law
//! (70 and 40), the 3.3 s hold and what leaves the smoke unplayed. The
//! disc-backed half is `crates/game/tests/damage_fx_ground_truth.rs`.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_title::Trigger::{DamageCritical, DamageMild, DamageModerate};
use oag_weapons::projectile::WeaponHit;

fn race_with_smoke(smoke: bool) -> Race {
    let mut handling = hulled_handling();
    handling.dimensions.shield = 100.0;
    let mut setup = setup(handling);
    if smoke {
        for trigger in [DamageMild, DamageModerate, DamageCritical] {
            let blob = super::respawn::one_emitter_pob(super::respawn::trigger_name(trigger), 0);
            let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
                .expect("the hand-laid effect parses");
            setup.handles.insert(trigger, effect);
        }
    }
    Race::start(setup)
}

fn hit_leaving(race: &mut Race, shield: f32, leach: bool) {
    race.sim.world.ships[0].physics.shield = shield;
    let mut hits = [WeaponHit::default(); MAX_SHIPS];
    hits[0].landed = true;
    race.throw_hit_sparks(&hits, leach);
}

#[test]
fn the_shield_left_picks_which_smoke_plays() {
    let mut race = race_with_smoke(true);
    hit_leaving(&mut race, 90.0, false);
    assert_eq!(race.damage_smokes_started_for_tests(), 1, "mild");

    let ticks = (crate::damage_fx::HOLD / race.dt()) as usize + 2;
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
    }
    hit_leaving(&mut race, 55.0, false);
    assert_eq!(race.damage_smokes_started_for_tests(), 2, "moderate");

    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
    }
    hit_leaving(&mut race, 20.0, false);
    assert_eq!(
        race.damage_smokes_started_for_tests(),
        4,
        "critical plays WO_DAMAGE_CRITICAL and WO_DAMAGE_MODERATE"
    );
}

#[test]
fn a_repeat_inside_the_hold_is_silent_and_a_change_is_not() {
    let mut race = race_with_smoke(true);
    hit_leaving(&mut race, 90.0, false);
    hit_leaving(&mut race, 85.0, false);
    assert_eq!(race.damage_smokes_started_for_tests(), 1, "same state");
    hit_leaving(&mut race, 65.0, false);
    assert_eq!(race.damage_smokes_started_for_tests(), 2, "state changed");

    let ticks = (crate::damage_fx::HOLD / race.dt()) as usize + 2;
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
    }
    hit_leaving(&mut race, 64.0, false);
    assert_eq!(race.damage_smokes_started_for_tests(), 3, "hold ran out");
}

#[test]
fn a_spent_shield_a_leachbeam_and_a_title_without_the_smoke_play_nothing() {
    let mut race = race_with_smoke(true);
    hit_leaving(&mut race, 0.0, false);
    hit_leaving(&mut race, 90.0, true);
    assert_eq!(race.damage_smokes_started_for_tests(), 0);

    let mut bare = race_with_smoke(false);
    hit_leaving(&mut bare, 90.0, false);
    assert_eq!(bare.damage_smokes_started_for_tests(), 0);
}

#[test]
fn a_finished_smoke_gives_its_stage_slot_back() {
    let mut race = race_with_smoke(true);
    hit_leaving(&mut race, 90.0, false);
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.view.damage_fx.riding_len(), 0);
}

/// HD's LeachBeam hit throws one attached burst per hit and no smoke; a title
/// that has locators for its hit sparks (Pulse) keeps its own path.
#[test]
fn a_leachbeam_hit_throws_the_attached_spark_where_the_title_has_no_locators() {
    let mut race = race_with_smoke(false);
    let blob = super::respawn::one_emitter_pob(
        super::respawn::trigger_name(oag_title::Trigger::LeachHitSpark),
        0,
    );
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view
        .handles
        .insert(oag_title::Trigger::LeachHitSpark, effect);
    hit_leaving(&mut race, 90.0, true);
    assert_eq!(race.damage_smokes_started_for_tests(), 1);
    hit_leaving(&mut race, 90.0, true);
    assert_eq!(race.damage_smokes_started_for_tests(), 2, "no hold");
    hit_leaving(&mut race, 90.0, false);
    assert_eq!(race.damage_smokes_started_for_tests(), 2, "not a LeachBeam");
}
