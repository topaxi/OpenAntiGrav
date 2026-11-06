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

/// Stands `slot` on the spline's last sample, where the simulation would have left it.
fn stand_at_the_far_end(race: &mut Race, slot: usize) -> usize {
    let last = race.spline().len() - 1;
    let pos = race.spline().sample(last).expect("a sample").pos;
    let ship = &mut race.sim.world.ships[slot];
    ship.physics.body.position = Vec3::from_array(pos);
    ship.segment = u16::try_from(last).expect("a short fixture");
    last
}

/// Photo mode frames its subject, so the track drawn around the picture is the subject's, not
/// the seat's: with the subject at the far end of the circuit, culling to the seat's section
/// leaves the scenery around the subject out of the frame.
#[test]
fn photo_modes_subject_places_the_cull_not_the_seat() {
    let mut race = race_on_a_sectioned_straight();
    let far = stand_at_the_far_end(&mut race, 3);
    let seat_section = race.section_of_slot(0);
    let far_section = race.section_of_slot(3);
    assert_ne!(seat_section, UNPLACED);
    assert_ne!(far_section, UNPLACED);
    assert_ne!(seat_section, far_section, "the fixture must tell the two apart");

    assert_eq!(race.visibility_sections().0, seat_section, "before the mode");
    assert!(race.photo_begin(false, false, None));
    for _ in 0..3 {
        race.photo_step(Pad {
            next: true,
            ..pad()
        });
    }
    assert_eq!(race.photo().map(crate::photo::Photo::subject), Some(3));

    let (craft, camera) = race.visibility_sections();
    assert_eq!(craft, far_section, "the craft half follows the subject");
    assert_eq!(camera, far_section, "the camera half is searched near the subject (sample {far})");

    // The free camera moved off the craft is still placed from the subject's sample.
    for _ in 0..30 {
        race.photo_step(Pad {
            stick: (1.0, 0.0),
            select: true,
            ..pad()
        });
    }
    assert_eq!(race.visibility_sections().0, far_section);

    race.photo_end();
    assert_eq!(race.visibility_sections().0, seat_section, "the seat again");
}

/// A recovery in flight blanks the shot of the craft the picture is about: the subject's,
/// not the seat's.
#[test]
fn photo_modes_respawn_cooldown_is_the_subjects() {
    let mut race = race_on_a_sectioned_straight();
    stand_at_the_far_end(&mut race, 1);
    assert!(race.photo_begin(false, false, None));
    race.photo_step(Pad {
        next: true,
        ..pad()
    });
    assert_eq!(race.photo().map(crate::photo::Photo::subject), Some(1));
    race.sim.respawn_cooldown[1] = 1;
    assert_eq!(race.visibility_sections(), (UNPLACED, UNPLACED));
    race.sim.respawn_cooldown[1] = 0;
    race.sim.respawn_cooldown[0] = 1;
    assert_ne!(race.visibility_sections(), (UNPLACED, UNPLACED));
}
