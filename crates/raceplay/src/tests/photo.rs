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
    assert!(race.photo_begin(false, false));
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
            assert!(photographed.photo_begin(false, false));
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
    assert!(!race.photo_begin(false, false));
    assert!(race.photo().is_none());
    assert_eq!(race.photo_step(pad()), Action::None);
}
