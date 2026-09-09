//! Which clock each movie runs on: the intro on its own sound, the backdrop
//! on the tick, and the playhead handed on between screens.
//!
//! Split out of `frontend/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

/// **The backdrop is tick-clocked, whatever the intro's sound is doing.**
///
/// ADR-0019's rule is per movie, and the two movies here are not the same
/// movie: the intro has a track and `Backdrop.PMF` has none - it is
/// video-only *and* its `FE Screen` widget is `sound="false"`. The playhead
/// passed into `update` belongs to the intro, which plays over the top of
/// this loop for the whole boot sequence, so a backdrop that read it would
/// be paced by a movie that ends - and would stop dead when it did.
///
/// Asserted as an equality against the same run with no playhead at all,
/// rather than as a range, because the property is that the value is
/// *ignored* rather than that its effect is small. An absurd playhead is
/// used for the same reason: 600 seconds is 65 times round this loop, so a
/// backdrop that read it could not accidentally agree.
/// **The intro is on the audio clock, and this is the only test that can
/// tell.** In every headless run the mixer is advanced exactly
/// `sample_rate / 60` frames a tick, so the audio playhead and the tick
/// clock are numerically identical and every capture measurement agrees
/// with both. Drop the playhead from `update_logo_fmv` and nothing else in
/// the suite notices.
///
/// So the clocks are made to disagree: `dt` is zero and the sound is five
/// seconds in. A tick-clocked player has not moved; an audio-clocked one is
/// on frame 149.
#[test]
fn the_movie_is_paced_by_its_sound_rather_than_by_the_tick() {
    // Both legs, because they are two different methods reaching the same
    // `advance_movie` and either could be the one that loses the argument.
    for (state, mut frontend) in [
        (states::LOGO_FMV, frontend(1200)),
        (states::INTRO_MOVIE, reel(1200)),
    ] {
        assert!(frontend.machine().is(state), "the leg under test");

        let mut input = Input::new();
        input.begin_frame(0);
        frontend.update(0.0, &mut input, None);
        assert_eq!(
            frontend.player().frame(),
            0,
            "{state}: no time and no sound is no movement"
        );

        input.begin_frame(0);
        frontend.update(0.0, &mut input, Some(5.0));
        assert_eq!(
            frontend.player().frame(),
            (5.0 * 30_000.0 / 1001.0) as usize,
            "{state}: the sound is what moved the picture, not the tick"
        );
    }
}

#[test]
fn the_backdrop_ignores_the_movies_audio_clock() {
    let run = |playhead: Option<f64>| {
        let mut frontend = frontend(300);
        frontend.set_backdrop(270, crate::frontend::FRAME_RATE, (480, 272));
        let mut input = Input::new();
        for tick in 0..400 {
            input.begin_frame(0);
            // Growing, the way a real playhead does, rather than one value
            // repeated - a backdrop reading a constant would look stuck
            // rather than wrong.
            frontend.update(FRAME, &mut input, playhead.map(|s| s * f64::from(tick)));
        }
        let player = frontend.backdrop().expect("a backdrop was set");
        (player.position(), player.frame())
    };

    let ticked = run(None);
    assert_eq!(ticked.0, 400, "the tick clock is the one it is on");
    assert_eq!(run(Some(1.5)), ticked, "an audio clock must change nothing");
    assert_eq!(run(Some(0.0)), ticked, "and neither must a stalled one");
}

#[test]
fn the_backdrops_draw_carries_a_position_that_outgrows_its_own_loop() {
    // What a `movie::Feed` is asked with. It decodes forward forever, so on
    // the second time round it holds positions 270..274 while `frame` has
    // gone back to 0 - and a draw carrying only `frame` takes nothing from
    // it from that moment on, which froze the picture nine seconds in.
    let mut frontend = frontend(300);
    frontend.set_backdrop(270, crate::frontend::FRAME_RATE, (480, 272));
    let mut input = Input::new();
    pick_a_language(&mut frontend, &mut input);
    assert!(frontend.machine().is(states::SHOW_LOGO));

    for _ in 0..300 {
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
    }

    let draws = frontend.draw_list();
    let Draw::Video {
        frame,
        position,
        source: Video::Backdrop,
        ..
    } = draws[1]
    else {
        panic!("Show Logo must draw the backdrop second: {draws:?}");
    };
    assert!(position >= 270, "the loop has been round once by here");
    assert_eq!(
        frame,
        (position % 270) as usize,
        "the frame is the position wrapped, and the draw carries both"
    );
    assert_eq!(
        position,
        frontend.backdrop().expect("a backdrop was set").position(),
        "and it is the playhead's own position, not a second count of it"
    );
}

