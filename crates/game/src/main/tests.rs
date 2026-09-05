//! What the composition root in [`super`] is asserted to do: trace-row poses,
//! the backdrop playhead and picture carried into the menus, and the
//! conversion-state flag.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of `main.rs`, and
//! `main/tests.rs` rather than the `crates/game/tests/` beside it: **an
//! integration test links the library, not the binary**, so nothing under
//! `crates/game/tests/` can see `pose_from_trace`, `menu_playhead` or
//! `parse_progress` at all. This is `crates/physics/src/airbrake.rs`'s
//! arrangement - `#[cfg(test)] mod tests;` and a `tests.rs` in the module's own
//! directory - which `main.rs` could not use until the 2026-08-16 split gave a
//! crate root a directory. See `scripts/check-file-size.py`, the rule as a gate.

use super::*;

/// A two-row capture in exactly the shape `psp-trace.py --camera` writes:
/// the required columns plus the all-or-nothing camera group. Hand-authored
/// like `oag_trace::trace`'s own fixtures - a real capture cannot be
/// committed.
const FIXTURE: &str = "\
tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,\
right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,\
pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed,\
cam_right_x,cam_right_y,cam_right_z,cam_up_x,cam_up_y,cam_up_z,\
cam_fwd_x,cam_fwd_y,cam_fwd_z,cam_pos_x,cam_pos_y,cam_pos_z
5,0.016683,1,100,0,0,0,0,22,1,0,0,0,1,0,0,0,1,10,2.5,-30,0,0,22,22,1,0,0,0,1,0,0,0,1,10,8,-45
6,0.016683,1,100,0,0,0,0,22,1,0,0,0,1,0,0,0,1,10,2.5,-29.6,0,0,22,22,1,0,0,0,1,0,0,0,1,10,8,-44.6
";

fn fixture_file(name: &str, text: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("oag-game-pose-from-{name}.csv"));
    std::fs::write(&path, text).expect("a writable temp dir");
    path
}

#[test]
fn a_pose_from_a_trace_row_is_exact_and_carries_the_camera() {
    let path = fixture_file("carries-camera", FIXTURE);
    let (pose, camera) = pose_from_trace(&path, 5, false, Some(70.0)).expect("the fixture row");
    let race::PoseRequest::Exact(pose) = pose else {
        panic!("--pose-from must not re-derive the pose from the spline");
    };
    assert_eq!(pose.position, oag_core::math::Vec3::new(10.0, 2.5, -30.0));
    let camera = camera.expect("the fixture has camera columns");
    assert_eq!(camera.eye, oag_core::math::Vec3::new(10.0, 8.0, -45.0));
    assert_eq!(camera.fov_deg, Some(70.0));
}

#[test]
fn no_camera_keeps_the_chase_camera_even_when_the_capture_has_one() {
    let path = fixture_file("no-camera", FIXTURE);
    let (_, camera) = pose_from_trace(&path, 5, true, None).expect("the fixture row");
    assert!(camera.is_none());
}

/// One frame of the PSP's backdrop, to the nanosecond.
const BACKDROP_FRAME: f64 = 1001.0 / 30_000.0;

#[test]
fn the_menus_open_on_the_playhead_the_front_end_was_running() {
    // The bug this is here for: the boot sequence handed over a backdrop
    // that was 400 frames into its loop and the menus started a new one at
    // zero, so the picture jumped back to the start of the movie the instant
    // START was pressed.
    let mut running = movie::Player::new(270, true, movie::FRAME_RATE);
    for _ in 0..400 {
        running.update(BACKDROP_FRAME);
    }
    assert_eq!(running.position(), 400);

    let opened = menu_playhead(Some(running), 270, movie::FRAME_RATE);
    assert_eq!(
        opened.position(),
        400,
        "the menus continue the playback rather than restarting it"
    );
    assert_eq!(opened.frame(), 400 % 270, "and it is mid-loop, not at zero");
    assert!(!opened.is_finished());
}

