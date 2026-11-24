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
        assert!(
            (reach - rest).abs() < 1e-3,
            "step {step}: {reach} against {rest}"
        );
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
    assert_eq!(seen, [Lens::Close, Lens::Free, Lens::Internal, Lens::Close]);
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
    assert_eq!(
        photo.lens(),
        Lens::Close,
        "the stick made the hold a modifier"
    );
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
fn start_cycles_the_help_then_the_effects_panel_then_neither() {
    let mut photo = Photo::new(0, false, None);
    let start = Pad {
        help: true,
        ..Pad::default()
    };
    photo.step(start, &[0]);
    assert!(photo.help() && photo.panel(), "the panel after the help");
    photo.step(start, &[0]);
    assert!(!photo.help() && !photo.panel(), "then neither");
    photo.step(start, &[0]);
    assert!(photo.help() && !photo.panel(), "and the help again");
}

fn pushed(x: f32, y: f32) -> Pad {
    stick(x, y)
}

fn on_the_panel() -> Photo {
    let mut photo = Photo::new(0, false, None);
    photo.toggle_panel();
    photo
}

#[test]
fn on_the_panel_the_stick_moves_the_cursor_and_the_value_and_the_camera_stands_still() {
    use effects::Row;
    let mut photo = on_the_panel();
    let pose = photo.pose(&craft(0), None);
    assert_eq!(photo.effects().selected(), Row::Exposure);
    // Down is negative on the stick; the list reads downward.
    photo.step(pushed(0.0, -1.0), &[0]);
    photo.step(pushed(0.0, 0.0), &[0]);
    assert_eq!(photo.effects().selected(), Row::Saturation);
    photo.step(pushed(1.0, 0.0), &[0]);
    photo.step(pushed(0.0, 0.0), &[0]);
    assert_eq!(photo.effects().value(Row::Saturation), 21);
    assert_eq!(
        photo.effects().value(Row::Exposure),
        33,
        "only the cursor's row moved"
    );
    assert_eq!(photo.pose(&craft(0), None), pose, "the camera did not move");
}

#[test]
fn a_held_push_repeats_after_a_pause() {
    use effects::Row;
    let mut photo = on_the_panel();
    for _ in 0..60 {
        photo.step(pushed(1.0, 0.0), &[0]);
    }
    let moved = photo.effects().value(Row::Exposure) - 33;
    assert!(moved > 10, "a held push keeps going: {moved}");
    assert!(moved < 40, "but not at the tick rate: {moved}");
}

#[test]
fn every_control_opens_neutral_and_returning_to_its_opening_is_neutral_again() {
    use effects::{Effects, Row};
    let mut effects = Effects::new();
    assert!(effects.is_neutral());
    assert!(
        effects
            .params(1.0, 500.0)
            .grade
            .is_some_and(|g| g.is_neutral())
    );
    effects.select(Row::Exposure);
    effects.adjust(1);
    assert!(!effects.is_neutral());
    effects.adjust(-1);
    assert!(effects.is_neutral(), "back where it opened");
    effects.select(Row::SpeedBlur);
    effects.adjust(1);
    assert_eq!(effects.value(Row::SpeedBlur), 5, "HD's step");
    assert!((effects.speed_blur() - 0.05).abs() < 1e-6);
    effects.adjust(-1);
    effects.select(Row::DepthOfField);
    effects.adjust(1);
    assert!(!effects.is_neutral());
    assert_eq!(
        effects.rows().len(),
        6,
        "aperture and focus appear with depth of field"
    );
    effects.adjust(-1);
    assert_eq!(effects.rows().len(), 4);
    assert!(effects.is_neutral());
}

#[test]
fn the_sliders_stop_at_zero_and_a_hundred_and_the_opening_values_are_hds() {
    use effects::{Effects, Row};
    let mut effects = Effects::new();
    assert_eq!(effects.value(Row::Saturation), 20);
    assert_eq!(effects.value(Row::Exposure), 33);
    effects.select(Row::Saturation);
    for _ in 0..200 {
        effects.adjust(1);
    }
    assert_eq!(effects.value(Row::Saturation), 100);
    // 100 * 0.05 = 5, HD's top.
    let grade = effects.params(1.0, 500.0).grade.expect("a grade");
    assert!((grade.saturation - 5.0).abs() < 1e-5);
    for _ in 0..200 {
        effects.adjust(-1);
    }
    assert_eq!(effects.value(Row::Saturation), 0);
    assert!(
        effects
            .params(1.0, 500.0)
            .grade
            .expect("a grade")
            .saturation
            .abs()
            < 1e-5
    );
}

