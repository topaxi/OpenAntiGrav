//! The screens after the picker: the five Pure walks, launching the game,
//! the storage warning, and what `Show Logo` waits for.
//!
//! Split out of `frontend/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

/// Pure's second movie is a movie leg, so the sound has to survive it.
///
/// This omission was a real defect rather than missing coverage: `main.rs`
/// stops the movie's sound on every tick `is_playing_movie` is false, so the
/// second boot movie was torn down on the first tick of its own state and
/// then paced by `dt` instead of by its own audio.
#[test]
fn the_second_boot_movie_counts_as_a_movie_leg() {
    let mut frontend = pure(60);
    let mut input = Input::new();
    // From wherever the boot leg starts to the picker. Which state that is
    // is Pure's own open question and not this test's business - see
    // `pure-boot.md`; what matters here is the movie leg.
    assert!(
        !frontend.is_playing_movie(),
        "the picker is not a movie screen"
    );
    reach_the_second_movie(&mut frontend, &mut input);
    assert!(
        frontend.machine().is(pure_states::FMV_INTRO),
        "the storage warning's exit is the second boot movie; got {:?}",
        frontend.machine().current()
    );
    assert!(
        frontend.is_playing_movie(),
        "FMV Intro is a movie screen; saying otherwise stops its sound on the \
         first tick of the leg"
    );

    run_until(&mut frontend, &mut input, 600, |f| {
        f.machine().is(pure_states::TITLE_SCREEN)
    });
    assert!(
        !frontend.is_playing_movie(),
        "the movie is over by Title Screen, so its sound has to stop"
    );
}

/// The order is the disc's, screen for screen.
///
/// The regression guard for the defect this chain exists to fix: descoping the
/// two screens between the picker and the movie left the *order* wrong, not
/// just their content, because the mechanism could only resolve one step after
/// the picker however much was implemented.
#[test]
fn pure_walks_the_five_screens_the_disc_walks() {
    let mut frontend = pure(60);
    let mut input = Input::new();
    assert!(frontend.machine().is(pure_states::LANGUAGE_SELECTION));

    input.begin_frame(Button::Cross.bit());
    frontend.update(FRAME, &mut input, None);
    assert!(
        frontend.machine().is(pure_states::DEVELOPER_PUBLISHER),
        "the picker goes to the developer and publisher cards, not the movie; got {:?}",
        frontend.machine().current()
    );

    // Timed, with no button: pressing through the cards was not observed to
    // work, and the disc's own redirect names no button.
    for _ in 0..600 {
        if frontend.machine().is(pure_states::MEMORY_STICK_WARNING) {
            break;
        }
        input.begin_frame(u32::MAX);
        frontend.update(FRAME, &mut input, None);
    }
    assert!(
        frontend.machine().is(pure_states::MEMORY_STICK_WARNING),
        "the cards advance themselves after about {DEVELOPER_PUBLISHER_SECONDS} s; got {:?}",
        frontend.machine().current()
    );

    // And this one waits, however long it is left.
    for _ in 0..600 {
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
    }
    assert!(
        frontend.machine().is(pure_states::MEMORY_STICK_WARNING),
        "the storage warning has no timeout - it holds for cross"
    );
    input.begin_frame(Button::Cross.bit());
    frontend.update(FRAME, &mut input, None);
    assert!(frontend.machine().is(pure_states::FMV_INTRO));

    run_until(&mut frontend, &mut input, 600, |f| {
        f.machine().is(pure_states::TITLE_SCREEN)
    });
    assert_eq!(
        frontend.machine().history(),
        [
            pure_states::LANGUAGE_SELECTION,
            pure_states::DEVELOPER_PUBLISHER,
            pure_states::MEMORY_STICK_WARNING,
            pure_states::FMV_INTRO,
            pure_states::TITLE_SCREEN,
        ],
        "the disc's own chain, cold-boot confirmed on both pressings"
    );
}

