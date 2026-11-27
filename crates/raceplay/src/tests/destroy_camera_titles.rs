//! The destroy camera's per-title laws (`docs/ghidra/functions/ps3-hdfury-eu/destroy-camera.md`):
//! HD's mode 8, 2048's and Omega's mode 9 with the hold view, HD's Zone end orbit, and Pulse's
//! mode 5 unchanged. The laws themselves are pinned in `oag_render::camera::destroy`.

use super::*;
use crate::destroy_camera::DestroyCamera;
use oag_physics::CraftState;
use oag_render::camera::destroy::{Basis, Station};
use oag_title::destroyed::DestroyedCraft;

const FRAME: f32 = 1.0 / 60.0;

fn near_station() -> Station {
    Station {
        eye: Vec3::new(400.0, 30.0, 0.0),
        aim: Vec3::new(10.0, 0.0, 0.0),
    }
}

fn far_station() -> Station {
    Station {
        eye: Vec3::new(-300.0, 20.0, 300.0),
        aim: Vec3::new(-250.0, 0.0, 250.0),
    }
}

/// Nearest to the craft by eye, farthest by aim: a pick by eye position takes this one.
fn decoy_station() -> Station {
    Station {
        eye: Vec3::new(20.0, 5.0, 5.0),
        aim: Vec3::new(-900.0, 0.0, 900.0),
    }
}

fn race_of(destroyed: DestroyedCraft) -> Race {
    let mut race = race_with_a_grid();
    race.core.sim.destroyed = destroyed;
    race.view.destroy_camera =
        DestroyCamera::new(vec![far_station(), decoy_station(), near_station()], 0.4);
    race.core.sim.world.ships[0].physics.body.position = Vec3::new(8.0, 0.0, 3.0);
    race
}

fn set_player(race: &mut Race, state: CraftState) {
    race.core.sim.world.ships[0].physics.craft_state = state;
    race.view.advance_destroy_camera(&race.core);
}

fn eye(race: &Race) -> Vec3 {
    race.view.camera_position(&race.core)
}

/// How many ticks the picture stays on the station after the cut, `None` if it never lets go in
/// `limit` ticks.
fn ticks_on_the_station(race: &mut Race, limit: u32) -> Option<u32> {
    for tick in 1..=limit {
        race.view.advance_destroy_camera(&race.core);
        if race
            .view
            .destroy_camera_now()
            .is_none_or(|camera| camera.basis != Basis::Flattened)
        {
            return Some(tick);
        }
    }
    None
}

#[test]
fn hd_cuts_to_the_station_whose_aim_is_nearest_with_hd_rates() {
    let mut race = race_of(DestroyedCraft::HD);
    set_player(&mut race, CraftState::Destroyed);
    assert!((eye(&race) - near_station().eye).length() < 1e-3);
    let camera = race.view.destroy_camera_now().expect("cut");
    assert_eq!(camera.substeps_per_second, Some(6000.0));
    // The draws in the cut's order: the starting field's offset, the field factor, the focus factor.
    let mut rng = oag_core::rng::Rng::new(crate::destroy_camera::DESTROY_CAMERA_SEED);
    let _offset = rng.next_f32();
    let field = 0.5 + (2.0 - 0.5) * rng.next_f32();
    let focus = 0.5 + (1.0 - 0.5) * rng.next_f32();
    assert!(
        (field - 1.0).abs() > 0.05 && (focus - 1.0).abs() > 0.05,
        "seed lands off 1.0"
    );
    assert!(
        (camera.fov_rate - 0.06 * field).abs() < 1e-6,
        "{}",
        camera.fov_rate
    );
    assert!(
        (camera.focus_rate - 0.4 * focus).abs() < 1e-6,
        "{}",
        camera.focus_rate
    );
}

#[test]
fn pulse_keeps_its_exact_rates_and_never_hands_back() {
    let mut race = race_of(DestroyedCraft::PULSE);
    set_player(&mut race, CraftState::Destroyed);
    let camera = race.view.destroy_camera_now().expect("cut");
    assert_eq!(camera.substeps_per_second, None);
    assert_eq!(camera.fov_rate, 0.06);
    assert_eq!(camera.focus_rate, 0.4);
    assert_eq!(ticks_on_the_station(&mut race, 60 * 20), None);
    assert!((eye(&race) - near_station().eye).length() < 1e-3);
}

