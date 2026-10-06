//! Hardware-free tests for [`super::Runner`]. The corpus half (`~BLOWUP` and
//! `~ROCKLOCK` from Pulse's `hud.bnk`) is
//! `crates/game/tests/sfx_repeating_ground_truth.rs`.

use crate::sblk::Bank;
use crate::sblk::timeline::tests::{Spec, build, key, raw};

fn no_rand() -> u32 {
    0
}

/// Key-on ticks (of waveform `offset`) over `ticks` master ticks, the runner
/// started once with `parameter` written straight after.
fn keyed_at(data: &[u8], offset: u32, ticks: u32, parameter: i8) -> Vec<u32> {
    let bank = Bank::parse(data).expect("parse");
    let mut runner = bank
        .cue_runner(&bank.cue_named("A").expect("cue"))
        .expect("runs");
    let mut got = Vec::new();
    let mut collect = |grains: Vec<crate::sblk::timeline::Grain>| {
        got.extend(
            grains
                .into_iter()
                .filter(|g| g.sound.offset == offset)
                .map(|g| g.tick),
        );
    };
    collect(runner.start(&mut no_rand));
    runner.set_parameter(0, parameter);
    for _ in 0..ticks {
        collect(runner.tick(&mut no_rand));
    }
    got
}

/// `~ROCKLOCK`'s list: a guard on parameter 0 in front of each of two key-ons.
fn lock() -> Vec<u8> {
    build(&[Spec(
        "A",
        100,
        vec![
            raw(0x15, 0, 0),
            raw(0x22, 0x0000_0100, 0),
            key(0, 0, 30),
            raw(0x22, 0x0001_0100, 0),
            key(16, 0, 15),
            raw(0x16, 0, 0),
        ],
    )])
}

#[test]
fn a_guard_picks_which_key_on_the_loop_plays_and_so_the_tempo() {
    let data = lock();
    // Parameter 0 skips the second key-on, taking its own delay with it: one
    // beep every 30 ticks.
    assert_eq!(keyed_at(&data, 0, 130, 0), vec![30, 60, 90, 120]);
    assert!(keyed_at(&data, 16, 130, 0).is_empty());
}

#[test]
fn a_parameter_written_mid_play_takes_hold_at_the_next_pass_of_the_loop() {
    let data = lock();
    // Written right after the start, when the first pass has tested its guards
    // against zero: it still keys the first waveform at 30, then every pass
    // skips it for the second, every 15.
    assert_eq!(keyed_at(&data, 0, 130, 1), vec![30]);
    assert_eq!(keyed_at(&data, 16, 130, 1), vec![45, 60, 75, 90, 105, 120]);
}

#[test]
fn a_loop_that_waits_repeats_on_its_delay_and_a_second_loop_back_in_a_tick_pays_one_tick() {
    // `~BLOWUP`: a held loop, a marker, a key-on, a random wait of 43 (operand
    // 0, a draw of zero) and the loop back.
    let data = build(&[Spec(
        "A",
        100,
        vec![
            key(0, 0, 0),
            raw(0x15, 0, 0),
            key(16, 0, 0),
            raw(0x1a, 0, 43),
            raw(0x16, 0, 0),
        ],
    )]);
    assert_eq!(keyed_at(&data, 0, 200, 0), vec![0]);
    assert_eq!(keyed_at(&data, 16, 200, 0), vec![0, 43, 86, 129, 172]);

    // No wait: the first `0x16` of a tick costs nothing, the second returns one
    // and pushes the next pass a tick out, so twice a tick, not forever.
    let spin = build(&[Spec(
        "A",
        100,
        vec![raw(0x15, 0, 0), key(0, 0, 0), raw(0x16, 0, 0)],
    )]);
    assert_eq!(keyed_at(&spin, 0, 2, 0), vec![0, 0, 1, 1, 2, 2]);
}

#[test]
fn a_random_wait_adds_its_draw_to_the_next_delay() {
    let data = build(&[Spec(
        "A",
        100,
        vec![
            raw(0x15, 0, 0),
            raw(0x1a, 10, 5),
            key(0, 0, 0),
            raw(0x2b, 0, 100),
        ],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    let mut runner = bank
        .cue_runner(&bank.cue_named("A").expect("cue"))
        .expect("runs");
    let mut seven = || 7;
    assert!(runner.start(&mut seven).is_empty());
    let mut at = None;
    for _ in 0..40 {
        if let Some(grain) = runner.tick(&mut seven).first() {
            at = Some(grain.tick);
            break;
        }
    }
    // `1a`'s own delay is 5, and it returns 7 % 11 onto the key-on's zero.
    assert_eq!(at, Some(12));
    assert!(runner.tick(&mut seven).is_empty());
}

#[test]
fn what_a_runner_does_not_run_it_refuses() {
    for opcode in [0x19, 0x1b, 0x24, 0x05, 0x04] {
        let data = build(&[Spec("A", 100, vec![key(0, 0, 0), raw(opcode, 0, 0)])]);
        let bank = Bank::parse(&data).expect("parse");
        assert!(
            bank.cue_runner(&bank.cue_named("A").unwrap()).is_none(),
            "opcode {opcode:#x}"
        );
    }
    // A guard on a runtime global (a negative index) is not a parameter.
    let data = build(&[Spec(
        "A",
        100,
        vec![raw(0x22, 0x0001_01ff, 0), key(0, 0, 0)],
    )]);
    let bank = Bank::parse(&data).expect("parse");
    assert!(bank.cue_runner(&bank.cue_named("A").unwrap()).is_none());
}

#[test]
fn a_loop_back_with_no_marker_behind_it_kills_the_handler() {
    let data = build(&[Spec("A", 100, vec![key(0, 0, 0), raw(0x16, 0, 0)])]);
    let bank = Bank::parse(&data).expect("parse");
    let mut runner = bank.cue_runner(&bank.cue_named("A").unwrap()).unwrap();
    assert_eq!(runner.start(&mut no_rand).len(), 1);
    assert!(runner.ended());
    assert!(runner.tick(&mut no_rand).is_empty());
}
