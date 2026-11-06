//! Photo mode's camera laws, off a made-up craft: no disc.

use super::*;

fn craft(slot: usize) -> Subject {
    Subject {
        slot,
        position: Vec3::new(10.0, 2.0, -30.0),
        orientation: Quat::IDENTITY,
    }
}

fn stick(x: f32, y: f32) -> Pad {
    Pad {
        stick: (x, y),
        ..Pad::default()
    }
}

fn on_close() -> Photo {
    let mut photo = Photo::new(0, true, None);
    photo.cycle_lens();
    assert_eq!(photo.lens(), Lens::Close);
    photo
}

#[test]
fn it_opens_on_the_internal_lens_with_the_help_up_and_the_hud_off() {
    let photo = Photo::new(3, false, None);
    assert_eq!(photo.lens(), Lens::Internal);
    assert_eq!(photo.subject(), 3);
    assert!(photo.help());
    assert!(!photo.hud());
}

#[test]
fn the_internal_lens_is_five_ahead_of_the_nose_and_the_close_lens_twelve_behind_and_three_above() {
    let craft = craft(0);
    let internal = Photo::new(0, false, None).pose(&craft, None);
    assert!((internal.eye - (craft.position + Vec3::new(0.0, 0.0, -5.0))).length() < 1e-5);
    let close = on_close().pose(&craft, None);
    assert!((close.eye - (craft.position + Vec3::new(0.0, 3.0, 12.0))).length() < 1e-5);
    assert_eq!(close.fov_deg, Some(65.0));
}

#[test]
fn the_orbit_keeps_the_eye_at_one_distance_from_the_craft_at_every_angle() {
    let craft = craft(0);
    let mut photo = on_close();
    let rest = (photo.pose(&craft, None).eye - craft.position).length();
    for step in 0..400 {
        let x = (step as f32 * 0.37).sin();
        let y = (step as f32 * 0.11).cos();
        photo.step(stick(x, y), &[0]);
        let reach = (photo.pose(&craft, None).eye - craft.position).length();
        assert!((reach - rest).abs() < 1e-3, "step {step}: {reach} against {rest}");
    }
}

#[test]
fn stick_up_lifts_the_eye_and_stick_left_swings_it_to_the_right_of_the_craft() {
    let craft = craft(0);
    let mut photo = on_close();
    let before = photo.pose(&craft, None).eye;
    photo.step(stick(0.0, 1.0), &[0]);
    assert!(photo.pose(&craft, None).eye.y > before.y);
    let mut photo = on_close();
    photo.step(stick(-1.0, 0.0), &[0]);
    assert!(photo.pose(&craft, None).eye.x > before.x);
}

#[test]
fn the_orbit_is_free_on_both_axes_and_wraps_at_two_pi() {
    let mut photo = on_close();
    let mut most: f32 = 0.0;
    let mut wrapped = false;
    let mut last = 0.0_f32;
    for _ in 0..1000 {
        photo.step(stick(0.0, 1.0), &[0]);
        let (_, pitch) = photo.orbit();
        assert!(pitch.abs() <= std::f32::consts::TAU + 1e-4);
        most = most.max(pitch);
        wrapped |= pitch < last - 1.0;
        last = pitch;
    }
    assert!(most > 6.0, "Pulse's clamp at 0.6 is gone: {most}");
    assert!(wrapped, "a turn past two pi comes back round");
}

#[test]
fn the_close_field_stops_at_ten_and_a_hundred() {
    let mut photo = on_close();
    let zoom = |y| Pad {
        select: true,
        stick: (0.0, y),
        ..Pad::default()
    };
    for _ in 0..1000 {
        photo.step(zoom(1.0), &[0]);
    }
    assert_eq!(photo.fov(), CLOSE_FOV.0);
    for _ in 0..1000 {
        photo.step(zoom(-1.0), &[0]);
    }
    assert_eq!(photo.fov(), CLOSE_FOV.1);
}

#[test]
fn the_track_field_stops_at_three_and_forty_and_the_stick_alone_zooms_it() {
    let mut photo = Photo::new(0, true, None);
    photo.cycle_lens();
    photo.cycle_lens();
    assert_eq!(photo.lens(), Lens::Track);
    assert_eq!(photo.fov(), TRACK_FOV.2);
    for _ in 0..1000 {
        photo.step(stick(0.0, 1.0), &[0]);
    }
    assert_eq!(photo.fov(), TRACK_FOV.0);
    for _ in 0..1000 {
        photo.step(stick(0.0, -1.0), &[0]);
    }
    assert_eq!(photo.fov(), TRACK_FOV.1);
}

#[test]
fn the_track_lens_sits_on_the_node_and_looks_at_the_craft() {
    let craft = craft(0);
    let mut photo = Photo::new(0, true, None);
    photo.cycle_lens();
    photo.cycle_lens();
    let node = craft.position + Vec3::new(40.0, 6.0, 0.0);
    let pose = photo.pose(&craft, Some(node));
    assert_eq!(pose.eye, node);
    let ahead = pose.orientation * Vec3::NEG_Z;
    let to_craft = (craft.position - node).normalize();
    assert!(ahead.dot(to_craft) > 0.95, "{ahead:?} against {to_craft:?}");
}

