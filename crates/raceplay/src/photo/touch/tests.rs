//! The fingers on the photo camera, off a made-up craft: no disc.

use super::*;

const LIVE: &[usize] = &[0];

fn touch(t: Touch) -> Pad {
    Pad {
        touch: t,
        ..Pad::default()
    }
}

fn on_close() -> Photo {
    let mut photo = Photo::new(0, true, None);
    photo.cycle_lens();
    photo
}

fn on_free() -> Photo {
    let pose = CameraOverride {
        eye: Vec3::new(0.0, 3.0, 12.0),
        orientation: Quat::IDENTITY,
        fov_deg: Some(60.0),
    };
    Photo::new(0, true, None).opening_on(pose, Vec3::ZERO)
}

fn eye_of(photo: &Photo) -> Vec3 {
    photo.free.expect("seeded").cam.position
}

#[test]
fn an_empty_touch_changes_nothing_on_any_lens() {
    for mut photo in [Photo::new(0, true, None), on_close(), on_free()] {
        let before = photo.clone();
        photo.step(Pad::default(), LIVE);
        let mut expected = before;
        expected.age += 1;
        assert_eq!(photo, expected);
        let mut zero = photo.clone();
        zero.step(
            touch(Touch {
                pinch: 0.0,
                ..Touch::default()
            }),
            LIVE,
        );
        zero.age = photo.age;
        assert_eq!(zero, photo);
    }
}

#[test]
fn a_drag_on_close_orbits_as_the_stick_does() {
    let mut finger = on_close();
    finger.step(
        touch(Touch {
            drag: (0.1, 0.05),
            ..Touch::default()
        }),
        LIVE,
    );
    let (yaw, pitch) = finger.orbit();
    assert!((yaw + 0.1 * TOUCH_ORBIT).abs() < 1e-5);
    assert!((pitch - 0.05 * TOUCH_ORBIT).abs() < 1e-5);
    let mut stick = on_close();
    stick.step(
        Pad {
            stick: (1.0, 0.0),
            ..Pad::default()
        },
        LIVE,
    );
    assert!(stick.orbit().0.signum() == yaw.signum());
}

#[test]
fn pinching_spread_narrows_the_field_and_clamps() {
    let mut photo = on_close();
    photo.step(
        touch(Touch {
            pinch: 2.0,
            ..Touch::default()
        }),
        LIVE,
    );
    assert!((photo.fov() - CLOSE_FOV.2 / 2.0).abs() < 1e-4);
    for _ in 0..20 {
        photo.step(
            touch(Touch {
                pinch: 2.0,
                ..Touch::default()
            }),
            LIVE,
        );
    }
    assert_eq!(photo.fov(), CLOSE_FOV.0);
    for _ in 0..40 {
        photo.step(
            touch(Touch {
                pinch: 0.5,
                ..Touch::default()
            }),
            LIVE,
        );
    }
    assert_eq!(photo.fov(), CLOSE_FOV.1);
}

#[test]
fn pinching_on_track_keeps_to_the_track_limits() {
    let mut photo = on_close();
    photo.cycle_lens();
    assert_eq!(photo.lens(), Lens::Track);
    for _ in 0..30 {
        photo.step(
            touch(Touch {
                pinch: 2.0,
                ..Touch::default()
            }),
            LIVE,
        );
    }
    assert_eq!(photo.fov(), TRACK_FOV.0);
}

#[test]
fn a_twist_tilts_clockwise_and_a_two_finger_drag_pans_within_the_limit() {
    let mut photo = on_close();
    photo.step(
        touch(Touch {
            twist: 0.3,
            ..Touch::default()
        }),
        LIVE,
    );
    assert!((photo.tilt - 0.3).abs() < 1e-6);
    // The picture follows the fingers: a drag right moves the eye left.
    photo.step(
        touch(Touch {
            pan: (0.1, -0.1),
            ..Touch::default()
        }),
        LIVE,
    );
    assert!(photo.pan.0 < 0.0 && photo.pan.1 < 0.0);
    for _ in 0..50 {
        photo.step(
            touch(Touch {
                pan: (0.5, 0.5),
                ..Touch::default()
            }),
            LIVE,
        );
    }
    assert_eq!(photo.pan, (-PAN_LIMIT, PAN_LIMIT));
}

