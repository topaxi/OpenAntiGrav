//! What a Quake's wave does to the craft it reaches: the smoothed wave height
//! `Ship::quake_lift`, the hit it gates, and the camera it lifts.
//!
//! The unit laws are in `oag_weapons::projectile::quake`'s tests; these are the
//! wiring of them through `Race::tick`.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;
use oag_title::Origin;
use oag_title::weapons::QuakeHit;
use oag_weapons::projectile::quake::{LAUNCH_LEAD, Wave};

fn quake_race() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_quake_table());
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.tick(&PlayerInputs::default());
    race
}

fn quake_hits(race: &mut Race) -> usize {
    race.drain_cues()
        .into_iter()
        .filter(|e| e.cue == Cue::QuakeHit && e.slot == 0)
        .count()
}

/// A wave the other craft fired, born so its bump arrives on slot 0 this tick.
fn wave_on_the_player(race: &Race, age: f32) -> Wave {
    let progress = race.sim.world.ships[0]
        .standing
        .progress
        .expect("the first tick locates the craft");
    let mut wave = Wave::launch(1, progress - LAUNCH_LEAD, 1.0, &Default::default());
    wave.damage = 5.0;
    wave.slowdown_time = 2.0;
    wave.age = age;
    wave
}

#[test]
fn a_wave_reaching_the_player_lifts_it_and_hits_once() {
    let mut race = quake_race();
    let _ = race.drain_cues();
    race.sim.world.quake = Some(wave_on_the_player(&race, 0.3));
    let mut hits = 0;
    let mut peak = 0.0f32;
    for _ in 0..3 {
        race.tick(&PlayerInputs::default());
        hits += quake_hits(&mut race);
        peak = peak.max(race.sim.world.ships[0].quake_lift);
    }
    assert!(peak > 0.1, "the wave never lifted the craft: {peak}");
    assert_eq!(hits, 1, "the hit lands once on the rising edge");
}

#[test]
fn the_lift_keeps_decaying_after_the_wave_is_gone() {
    let mut race = quake_race();
    // Born one tick short of its end, so it expires this tick and is dropped.
    let mut wave = wave_on_the_player(&race, 4.999);
    wave.owner = 1;
    race.sim.world.quake = Some(wave);
    race.sim.world.ships[0].quake_lift = 1.0;
    race.tick(&PlayerInputs::default());
    assert!(
        race.sim.world.quake.is_none(),
        "the wave should have expired"
    );
    let after_expiry = race.sim.world.ships[0].quake_lift;
    assert!(
        after_expiry > 0.0 && after_expiry < 1.0,
        "the lift froze or vanished with the wave: {after_expiry}"
    );
    race.tick(&PlayerInputs::default());
    assert!(race.sim.world.ships[0].quake_lift < after_expiry);
}

#[test]
fn the_cockpit_view_rises_with_the_lift() {
    let mut race = quake_race();
    race.view.camera_view = oag_display::display::CameraView::Internal;
    let level = race.tick_view_unshaken();
    race.sim.world.ships[0].quake_lift = 2.0;
    let lifted = race.tick_view_unshaken();
    assert_ne!(level, lifted, "the cockpit eye ignores the wave height");
}

/// What the player's cockpit looked like at the tick a wave lifted the craft most.
struct Flown {
    hits: usize,
    peak: f32,
    view: Mat4,
    level: Mat4,
    lifted: Mat4,
}

/// The cockpit rig's view of the player's craft with the Quake lift forced to `lift`.
fn cockpit_view_with(race: &Race, lift: f32) -> Mat4 {
    oag_render::camera::internal::view(
        race.camera_target(),
        &race.view.internal_params,
        race.ship().physics.roll_phase,
        race.ship().physics.camera_lean,
        lift,
    )
}

/// A real wave over the player under `law`, in the cockpit view.
fn fly_wave_in_the_cockpit(law: QuakeHit) -> Flown {
    let mut race = quake_race();
    race.sim.quake_hit = law;
    race.view.camera_view = oag_display::display::CameraView::Internal;
    let _ = race.drain_cues();
    race.sim.world.quake = Some(wave_on_the_player(&race, 0.3));
    let mut flown = Flown {
        hits: 0,
        peak: 0.0,
        view: Mat4::IDENTITY,
        level: Mat4::IDENTITY,
        lifted: Mat4::IDENTITY,
    };
    for _ in 0..3 {
        race.tick(&PlayerInputs::default());
        flown.hits += quake_hits(&mut race);
        let lift = race.sim.world.ships[0].quake_lift;
        if lift > flown.peak {
            flown.peak = lift;
            flown.view = race.tick_view_unshaken();
            flown.level = cockpit_view_with(&race, 0.0);
            flown.lifted = cockpit_view_with(&race, lift);
        }
    }
    flown
}

#[test]
fn a_title_that_lifts_the_eye_shows_the_wave_in_the_cockpit() {
    let flown = fly_wave_in_the_cockpit(QuakeHit::WaveHeight {
        origin: Origin::Chosen,
    });
    assert!(flown.peak > 0.1, "the wave never lifted the craft");
    assert_ne!(flown.level, flown.lifted, "the lift must move the eye");
    assert_eq!(flown.view, flown.lifted, "the eye did not ride the wave");
    assert_eq!(flown.hits, 1);
}

/// Pure's law: the hit still lands once and the smoothed height still rises (it is
/// the gate's state), but the cockpit view is the level one at the tick it peaks.
#[test]
fn a_title_that_holds_the_eye_level_keeps_the_hit_and_not_the_lift() {
    let flown = fly_wave_in_the_cockpit(QuakeHit::WaveHeightEyeLevel {
        origin: Origin::Chosen,
        eye_origin: Origin::Chosen,
    });
    assert!(flown.peak > 0.1, "the wave never lifted the craft");
    assert_ne!(flown.level, flown.lifted, "the lift must move the eye");
    assert_eq!(flown.view, flown.level, "the eye rode a wave it should not");
    assert_eq!(flown.hits, 1, "the hit gate must survive a level eye");
}