/// START on `Title Screen` hands off to the menus, and nothing else does.
///
/// The counterpart to [`start_on_show_logo_launches_the_game`], and pinned
/// for the same reason: the button is measured (the screen's own
/// `idstring="PRESS START"` widget, plus a PPSSPP session) and the
/// destination is this build's divergence, so a change to either should
/// have to come here and say so. See [`Frontend::update_title_screen`].
#[test]
fn start_on_pures_title_screen_launches_the_game() {
    let mut frontend = pure(60);
    let mut input = Input::new();
    reach_the_second_movie(&mut frontend, &mut input);
    run_until(&mut frontend, &mut input, 600, |f| {
        f.machine().is(pure_states::TITLE_SCREEN)
    });

    // Every button the abstract layer carries except START, held for ten
    // seconds. The screen has no timeout of its own either - its unnamed
    // redirect is left unfired on purpose - so this must not move.
    for _ in 0..600 {
        input.begin_frame(!Button::Start.bit());
        frontend.update(FRAME, &mut input, None);
    }
    assert!(
        frontend.machine().is(pure_states::TITLE_SCREEN),
        "only START leaves Title Screen, and it does not time out; got {:?}",
        frontend.machine().current()
    );
    assert!(!frontend.is_finished());

    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    assert!(
        frontend.machine().is(states::LAUNCH_GAME),
        "START hands off to the menus, the way Pulse's own last boot screen \
         does; got {:?}",
        frontend.machine().current()
    );
    assert!(
        frontend.is_finished(),
        "and the composition root is told, or the menus never open"
    );
}

/// The storage warning says what is true here, not what the disc said.
#[test]
fn the_storage_warning_is_worded_for_a_machine_with_storage() {
    let mut frontend = pure(60);
    let mut input = Input::new();
    reach_the_second_movie(&mut frontend, &mut input);
    // Back up: walk a fresh one only as far as the warning.
    let mut frontend = pure(60);
    let mut input = Input::new();
    input.begin_frame(Button::Cross.bit());
    frontend.update(FRAME, &mut input, None);
    run_until(&mut frontend, &mut input, 1200, |f| {
        f.machine().is(pure_states::MEMORY_STICK_WARNING)
    });

    let text: String = frontend
        .draw_list()
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text.contains("AUTOMATICALLY") && text.contains("PRESS X TO CONTINUE"),
        "the screen keeps its gate and states the autosave behaviour; got {text:?}"
    );
    assert!(
        !text.to_uppercase().contains("MEMORY STICK"),
        "the disc's removable-card wording is deliberately not reproduced; got {text:?}"
    );
}

/// One renderer, one set of planes, one video draw - so one video per list.
///
/// `sync_video` takes the *first* video in a list and the renderer takes the
/// *last*, so a list carrying two would upload one movie and draw it at the
/// other's rect. Unreachable on today's discs, since no Pure pressing ships
/// `Data\Movies\Backdrop.PMF`, which is exactly why it is pinned here rather
/// than left to a screenshot to catch.
#[test]
fn no_screen_ever_draws_two_videos_at_once() {
    for (what, mut frontend) in [("pulse", frontend(300)), ("pure", pure(60))] {
        frontend.set_backdrop(270, crate::frontend::FRAME_RATE, (480, 272));
        let mut input = Input::new();
        let mut seen = 0;
        // Every state the leg walks, the backdrop present throughout.
        for _ in 0..1200 {
            let videos = frontend
                .draw_list()
                .iter()
                .filter(|draw| matches!(draw, Draw::Video { .. }))
                .count();
            assert!(
                videos <= 1,
                "{what}: state {:?} drew {videos} videos; the renderer draws one",
                frontend.machine().current()
            );
            seen += 1;
            input.begin_frame(if seen % 120 == 0 {
                Button::Cross.bit()
            } else {
                0
            });
            frontend.update(FRAME, &mut input, None);
        }
    }
}

#[test]
fn picking_a_language_shows_the_logo_rather_than_launching() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    pick_a_language(&mut frontend, &mut input);

    assert_eq!(frontend.chosen(), Some("German"));
    assert!(
        frontend.machine().is(states::SHOW_LOGO),
        "the picker's own exit is PRESS START, not the menus"
    );
    assert!(
        !frontend.is_finished(),
        "the front end is not done until Show Logo is pressed through"
    );
}