/// Every screen between the intro and the menus draws the one loop.
///
/// `Launch Game` is the one that matters: the composition root draws it
/// **once and then stalls** building the menus, so it is on screen for the
/// whole of that load. Without the backdrop it was a few hundred
/// milliseconds of black between `Show Logo` and the menus - the flicker on
/// the START press, and the half of it no amount of feed bookkeeping could
/// have fixed, because a screen that emits no video draw has nothing to
/// take a frame for.
#[test]
fn the_screens_after_the_intro_all_sit_on_the_backdrop() {
    for state in [
        states::LANGUAGE_SELECTION,
        states::SHOW_LOGO,
        states::LAUNCH_GAME,
    ] {
        let mut frontend = frontend(300);
        frontend.set_backdrop(270, crate::frontend::FRAME_RATE, (480, 272));
        let mut input = Input::new();
        if state == states::LANGUAGE_SELECTION {
            // START skips the movie, and the redirect that follows it
            // spends one update on its way to the picker.
            input.begin_frame(Button::Start.bit());
            frontend.update(FRAME, &mut input, None);
            input.begin_frame(0);
            frontend.update(FRAME, &mut input, None);
        } else {
            pick_a_language(&mut frontend, &mut input);
            if state == states::LAUNCH_GAME {
                // `Show Logo` leaves on START and on nothing else.
                input.begin_frame(0);
                input.begin_frame(Button::Start.bit());
                frontend.update(FRAME, &mut input, None);
            }
        }
        assert!(
            frontend.machine().is(state),
            "meant to be on {state}, got {:?}",
            frontend.machine().current()
        );

        let draws = frontend.draw_list();
        assert!(
            matches!(draws[0], Draw::Fill { .. }),
            "{state} still clears to black first: {draws:?}"
        );
        assert!(
            matches!(
                draws[1],
                Draw::Video {
                    source: Video::Backdrop,
                    ..
                }
            ),
            "{state} must draw the backdrop under its own widgets: {draws:?}"
        );
    }
}

#[test]
fn the_backdrops_playhead_is_handed_on_rather_than_left_behind() {
    // `Show Logo` and the menus are two screens in front of one playback.
    // Whoever opens the menus takes this player as it stands; anything that
    // rebuilt it would put the picture back to the start of the loop.
    let mut frontend = frontend(300);
    frontend.set_backdrop(270, crate::frontend::FRAME_RATE, (480, 272));
    let mut input = Input::new();
    pick_a_language(&mut frontend, &mut input);
    for _ in 0..500 {
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
    }

    let running = frontend.backdrop().expect("a backdrop was set").position();
    assert!(running > 0, "the playhead must have got somewhere");

    let handed = frontend.take_backdrop().expect("a backdrop was set");
    assert_eq!(
        handed.position(),
        running,
        "the menus continue the playback rather than starting one"
    );
    assert!(!handed.is_finished(), "and it still loops");
    assert!(
        frontend.backdrop().is_none(),
        "the sequence is over; two playheads on one feed is the bug this fixes"
    );
    // And with it gone the sequence stops asking for a picture, rather than
    // drawing one nobody is advancing.
    let draws = frontend.draw_list();
    assert!(!draws.iter().any(|d| matches!(d, Draw::Video { .. })));
}

#[test]
fn no_backdrop_leaves_show_logo_on_black() {
    // A source without one, `--no-video`, and a backdrop whose planes are
    // not the intro's all land here, and none of them may emit a video draw
    // the renderer has no feed for.
    let mut frontend = frontend(300);
    let mut input = Input::new();
    pick_a_language(&mut frontend, &mut input);

    let draws = frontend.draw_list();
    assert!(!draws.iter().any(|d| matches!(d, Draw::Video { .. })));
    assert!(matches!(draws[0], Draw::Fill { .. }));
}