#[test]
fn exposure_up_is_brighter_and_the_effects_die_with_the_mode() {
    use effects::Row;
    let mut photo = on_the_panel();
    photo.effects_mut().adjust(1);
    assert!(photo.effects().value(Row::Exposure) > 33);
    let grade = photo.effects().params(1.0, 500.0).grade.expect("a grade");
    assert!(grade.exposure > 0.0, "more exposure is more light");
    let fresh = Photo::new(0, false, None);
    assert!(
        fresh.effects().is_neutral(),
        "a mode opened again starts neutral"
    );
}

#[test]
fn the_speed_blur_is_the_zoom_ring_held_at_the_rows_value_and_needs_the_ring() {
    use effects::Row;
    let mut photo = on_the_panel();
    photo.effects_mut().select(Row::SpeedBlur);
    photo.effects_mut().adjust(4);
    assert!(photo.speed_blur().is_none(), "no ring handed in, no streak");
    let mut photo = photo.with_zoom(Some(crate::zoom::tuning(&oag_hd::race::ZOOM_RING)));
    let (_, frame) = photo.speed_blur().expect("a pulse");
    assert!((frame.boost - 0.2).abs() < 1e-6 && frame.damage == 0.0);
    let first = frame.tick;
    photo.step(stick(0.0, 0.0), &[0]);
    assert_ne!(
        photo.speed_blur().expect("a pulse").1.tick,
        first,
        "a tick of its own per step"
    );
    photo.effects_mut().adjust(-4);
    assert!(photo.speed_blur().is_none(), "at zero the ring is off");
}

#[test]
fn a_jump_of_the_picture_is_counted_so_the_upscaler_drops_its_history() {
    let mut photo = Photo::new(0, true, None);
    let before = photo.cuts();
    photo.cycle_lens();
    assert_ne!(photo.cuts(), before);
}

#[test]
fn letting_go_of_the_button_that_opened_the_mode_is_not_a_tap() {
    let mut photo = Photo::new(0, true, None).holding(true, true);
    photo.step(
        Pad {
            select: true,
            square: true,
            ..Pad::default()
        },
        &[0],
    );
    photo.step(Pad::default(), &[0]);
    assert_eq!(photo.lens(), Lens::Internal);
    assert!(!photo.hud());
}

#[test]
fn the_right_stick_zooms_up_and_down_and_tilts_left_and_right_and_does_not_orbit() {
    let mut photo = on_close();
    for _ in 0..10 {
        photo.step(
            Pad {
                right: (1.0, 1.0),
                ..Pad::default()
            },
            &[0],
        );
    }
    assert!(photo.fov() < CLOSE_FOV.2, "up zooms in: {}", photo.fov());
    assert!(photo.tilt() > 0.0, "right tilts: {}", photo.tilt());
    assert_eq!(photo.orbit(), (0.0, 0.0));
    assert_eq!(photo.pan(), (0.0, 0.0));
}

#[test]
fn the_right_stick_zooms_and_tilts_the_track_lens_and_does_nothing_on_the_internal_one() {
    let mut track = Photo::new(0, true, None);
    track.cycle_lens();
    track.cycle_lens();
    assert_eq!(track.lens(), Lens::Track);
    track.step(
        Pad {
            right: (0.5, 1.0),
            ..Pad::default()
        },
        &[0],
    );
    assert!(track.fov() < TRACK_FOV.2);
    assert!(track.tilt() > 0.0);
    let mut internal = Photo::new(0, false, None);
    internal.step(
        Pad {
            right: (1.0, 1.0),
            ..Pad::default()
        },
        &[0],
    );
    assert_eq!(
        (internal.tilt(), internal.fov()),
        (0.0, Photo::new(0, false, None).fov())
    );
}