#[test]
fn start_on_show_logo_launches_the_game() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    pick_a_language(&mut frontend, &mut input);

    input.begin_frame(0);
    input.begin_frame(Button::Start.bit());
    let events = frontend.update(FRAME, &mut input, None);

    assert!(frontend.is_finished());
    assert!(events.contains(&Event::Enter(states::LAUNCH_GAME.into())));
    assert_eq!(
        frontend.machine().history(),
        [
            states::LOGO_FMV,
            states::LOGO_FMV_REDIRECT,
            states::LANGUAGE_SELECTION,
            states::SHOW_LOGO,
            states::LAUNCH_GAME,
        ]
    );
}

#[test]
fn show_logo_waits_and_ignores_every_button_but_start() {
    // The screen's XML has one `forward="start"` redirect and no timer, so
    // neither time passing nor the buttons `LogoFMV` itself accepts may move
    // it on. Cross is the one that matters: it skips the movie, and it is
    // also the button that was just pressed to pick a language.
    for held in [Button::Cross, Button::Circle, Button::Down] {
        let mut frontend = frontend(300);
        let mut input = Input::new();
        pick_a_language(&mut frontend, &mut input);

        for _ in 0..600 {
            input.begin_frame(0);
            frontend.update(FRAME, &mut input, None);
            input.begin_frame(1 << held.index());
            frontend.update(FRAME, &mut input, None);
        }
        assert!(
            frontend.machine().is(states::SHOW_LOGO),
            "button {held} must not leave Show Logo"
        );
        assert!(!frontend.is_finished());
    }
}

#[test]
fn show_logo_draws_the_backdrop_under_its_own_widgets() {
    let mut frontend = frontend(300);
    // 270 frames at the PSP's own rate, which is what `Backdrop.PMF` is.
    frontend.set_backdrop(270, crate::frontend::FRAME_RATE, (480, 272));
    let mut input = Input::new();
    pick_a_language(&mut frontend, &mut input);
    assert!(frontend.machine().is(states::SHOW_LOGO));

    let draws = frontend.draw_list();
    // Under the widgets and over the clear, because the list is painted in
    // its own order: black fill, backdrop, then whatever the screen has.
    assert!(matches!(draws[0], Draw::Fill { .. }));
    let Draw::Video { rect, source, .. } = draws[1] else {
        panic!("Show Logo must draw the backdrop second: {draws:?}");
    };
    assert_eq!(
        source,
        Video::Backdrop,
        "the intro is over by here; naming the wrong movie draws a picture rather than failing"
    );
    assert_eq!(
        rect,
        [0.0, 0.0, SCREEN.0, SCREEN.1],
        "a .PMF fills the screen"
    );
    assert!(
        draws[2..].iter().any(|d| matches!(d, Draw::Text { .. })),
        "and PRESS START on top of it: {draws:?}"
    );
}

#[test]
fn the_backdrop_is_already_running_by_the_time_show_logo_is_reached() {
    // It is `autostart` on a screen the boot opens long before this build
    // draws it, so it must not be sitting at frame zero when it appears.
    let mut frontend = frontend(300);
    frontend.set_backdrop(270, crate::frontend::FRAME_RATE, (480, 272));
    let mut input = Input::new();

    for _ in 0..200 {
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
    }
    let running = frontend.backdrop().expect("a backdrop was set").frame();
    assert!(running > 0, "the playhead must advance during LogoFMV");

    // And it loops rather than ending, so it is still there after its own
    // length has gone by twice over.
    for _ in 0..600 {
        input.begin_frame(0);
        frontend.update(FRAME, &mut input, None);
    }
    let player = frontend.backdrop().expect("a backdrop was set");
    assert!(!player.is_finished(), "repeat=true has no end to run off");
    assert!(player.frame() < 270, "and it wraps inside its own length");
}
