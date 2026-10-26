//! The static a weapon hit throws over the player's cockpit view - see
//! `race::cockpit_hit`.
//!
//! Hand-laid noise and locators, no disc: what is asserted is the wiring (a
//! weapon hit on the player arms the node, a rival's does not, a title with no
//! noise holds none, the draw is the internal view's alone, a run is 36 ticks of
//! `Race::tick`), and that a rival's Quake reaches it.

use super::*;
use oag_display::display::CameraView;
use oag_gameplay::PlayerInputs;
use oag_weapons::projectile::WeaponHit;

fn race_with_noise(noise: bool) -> Race {
    let mut handling = hulled_handling();
    handling.dimensions.shield = 100.0;
    let mut setup = setup(handling);
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_quake_table());
    if noise {
        setup.cockpit_hit_noise = Some(oag_fx::exhaust::FlareTexture::placeholder(64));
    }
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.tick(&PlayerInputs::none());
    race
}

fn landed_on(slot: usize) -> [WeaponHit; MAX_SHIPS] {
    let mut hits = [WeaponHit::default(); MAX_SHIPS];
    hits[slot].landed = true;
    hits
}

/// Only a hit on the player's craft arms it, and only where the title loaded
/// the noise: the shake's own gate, `+0x368 == 0`.
#[test]
fn a_weapon_hit_on_the_player_arms_the_static_and_a_rivals_does_not() {
    let mut race = race_with_noise(true);
    race.throw_hit_sparks(&landed_on(1), false);
    assert!(
        !race.cockpit_hit_running_for_tests(),
        "a rival's hit armed it"
    );
    race.throw_hit_sparks(&landed_on(0), false);
    assert!(race.cockpit_hit_running_for_tests());

    let mut bare = race_with_noise(false);
    bare.throw_hit_sparks(&landed_on(0), false);
    assert!(
        !bare.cockpit_hit_running_for_tests(),
        "a title with no noise armed a static"
    );
    assert!(bare.cockpit_hit_noise().is_none());
}

/// The node runs in every camera view and only the internal one draws it.
#[test]
fn the_static_draws_in_the_internal_view_alone() {
    let mut race = race_with_noise(true);
    race.force_weapon_hit(0);
    race.tick(&PlayerInputs::none());
    assert!(race.cockpit_hit_running_for_tests());
    assert_eq!(race.view.camera_view, CameraView::Close);
    assert!(
        race.cockpit_hit_strips().is_none(),
        "the chase view drew the cockpit static"
    );
    race.view.camera_view = CameraView::Internal;
    assert!(race.cockpit_hit_strips().is_some());
    race.view.camera_view = CameraView::Far;
    assert!(race.cockpit_hit_strips().is_none());
}

/// A run is 36 ticks of the race's own step, 0.6 s, and a second hit inside it
/// neither restarts nor stacks it.
#[test]
fn a_run_lasts_thirty_six_ticks_and_a_second_hit_inside_it_changes_nothing() {
    let mut race = race_with_noise(true);
    race.view.camera_view = CameraView::Internal;
    race.force_weapon_hit(0);
    let mut drawn = 0;
    for tick in 0..60 {
        race.tick(&PlayerInputs::none());
        if tick == 10 {
            race.force_weapon_hit(0);
        }
        drawn += usize::from(race.cockpit_hit_strips().is_some());
    }
    assert_eq!(drawn, 36);
    assert!(!race.cockpit_hit_running_for_tests());
}

/// The wiring end to end: a rival's Quake reaching the player throws the
/// static, through the same `Ship_Damage` branch as the sparks.
#[test]
fn a_quake_that_reaches_the_player_throws_the_static() {
    let mut race = race_with_noise(true);
    race.view.camera_view = CameraView::Internal;
    assert!(race.cockpit_hit_strips().is_none());
    assert!(race.force_quake(1));
    let mut seen = false;
    for _ in 0..10 {
        race.tick(&PlayerInputs::none());
        seen |= race.cockpit_hit_strips().is_some();
    }
    assert!(seen, "the Quake hit and the cockpit showed no static");
}

/// An extra seat's view is a chase camera and wears none of the player's static.
#[test]
fn another_seats_view_wears_none() {
    let mut race = race_with_noise(true);
    race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
    assert_eq!(race.set_viewport_count(2), 2);
    race.view.camera_view = CameraView::Internal;
    race.force_weapon_hit(0);
    race.tick(&PlayerInputs::none());
    assert!(race.cockpit_hit_strips().is_some());
    race.set_viewpoint(1);
    assert!(race.cockpit_hit_strips().is_none());
    race.reset_viewpoint();
    assert!(race.cockpit_hit_strips().is_some());
}

/// Which titles bind the noise: Pulse measured, Pure by Pulse's law, and none of
/// HD, 2048 or Omega (no `noisematt` string in HD's or 2048's executable, and
/// Omega follows HD). A title that binds none requests nothing and draws nothing.
#[test]
fn only_pulse_and_pure_bind_the_cockpit_hit_noise() {
    let entry = |title: &oag_title::Title| title.weapon_models.cockpit_hit.map(|hit| hit.entry);
    assert_eq!(entry(oag_pulse::TITLE), Some(r"Data\Tex\noisematt.mip"));
    assert_eq!(entry(oag_pure::TITLE), Some(r"Data\Tex\noisematt.mip"));
    for other in [oag_hd::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(entry(other), None, "{} binds a cockpit static", other.name);
    }
    let origin = |title: &oag_title::Title| title.weapon_models.cockpit_hit.map(|hit| hit.origin);
    assert_eq!(origin(oag_pulse::TITLE), Some(oag_title::Origin::Measured));
    assert_eq!(
        origin(oag_pure::TITLE),
        Some(oag_title::Origin::InheritedFrom("Wipeout Pulse"))
    );
}
