use super::*;

const P: f32 = 1.0 / 60.0;

fn s(cpu_share: f32, gpu_share: Option<f32>) -> Sample {
    Sample {
        cpu: cpu_share * P,
        gpu: gpu_share.map(|g| g * P),
        period: Some(P),
    }
}

/// Feeds `frames` identical samples and returns the changes, in order.
fn run(g: &mut Governor, frames: u32, sample: Sample, steps: usize) -> Vec<Change> {
    (0..frames)
        .filter_map(|_| g.record(sample, steps))
        .collect()
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
    let changes = run(&mut g, seconds(10.0), s(1.4, Some(0.3)), 3);
    let levels: Vec<_> = changes.iter().map(|c| (c.from, c.to)).collect();
    assert_eq!(levels, [(0, 1), (1, 2), (2, 3)]);
    assert!(run(&mut g, seconds(10.0), s(1.4, Some(0.3)), 3).is_empty());
    assert_eq!(g.level(), 3);
}

#[test]
fn it_never_goes_past_the_steps_the_caller_has() {
    let mut g = Governor::new();
    run(&mut g, seconds(20.0), s(1.4, Some(0.3)), 1);
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
    let change = g.record(none, 3).expect("drops to full quality");
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
    run(&mut g, seconds(5.0), s(1.4, Some(0.3)), 2);
    assert_eq!(g.level(), 2);
    let changes = run(&mut g, seconds(60.0), s(0.3, Some(0.3)), 2);
    let levels: Vec<_> = changes.iter().map(|c| (c.from, c.to)).collect();
    assert_eq!(levels, [(2, 1), (1, 0)]);
    assert_eq!(g.level(), 0);
}

#[test]
fn a_gpu_that_fills_up_blocks_the_restore() {
    let mut g = Governor::new();
    run(&mut g, seconds(5.0), s(1.4, Some(0.3)), 2);
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
        changes += usize::from(g.record(s(share, Some(0.3)), 3).is_some());
    }
    assert!(changes <= 1, "{changes} changes on a boundary sequence");
}

#[test]
fn a_relapse_doubles_the_restore_patience() {
    let mut g = Governor::new();
    let wait = seconds(RESTORE_SECONDS + COOLDOWN_SECONDS + 1.0);
    // The knob saves nothing here, so the load really falling later is what lets it back.
    run(
        &mut g,
        seconds(SHED_SECONDS + COOLDOWN_SECONDS + 0.5),
        s(1.4, Some(0.3)),
        1,
    );
    let restored = run(&mut g, wait, s(0.2, Some(0.3)), 1);
    assert_eq!(restored.len(), 1);
    assert_eq!(g.level(), 0);
    // It does not hold: shed again at once.
    let shed = run(
        &mut g,
        seconds(SHED_SECONDS + COOLDOWN_SECONDS + 0.5),
        s(1.4, Some(0.3)),
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
    run(&mut g, seconds(10.0), s(1.4, Some(0.3)), 3);
    assert_eq!(g.level(), 3);
    let change = g.record(s(1.4, Some(0.3)), 1).expect("clamped");
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