/// The clock is the camera's own and free-running, so the station view lasts what is left of the
/// 8 s it was in when the craft blew up, and the chase camera is what remains.
#[test]
fn hd_hands_back_to_the_chase_camera_when_the_8_s_clock_runs_out_and_stays_handed_back() {
    let mut race = race_of(DestroyedCraft::HD);
    for _ in 0..300 {
        set_player(&mut race, CraftState::Racing);
    }
    set_player(&mut race, CraftState::Destroyed);
    let lasted = ticks_on_the_station(&mut race, 600).expect("it hands back inside 8 s");
    let seconds = lasted as f32 * FRAME;
    assert!(
        (2.5..3.5).contains(&seconds),
        "{seconds} s left of 8 on a clock at 5 s"
    );
    assert!(
        race.view.destroy_camera_now().is_none(),
        "HD has no hold view"
    );
    let cuts = race.view.camera_cuts(&race.core);
    for _ in 0..600 {
        set_player(&mut race, CraftState::Eliminated);
        assert!(
            race.view.destroy_camera_now().is_none(),
            "no second cut to the station"
        );
    }
    assert_eq!(
        race.view.camera_cuts(&race.core),
        cuts,
        "the clock's wrap is not a cut"
    );
    set_player(&mut race, CraftState::Racing);
    assert!(race.view.destroy_camera_now().is_none());
}

#[test]
fn the_phase_of_the_clock_at_the_blast_sets_how_long_the_station_view_lasts() {
    let lasts = |racing_ticks: u32| {
        let mut race = race_of(DestroyedCraft::HD);
        for _ in 0..racing_ticks {
            set_player(&mut race, CraftState::Racing);
        }
        set_player(&mut race, CraftState::Destroyed);
        ticks_on_the_station(&mut race, 600).expect("hands back")
    };
    assert!(lasts(60) > lasts(300) + 100);
}

#[test]
fn vita_holds_the_view_still_after_the_hand_back_until_the_craft_races_again() {
    let mut race = race_of(DestroyedCraft::HD_VITA);
    for _ in 0..300 {
        set_player(&mut race, CraftState::Racing);
    }
    set_player(&mut race, CraftState::Destroyed);
    ticks_on_the_station(&mut race, 600).expect("hands back");
    let held = race.view.destroy_camera_now().expect("the hold view");
    assert_eq!(held.basis, Basis::LookAt);
    assert_eq!(held.fov, oag_render::camera::destroy::HOLD_FOV);
    let eye_then = held.eye;
    assert_ne!(
        eye_then,
        near_station().eye,
        "the chase camera's eye, not the station's"
    );
    race.core.sim.world.ships[0].physics.body.position += Vec3::new(5.0, 0.0, 0.0);
    for _ in 0..300 {
        set_player(&mut race, CraftState::Eliminated);
        let camera = race.view.destroy_camera_now().expect("still held");
        assert_eq!(camera.eye, eye_then);
    }
    let looks_at = race.view.destroy_camera_now().expect("held").focus;
    assert!((looks_at - race.core.sim.world.ships[0].physics.body.position).length() < 1e-3);
    set_player(&mut race, CraftState::Racing);
    assert!(race.view.destroy_camera_now().is_none());
}

#[test]
fn omega_holds_like_vita() {
    let mut race = race_of(DestroyedCraft::HD_BLOWUP);
    for _ in 0..300 {
        set_player(&mut race, CraftState::Racing);
    }
    set_player(&mut race, CraftState::Destroyed);
    ticks_on_the_station(&mut race, 600).expect("hands back");
    assert!(
        race.view
            .destroy_camera_now()
            .is_some_and(|camera| camera.basis == Basis::LookAt)
    );
}

/// HD's Zone race with its Close rig at HD's 8.7 units (`camera.md`) rather than the fixture's 4.1, on
/// a craft that flies at `speed` along its own nose, the chase rig settled behind it.
fn hd_zone_race_at(speed: f32) -> Race {
    zone_race_at(DestroyedCraft::HD, speed)
}

