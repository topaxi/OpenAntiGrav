//! What a craft shows when a weapon hits it, absorbs a pickup or is wrecked, on the titles that
//! take those rules by inheritance: Pure (Pulse's `Ship_Damage` sparks and wreck node loop) and
//! 2048 and Omega (Wipeout HD's Cannon craft-hit spark).
//!
//! **`#[ignore]`d and never run in CI.** Each test plays the title's own effect files on the
//! title's own hull locators; a table entry dropped, or a loader that no longer reads the
//! locators, brings the count back to zero.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay hit_effects_inherit_ground_truth:: --run-ignored all
//! ```

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

const WARM_UP_TICKS: u64 = oag_race::COUNTDOWN_TICKS + 60;

fn started(image: &str) -> Option<race::Race> {
    let image = oag_testdata::image(image)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(snapshot(0)));
    }
    Some(race)
}

fn snapshot(extra: u32) -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(Button::Cross.bit());
    buttons.begin_frame(Button::Cross.bit() | extra);
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// Pure: a weapon hit throws Pulse's hull sparks from its own `Ship Collision Fx` locators, and a
/// wrecked craft throws the node blast and death sparks - and not the big blast, which Pure's
/// disc and executable do not author.
#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn pure_throws_hit_sparks_and_the_wreck_node_effects_and_no_big_blast() {
    let Some(mut race) = started("data/images/pure-psp-eu.chd") else {
        return;
    };
    race.force_weapon_hit(0);
    assert!(
        race.hit_sparks_started_for_tests() > 0,
        "a weapon hit threw no spark on Pure"
    );
    race.force_destroy(0);
    for _ in 0..90 {
        race.tick(&PlayerInputs::single(snapshot(0)));
    }
    assert!(
        race.wreck_fx_started_for_tests() >= 2,
        "a Pure wreck threw no node effect ({})",
        race.wreck_fx_started_for_tests()
    );
    assert_eq!(
        race.wreck_explosion_at_for_tests(),
        None,
        "Pure threw the big blast it does not author"
    );
}

/// A Cannon round on a craft in front of the player, as `oag_raceplay`'s unit test lays it, on
/// the title's real hulls: the number of craft-hit sparks started.
fn cannon_craft_hit_sparks(image: &str) -> Option<u32> {
    let mut race = started(image)?;
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Cannon);
    for _ in 0..90 {
        race.sim.world.ships[0].pickup.weapon = Some(Weapon::Cannon);
        let forward = race.sim.world.ships[0].physics.body.forward();
        let ahead = race.sim.world.ships[0].physics.body.position + forward * 15.0;
        race.sim.world.ships[1].physics.body.position = ahead;
        race.sim.world.ships[1].physics.body.linear_velocity = oag_core::math::Vec3::ZERO;
        race.tick(&PlayerInputs::single(snapshot(Button::Square.bit())));
    }
    Some(race.weapon_sparks_started_for_tests())
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_2048_cannon_sparks_the_struck_craft_by_hds_rule() {
    let Some(count) = cannon_craft_hit_sparks("data/extracted/vita/PCSF00007") else {
        return;
    };
    assert!(count > 0, "a 2048 Cannon hit threw no weapon spark");
}

#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn omega_cannon_sparks_the_struck_craft_by_hds_rule() {
    let Some(count) = cannon_craft_hit_sparks("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    assert!(count > 0, "an Omega Cannon hit threw no weapon spark");
}
