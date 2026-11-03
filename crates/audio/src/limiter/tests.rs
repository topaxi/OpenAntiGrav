use super::*;

const RATE: u32 = 48_000;

fn run(limiter: &mut Limiter, frames: usize, level: impl Fn(usize) -> f32) -> Vec<f32> {
    let mut out = Vec::new();
    for i in 0..frames {
        let mut frame = [level(i), -level(i)];
        limiter.process(&mut frame);
        out.extend_from_slice(&frame);
    }
    out
}

#[test]
fn a_quiet_signal_passes_unchanged_after_the_delay() {
    let mut limiter = Limiter::new(RATE);
    let out = run(&mut limiter, 1000, |_| 0.5);
    let lag = limiter.latency();
    assert!(out[..lag * CHANNELS].iter().all(|s| *s == 0.0));
    assert!(
        out[lag * CHANNELS..]
            .chunks(2)
            .all(|f| f[0] == 0.5 && f[1] == -0.5)
    );
    assert_eq!(limiter.limited(), 0);
}

#[test]
fn a_loud_burst_never_leaves_the_ceiling() {
    let mut limiter = Limiter::new(RATE);
    let out = run(&mut limiter, 20_000, |i| {
        if (5_000..9_000).contains(&i) {
            3.0
        } else {
            0.2
        }
    });
    let peak = out.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    assert!(peak <= CEILING + 1e-5, "peak {peak}");
    assert!(
        peak > CEILING - 0.02,
        "the limiter should land on the ceiling, not far under: {peak}"
    );
}

#[test]
fn gain_comes_back_after_a_burst() {
    let mut limiter = Limiter::new(RATE);
    let out = run(&mut limiter, RATE as usize * 2, |i| {
        if i < 2_000 { 3.0 } else { 0.2 }
    });
    let last = out[out.len() - 2];
    assert!((last - 0.2).abs() < 1e-3, "gain did not return: {last}");
}

#[test]
fn a_single_sample_spike_is_caught() {
    let mut limiter = Limiter::new(RATE);
    let out = run(&mut limiter, 5_000, |i| if i == 2_500 { 10.0 } else { 0.1 });
    assert!(out.iter().all(|s| s.abs() <= CEILING + 1e-5));
}

#[test]
fn a_sustained_low_note_over_full_scale_does_not_ripple_the_gain() {
    let mut limiter = Limiter::new(RATE);
    let tau = std::f32::consts::TAU;
    let out = run(&mut limiter, RATE as usize, |i| {
        2.0 * (tau * 50.0 * i as f32 / RATE as f32).sin()
    });
    let settled = &out[RATE as usize / 4 * CHANNELS..];
    let periods = RATE as usize / 50 * CHANNELS;
    let peaks: Vec<f32> = settled
        .chunks(periods)
        .filter(|c| c.len() == periods)
        .map(|c| c.iter().step_by(2).fold(0.0_f32, |m, s| m.max(s.abs())))
        .collect();
    let (lo, hi) = peaks
        .iter()
        .fold((f32::MAX, 0.0_f32), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    assert!(hi <= CEILING + 1e-5, "{hi}");
    assert!((hi - lo) / hi < 0.01, "envelope moved {lo}..{hi}");
}

#[test]
fn after_a_long_burst_the_limiter_goes_idle_and_stays_idle() {
    let mut limiter = Limiter::new(RATE);
    run(&mut limiter, RATE as usize * 20, |i| {
        if i % 3000 < 1500 { 4.0 } else { 0.3 }
    });
    run(&mut limiter, RATE as usize * 3, |_| 0.1);
    let before = limiter.limited();
    run(&mut limiter, RATE as usize, |_| 0.1);
    assert_eq!(limiter.limited(), before);
    assert_eq!(limiter.average, limiter.len as f64);
}

/// Cost of the limiter per device callback. Run in release:
/// `cargo nextest run --release -p oag-audio --run-ignored only limiter_cost`.
#[test]
#[ignore = "a timing probe; prints microseconds per 1024-frame callback"]
fn limiter_cost() {
    let tau = std::f32::consts::TAU;
    for (name, level) in [("idle (0.3 sine)", 0.3_f32), ("limiting (2.0 sine)", 2.0)] {
        let mut limiter = Limiter::new(RATE);
        let mut buffer: Vec<f32> = (0..1024 * CHANNELS)
            .map(|i| level * (tau * 90.0 * (i / 2) as f32 / RATE as f32).sin())
            .collect();
        let template = buffer.clone();
        let callbacks = 20_000;
        let start = std::time::Instant::now();
        for _ in 0..callbacks {
            buffer.copy_from_slice(&template);
            limiter.process(&mut buffer);
        }
        let micros = start.elapsed().as_secs_f64() * 1e6 / f64::from(callbacks);
        println!("{name}: {micros:.1} us per 1024-frame callback");
    }
}