/// [`hd_zone_race_at`] with another title's destroyed-craft sequence.
fn zone_race_at(destroyed: DestroyedCraft, speed: f32) -> Race {
    let mut race = super::zone_end::zone_race(destroyed);
    race.view.destroy_camera = DestroyCamera::new(vec![near_station()], 0.4);
    race.core.sim.world.ships[0].handling.dimensions = oag_physics::params::Dimensions {
        width: 5.5,
        height: 3.5,
        length: 13.0,
        ..Default::default()
    };
    race.view.chase_params.pos_length = 8.6;
    race.view.chase_params.pos_height = 1.3;
    race.core.sim.world.ships[0].physics.body.linear_velocity = Vec3::NEG_Z * speed;
    race.view.camera = oag_render::camera::chase::Chase::snapped(
        race.view.camera_target(&race.core),
        &race.view.chase_params,
    );
    race
}

/// One tick of the craft flying on and the camera after it.
fn fly_and_advance(race: &mut Race) {
    let ship = &mut race.core.sim.world.ships[0].physics;
    ship.body.position += ship.body.linear_velocity * FRAME;
    race.view.advance_destroy_camera(&race.core);
}

/// The Zone end takes the orbit, not a station: the eye is `5 + dist` from the craft on every frame
/// (HD's render camera read on RPCS3; the same `cam+0x3b4 = 5.0` law read in 2048's and Omega's
/// code), so 5.5 at the cut and farther as the radius opens, and the field narrows to about 30
/// degrees at 2.5 s.
#[test]
fn a_zone_end_is_the_close_orbit_five_beyond_its_radius_on_hd_2048_and_omega() {
    for destroyed in [
        DestroyedCraft::HD,
        DestroyedCraft::HD_VITA,
        DestroyedCraft::HD_BLOWUP,
    ] {
        assert_zone_end_orbit(destroyed);
    }
}

fn assert_zone_end_orbit(destroyed: DestroyedCraft) {
    let mut race = zone_race_at(destroyed, 0.0);
    set_player(&mut race, CraftState::Destroyed);
    let craft = race.core.sim.world.ships[0].physics.body.position;
    let camera = race.view.destroy_camera_now().expect("the orbit");
    assert_eq!(camera.basis, Basis::LookAt);
    let start = camera.eye.distance(craft);
    assert!((start - 5.5).abs() < 0.01, "starts {start} off the craft");
    // The first frame eases the radius from 0.5 toward the enter's 4.0, not yet toward 5.0
    // (`destroy-camera.md`, "The orbit on 2048 and Omega").
    race.view.advance_destroy_camera(&race.core);
    let camera = race.view.destroy_camera_now().expect("the orbit");
    let first = camera.eye.distance(camera.focus);
    assert!(
        (first - 5.57).abs() < 0.005,
        "first frame {first} off the focus"
    );
    for _ in 0..59 {
        race.view.advance_destroy_camera(&race.core);
    }
    let camera = race.view.destroy_camera_now().expect("the orbit");
    let expect = 5.0 + 5.0 - 4.5 * 0.98_f32.powi(60);
    assert!(
        (camera.eye.distance(camera.focus) - expect).abs() < 0.05,
        "a second on it is {} from the focus, want {expect}",
        camera.eye.distance(camera.focus)
    );
    for _ in 0..90 {
        race.view.advance_destroy_camera(&race.core);
    }
    let camera = race.view.destroy_camera_now().expect("the orbit");
    assert!(
        (camera.fov - 30.0).abs() < 1.5,
        "field {} at 2.5 s",
        camera.fov
    );
}

