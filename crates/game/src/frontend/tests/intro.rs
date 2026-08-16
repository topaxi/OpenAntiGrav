//! The boot sequence: the screen the disc boots into, the logo FMV leg and
//! its holds, the skip redirect, and the language picker.
//!
//! Split out of `frontend/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

#[test]
fn boots_into_the_screen_the_disc_boots_into() {
    let frontend = frontend(300);
    assert_eq!(frontend.machine().current(), Some(states::LOGO_FMV));
    assert!(
        !frontend.machine().is_in(states::INTRO),
        "the dev/pub reel state is not on the boot path"
    );
}

#[test]
fn the_logo_fmv_leg_plays_straight_through_with_no_holds() {
    let mut frontend = frontend(300);
    let mut input = Input::new();

    // Past both of the reel state's pause frames, and still playing: those
    // counters belong to the other leg.
    for _ in 0..250 {
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
    }
    assert!(!frontend.player().is_paused(), "LogoFMV has no frame holds");
    assert_eq!(frontend.player().frames_produced(), 251);
    assert!(frontend.machine().is(states::LOGO_FMV));

    assert!(
        run_until(&mut frontend, &mut input, 400, |f| f
            .machine()
            .is(states::LANGUAGE_SELECTION)),
        "the movie ending must go on to the picker"
    );
    assert_eq!(
        frontend.machine().history(),
        [states::LOGO_FMV, states::LANGUAGE_SELECTION],
        "and not through DevPubRedirect, which is the other leg's"
    );
}

#[test]
fn either_skip_button_leaves_the_logo_fmv_leg() {
    for skip in [button::START, button::CROSS] {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        input.begin_frame(1 << skip);
        frontend.update(FRAME, &mut input, None);
        // A skip goes through the redirect screen the XML sends it to, the
        // way the reel leg goes through `DevPubRedirect`. The movie *ending*
        // does not: its `AutoRedirect` is a different exit.
        assert_eq!(
            frontend.machine().current(),
            Some(states::LOGO_FMV_REDIRECT),
            "button {skip} must skip the movie through the redirect screen"
        );

        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
        assert!(frontend.machine().is(states::LANGUAGE_SELECTION));
        assert_eq!(
            frontend.machine().history(),
            [
                states::LOGO_FMV,
                states::LOGO_FMV_REDIRECT,
                states::LANGUAGE_SELECTION,
            ]
        );
    }
}

#[test]
fn pauses_at_144_then_231_then_finishes_at_260() {
    let mut frontend = reel(300);
    let mut input = Input::new();

    // Frame 144 is reached after 143 steps from frame 1.
    for _ in 0..143 {
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
    }
    assert_eq!(frontend.player().frames_produced(), 144);
    assert!(frontend.player().is_paused(), "must pause at 144");

    // Two seconds is 59.94 frames, so 59 is not yet up and 61 is.
    for _ in 0..59 {
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
    }
    assert!(frontend.player().is_paused(), "two seconds is not up yet");
    assert_eq!(
        frontend.player().frames_produced(),
        144,
        "a hold must not advance the movie"
    );
    for _ in 0..2 {
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
    }
    assert!(!frontend.player().is_paused(), "the hold must release");

    assert!(
        run_until(&mut frontend, &mut input, 2000, |f| f
            .machine()
            .is(states::LANGUAGE_SELECTION)),
        "the intro must end at the language picker"
    );

    // Both pauses and the finish must have happened, in order.
    let history = frontend.machine().history();
    assert_eq!(
        history,
        [
            states::INTRO,
            states::INTRO_MOVIE,
            states::DEV_PUB_REDIRECT,
            states::LANGUAGE_SELECTION,
        ]
    );
}

#[test]
fn start_skips_the_intro_by_firing_the_redirect() {
    let mut frontend = reel(300);
    let mut input = Input::new();

    input.begin_frame(1 << button::START);
    frontend.update(FRAME, &mut input, None);
    assert_eq!(
        frontend.machine().current(),
        Some(states::DEV_PUB_REDIRECT),
        "skipping goes through the redirect, not straight to the picker"
    );
    // The player is not stopped: teardown is a consequence of the transition.
    assert!(!frontend.player().is_finished());

    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    assert!(frontend.machine().is(states::LANGUAGE_SELECTION));
}

#[test]
fn the_redirect_target_is_cleared_so_it_cannot_fire_twice() {
    let mut frontend = reel(300);
    let mut input = Input::new();

    input.begin_frame(1 << button::START);
    frontend.update(FRAME, &mut input, None);
    assert!(
        run_until(&mut frontend, &mut input, 2000, |f| f
            .machine()
            .is(states::LANGUAGE_SELECTION)),
        "must arrive at the picker"
    );

    // The intro's own frame counters would fire the redirect again if the
    // target were not cleared, which would bounce out of the picker.
    let entries = frontend
        .machine()
        .history()
        .iter()
        .filter(|name| *name == states::DEV_PUB_REDIRECT)
        .count();
    assert_eq!(entries, 1, "{:?}", frontend.machine().history());
}

#[test]
fn a_reel_shorter_than_260_frames_still_ends() {
    let mut frontend = reel(30);
    let mut input = Input::new();
    assert!(
        run_until(&mut frontend, &mut input, 500, |f| f
            .machine()
            .is(states::LANGUAGE_SELECTION)),
        "a short movie must not stall the boot"
    );
}

#[test]
fn no_picture_still_plays_the_sequence() {
    let mut frontend = Frontend::new(
        Screens::from_xml(XML),
        StringTable::default(),
        languages(),
        Vec::new(),
        0,
        false,
    );
    let mut input = Input::new();
    assert!(run_until(&mut frontend, &mut input, 2000, |f| f
        .machine()
        .is(states::LANGUAGE_SELECTION)));
}

#[test]
fn the_picker_moves_and_wraps_both_ways() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    input.begin_frame(1 << button::START);
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    assert!(frontend.machine().is(states::LANGUAGE_SELECTION));

    assert_eq!(frontend.selected(), 0);
    input.begin_frame(1 << button::DOWN);
    frontend.update(FRAME, &mut input, None);
    assert_eq!(frontend.selected(), 1);

    input.begin_frame(0);
    input.begin_frame(1 << button::UP);
    frontend.update(FRAME, &mut input, None);
    assert_eq!(frontend.selected(), 0);

    input.begin_frame(0);
    input.begin_frame(1 << button::UP);
    frontend.update(FRAME, &mut input, None);
    assert_eq!(
        frontend.selected(),
        2,
        "up from the first wraps to the last"
    );
}