#[test]
fn a_stick_moved_with_square_held_pans_and_is_not_a_hud_tap() {
    let mut photo = on_close();
    let down = Pad {
        square: true,
        stick: (1.0, 0.0),
        ..Pad::default()
    };
    photo.step(down, &[0]);
    photo.step(Pad::default(), &[0]);
    assert!(!photo.hud(), "the stick used SQUARE as a modifier");
    assert!(photo.pan().0 > 0.0, "and panned");
}

#[test]
fn step_among_wraps_skips_and_lands_on_the_first_when_the_craft_is_gone() {
    assert_eq!(step_among(&[0, 2, 5], 2, true), Some(5));
    assert_eq!(step_among(&[0, 2, 5], 5, true), Some(0));
    assert_eq!(step_among(&[0, 2, 5], 0, false), Some(5));
    assert_eq!(
        step_among(&[0, 2, 5], 3, true),
        Some(0),
        "3 is out of the race"
    );
    assert_eq!(step_among(&[4], 4, true), Some(4));
    assert_eq!(step_among(&[], 4, true), None);
}

#[test]
fn a_scrub_moves_the_rewind_row_by_the_pull_and_stops_at_both_ends() {
    use crate::photo::effects::{Effects, scrub_ticks};
    let mut effects = Effects::default().offering(false, true);
    effects.set_live(100);
    effects.scrub(-30);
    assert_eq!(effects.target_tick(), 70);
    effects.scrub(-500);
    assert_eq!(effects.target_tick(), 0);
    effects.scrub(500);
    assert_eq!(effects.target_tick(), 100);
    assert_eq!(scrub_ticks(0.0, 30), 0);
    assert_eq!(scrub_ticks(0.05, 30), 1);
    assert_eq!(scrub_ticks(1.0, 30), 30);
    assert!(scrub_ticks(0.5, 30) < scrub_ticks(0.8, 30));
}

fn pose_at(eye: Vec3, yaw: f32, pitch: f32, roll: f32) -> CameraOverride {
    CameraOverride {
        eye,
        orientation: Free {
            position: eye,
            yaw,
            pitch,
            roll,
        }
        .orientation(),
        fov_deg: Some(70.0),
    }
}

/// The free lens opened on a rolled, pitched pose, and the craft it is relative to.
fn on_free() -> (Photo, Subject, CameraOverride) {
    let craft = craft(0);
    let start = pose_at(craft.position + Vec3::new(3.0, 5.0, 20.0), 0.7, 0.2, 0.3);
    (
        Photo::new(0, true, None).opening_on(start, craft.position),
        craft,
        start,
    )
}

fn camera_of(photo: &Photo, craft: &Subject) -> Free {
    let pose = photo.pose(craft, None);
    Free::from_orientation(pose.eye, pose.orientation)
}

/// Choosing the free lens from another starts on the picture that lens drew, its field included.
/// Fails while the seed is held to the close lens's field: the track lens zooms to 3 degrees.
#[test]
#[ignore = "photo-fx-review finding 10"]
fn the_free_lens_seeded_from_a_zoomed_track_lens_keeps_its_field() {
    let craft = craft(0);
    let mut photo = Photo::new(0, true, None);
    photo.cycle_lens();
    photo.cycle_lens();
    assert_eq!(photo.lens(), Lens::Track);
    for _ in 0..40 {
        photo.step(stick(0.0, 1.0), &[0]);
    }
    assert_eq!(photo.fov(), TRACK_FOV.0, "zoomed all the way in");
    let node = Some(craft.position + Vec3::new(40.0, 10.0, 0.0));
    let on_screen = photo.pose(&craft, node);
    assert_eq!(on_screen.fov_deg, Some(TRACK_FOV.0));
    photo.cycle_lens();
    assert!(photo.needs_free_seed());
    photo.seed_free(on_screen, craft.position);
    assert_eq!(
        photo.pose(&craft, node).fov_deg,
        on_screen.fov_deg,
        "the picture jumped: the free lens opened on another field"
    );
}

