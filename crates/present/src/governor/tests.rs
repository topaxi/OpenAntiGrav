use super::*;

const P: f32 = 1.0 / 60.0;

fn s(cpu_share: f32, gpu_share: Option<f32>) -> Sample {
    Sample {
        cpu: cpu_share * P,
        gpu: gpu_share.map(|g| g * P),
        period: Some(P),
    }
}

const KNOBS: [usize; MAX_LEVEL] = [0, 1, 2, 3, 4, 5, 6, 7];

/// Feeds `frames` identical samples and returns the changes, in order. `steps` knobs, ids 0..steps.
fn run(g: &mut Governor, frames: u32, sample: Sample, steps: usize) -> Vec<Change> {
    (0..frames)
        .filter_map(|_| g.record(sample, &KNOBS[..steps]))
        .collect()
}

/// Feeds `frames` samples whose CPU is `base` less what each knob that is down saves (`saves`,
/// by knob id, in shares of the period), so a shed changes what the next frame reads.
fn run_model(g: &mut Governor, frames: u32, base: f32, saves: &[f32], steps: usize) -> Vec<Change> {
    let mut changes = Vec::new();
    for _ in 0..frames {
        let down = KNOBS[..steps]
            .iter()
            .filter(|&&k| !g.is_skipped(k))
            .take(g.level());
        let cpu = base - down.map(|&k| saves[k]).sum::<f32>();
        changes.extend(g.record(s(cpu, Some(0.3)), &KNOBS[..steps]));
    }
    changes
}

fn kinds(changes: &[Change]) -> Vec<(Kind, Option<usize>)> {
    changes.iter().map(|c| (c.kind, c.knob)).collect()
}

fn seconds(n: f32) -> u32 {
    (n / P).ceil() as u32
}

#[test]
fn a_frame_under_budget_changes_nothing() {
    let mut g = Governor::new();
    assert!(run(&mut g, seconds(30.0), s(0.7, Some(0.3)), 3).is_empty());
    assert_eq!(g.level(), 0);
}

#[test]
fn over_budget_for_less_than_the_streak_changes_nothing() {
    let mut g = Governor::new();
    assert!(run(&mut g, seconds(SHED_SECONDS) - 2, s(1.4, Some(0.3)), 3).is_empty());
    assert_eq!(g.level(), 0);
}

#[test]
fn a_sustained_cpu_overrun_sheds_one_knob_and_then_waits() {
    let mut g = Governor::new();
    let changes = run(&mut g, seconds(SHED_SECONDS + 0.5), s(1.4, Some(0.3)), 3);
    assert_eq!(changes.len(), 1);
    assert_eq!((changes[0].from, changes[0].to), (0, 1));
    assert_eq!(g.level(), 1);
}

#[test]
fn a_still_over_budget_cpu_sheds_the_next_knob_after_the_cooldown() {
    let mut g = Governor::new();
    let saves = [0.3; 3];
    let changes = run_model(&mut g, seconds(10.0), 2.0, &saves, 3);
    let levels: Vec<_> = changes.iter().map(|c| (c.from, c.to)).collect();
    assert_eq!(levels, [(0, 1), (1, 2), (2, 3)]);
    assert!(run_model(&mut g, seconds(10.0), 2.0, &saves, 3).is_empty());
    assert_eq!(g.level(), 3);
}

#[test]
fn it_never_goes_past_the_steps_the_caller_has() {
    let mut g = Governor::new();
    run_model(&mut g, seconds(20.0), 1.4, &[0.2], 1);
    assert_eq!(g.level(), 1);
}

#[test]
fn a_gpu_bound_frame_is_left_to_resolution_control() {
    let mut g = Governor::new();
    assert!(run(&mut g, seconds(30.0), s(1.4, Some(1.2)), 3).is_empty());
    assert_eq!(g.level(), 0);
}

#[test]
fn no_gpu_reading_holds() {
    let mut g = Governor::new();
    assert!(run(&mut g, seconds(30.0), s(1.4, None), 3).is_empty());
}

#[test]
fn no_period_is_inert_and_lets_go_of_a_shed_level() {
    let mut g = Governor::new();
    run(&mut g, seconds(SHED_SECONDS + 0.5), s(1.4, Some(0.3)), 3);
    assert_eq!(g.level(), 1);
    let none = Sample {
        cpu: 0.05,
        gpu: Some(0.001),
        period: None,
    };
    let change = g.record(none, &KNOBS[..3]).expect("drops to full quality");
    assert_eq!((change.from, change.to), (1, 0));
    assert!(run(&mut g, seconds(5.0), none, 3).is_empty());
}

