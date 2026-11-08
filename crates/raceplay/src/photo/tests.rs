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