#[test]
fn a_drag_on_internal_or_track_takes_the_free_lens_to_be_seeded() {
    let mut photo = Photo::new(0, true, None);
    photo.step(
        touch(Touch {
            drag: (0.1, 0.0),
            ..Touch::default()
        }),
        LIVE,
    );
    assert_eq!(photo.lens(), Lens::Free);
    assert!(photo.needs_free_seed());

    let mut photo = on_close();
    photo.cycle_lens();
    photo.step(
        touch(Touch {
            drag: (0.0, 0.1),
            ..Touch::default()
        }),
        LIVE,
    );
    assert_eq!(photo.lens(), Lens::Free);
    assert!(photo.needs_free_seed());

    // A twist alone stays on internal and tilts.
    let mut photo = Photo::new(0, true, None);
    photo.step(
        touch(Touch {
            twist: 0.2,
            ..Touch::default()
        }),
        LIVE,
    );
    assert_eq!(photo.lens(), Lens::Internal);
    assert!((photo.tilt - 0.2).abs() < 1e-6);
}

#[test]
fn a_drag_on_the_free_lens_turns_it_by_the_field_so_the_scene_follows() {
    let mut photo = on_free();
    let before = photo.free.expect("seeded").cam;
    photo.step(
        touch(Touch {
            drag: (0.25, 0.0),
            ..Touch::default()
        }),
        LIVE,
    );
    let after = photo.free.expect("seeded").cam;
    assert!((after.yaw - before.yaw + 0.25 * 60f32.to_radians()).abs() < 1e-4);
    assert_eq!(after.position, before.position);
    photo.step(
        touch(Touch {
            drag: (0.0, 0.1),
            ..Touch::default()
        }),
        LIVE,
    );
    assert!(photo.free.expect("seeded").cam.pitch > after.pitch);
}

#[test]
fn a_drag_with_the_drag_set_to_move_moves_the_free_lens() {
    let mut photo = on_free();
    photo.drag_moves = true;
    let before = eye_of(&photo);
    photo.step(
        touch(Touch {
            drag: (0.1, 0.0),
            ..Touch::default()
        }),
        LIVE,
    );
    assert!((eye_of(&photo) - before).length() > 1.0);
}

#[test]
fn pinching_dollies_the_free_lens_and_two_fingers_slide_it() {
    let mut photo = on_free();
    let cam = photo.free.expect("seeded").cam;
    let before = eye_of(&photo);
    photo.step(
        touch(Touch {
            pinch: 1.5,
            ..Touch::default()
        }),
        LIVE,
    );
    let moved = eye_of(&photo) - before;
    assert!(moved.dot(cam.forward()) > 0.0, "spreading goes forward");
    let before = eye_of(&photo);
    photo.step(
        touch(Touch {
            pan: (0.1, 0.0),
            ..Touch::default()
        }),
        LIVE,
    );
    let slid = eye_of(&photo) - before;
    assert!(slid.dot(cam.right()) < 0.0, "the scene follows the fingers");
    assert_eq!(photo.fov(), 60.0);
}

#[test]
fn the_free_lens_stays_inside_its_range() {
    let mut photo = on_free();
    for _ in 0..200 {
        photo.step(
            touch(Touch {
                pinch: 3.0,
                ..Touch::default()
            }),
            LIVE,
        );
    }
    assert!(eye_of(&photo).length() <= FREE_RANGE + 1e-3);
}

#[test]
fn a_twist_rolls_the_free_lens_the_way_it_turns() {
    let mut photo = on_free();
    photo.step(
        touch(Touch {
            twist: 0.2,
            ..Touch::default()
        }),
        LIVE,
    );
    let mut tilt = on_close();
    tilt.step(
        touch(Touch {
            twist: 0.2,
            ..Touch::default()
        }),
        LIVE,
    );
    // The same sign as the tilt a stick gives, which rolls by `-tilt`.
    assert!(photo.free.expect("seeded").cam.roll < 0.0 && tilt.tilt > 0.0);
}

#[test]
fn touch_is_ignored_while_the_panel_is_up() {
    let mut photo = on_close();
    photo.panel = true;
    let before = photo.orbit();
    photo.step(
        touch(Touch {
            drag: (0.3, 0.3),
            ..Touch::default()
        }),
        LIVE,
    );
    assert_eq!(photo.orbit(), before);
}