#[test]
fn headroom_restores_a_knob_after_the_patience() {
    let mut g = Governor::new();
    run(&mut g, seconds(SHED_SECONDS + 0.5), s(1.4, Some(0.3)), 3);
    assert_eq!(g.level(), 1);
    // The shed knob saved 0.6 of the period: 1.4 fell to 0.8.
    let early = run(&mut g, seconds(RESTORE_SECONDS) - 10, s(0.8, Some(0.3)), 3);
    assert!(early.is_empty(), "restore waits out its patience");
    assert_eq!(g.level(), 1);
}

#[test]
fn a_restore_that_would_not_fit_is_not_taken() {
    let mut g = Governor::new();
    run(&mut g, seconds(SHED_SECONDS + 0.5), s(1.4, Some(0.3)), 3);
    // Learn a 0.6 saving, then hold at 0.7 of the budget: 0.7 + 0.6 > 0.8.
    assert!(run(&mut g, seconds(40.0), s(0.7, Some(0.3)), 3).is_empty());
    assert_eq!(g.level(), 1);
}

#[test]
fn plenty_of_headroom_restores_everything_one_at_a_time() {
    let mut g = Governor::new();
    run_model(&mut g, seconds(5.0), 1.6, &[0.3, 0.3], 2);
    assert_eq!(g.level(), 2);
    let changes = run(&mut g, seconds(60.0), s(0.3, Some(0.3)), 2);
    let levels: Vec<_> = changes.iter().map(|c| (c.from, c.to)).collect();
    assert_eq!(levels, [(2, 1), (1, 0)]);
    assert_eq!(g.level(), 0);
}

#[test]
fn a_gpu_that_fills_up_blocks_the_restore() {
    let mut g = Governor::new();
    run_model(&mut g, seconds(5.0), 1.6, &[0.3, 0.3], 2);
    let level = g.level();
    assert!(run(&mut g, seconds(60.0), s(0.2, Some(0.95)), 2).is_empty());
    assert_eq!(g.level(), level);
}

#[test]
fn a_boundary_sequence_does_not_oscillate() {
    let mut g = Governor::new();
    let mut changes = 0;
    for frame in 0..seconds(120.0) {
        let share = if frame % 2 == 0 { 0.96 } else { 1.04 };
        let share = share - 0.1 * g.level() as f32;
        changes += usize::from(g.record(s(share, Some(0.3)), &KNOBS[..3]).is_some());
    }
    assert!(changes <= 1, "{changes} changes on a boundary sequence");
}

#[test]
fn a_relapse_doubles_the_restore_patience() {
    let mut g = Governor::new();
    let wait = seconds(RESTORE_SECONDS + COOLDOWN_SECONDS + 1.0);
    // The knob saves 0.5 of the period, so the load really falling later is what lets it back.
    run_model(
        &mut g,
        seconds(SHED_SECONDS + COOLDOWN_SECONDS + 0.5),
        1.4,
        &[0.5],
        1,
    );
    let restored = run(&mut g, wait, s(0.2, Some(0.3)), 1);
    assert_eq!(restored.len(), 1);
    assert_eq!(g.level(), 0);
    // It does not hold: shed again at once.
    let shed = run_model(
        &mut g,
        seconds(SHED_SECONDS + COOLDOWN_SECONDS + 0.5),
        1.4,
        &[0.5],
        1,
    );
    assert_eq!(shed.len(), 1);
    // The next restore now waits twice the patience: one patience is not enough.
    let early = run(&mut g, wait, s(0.2, Some(0.3)), 1);
    assert!(early.is_empty(), "patience doubled after a relapse");
    let later = run(&mut g, wait, s(0.2, Some(0.3)), 1);
    assert_eq!(later.len(), 1);
}

#[test]
fn losing_a_knob_drops_the_level() {
    let mut g = Governor::new();
    run_model(&mut g, seconds(10.0), 2.0, &[0.3; 3], 3);
    assert_eq!(g.level(), 3);
    let change = g.record(s(1.4, Some(0.3)), &KNOBS[..1]).expect("clamped");
    assert_eq!((change.from, change.to), (3, 1));
}

#[test]
fn reset_returns_to_full_quality_and_forgets() {
    let mut g = Governor::new();
    run(&mut g, seconds(5.0), s(1.4, Some(0.3)), 3);
    g.reset();
    assert_eq!(g.level(), 0);
    assert!(g.cpu().is_none());
}

