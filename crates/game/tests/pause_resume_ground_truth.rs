//! A resume countdown holds a real race still: no tick runs, so the state hash
//! does not move until `GO`, and it moves after.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship; run it with `just test-data`.
//!
//! The pause rule is `oag_game::pause`, the same functions `Session::frame` steps
//! once a tick: this drives a loaded Pulse race through them at 60 Hz of wall clock
//! and calls `Race::tick` exactly when `State::ticks` says so, as the session's
//! frozen arm and tick arm do. The countdown is chosen, not measured (the original
//! has no resume countdown), so what is pinned is ours: the length
//! `RESUME_SECONDS`, and that nothing steps through it.

use oag_game::pause::{RESUME_SECONDS, State, Table, go, step};
use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;
use oag_raceplay as race;

const P1: u8 = 0b01;
const FRAME: f64 = 1.0 / 60.0;
const TABLE: Table = Table {
    present: 0b11,
    countdown: true,
};

fn loaded(image: &str) -> Option<race::Race> {
    let image = oag_testdata::image(image)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// One frame of the session's loop: the pause rule, then a tick if it allows one.
fn frame(race: &mut race::Race, state: &mut State, presses: u8, now: f64) -> bool {
    let before = *state;
    *state = step(before, presses, now, TABLE);
    if state.ticks() {
        race.tick(&PlayerInputs::none());
    }
    go(before, *state)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_countdown_holds_the_state_hash_and_go_releases_it() {
    let Some(mut race) = loaded("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut state = State::Running;
    let mut now = 0.0;
    // Past the start's own countdown, so a resume counts down.
    for _ in 0..300 {
        frame(&mut race, &mut state, 0, now);
        now += FRAME;
    }
    assert!(race.sim.world.tick > 272, "the race is under way");

    frame(&mut race, &mut state, P1, now);
    now += FRAME;
    assert!(!state.ticks(), "Start pauses");
    let frozen = race.sim.state_hash();
    let tick = race.sim.world.tick;
    for _ in 0..30 {
        frame(&mut race, &mut state, 0, now);
        now += FRAME;
    }
    assert_eq!(race.sim.state_hash(), frozen, "paused: the hash holds");

    frame(&mut race, &mut state, P1, now);
    assert!(matches!(state, State::Resuming { .. }), "{state:?}");
    let begun = now;
    let mut frames = 0;
    let mut went = false;
    while !went {
        now += FRAME;
        went = frame(&mut race, &mut state, 0, now);
        frames += 1;
        if !went {
            assert_eq!(
                race.sim.state_hash(),
                frozen,
                "frame {frames} of the countdown stepped the race"
            );
            assert_eq!(race.sim.world.tick, tick);
        }
        assert!(frames < 400, "the countdown never ended");
    }
    let seconds = now - begun;
    assert!(
        (seconds - RESUME_SECONDS).abs() <= FRAME,
        "GO after {seconds} s, not {RESUME_SECONDS} s"
    );

    // From GO the race moves again.
    for _ in 0..10 {
        now += FRAME;
        frame(&mut race, &mut state, 0, now);
    }
    assert_ne!(race.sim.state_hash(), frozen, "after GO the race runs");
    assert!(race.sim.world.tick > tick);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn start_during_the_countdown_pauses_again_without_a_tick() {
    let Some(mut race) = loaded("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut state = State::Running;
    let mut now = 0.0;
    for _ in 0..300 {
        frame(&mut race, &mut state, 0, now);
        now += FRAME;
    }
    frame(&mut race, &mut state, P1, now);
    let frozen = race.sim.state_hash();
    now += FRAME;
    frame(&mut race, &mut state, P1, now);
    assert!(matches!(state, State::Resuming { .. }));
    for _ in 0..60 {
        now += FRAME;
        frame(&mut race, &mut state, 0, now);
    }
    now += FRAME;
    frame(&mut race, &mut state, P1, now);
    assert!(matches!(state, State::Paused { .. }), "{state:?}");
    for _ in 0..240 {
        now += FRAME;
        frame(&mut race, &mut state, 0, now);
    }
    assert_eq!(race.sim.state_hash(), frozen, "no tick through any of it");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_start_voice_cues_are_raised_where_the_title_has_them() {
    let Some(mut pulse) = loaded("data/images/pulse-psp-usa.chd") else {
        return;
    };
    pulse.raise_start_cue(Cue::Ready);
    pulse.raise_start_cue(Cue::Go);
    let raised: Vec<Cue> = pulse.pending_cues().iter().map(|event| event.cue).collect();
    assert_eq!(raised, vec![Cue::Ready, Cue::Go]);
    let hash = pulse.sim.state_hash();
    pulse.raise_start_cue(Cue::Go);
    assert_eq!(
        pulse.sim.state_hash(),
        hash,
        "a cue is an output, not state"
    );
    // The session raises GO at the top of the tick that lets the race move; that
    // tick must not drop it before the sound step drains it.
    let queued = pulse.pending_cues().len();
    pulse.tick(&PlayerInputs::none());
    assert!(
        pulse.pending_cues().len() >= queued,
        "a tick dropped a raised cue: {:?}",
        pulse.pending_cues()
    );
    assert_eq!(
        pulse.pending_cues()[..queued]
            .iter()
            .map(|event| event.cue)
            .collect::<Vec<_>>(),
        vec![Cue::Ready, Cue::Go, Cue::Go],
        "the raised cues are still there, in order"
    );

    let Some(mut pure) = loaded("data/images/pure-psp-usa.chd") else {
        return;
    };
    pure.raise_start_cue(Cue::Go);
    assert!(
        pure.pending_cues().is_empty(),
        "Pure voices no start, so a resume voices none"
    );
}
