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