#[test]
fn a_step_that_saved_nothing_is_undone_and_a_second_time_skipped() {
    let mut g = Governor::new();
    let changes = run_model(&mut g, seconds(60.0), 1.4, &[0.0], 1);
    assert_eq!(
        kinds(&changes),
        [
            (Kind::Shed, Some(0)),
            (Kind::Undo, Some(0)),
            (Kind::Shed, Some(0)),
            (Kind::Undo, Some(0)),
        ]
    );
    assert_eq!(g.level(), 0);
    assert!(g.is_skipped(0));
}

#[test]
fn a_skipped_knob_is_not_retried_and_the_next_shed_jumps_over_it() {
    let mut g = Governor::new();
    // Knob 1 saves nothing; 0 and 2 save 0.3 each.
    let changes = run_model(&mut g, seconds(60.0), 1.7, &[0.3, 0.0, 0.3], 3);
    assert_eq!(
        kinds(&changes),
        [
            (Kind::Shed, Some(0)),
            (Kind::Shed, Some(1)),
            (Kind::Undo, Some(1)),
            (Kind::Shed, Some(1)),
            (Kind::Undo, Some(1)),
            (Kind::Shed, Some(2)),
        ]
    );
    assert!(g.is_skipped(1));
    assert!(!g.is_skipped(0) && !g.is_skipped(2));
    assert_eq!(g.level(), 2);
    // Nothing is left to shed, and the skipped knob is never tried again.
    assert!(run_model(&mut g, seconds(60.0), 1.7, &[0.3, 0.0, 0.3], 3).is_empty());
}

#[test]
fn a_step_that_saved_something_stays() {
    let mut g = Governor::new();
    let changes = run_model(&mut g, seconds(30.0), 1.4, &[0.3], 1);
    assert_eq!(kinds(&changes), [(Kind::Shed, Some(0))]);
    assert_eq!(g.level(), 1);
    assert!(!g.is_skipped(0));
    let reading = g.take_reading().expect("the shed was measured");
    assert_eq!(reading.knob, 0);
    assert!((reading.saving - 0.3 * P).abs() < 0.01 * P);
}

#[test]
fn one_weak_reading_undoes_a_knob_but_does_not_skip_it() {
    let mut g = Governor::new();
    // The first shed reads nothing (the load rose meanwhile), the second reads the real saving.
    let first = run_model(
        &mut g,
        seconds(SHED_SECONDS) + seconds(COOLDOWN_SECONDS),
        1.4,
        &[0.0],
        1,
    );
    assert_eq!(
        kinds(&first),
        [(Kind::Shed, Some(0)), (Kind::Undo, Some(0))]
    );
    assert!(!g.is_skipped(0));
    let second = run_model(&mut g, seconds(30.0), 1.4, &[0.3], 1);
    assert_eq!(kinds(&second), [(Kind::Shed, Some(0))]);
    assert_eq!(g.level(), 1);
    assert!(!g.is_skipped(0));
    // The strike was forgotten: one more weak reading later would not skip it.
    g.reset();
    let third = run_model(
        &mut g,
        seconds(SHED_SECONDS) + seconds(COOLDOWN_SECONDS),
        1.4,
        &[0.0],
        1,
    );
    assert_eq!(
        kinds(&third),
        [(Kind::Shed, Some(0)), (Kind::Undo, Some(0))]
    );
    assert!(!g.is_skipped(0));
}

#[test]
fn a_skip_lasts_past_a_reset() {
    let mut g = Governor::new();
    run_model(&mut g, seconds(60.0), 1.4, &[0.0], 1);
    assert!(g.is_skipped(0));
    g.reset();
    assert!(g.is_skipped(0));
    // With its only knob skipped the governor has nothing to shed.
    assert!(run_model(&mut g, seconds(30.0), 1.4, &[0.0], 1).is_empty());
}

#[test]
fn an_undo_is_not_a_restore_and_does_not_double_the_next_restore_patience() {
    let mut g = Governor::new();
    // Knob 0 saves nothing and is skipped; knob 1 saves 0.5.
    run_model(&mut g, seconds(60.0), 1.4, &[0.0, 0.5], 2);
    assert!(g.is_skipped(0));
    assert_eq!(g.level(), 1);
    // The load falls: knob 1 comes back after one patience, not two.
    let restored = run(&mut g, seconds(RESTORE_SECONDS + 1.0), s(0.2, Some(0.3)), 2);
    assert_eq!(kinds(&restored), [(Kind::Restore, Some(1))]);
}

const A: u64 = 1;
const B: u64 = 2;

