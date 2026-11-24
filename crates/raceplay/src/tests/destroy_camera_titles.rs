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

/// HD's Zone end takes the orbit, not a station: the eye starts where the chase camera was and
/// eases to the orbit, and the field narrows to about 30 degrees at 2.5 s.
#[test]
fn hd_zone_end_is_the_orbit_cut_in_from_the_chase_eye() {
    let mut race = super::zone_end::zone_race(DestroyedCraft::HD);
    race.view.destroy_camera = DestroyCamera::new(vec![near_station()], 0.4);
    let chase = eye(&race);
    set_player(&mut race, CraftState::Destroyed);
    let craft = race.core.sim.world.ships[0].physics.body.position;
    let camera = race.view.destroy_camera_now().expect("the orbit");
    assert_eq!(camera.basis, Basis::LookAt);
    let start = camera.eye.distance(craft);
    assert!(
        (start - chase.distance(craft)).abs() < 0.5,
        "starts {start} off the craft, the chase eye was {}",
        chase.distance(craft)
    );
    for _ in 0..150 {
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
/// its box then. RPCS3 read the eye 13.9, 9.1, 8.9 and 7.0 units from the craft at the blast, 1.4, 1.6
/// and 2.5 s, and the frames show the whole craft from outside; ours sat 0.5 off the craft at the cut
/// and about 2.5 at 0.5 s, inside a hull that is 13 long (`destroy-camera.md`, "Measured on RPCS3").
#[test]
fn hd_zone_end_orbit_never_puts_the_eye_inside_the_hull() {
    let mut race = super::zone_end::zone_race(DestroyedCraft::HD);
    race.view.destroy_camera = DestroyCamera::new(vec![near_station()], 0.4);
    race.core.sim.world.ships[0].handling.dimensions = oag_physics::params::Dimensions {
        width: 5.5,
        height: 3.5,
        length: 13.0,
        ..Default::default()
    };
    // Put the chase eye where a Zone chase rig sits: behind and above, 14 units out.
    race.view.camera_override = Some(crate::options::CameraOverride {
        eye: race.core.sim.world.ships[0].physics.body.position + Vec3::new(-11.0, 8.0, 0.0),
        orientation: Quat::IDENTITY,
        fov_deg: None,
    });
    set_player(&mut race, CraftState::Destroyed);
    race.view.camera_override = None;
    let half = oag_physics::pair::half_extents(&race.core.sim.world.ships[0].handling.dimensions);
    for tick in 0..150 {
        let ship = &race.core.sim.world.ships[0].physics.body;
        let camera = race.view.destroy_camera_now().expect("the orbit");
        let local = ship.orientation.inverse() * (camera.eye - ship.position);
        assert!(
            local.x.abs() > half.x || local.y.abs() > half.y || local.z.abs() > half.z,
            "tick {tick}: the eye is inside the hull's box, {local:?} within {half:?}"
        );
        race.view.advance_destroy_camera(&race.core);
    }
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
