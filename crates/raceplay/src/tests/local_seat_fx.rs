//! A split screen's or a window's extra person sees their own wall sparks, shake, boost kick,
//! boost zoom and airbrake flaps: every effect that belongs to a seat is stepped for each local
//! seat and drawn in that seat's view, and no seat's hit moves another seat's picture.
//!
//! Four people fly in all of them; `views` of the grid have a camera (2 or 4), so the seats
//! without one prove a person without a view is left alone.

use super::respawn::one_emitter_pob;
use super::*;
use oag_display::display::{BoostFovKick, Fov};
use oag_gameplay::{Controller, PlayerInputs};

/// How many views the split fields: two people side by side or above, then all four.
const VIEWS: [usize; 2] = [2, 4];

fn local_people(views: usize, zoom: bool) -> Race {
    let mut setup = setup_with(
        hulled_handling(),
        vec![upward_plane(-40.0, oag_physics::Surface::Wall)],
    );
    setup.mode = Mode::SingleRace;
    let effect = oag_fx::psys::Effect::parse(
        &one_emitter_pob("WO_TEST_SPARK", 0),
        oag_fx::psys::ColourScale::Full,
    )
    .expect("the hand-laid effect parses");
    setup.handles.insert(Trigger::CollisionSpark, effect);
    setup.zoom_ring = zoom.then_some(&oag_hd::race::ZOOM_RING);
    setup.zoom_borrowed = zoom;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let mut race = Race::start(setup);
    for slot in 1..4 {
        race.sim.world.controllers[slot] = Controller::Local;
    }
    assert_eq!(race.set_viewport_count(views), views);
    race
}

fn push_toward_wall(race: &mut Race, slot: usize) {
    let body = &mut race.sim.world.ships[slot].physics.body;
    body.position = Vec3::new(20.0, -39.7, 10.0 * slot as f32);
    body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
    body.orientation = oag_core::math::Quat::IDENTITY;
    body.angular_velocity = Vec3::ZERO;
}

/// The shake on `slot`'s view: whether it is the picture's and not the unshaken one's.
fn view_is_shaken(race: &Race, slot: usize) -> bool {
    race.set_viewpoint(slot);
    let shaken = race.view() != race.view_unshaken();
    let rotated = race.view_shake_rotation() != Mat4::IDENTITY;
    race.reset_viewpoint();
    assert_eq!(shaken, rotated, "slot {slot}: the lens map disagrees");
    shaken
}

fn spark_vertices(race: &Race) -> usize {
    let (mut additive, mut alpha) = (Vec::new(), Vec::new());
    race.extend_spark_vertices(&mut additive, &mut alpha, Vec3::X, Vec3::Y);
    additive.len() + alpha.len()
}

#[test]
fn a_local_seats_wall_hit_throws_its_sparks_and_shakes_its_own_view() {
    for views in VIEWS {
        for seat in 1..views {
            let mut race = local_people(views, false);
            push_toward_wall(&mut race, seat);
            let evaluations = race.tick(&PlayerInputs::none());
            assert!(evaluations.get(seat).is_some_and(|e| e.wall.impact));
            let own = race.view.viewpoints.contact[seat].as_ref().expect("a seat");
            assert_eq!(own.ignitions, 1, "{views} views, seat {seat}: no sparks");
            assert!(own.shake.active(), "{views} views, seat {seat}: no shake");
            assert!(
                spark_vertices(&race) > 0,
                "seat {seat}'s sparks are not drawn"
            );
            assert_eq!(race.spark_ignitions(), 0, "the player's sparks lit");
            assert!(!race.view.contact.shake.active(), "the player shook");
            for other in 0..views {
                assert_eq!(
                    view_is_shaken(&race, other),
                    other == seat,
                    "{views} views: seat {seat}'s hit, view {other}"
                );
            }
        }
    }
}

#[test]
fn the_players_wall_hit_leaves_the_other_seats_alone() {
    for views in VIEWS {
        let mut race = local_people(views, false);
        push_toward_wall(&mut race, 0);
        race.tick(&PlayerInputs::none());
        assert_eq!(race.spark_ignitions(), 1);
        for seat in 1..views {
            assert!(!view_is_shaken(&race, seat), "{views} views, seat {seat}");
            let own = race.view.viewpoints.contact[seat].as_ref().expect("a seat");
            assert_eq!(own.ignitions, 0);
        }
    }
}

