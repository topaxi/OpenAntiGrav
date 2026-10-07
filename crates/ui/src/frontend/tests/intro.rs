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
    for skip in [Button::Start, Button::Cross] {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        input.begin_frame(1 << skip.index());
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

    input.begin_frame(Button::Start.bit());
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

    input.begin_frame(Button::Start.bit());
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
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    assert!(frontend.machine().is(states::LANGUAGE_SELECTION));

    assert_eq!(frontend.selected(), 0);
    input.begin_frame(Button::Down.bit());
    frontend.update(FRAME, &mut input, None);
    assert_eq!(frontend.selected(), 1);

    input.begin_frame(0);
    input.begin_frame(Button::Up.bit());
    frontend.update(FRAME, &mut input, None);
    assert_eq!(frontend.selected(), 0);

    input.begin_frame(0);
    input.begin_frame(Button::Up.bit());
    frontend.update(FRAME, &mut input, None);
    assert_eq!(
        frontend.selected(),
        2,
        "up from the first wraps to the last"
    );
}

/// Running out of boot chain opens the menus instead of stopping dead.
///
/// **This is the whole of what let Wipeout HD past its logo.** Six of its eight
/// declared steps are dialogs nothing here drives, so the walked chain is the
/// picker and `Studio Logo`; when that movie ended, `advance` found no next step
/// and returned silently. Four hundred ticks of START and CROSS left the state
/// on `Studio Logo`, which is indistinguishable from a hang.
///
/// The destination is [`states::LAUNCH_GAME`] because that is where this
/// build's own menu tree opens - the same deliberate divergence
/// `update_show_logo` and `update_title_screen` make, and documented as such on
/// `Frontend::advance`.
#[test]
fn a_boot_that_runs_out_of_chain_opens_the_menus() {
    let mut frontend = hd(20, false);
    let mut input = Input::new();

    // Pick a language, which is the only step ahead of the reel.
    input.begin_frame(Button::Cross.bit());
    frontend.update(FRAME, &mut input, None);
    assert!(
        run_until(&mut frontend, &mut input, 200, |f| f
            .machine()
            .is(hd_states::STUDIO_LOGO)),
        "the picker hands on to the logo reel"
    );

    // And then nothing, on a disc whose next six screens this build skips.
    assert!(
        run_until(&mut frontend, &mut input, 600, |f| f
            .machine()
            .is(states::LAUNCH_GAME)),
        "the reel running out has to reach the menus, not stall on the reel"
    );
    assert!(frontend.is_finished());
}

/// And it says so, naming the screen it ran out on.
///
/// A silent hand-off would be the same failure in the other direction: the
/// order Wipeout HD boots in is *declared* rather than measured (ADR-0025), so a
/// transition this build invents has to be visible in the report next to the
/// ones the disc states.
#[test]
fn the_chain_running_out_is_reported_rather_than_silent() {
    let mut frontend = hd(20, false);
    let mut input = Input::new();
    input.begin_frame(Button::Cross.bit());
    frontend.update(FRAME, &mut input, None);
    run_until(&mut frontend, &mut input, 600, |f| {
        f.machine().is(states::LAUNCH_GAME)
    });

    let notes = frontend.take_notes();
    assert!(
        notes.iter().any(|note| note.contains("boot chain ends")
            && note.contains(hd_states::STUDIO_LOGO)
            && note.contains(states::LAUNCH_GAME)),
        "the hand-off has to name where it ran out and where it went: {notes:#?}"
    );
}

/// A chain with something left ahead of it is untouched.
///
/// The guard on the change above: both PSP titles leave their last screen by
/// their own handler on a START press rather than by advancing past it, so
/// neither must ever reach the new arm. Pure's boot is five steps and this walks
/// the first four - if running out fired early, the picker alone would land on
/// the menus.
#[test]
fn a_chain_with_steps_left_advances_through_them_as_before() {
    let mut frontend = pure(30);
    let mut input = Input::new();
    reach_the_second_movie(&mut frontend, &mut input);
    assert!(frontend.machine().is(pure_states::FMV_INTRO));
    assert!(
        !frontend.machine().is(states::LAUNCH_GAME),
        "four steps in, a five-step chain has not run out"
    );

    // And the fifth is `Title Screen`, which is left by its own handler.
    assert!(
        run_until(&mut frontend, &mut input, 600, |f| f
            .machine()
            .is(pure_states::TITLE_SCREEN)),
        "the movie ending advances to the last step rather than past it"
    );
    assert!(!frontend.is_finished(), "PRESS START has not been pressed");
}