/// The free lens stands where it was seeded until it is moved. Fails while the range is applied
/// by every tick of the lens and not by the seed: a seed further than `FREE_RANGE` from the
/// craft is drawn once where it was and then pulled in with nothing held.
#[test]
#[ignore = "photo-fx-review finding 10"]
fn the_free_lens_seeded_beyond_its_range_does_not_move_with_nothing_held() {
    let craft = craft(0);
    let far = pose_at(craft.position + Vec3::new(450.0, 20.0, 0.0), 0.0, 0.0, 0.0);
    let mut photo = Photo::new(0, true, None).opening_on(far, craft.position);
    let seeded = photo.pose(&craft, None).eye;
    assert!(
        (seeded - far.eye).length() < 1e-3,
        "the first frame is the seed"
    );
    photo.step(Pad::default(), &[0]);
    let after = photo.pose(&craft, None).eye;
    assert!(
        (after - seeded).length() < 1e-3,
        "the eye moved {} units with nothing held",
        (after - seeded).length()
    );
}

#[test]
fn opening_on_a_pose_shows_that_pose_to_the_bit() {
    let (photo, craft, start) = on_free();
    assert_eq!(photo.lens(), Lens::Free);
    let pose = photo.pose(&craft, None);
    assert!((pose.eye - start.eye).length() < 1e-4, "{pose:?}");
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        assert!((pose.orientation * axis - start.orientation * axis).length() < 1e-4);
    }
    assert_eq!(pose.fov_deg, Some(70.0));
}

#[test]
fn the_free_lens_walks_the_eye_along_its_own_axes() {
    let (mut photo, craft, start) = on_free();
    let eye = Free::from_orientation(start.eye, start.orientation);
    let ticks = 10;
    // Held for ten ticks the speed has ramped as `fly` ramps it.
    let travelled: f32 = (1..=ticks)
        .map(|t| {
            FREE_WALK_RATE * (1.0 + (FREE_WALK_BOOST - 1.0) * (t as f32 / FREE_RAMP_TICKS as f32))
        })
        .sum();
    for _ in 0..ticks {
        photo.step(stick(0.0, 1.0), &[0]);
    }
    let moved = photo.pose(&craft, None).eye - start.eye;
    assert!(
        (moved - eye.forward() * travelled).length() < 1e-3,
        "{moved:?}"
    );
    let before = photo.pose(&craft, None).eye;
    for _ in 0..ticks {
        photo.step(stick(1.0, 0.0), &[0]);
    }
    let strafed = photo.pose(&craft, None).eye - before;
    assert!(strafed.normalize().dot(eye.right()) > 0.999, "{strafed:?}");
    let before = photo.pose(&craft, None).eye;
    let lift = Pad {
        square: true,
        stick: (0.0, 1.0),
        ..Pad::default()
    };
    photo.step(lift, &[0]);
    let lifted = photo.pose(&craft, None).eye - before;
    assert!(lifted.normalize().dot(eye.up()) > 0.999, "{lifted:?}");
}

#[test]
fn holding_a_direction_speeds_the_free_lens_up_and_letting_go_slows_it() {
    let (mut photo, craft, _) = on_free();
    let step_length = |photo: &mut Photo, pad: Pad| {
        let before = photo.pose(&craft, None).eye;
        photo.step(pad, &[0]);
        (photo.pose(&craft, None).eye - before).length()
    };
    let first = step_length(&mut photo, stick(0.0, 1.0));
    for _ in 0..FREE_RAMP_TICKS {
        photo.step(stick(0.0, 1.0), &[0]);
    }
    let held = step_length(&mut photo, stick(0.0, 1.0));
    assert!(
        (held / first - FREE_WALK_BOOST).abs() < 0.2,
        "{first} {held}"
    );
    photo.step(Pad::default(), &[0]);
    let again = step_length(&mut photo, stick(0.0, 1.0));
    assert!((again - first).abs() < 1e-3, "{again} against {first}");
}