/// A person without a view (four fly, two views) has nothing to throw sparks or shake into.
#[test]
fn a_person_with_no_view_throws_nothing() {
    let mut race = local_people(2, false);
    push_toward_wall(&mut race, 3);
    race.tick(&PlayerInputs::none());
    assert_eq!(race.spark_ignitions(), 0);
    assert!(race.view.viewpoints.contact[3].is_none());
    assert!(!view_is_shaken(&race, 1));
}

#[test]
fn a_local_seats_shake_decays_as_the_players_does() {
    let mut race = local_people(2, false);
    push_toward_wall(&mut race, 1);
    race.tick(&PlayerInputs::none());
    assert!(view_is_shaken(&race, 1));
    race.sim.world.ships[1].physics.body.position = Vec3::new(20.0, 100.0, 10.0);
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    assert!(!view_is_shaken(&race, 1), "the shake never ended");
}

#[test]
fn a_weapon_hit_shakes_the_seat_it_landed_on() {
    for views in VIEWS {
        for seat in 0..views {
            let mut race = local_people(views, false);
            race.force_weapon_hit(seat);
            for view in 0..views {
                assert_eq!(
                    view_is_shaken(&race, view),
                    view == seat,
                    "{views} views: a hit on seat {seat}, view {view}"
                );
            }
        }
    }
}

#[test]
fn a_seats_own_explosions_shake_its_view() {
    use oag_physics::CraftState;
    for views in VIEWS {
        for seat in 1..views {
            let mut race = local_people(views, false);
            race.advance_craft_flashes();
            race.sim.world.ships[seat].physics.craft_state = CraftState::Destroyed;
            race.advance_craft_flashes();
            race.sim.world.ships[seat].physics.craft_state = CraftState::Eliminated;
            race.advance_craft_flashes();
            for view in 0..views {
                assert_eq!(
                    view_is_shaken(&race, view),
                    view == seat,
                    "{views} views: seat {seat}'s blast, view {view}"
                );
            }
        }
    }
}

#[test]
fn a_local_seats_boost_kicks_its_own_field_of_view() {
    for views in VIEWS {
        for seat in 1..views {
            let mut race = local_people(views, false);
            race.view.boost_fov_kick = BoostFovKick::DEFAULT;
            race.sim.world.ships[seat].physics.pad_timer = 1.0;
            race.tick(&PlayerInputs::none());
            let fov = |view| {
                race.set_viewpoint(view);
                let fov = race.extra_vertical_fov(1.6, Fov::AUTHORED);
                race.reset_viewpoint();
                fov
            };
            let kicked = fov(seat).expect("a view");
            for other in (1..views).filter(|&o| o != seat) {
                let rest = fov(other).expect("a view");
                assert!(kicked > rest, "{views} views, seat {seat}: no kick");
            }
        }
    }
}

#[test]
fn a_local_seats_boost_pulse_draws_in_its_view_alone() {
    for views in VIEWS {
        for seat in 0..views {
            let mut race = local_people(views, true);
            race.fire_zoom_boost(seat);
            for _ in 0..6 {
                race.advance_zoom();
            }
            for view in 0..views {
                race.set_viewpoint(view);
                let boost = race.hd_zoom_frame().expect("a ring").boost;
                race.reset_viewpoint();
                assert_eq!(boost > 0.0, view == seat, "{views} views: {seat} -> {view}");
            }
        }
    }
}

#[test]
fn a_local_seats_airbrake_swings_its_own_flaps() {
    for views in VIEWS {
        for seat in 1..views {
            let mut race = local_people(views, false);
            race.sim.world.ships[seat].physics.airbrake_left = 100.0;
            race.tick(&PlayerInputs::none());
            for slot in 0..4 {
                assert_eq!(
                    race.airbrake_flaps_of(slot)[0] > 0.0,
                    slot == seat,
                    "{views} views: seat {seat}'s brake, slot {slot}"
                );
            }
        }
    }
}
