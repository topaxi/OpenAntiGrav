use super::*;

const P1: u8 = 0b01;
const P2: u8 = 0b10;

const TWO: Table = Table {
    present: 0b11,
    countdown: true,
};
const ALONE: Table = Table {
    present: u8::MAX,
    countdown: false,
};

fn paused_by_two(since: f64) -> State {
    step(State::Running, P2, since, TWO)
}

#[test]
fn whoever_pauses_owns_the_pause() {
    assert_eq!(step(State::Running, P2, 5.0, TWO), State::pause_by(1, 5.0));
    assert_eq!(
        step(State::Running, P1 | P2, 5.0, TWO),
        State::pause_by(0, 5.0),
        "lowest slot on a tie"
    );
    assert_eq!(step(State::Running, 0, 5.0, TWO), State::Running);
}

#[test]
fn only_the_owner_resumes() {
    let paused = paused_by_two(0.0);
    assert_eq!(
        step(paused, P1, 1.0, TWO),
        paused,
        "player 1 cannot resume player 2's pause"
    );
    assert_eq!(step(paused, P2, 1.0, TWO), State::Resuming { since: 1.0 });
    assert_eq!(
        step(paused, P1 | P2, 1.0, TWO),
        State::Resuming { since: 1.0 },
        "the owner's press counts in a tie"
    );
}

#[test]
fn only_the_owner_leaves() {
    let paused = paused_by_two(0.0);
    assert!(!leaves(paused, P1));
    assert!(leaves(paused, P2));
    assert!(
        !leaves(State::Running, P1 | P2),
        "a running race is not left by Circle"
    );
}

#[test]
fn nobody_leaves_through_the_countdown() {
    let counting = State::Resuming { since: 1.0 };
    assert!(!leaves(counting, P1 | P2));
}

#[test]
fn an_unowned_pause_is_anyones() {
    let paused = State::unowned(0.0);
    assert_eq!(step(paused, P2, 1.0, TWO), State::Resuming { since: 1.0 });
    assert!(leaves(paused, P1));
    assert!(!leaves(paused, 0));
}

#[test]
fn one_player_resumes_at_once_as_before() {
    let paused = step(State::Running, P1, 0.0, ALONE);
    assert_eq!(paused, State::pause_by(0, 0.0));
    assert_eq!(
        step(paused, P1, 1.0, ALONE),
        State::Running,
        "no countdown for one"
    );
    assert!(!go(paused, State::Running), "and no GO cue for it");
}

#[test]
fn the_owner_claim_expires_on_the_boundary() {
    let paused = paused_by_two(10.0);
    // 29.999 s after the pause: still the owner's.
    let early = step(paused, P1, 10.0 + 29.999, TWO);
    assert_eq!(early, paused, "player 1 is still refused at 29.999 s");
    // Exactly 30 s: anyone's, and the same step lets player 1 resume.
    let at = step(paused, P1, 10.0 + 30.0, TWO);
    assert_eq!(at, State::Resuming { since: 40.0 });
}

#[test]
fn a_lapsed_claim_stays_lapsed_without_a_press() {
    let paused = paused_by_two(0.0);
    let lapsed = step(paused, 0, 31.0, TWO);
    assert_eq!(
        lapsed,
        State::Paused {
            owner: None,
            by: Some(1),
            since: 0.0
        },
        "the owner is gone but the line can still name who paused"
    );
    assert!(leaves(lapsed, P1), "and anyone may now leave");
    assert_eq!(step(lapsed, 0, 32.0, TWO), lapsed, "it does not come back");
}

#[test]
fn the_owner_resuming_late_still_counts() {
    let paused = paused_by_two(0.0);
    assert_eq!(
        step(paused, P2, 100.0, TWO),
        State::Resuming { since: 100.0 }
    );
}

#[test]
fn a_disconnected_owner_lifts_the_claim_at_once() {
    let paused = paused_by_two(0.0);
    let gone = Table { present: P1, ..TWO };
    let lifted = step(paused, 0, 1.0, gone);
    assert_eq!(
        lifted,
        State::Paused {
            owner: None,
            by: Some(1),
            since: 0.0
        }
    );
    assert_eq!(step(paused, P1, 1.0, gone), State::Resuming { since: 1.0 });
    // The same pause with the owner's pad still there keeps refusing player 1.
    assert_eq!(step(paused, P1, 1.0, TWO), paused);
}

