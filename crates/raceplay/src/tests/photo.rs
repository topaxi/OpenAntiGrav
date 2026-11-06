//! Photo mode on a synthetic race: the camera is imposed and put back, the world is not
//! touched, and a split screen is refused.

use super::*;
use crate::photo::{Action, Lens, Pad};
use oag_gameplay::PlayerInputs;

fn pad() -> Pad {
    Pad::default()
}

#[test]
fn photo_mode_imposes_a_camera_and_leaving_puts_back_the_one_that_was_in_force() {
    let mut race = race_with_a_grid();
    race.tick(&PlayerInputs::none());
    let before = race.camera_position();
    assert!(race.photo_begin(false, false, None));
    assert!(race.photo().is_some());
    race.photo_step(Pad {
        select: true,
        ..pad()
    });
    race.photo_step(pad());
    assert_eq!(
        race.photo().map(crate::photo::Photo::lens),
        Some(Lens::Close)
    );
    for _ in 0..30 {
        race.photo_step(Pad {
            stick: (1.0, 0.5),
            ..pad()
        });
    }
    assert_ne!(
        race.camera_position(),
        before,
        "the eye moved off the chase camera"
    );
    race.photo_end();
    assert!(race.photo().is_none());
    assert_eq!(race.camera_position(), before, "the chase camera is back");
}

#[test]
fn a_race_taken_through_photo_mode_hashes_as_one_that_was_not() {
    let mut control = race_with_a_grid();
    let mut photographed = race_with_a_grid();
    for tick in 0..120 {
        control.tick(&PlayerInputs::none());
        photographed.tick(&PlayerInputs::none());
        if tick == 60 {
            assert!(photographed.photo_begin(false, false, None));
            for step in 0..90 {
                let moved = Pad {
                    stick: ((step as f32 * 0.2).sin(), (step as f32 * 0.1).cos()),
                    next: step % 30 == 29,
                    reset: step == 70,
                    ..pad()
                };
                photographed.photo_step(moved);
            }
            photographed.photo_end();
        }
        assert_eq!(
            control.sim.state_hash(),
            photographed.sim.state_hash(),
            "diverged at tick {tick}"
        );
    }
}

#[test]
fn a_race_split_across_views_does_not_open_photo_mode() {
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    assert!(!race.photo_begin(false, false, None));
    assert!(race.photo().is_none());
    assert_eq!(race.photo_step(pad()), Action::None);
}

/// A grid on a straight whose control points each author their own section, so two craft
/// at opposite ends of it stand in different sections.
fn race_on_a_sectioned_straight() -> Race {
    let mut setup = setup(hulled_handling());
    let ai = straight_track_in_sections(12.0, 12.0, |k| u8::try_from(k + 1).unwrap_or(u8::MAX));
    setup.spline = Spline::from_track(&ai);
    setup.course = Course::from_track(&ai, None);
    setup.ai = ai;
    setup.mode = Mode::SingleRace;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(setup)
}

/// What the race's cull allows from the seat: each section sees only itself, so a seat in one
/// section is a set of one.
fn each_section_sees_itself() -> crate::visibility::TrackVisibility {
    let pvs = oag_vex::pvs::TrackPvs::from_sections((1u8..=8).map(|index| oag_vex::pvs::Section {
        index,
        visible: 1u64 << index,
        bounds: None,
    }));
    crate::visibility::TrackVisibility::declaring(pvs)
}

/// While photo mode is open nothing is culled by section, on whichever craft it frames: a shot
/// must not lose scenery to a set authored for a craft in a race. Leaving restores the cull.
#[test]
fn photo_mode_draws_every_section_and_leaving_restores_the_cull() {
    let mut race = race_on_a_sectioned_straight();
    let visibility = each_section_sees_itself();
    let view = Mat4::ZERO;
    let seat = race.visible_set(&visibility, &view);
    assert!(!seat.is_everything(), "the race culls: {seat:?}");

    assert!(race.photo_begin(false, false, None));
    assert!(
        race.visible_set(&visibility, &view).is_everything(),
        "the seat's own picture"
    );
    for _ in 0..3 {
        race.photo_step(Pad {
            next: true,
            ..pad()
        });
    }
    assert_eq!(race.photo().map(crate::photo::Photo::subject), Some(3));
    assert!(
        race.visible_set(&visibility, &view).is_everything(),
        "another craft's"
    );

    race.photo_end();
    assert_eq!(
        race.visible_set(&visibility, &view),
        seat,
        "the cull is back"
    );
}

/// The weather's section is the framed craft's, located from its driver where it is an
/// opponent (whose `Ship::segment` is never written).
#[test]
fn photo_modes_weather_section_is_the_subjects() {
    let mut race = race_on_a_sectioned_straight();
    let last = race.spline().len() - 1;
    let pos = race.spline().sample(last).expect("a sample").pos;
    let line = (0..race.sim.ai_order.len())
        .find(|&k| race.ai_sample_index(k) == Some(last))
        .expect("the last sample is on the racing line");
    let ship = &mut race.sim.world.ships[3];
    ship.physics.body.position = Vec3::from_array(pos);
    ship.driver.index = u32::try_from(line).expect("a short fixture");
    let seat = race.section_of_framed();
    assert!(race.photo_begin(false, false, None));
    for _ in 0..3 {
        race.photo_step(Pad {
            next: true,
            ..pad()
        });
    }
    let subject = race.section_of_framed();
    assert_ne!(subject, UNPLACED);
    assert_ne!(subject, seat);
    assert_eq!(
        subject,
        race.spline().sample(last).expect("a sample").section_id
    );
}