#[test]
fn a_playhead_carried_across_keeps_running_from_where_it_was() {
    // Not just the position at the handoff: the next frame after it has to
    // be the next frame of the same playback, wrap included.
    let mut running = movie::Player::new(270, true, movie::FRAME_RATE);
    for _ in 0..269 {
        running.update(BACKDROP_FRAME);
    }
    let mut opened = menu_playhead(Some(running), 270, movie::FRAME_RATE);
    opened.update(BACKDROP_FRAME);
    assert_eq!(opened.position(), 270);
    assert_eq!(opened.frame(), 0, "it wraps rather than ending");
}

#[test]
fn with_nothing_to_carry_the_menus_start_the_loop_themselves() {
    // Leaving a race: the stage that owned the playhead is gone, and
    // `open_menus` restarts the feed to match this. See `menu_playhead`.
    let fresh = menu_playhead(None, 270, movie::FRAME_RATE);
    assert_eq!(fresh.position(), 0);
    assert_eq!(fresh.frames(), 270);
    assert!(!fresh.is_finished(), "270 frames of loop are not an ending");
}

/// A `HeldFrame` distinguishable from any other by its index alone - the
/// picture is never inspected by [`backdrop_seed`], only carried.
fn held(index: usize) -> HeldFrame {
    HeldFrame {
        index,
        picture: movie::VideoFrame::default(),
    }
}

/// Task #8: `escape` -> menus used to show one to three black frames while a
/// freshly restarted feed produced its first picture, because nothing carried
/// a picture across the race the way the boot path already carried one from
/// the front end. `backdrop_seed` is the decision at the heart of the fix;
/// `Session::open_menus` is the GPU-touching caller around it, which is why
/// the assertions are on this pure core rather than on a live `Session`.
#[test]
fn escape_seeds_the_menus_from_the_race_side_backdrop_not_the_frontend_side_one() {
    let from_frontend = held(11);
    let from_race = held(22);
    let seeded = backdrop_seed(Some(&from_frontend), true, Some(&from_race))
        .expect("a race that carried a picture seeds one");
    assert_eq!(
        seeded.index, 22,
        "escape reads held_menu_backdrop, not the frontend's held_backdrop, \
         which is stale or absent once a race has run"
    );
}

#[test]
fn booting_into_the_menus_seeds_from_the_frontend_side_backdrop() {
    let from_frontend = held(11);
    let seeded = backdrop_seed(Some(&from_frontend), false, None)
        .expect("the boot path seeds from the front end's held frame");
    assert_eq!(seeded.index, 11);
}

#[test]
fn a_source_with_no_backdrop_seeds_nothing_on_either_path() {
    assert!(
        backdrop_seed(None, false, None).is_none(),
        "no front-end picture, boot path"
    );
    assert!(
        backdrop_seed(None, true, None).is_none(),
        "no held race picture, escape path"
    );
}

/// The two figures, and the three fields that follow from them.
#[test]
fn a_stated_conversion_state_is_read_as_the_worker_would_report_it() {
    let midway = parse_progress("37/115").expect("37 of 115");
    assert!(!midway.planning, "a stated total means planning is over");
    assert!(!midway.finished);
    assert_eq!((midway.done, midway.total), (37, 115));
    assert!(midway.current.is_some(), "something is converting");

    let done = parse_progress("115/115").expect("all of them");
    assert!(done.finished);
    assert_eq!(done.fraction(), 1.0);
    assert_eq!(done.current, None, "nothing is converting any more");
}

/// Every rejection names what to write instead, because the flag is typed
/// by hand and the shape is not guessable.
#[test]
fn a_state_that_is_not_two_numbers_is_refused() {
    for spec in ["37", "37/", "a/b", "37 115", ""] {
        let error = parse_progress(spec).expect_err("{spec} is not a state");
        assert!(error.to_string().contains("DONE/TOTAL"), "{spec}: {error}");
    }
    let error = parse_progress("200/115").expect_err("more than all of them");
    assert!(error.to_string().contains("more than all"), "{error}");
}

#[test]
fn a_missing_tick_is_an_error_that_names_the_range() {
    let path = fixture_file("missing-tick", FIXTURE);
    let error = pose_from_trace(&path, 99, false, None).expect_err("tick 99 is not there");
    assert!(error.to_string().contains("no tick 99"), "{error}");
}