/// The craft is drawn for the first 2.5 s of the orbit (the shell, state 5): the eye is never inside
/// its box then, at rest or flying at the speeds of a Zone (93 units a second read on RPCS3), with no
/// push-out: the read law's `5 + dist` already stands outside the hull (`destroy-camera.md`, "The orbit
/// eye").
#[test]
fn hd_zone_end_orbit_never_puts_the_eye_inside_the_hull() {
    for speed in [0.0, 19.0, 93.0, 240.0] {
        let mut race = hd_zone_race_at(speed);
        set_player(&mut race, CraftState::Destroyed);
        let half =
            oag_physics::pair::half_extents(&race.core.sim.world.ships[0].handling.dimensions)
                + Vec3::splat(0.0);
        for tick in 0..150 {
            let ship = &race.core.sim.world.ships[0].physics.body;
            let camera = race.view.destroy_camera_now().expect("the orbit");
            let local = ship.orientation.inverse() * (camera.eye - ship.position);
            assert!(
                local.x.abs() > half.x || local.y.abs() > half.y || local.z.abs() > half.z,
                "{speed} units/s, tick {tick}: the eye is inside the hull's box, {local:?} within {half:?}"
            );
            fly_and_advance(&mut race);
        }
    }
}

/// The orbit reads no previous eye (`0x000ae6c0`), so a death in the cockpit view takes the same orbit as
/// one in the chase view, not one started from the hull-bolted cockpit eye.
#[test]
fn a_zone_death_in_the_cockpit_view_takes_the_same_orbit() {
    let mut race = hd_zone_race_at(0.0);
    race.view.internal_params.length = 3.0;
    race.view.internal_params.height = 0.0;
    race.set_camera_view(oag_display::display::CameraView::Internal);
    race.view.camera = oag_render::camera::chase::Chase::snapped(
        race.view.camera_target(&race.core),
        &race.view.chase_params,
    );
    set_player(&mut race, CraftState::Destroyed);
    let craft = race.core.sim.world.ships[0].physics.body.position;
    let start = race
        .view
        .destroy_camera_now()
        .expect("the orbit")
        .eye
        .distance(craft);
    assert!((start - 5.5).abs() < 0.01, "started {start} off the craft");
}

/// A seek in a Zone replay puts the wreck hundreds of units from where the chase rig last was; the orbit
/// still starts from a rig settled on the restored craft, not stretched across the jump.
#[test]
fn a_seek_to_a_zone_end_starts_the_orbit_from_a_settled_chase_rig() {
    let mut race = hd_zone_race_at(0.0);
    race.core.sim.world.ships[0].physics.body.position += Vec3::new(1000.0, 0.0, 0.0);
    race.core.sim.world.ships[0].physics.craft_state = CraftState::Destroyed;
    let later = race.snapshot_sim();
    race.core.sim.world.ships[0].physics.body.position -= Vec3::new(1000.0, 0.0, 0.0);
    race.core.sim.world.ships[0].physics.craft_state = CraftState::Racing;
    race.restore_for_viewing(&later);
    race.view.advance_destroy_camera(&race.core);
    let craft = race.core.sim.world.ships[0].physics.body.position;
    let start = race
        .view
        .destroy_camera_now()
        .expect("the orbit")
        .eye
        .distance(craft);
    assert!(start < 20.0, "started {start} off the craft");
}

/// A title whose Zone craft is a wreck (Pulse) keeps its station view there.
#[test]
fn pulse_zone_has_no_orbit() {
    let mut race = super::zone_end::zone_race(DestroyedCraft::PULSE);
    race.view.destroy_camera = DestroyCamera::new(vec![near_station()], 0.4);
    set_player(&mut race, CraftState::Destroyed);
    assert_eq!(
        race.view.destroy_camera_now().expect("station").basis,
        Basis::Flattened
    );
}

/// Omega's pick finding no station goes to the hold view at the cut (row W8); HD, which has no
/// hold view, keeps the chase camera.
#[test]
fn a_circuit_without_stations_holds_where_the_title_holds() {
    let chase = |destroyed| {
        let mut race = race_of(destroyed);
        race.view.destroy_camera = DestroyCamera::new(Vec::new(), 0.4);
        let chase_eye = eye(&race);
        set_player(&mut race, CraftState::Destroyed);
        (race, chase_eye)
    };
    let (race, chase_eye) = chase(DestroyedCraft::HD_BLOWUP);
    let held = race.view.destroy_camera_now().expect("the hold view");
    assert_eq!(held.basis, Basis::LookAt);
    assert_eq!(held.eye, chase_eye);
    assert_eq!(race.view.camera_cuts(&race.core), 1);
    let (race, _) = chase(DestroyedCraft::HD);
    assert!(race.view.destroy_camera_now().is_none());
}