#[test]
fn the_right_stick_looks_and_pitch_stops_short_of_vertical() {
    let (mut photo, craft, _) = on_free();
    let before = camera_of(&photo, &craft);
    photo.step(
        Pad {
            right: (1.0, 0.0),
            ..Pad::default()
        },
        &[0],
    );
    let turned = camera_of(&photo, &craft);
    assert!(
        (turned.yaw - before.yaw - FREE_LOOK_RATE).abs() < 1e-4,
        "{turned:?}"
    );
    for _ in 0..1000 {
        photo.step(
            Pad {
                right: (0.0, 1.0),
                ..Pad::default()
            },
            &[0],
        );
    }
    assert!((camera_of(&photo, &craft).pitch - oag_mesh::orbit::PITCH_LIMIT).abs() < 1e-3);
    for _ in 0..2000 {
        photo.step(
            Pad {
                right: (0.0, -1.0),
                ..Pad::default()
            },
            &[0],
        );
    }
    assert!((camera_of(&photo, &craft).pitch + oag_mesh::orbit::PITCH_LIMIT).abs() < 1e-3);
}

#[test]
fn select_with_the_stick_zooms_and_rolls_the_free_lens_within_the_close_field() {
    let (mut photo, craft, _) = on_free();
    let roll_before = camera_of(&photo, &craft).roll;
    let pad = Pad {
        select: true,
        stick: (1.0, 1.0),
        ..Pad::default()
    };
    photo.step(pad, &[0]);
    assert!((photo.fov() - (70.0 - ZOOM_RATE)).abs() < 1e-4);
    assert!(camera_of(&photo, &craft).roll < roll_before);
    for _ in 0..1000 {
        photo.step(pad, &[0]);
    }
    assert_eq!(photo.fov(), CLOSE_FOV.0);
}

#[test]
fn the_free_eye_stays_within_range_of_the_craft_and_triangle_comes_home() {
    let (mut photo, craft, start) = on_free();
    for _ in 0..5000 {
        photo.step(stick(0.0, 1.0), &[0]);
    }
    let away = photo.pose(&craft, None).eye - craft.position;
    assert!(
        (away.length() - FREE_RANGE).abs() < 1e-2,
        "{}",
        away.length()
    );
    photo.step(
        Pad {
            reset: true,
            ..Pad::default()
        },
        &[0],
    );
    let pose = photo.pose(&craft, None);
    assert!((pose.eye - start.eye).length() < 1e-4);
    assert_eq!(pose.fov_deg, Some(70.0));
}

#[test]
fn a_drag_looks_by_default_and_moves_with_the_drag_chip_and_the_wheel_dollies() {
    let (mut photo, craft, _) = on_free();
    let before = camera_of(&photo, &craft);
    let drag = Pad {
        drag: (100.0, 0.0),
        ..Pad::default()
    };
    photo.step(drag, &[0]);
    let looked = camera_of(&photo, &craft);
    assert!((looked.yaw - before.yaw - 100.0 * FREE_DRAG_LOOK).abs() < 1e-4);
    assert!(
        (looked.position - before.position).length() < 1e-4,
        "a look does not travel"
    );
    photo.toggle_drag_moves();
    photo.step(drag, &[0]);
    let moved = camera_of(&photo, &craft);
    assert!(
        (moved.yaw - looked.yaw).abs() < 1e-4,
        "a move does not turn"
    );
    assert!(((moved.position - looked.position).length() - 100.0 * FREE_DRAG_MOVE).abs() < 1e-3);
    let wheel = Pad {
        scroll: -2.0,
        ..Pad::default()
    };
    photo.step(wheel, &[0]);
    let dollied = camera_of(&photo, &craft);
    let along = (dollied.position - moved.position).dot(dollied.forward());
    assert!((along - 2.0 * FREE_DOLLY).abs() < 1e-3, "{along}");
}

#[test]
fn the_free_eye_keeps_its_offset_when_the_craft_moves_or_changes() {
    let (photo, craft, _) = on_free();
    let mut later = craft;
    later.position += Vec3::new(40.0, 0.0, 0.0);
    let (a, b) = (photo.pose(&craft, None), photo.pose(&later, None));
    assert!((b.eye - a.eye - Vec3::new(40.0, 0.0, 0.0)).length() < 1e-4);
    assert!((a.orientation * Vec3::NEG_Z - b.orientation * Vec3::NEG_Z).length() < 1e-6);
}