/// `--icon-size` is `requires = "write_icon"` with its own `default_value_t`,
/// which is the exact clap footgun where a defaulted arg counts as "present"
/// for requirement validation and every ordinary run - none of which pass
/// `--write-icon` - refuses to start. Clap 4 scopes `requires` to arguments
/// actually seen on the command line, so this passes today; it exists to
/// catch a clap upgrade (or a copy-pasted `requires` on some future flag)
/// that changes that.
#[test]
fn the_icon_flags_do_not_make_themselves_required() {
    Cli::try_parse_from(["oag-game"]).expect("no flags at all must still parse");
}

/// A one-page, one-row definition whose row is a binding for `button` -
/// [`menu::tests::an_unbound_button_draws_the_word_rather_than_nothing`]'s
/// fixture shape, reused here for `Session::maybe_begin_binding` rather than
/// for drawing.
fn binding_menu(button: &str) -> menu::Menu {
    let definition = menu::Definition::parse(
        &format!(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            [[page.entry]]
            kind = "binding"
            label = "ROW"
            button = "{button}"
            "#
        ),
        &language::StringTable::default(),
    )
    .expect("a one-row binding definition parses");
    menu::Menu::new(definition)
}

#[test]
fn confirming_a_binding_row_starts_a_capture_for_its_button() {
    let menu = binding_menu("cross");
    let mut controls = Controls::without_pad();
    let mut awaiting = None;
    controls
        .buttons_mut()
        .begin_frame(oag_gameplay::input::Button::Cross.bit());

    Session::maybe_begin_binding(&mut awaiting, &mut controls, &menu);

    assert_eq!(awaiting, Some(oag_gameplay::input::Button::Cross));
}

#[test]
fn confirming_with_start_works_the_same_as_cross() {
    let menu = binding_menu("up");
    let mut controls = Controls::without_pad();
    let mut awaiting = None;
    controls
        .buttons_mut()
        .begin_frame(oag_gameplay::input::Button::Start.bit());

    Session::maybe_begin_binding(&mut awaiting, &mut controls, &menu);

    assert_eq!(awaiting, Some(oag_gameplay::input::Button::Up));
}

#[test]
fn nothing_pressed_leaves_the_capture_closed() {
    let menu = binding_menu("cross");
    let mut controls = Controls::without_pad();
    let mut awaiting = None;

    Session::maybe_begin_binding(&mut awaiting, &mut controls, &menu);

    assert_eq!(awaiting, None);
}

/// A capture already open must not be disturbed by a second confirm - holding
/// Cross down through the capture must not restart it against whatever row
/// the cursor has since moved to.
#[test]
fn a_capture_already_open_is_left_alone() {
    let menu = binding_menu("up");
    let mut controls = Controls::without_pad();
    controls
        .buttons_mut()
        .begin_frame(oag_gameplay::input::Button::Cross.bit());
    let mut awaiting = Some(oag_gameplay::input::Button::Circle);

    Session::maybe_begin_binding(&mut awaiting, &mut controls, &menu);

    assert_eq!(awaiting, Some(oag_gameplay::input::Button::Circle));
}

/// `app.rs` diverts every keyboard event away from `Controls::set_key` for as
/// long as a capture is open, so the key that confirmed this row never gets
/// to report its own release - the same failure `Keyboard::release_all`'s own
/// doc comment describes for a window losing focus while a key is held.
/// Without the release in `maybe_begin_binding`, Cross would still read held
/// on the very next tick's snapshot, with nobody touching a keyboard.
#[test]
fn opening_a_capture_releases_whatever_confirmed_it() {
    let menu = binding_menu("cross");
    let mut controls = Controls::without_pad();
    controls.set_key(&winit::keyboard::Key::Character("x".into()), true);
    controls
        .buttons_mut()
        .begin_frame(oag_gameplay::input::Button::Cross.bit());
    let mut awaiting = None;

    Session::maybe_begin_binding(&mut awaiting, &mut controls, &menu);
    assert_eq!(awaiting, Some(oag_gameplay::input::Button::Cross));

    let next_tick = controls.snapshot();
    assert!(
        !next_tick
            .buttons
            .is_held(oag_gameplay::input::Button::Cross),
        "the swallowed key-up must not leave Cross stuck held"
    );
}