#[test]
fn the_lens_list_leaves_track_out_where_the_circuit_has_no_node() {
    let mut photo = Photo::new(0, false, None);
    let mut seen = Vec::new();
    for _ in 0..4 {
        photo.cycle_lens();
        seen.push(photo.lens());
    }
    assert_eq!(
        seen,
        [Lens::Close, Lens::Internal, Lens::Close, Lens::Internal]
    );
}

#[test]
fn pan_stops_at_five_either_way() {
    let mut photo = on_close();
    let pan = |x| Pad {
        square: true,
        stick: (x, 1.0),
        ..Pad::default()
    };
    for _ in 0..1000 {
        photo.step(pan(1.0), &[0]);
    }
    assert_eq!(photo.pan(), (PAN_LIMIT, PAN_LIMIT));
    for _ in 0..1000 {
        photo.step(pan(-1.0), &[0]);
    }
    assert_eq!(photo.pan().0, -PAN_LIMIT);
}

#[test]
fn reset_puts_the_orbit_the_tilt_the_pan_and_the_fields_back() {
    let mut photo = on_close();
    photo.step(stick(1.0, 1.0), &[0]);
    photo.step(
        Pad {
            select: true,
            stick: (1.0, 1.0),
            ..Pad::default()
        },
        &[0],
    );
    photo.step(
        Pad {
            square: true,
            stick: (1.0, 1.0),
            ..Pad::default()
        },
        &[0],
    );
    assert_ne!(photo.orbit(), (0.0, 0.0));
    assert_ne!(photo.tilt(), 0.0);
    assert_ne!(photo.pan(), (0.0, 0.0));
    photo.step(
        Pad {
            reset: true,
            ..Pad::default()
        },
        &[0],
    );
    assert_eq!(photo.orbit(), (0.0, 0.0));
    assert_eq!(photo.tilt(), 0.0);
    assert_eq!(photo.pan(), (0.0, 0.0));
    assert_eq!(photo.fov(), CLOSE_FOV.2);
}

#[test]
fn the_craft_is_chosen_with_a_wrap_and_a_craft_out_of_the_race_is_skipped() {
    let mut photo = Photo::new(0, false, None);
    let next = Pad {
        next: true,
        ..Pad::default()
    };
    let previous = Pad {
        previous: true,
        ..Pad::default()
    };
    let live = [0, 2, 5];
    photo.step(next, &live);
    assert_eq!(photo.subject(), 2);
    photo.step(next, &live);
    photo.step(next, &live);
    assert_eq!(photo.subject(), 0, "wraps over the end");
    photo.step(previous, &live);
    assert_eq!(photo.subject(), 5, "wraps over the start");
    photo.step(next, &[0, 2]);
    assert_ne!(photo.subject(), 5);
}

#[test]
fn a_craft_that_leaves_the_race_hands_the_picture_to_one_that_is_in_it() {
    let mut photo = Photo::new(4, false, None);
    photo.step(Pad::default(), &[1, 2]);
    assert_eq!(photo.subject(), 1);
}

#[test]
fn no_shot_is_taken_in_the_guard_and_one_is_after_it() {
    let mut photo = Photo::new(0, false, None);
    let shot = Pad {
        shot: true,
        ..Pad::default()
    };
    for tick in 1..GUARD_TICKS {
        assert_eq!(photo.step(shot, &[0]), Action::None, "tick {tick}");
    }
    assert_eq!(photo.step(shot, &[0]), Action::Shot);
}

#[test]
fn circle_leaves_even_inside_the_guard() {
    let mut photo = Photo::new(0, false, None);
    let leave = Pad {
        leave: true,
        ..Pad::default()
    };
    assert_eq!(photo.step(leave, &[0]), Action::Leave);
}

#[test]
fn a_tap_on_select_changes_the_lens_and_a_hold_with_the_stick_does_not() {
    let mut photo = Photo::new(0, true, None);
    let held = |stick| Pad {
        select: true,
        stick,
        ..Pad::default()
    };
    photo.step(held((0.0, 0.0)), &[0]);
    photo.step(Pad::default(), &[0]);
    assert_eq!(photo.lens(), Lens::Close);
    photo.step(held((0.0, 0.5)), &[0]);
    photo.step(Pad::default(), &[0]);
    assert_eq!(photo.lens(), Lens::Close, "the stick made the hold a modifier");
}

#[test]
fn a_tap_on_square_turns_the_hud_on_and_a_pan_does_not() {
    let mut photo = on_close();
    let held = |stick| Pad {
        square: true,
        stick,
        ..Pad::default()
    };
    photo.step(held((0.0, 0.0)), &[0]);
    photo.step(Pad::default(), &[0]);
    assert!(photo.hud());
    photo.step(held((1.0, 0.0)), &[0]);
    photo.step(Pad::default(), &[0]);
    assert!(photo.hud(), "a pan is not a tap");
}

#[test]
fn start_hides_and_shows_the_help() {
    let mut photo = Photo::new(0, false, None);
    let start = Pad {
        help: true,
        ..Pad::default()
    };
    photo.step(start, &[0]);
    assert!(!photo.help());
    photo.step(start, &[0]);
    assert!(photo.help());
}

#[test]
fn a_jump_of_the_picture_is_counted_so_the_upscaler_drops_its_history() {
    let mut photo = Photo::new(0, true, None);
    let before = photo.cuts();
    photo.cycle_lens();
    assert_ne!(photo.cuts(), before);
}