/// A decoded `Studio Logo` reel does not carry the frame counter over it.
///
/// **The bug this guards**: HD's chain opens on `Language Selection`, which
/// plays no movie at all, so `Frontend::booting`'s old `overlay: !first.has_picture`
/// read *that* screen's placeholder rather than the reel's own flag - forcing
/// the counter on for the whole boot even when `Studio Logo`'s own `.bik`
/// decoded and drew a picture. Pulse and Pure never hit this because their
/// first step already is the movie leg.
#[test]
fn the_overlay_does_not_default_on_when_the_reel_has_a_picture() {
    let mut frontend = hd(20, true);
    let mut input = Input::new();
    input.begin_frame(Button::Cross.bit());
    frontend.update(FRAME, &mut input, None);
    assert!(
        run_until(&mut frontend, &mut input, 200, |f| f
            .machine()
            .is(hd_states::STUDIO_LOGO)),
        "the picker hands on to the logo reel"
    );

    let draws = frontend.draw_list();
    assert!(
        !draws
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if text.contains("INTRO FRAME"))),
        "a picture was decoded, so nothing should stand in for it: {draws:#?}"
    );
}

/// A reel with no picture is left the way a Start press leaves it.
///
/// It would otherwise be a black screen for as long as the movie runs, forty
/// seconds for Pulse's intro, on a phone or any machine without `ffmpeg`.
#[test]
fn a_reel_with_no_picture_is_skipped_straight_ahead() {
    let mut frontend = hd(20, false);
    let mut input = Input::new();
    input.begin_frame(Button::Cross.bit());
    frontend.update(FRAME, &mut input, None);
    assert!(
        run_until(&mut frontend, &mut input, 200, |f| f
            .machine()
            .is(hd_states::STUDIO_LOGO)),
        "the picker hands on to the logo reel"
    );
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    assert!(
        !frontend.machine().is(hd_states::STUDIO_LOGO),
        "the reel has no picture, so it leaves on the next tick with no button"
    );
    assert!(
        !frontend
            .draw_list()
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if text.contains("INTRO FRAME"))),
        "the frame counter is off unless --overlay asks"
    );
}

/// `skip_never_shown_picker` on a fresh HD boot: no settings language yet,
/// and the picker must still never wait on a player - see
/// `docs/formats/hd-frontend.md#is-the-language-picker-ever-shown` for the
/// four RPCS3 boots this defaults on.
#[test]
fn skip_never_shown_picker_defaults_hd_to_english_with_no_input() {
    let mut frontend = hd(20, true);
    assert!(frontend.machine().is(states::LANGUAGE_SELECTION));

    assert!(frontend.skip_never_shown_picker(oag_hd::TITLE));

    let mut input = Input::new();
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    assert!(
        frontend.machine().is(hd_states::STUDIO_LOGO),
        "the redirect must fire on its own frame, with no button ever pressed"
    );
    assert_eq!(frontend.chosen(), Some("English"));
}

/// The escape hatch is HD's alone: Pure's own picker (ADR-0023's own reason
/// to exist) must keep waiting on a player exactly as before.
#[test]
fn skip_never_shown_picker_is_a_no_op_off_hd() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    assert!(frontend.machine().is(states::LANGUAGE_SELECTION));

    assert!(!frontend.skip_never_shown_picker(oag_pure::TITLE));

    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    assert!(
        frontend.machine().is(states::LANGUAGE_SELECTION),
        "no title but HD gets the default - Pure's picker still waits"
    );
}