/// Learns that knob 0 saves nothing: two weak readings, so it ends skipped.
fn learn_knob_zero_is_useless(g: &mut Governor) {
    run_model(g, seconds(60.0), 1.4, &[0.0], 1);
    assert!(g.is_skipped(0));
}

#[test]
fn a_knob_skipped_on_one_combination_is_shed_on_another() {
    let mut g = Governor::new();
    g.enter(A);
    learn_knob_zero_is_useless(&mut g);
    g.enter(B);
    assert!(!g.is_skipped(0));
    // On B it pays, so it is shed and kept.
    let changes = run_model(&mut g, seconds(30.0), 1.4, &[0.3], 1);
    assert_eq!(kinds(&changes), [(Kind::Shed, Some(0))]);
    assert_eq!(g.level(), 1);
}

#[test]
fn returning_to_a_combination_has_its_skip_again_without_new_readings() {
    let mut g = Governor::new();
    g.enter(A);
    learn_knob_zero_is_useless(&mut g);
    g.enter(B);
    g.enter(A);
    assert!(g.is_skipped(0));
    // No readings are needed: with its only knob skipped there is nothing to shed.
    assert!(run_model(&mut g, seconds(30.0), 1.4, &[0.0], 1).is_empty());
    assert_eq!(g.level(), 0);
}

#[test]
fn a_weak_reading_is_remembered_with_its_combination() {
    let mut g = Governor::new();
    g.enter(A);
    let weak_once = seconds(SHED_SECONDS) + seconds(COOLDOWN_SECONDS);
    run_model(&mut g, weak_once, 1.4, &[0.0], 1);
    assert!(!g.is_skipped(0));
    g.enter(B);
    g.enter(A);
    // The strike survived the round trip, so one more weak reading skips the knob.
    run_model(&mut g, weak_once, 1.4, &[0.0], 1);
    assert!(g.is_skipped(0));
    // B never saw a strike.
    g.enter(B);
    run_model(&mut g, weak_once, 1.4, &[0.0], 1);
    assert!(!g.is_skipped(0));
}

#[test]
fn naming_the_same_combination_again_keeps_the_level_and_the_skip() {
    let mut g = Governor::new();
    g.enter(A);
    run_model(&mut g, seconds(30.0), 1.4, &[0.3], 1);
    assert_eq!(g.level(), 1);
    g.enter(A);
    assert_eq!(g.level(), 1);
}

#[test]
fn a_new_combination_starts_at_full_quality() {
    let mut g = Governor::new();
    g.enter(A);
    run_model(&mut g, seconds(30.0), 1.4, &[0.3], 1);
    assert_eq!(g.level(), 1);
    g.enter(B);
    assert_eq!(g.level(), 0);
    assert_eq!(g.cpu(), None);
}

#[test]
fn a_stage_restart_of_the_same_combination_keeps_the_skip() {
    let mut g = Governor::new();
    g.enter(A);
    learn_knob_zero_is_useless(&mut g);
    // The game resets between stages and names the combination again when the race starts.
    g.reset();
    g.enter(A);
    assert!(g.is_skipped(0));
    assert!(run_model(&mut g, seconds(30.0), 1.4, &[0.0], 1).is_empty());
}

#[test]
fn a_reset_keeps_what_the_current_combination_learned_and_what_is_shelved() {
    let mut g = Governor::new();
    g.enter(A);
    learn_knob_zero_is_useless(&mut g);
    g.enter(B);
    g.reset();
    assert!(!g.is_skipped(0));
    g.enter(A);
    assert!(g.is_skipped(0));
    g.reset();
    assert!(g.is_skipped(0));
}

#[test]
fn a_combination_that_learned_nothing_takes_no_room() {
    let mut g = Governor::new();
    for key in 0..(MAX_COMBINATIONS as u64 * 3) {
        g.enter(key);
    }
    assert!(g.shelf.is_empty());
}

#[test]
fn past_the_bound_the_combination_entered_longest_ago_is_forgotten() {
    let mut g = Governor::new();
    let first = 1000;
    for key in first..first + MAX_COMBINATIONS as u64 + 2 {
        g.enter(key);
        learn_knob_zero_is_useless(&mut g);
    }
    assert_eq!(g.shelf.len(), MAX_COMBINATIONS);
    // The first two fell off; the latest still holds.
    g.enter(first);
    assert!(!g.is_skipped(0));
    g.enter(first + MAX_COMBINATIONS as u64 + 1);
    assert!(g.is_skipped(0));
}