#[test]
fn the_countdown_runs_out_into_the_race() {
    let counting = step(paused_by_two(0.0), P2, 1.0, TWO);
    assert_eq!(counting, State::Resuming { since: 1.0 });
    let almost = step(counting, 0, 1.0 + RESUME_SECONDS - 0.001, TWO);
    assert_eq!(almost, counting, "frozen until the last instant");
    assert!(!almost.ticks());
    let running = step(counting, 0, 1.0 + RESUME_SECONDS, TWO);
    assert_eq!(running, State::Running);
    assert!(go(counting, running));
    assert!(running.ticks());
}

#[test]
fn start_during_the_countdown_pauses_again_and_the_presser_owns_it() {
    let counting = State::Resuming { since: 1.0 };
    let again = step(counting, P1, 2.0, TWO);
    assert_eq!(
        again,
        State::pause_by(0, 2.0),
        "the clock restarts from the new pause"
    );
    assert!(!again.ticks());
    assert_eq!(
        step(again, P2, 3.0, TWO),
        again,
        "player 2 cannot lift player 1's"
    );
    // The 30 s runs from the new pause, not the old one.
    assert_eq!(step(again, P2, 31.9, TWO), again);
    assert_eq!(step(again, P2, 32.0, TWO), State::Resuming { since: 32.0 });
}

#[test]
fn only_a_running_state_ticks() {
    assert!(State::Running.ticks());
    assert!(!State::unowned(0.0).ticks());
    assert!(!State::Resuming { since: 0.0 }.ticks());
}

#[test]
fn the_line_counts_down_and_then_says_anyone() {
    let paused = paused_by_two(0.0);
    assert_eq!(
        line(paused, 5.0, TWO),
        Some(Line::Owned {
            player: 1,
            seconds_left: 25
        })
    );
    assert_eq!(
        line(paused, 29.2, TWO),
        Some(Line::Owned {
            player: 1,
            seconds_left: 1
        }),
        "whole seconds, rounded up, so it never reads 0:00 while still owned"
    );
    assert_eq!(
        line(paused, 30.0, TWO),
        Some(Line::Open { player: Some(1) })
    );
    assert_eq!(
        line(State::unowned(0.0), 1.0, TWO),
        Some(Line::Open { player: None })
    );
    assert_eq!(
        line(State::Resuming { since: 0.0 }, 1.0, TWO),
        Some(Line::Resuming)
    );
    assert_eq!(line(State::Running, 1.0, TWO), None);
    assert_eq!(clock(25), "0:25");
    assert_eq!(clock(60), "1:00");
}

#[test]
fn the_line_agrees_with_the_step_about_a_gone_owner() {
    let paused = paused_by_two(0.0);
    let gone = Table { present: P1, ..TWO };
    assert_eq!(
        line(paused, 1.0, gone),
        Some(Line::Open { player: Some(1) })
    );
}

#[test]
fn the_art_clock_puts_go_at_the_end_of_the_countdown() {
    let counting = State::Resuming { since: 10.0 };
    assert_eq!(art_seconds(counting, None, 10.0), Some(ART_CLOCK_START));
    let last = art_seconds(counting, None, 10.0 + RESUME_SECONDS).unwrap();
    assert!(
        (last - ART_GO_SECONDS).abs() < 1e-5,
        "GO is lit as the race moves"
    );
    assert_eq!(art_seconds(State::unowned(0.0), None, 5.0), None);
}

#[test]
fn go_lingers_then_goes() {
    assert!(art_seconds(State::Running, Some(10.0), 10.5).is_some());
    assert_eq!(
        art_seconds(State::Running, Some(10.0), 10.0 + GO_LINGER),
        None
    );
    assert_eq!(art_seconds(State::Running, None, 10.5), None);
}

#[test]
fn the_seats_present_follow_the_assignment() {
    use oag_input::pad::Assignment;
    let merged: Assignment<u32> = Assignment::default();
    assert_eq!(present_seats(&merged), u8::MAX);
    assert!(!several_people(&merged));
    let mut seated: Assignment<u32> = Assignment::seated(2);
    assert_eq!(
        present_seats(&seated),
        P1,
        "the keyboard drives slot 0 and nobody has slot 1"
    );
    assert!(several_people(&seated));
    seated.reconcile(&[7, 8]);
    assert_eq!(present_seats(&seated), P1 | P2, "two pads fill both seats");
    seated.reconcile(&[7]);
    assert_eq!(
        present_seats(&seated),
        P1,
        "unplugging the second empties its seat"
    );
}
