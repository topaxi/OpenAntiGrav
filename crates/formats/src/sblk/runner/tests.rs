//! Hardware-free tests for [`super::Runner`]. The corpus half (`~BLOWUP` and
//! `~ROCKLOCK` from Pulse's `hud.bnk`) is
//! `crates/raceplay/tests/sfx_repeating_ground_truth.rs`.

use super::Runner;
use crate::sblk::Bank;
use crate::sblk::timeline::WalkModel;
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

fn hd() -> WalkModel {
    WalkModel {
        cue_variables: Some([0; 4]),
        ..WalkModel::default()
    }
}

fn hd_runner(grains: Vec<crate::sblk::timeline::tests::Grain>) -> Option<(Vec<u8>, Runner)> {
    let data = build(&[Spec("A", 100, grains)]);
    let runner = {
        let bank = Bank::parse(&data).expect("parse");
        bank.cue_runner_modelled(&bank.cue_named("A").expect("cue"), hd())
    }?;
    Some((data, runner))
}

/// `(tick, waveform offset)` of every key-on over `ticks` master ticks.
fn hd_keys(runner: &mut Runner, ticks: u32, rand: &mut dyn FnMut() -> u32) -> Vec<(u32, u32)> {
    let mut got = runner.start(rand);
    for _ in 0..ticks {
        got.extend(runner.tick(rand));
    }
    got.into_iter().map(|g| (g.tick, g.sound.offset)).collect()
}

/// HD's `~ROCKLOCK`: guards on variable 0 in front of two fixed bends, then a
/// pair, in a loop.
fn hd_lock() -> Vec<crate::sblk::timeline::tests::Grain> {
    vec![
        raw(0x15, 0, 0),
        raw(0x22, 0x00_0100, 0),
        raw(0x1c, 0, 50),
        raw(0x22, 0x00_0101, 0),
        raw(0x1c, 0x06_0000, 15),
        key(0, 0, 0),
        key(16, 0, 3),
        raw(0x16, 0, 0),
    ]
}

#[test]
fn hd_s_guard_reads_its_variable_in_the_high_byte_and_the_bend_follows() {
    let (_data, mut seeking) = hd_runner(hd_lock()).expect("runs");
    assert_eq!(
        hd_keys(&mut seeking, 106, &mut no_rand),
        vec![(50, 0), (53, 16), (103, 0), (106, 16)]
    );
    assert_eq!(seeking.bend(), 0);

    // The original writes variable 0 after the start, so the first pass still
    // waits out the seeking branch it had already chosen.
    let (_data, mut locked) = hd_runner(hd_lock()).expect("runs");
    assert!(locked.start(&mut no_rand).is_empty());
    locked.set_parameter(0, 1);
    let mut got = Vec::new();
    for _ in 0..104 {
        got.extend(locked.tick(&mut no_rand).iter().map(|g| g.tick));
    }
    assert_eq!(got, vec![65, 68, 83, 86, 101, 104]);
    // `a = 6` is `6 * 0x7fff / 0x7f`.
    assert_eq!(locked.bend(), 1548);
}

#[test]
fn hd_s_fixed_bend_shifts_a_negative_operand_instead_of_scaling_it() {
    let (_data, mut runner) = hd_runner(vec![raw(0x1c, 0xfe_0000, 0), key(0, 0, 0)]).expect("runs");
    let _ = runner.start(&mut no_rand);
    assert_eq!(runner.bend(), -512);
}

#[test]
fn hd_s_alternate_runs_one_block_never_the_last_and_skips_the_rest() {
    // Two blocks of one command, then a third waveform, in a loop. The bank's
    // cached pick is 0, so a draw of 0 takes block 1, then block 0, in turn.
    let (_data, mut runner) = hd_runner(vec![
        raw(0x15, 0, 0),
        raw(0x19, 0x02_0100, 10),
        key(0, 0, 0),
        key(16, 0, 0),
        key(32, 0, 0),
        raw(0x16, 0, 0),
    ])
    .expect("runs");
    assert_eq!(
        hd_keys(&mut runner, 30, &mut no_rand),
        vec![(10, 16), (10, 32), (20, 0), (20, 32), (30, 16), (30, 32)]
    );
}

#[test]
fn hd_s_gotos_land_on_the_marker_their_high_byte_names() {
    let (_data, mut jump) = hd_runner(vec![
        raw(0x24, 0x07_0000, 0),
        key(0, 0, 0),
        raw(0x23, 0x07_0000, 0),
        key(16, 0, 0),
    ])
    .expect("runs");
    assert_eq!(hd_keys(&mut jump, 0, &mut no_rand), vec![(0, 16)]);

    // `0x25` draws a marker id in `a..=b`: 2 + (7 % 2) is 3.
    let (_data, mut random) = hd_runner(vec![
        raw(0x25, 0x02_0300, 0),
        raw(0x23, 0x02_0000, 0),
        key(0, 0, 0),
        raw(0x18, 0, 0),
        raw(0x23, 0x03_0000, 0),
        key(16, 0, 0),
    ])
    .expect("runs");
    assert_eq!(hd_keys(&mut random, 0, &mut || 7), vec![(0, 16)]);
}

#[test]
fn hd_s_goto_with_no_wait_pays_a_tick_after_the_eighth() {
    let (_data, mut runner) =
        hd_runner(vec![raw(0x23, 0, 0), key(0, 0, 0), raw(0x24, 0, 0)]).expect("runs");
    // Eight gotos land in the first tick, so it keys nine times; the ninth goto
    // waits a tick.
    let keys = hd_keys(&mut runner, 1, &mut no_rand);
    assert_eq!(keys.iter().filter(|k| k.0 == 0).count(), 9);
    assert_eq!(keys.iter().filter(|k| k.0 == 1).count(), 8);
}

#[test]
fn hd_s_variable_writes_reach_the_guards_and_stop_ends_the_list() {
    // Set variable 1 to 5, add one, and skip the key unless it reads 6.
    let (_data, mut runner) = hd_runner(vec![
        raw(0x1e, 0x01_0500, 0),
        raw(0x20, 0x01_0000, 0),
        raw(0x22, 0x01_0106, 0),
        key(0, 0, 0),
        raw(0x18, 0, 0),
        key(16, 0, 0),
    ])
    .expect("runs");
    assert_eq!(hd_keys(&mut runner, 4, &mut no_rand), vec![(0, 0)]);
    assert!(runner.ended());
}

#[test]
fn hd_s_layout_refuses_what_it_does_not_run_and_pulse_refuses_hd_s_bend() {
    for opcode in [0x04, 0x05, 0x1b, 0x26, 0x29] {
        assert!(
            hd_runner(vec![key(0, 0, 0), raw(opcode, 0, 0)]).is_none(),
            "opcode {opcode:#x}"
        );
    }
    // A guard on HD's global table (a negative variable) is not read.
    assert!(hd_runner(vec![raw(0x22, 0xff_0100, 0), key(0, 0, 0)]).is_none());
    let data = build(&[Spec("A", 100, vec![raw(0x1c, 0, 0), key(0, 0, 0)])]);
    let bank = Bank::parse(&data).expect("parse");
    assert!(bank.cue_runner(&bank.cue_named("A").unwrap()).is_none());
}
