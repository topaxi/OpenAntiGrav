//! Smooth, seeded, repeatable noise: the wobble a driver's line is given.
//!
//! # Why not `sin`
//!
//! IEEE-754 requires `sqrt` to be correctly rounded and does **not** require it
//! of `sin`, so a sine resolves to the platform's libm and the simulation stops
//! being bit-identical across the three CI operating systems
//! (`docs/architecture/determinism.md`: bring our own implementation rather
//! than weaken the test).
//!
//! So this is value noise: an integer hash at each whole step and a smoothstep
//! between neighbours. Every operation is an integer op or an `f32` multiply-add,
//! both exactly specified, and `wobble(seed, t)` is a pure function of its
//! arguments, so a replay needs nothing stored.

/// Avalanches an integer so neighbouring inputs give unrelated outputs: the
/// 32-bit MurmurHash3 finaliser, chosen because it is published, so its
/// constants can be checked against a reference.
const fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^ (x >> 16)
}

/// The value at whole step `step` of `seed`'s own sequence, in `-1..1`.
fn value(seed: u32, step: u32) -> f32 {
    // 24 bits scaled by 2^-24: exact, as `oag_core::Rng::next_f32` is; a
    // division could round.
    let unit = (mix(seed ^ mix(step)) >> 8) as f32 * (1.0 / 16_777_216.0);
    unit * 2.0 - 1.0
}

/// A smooth signal in `-1..1`, one whole step of `t` per whole step of the
/// sequence. Two octaves (the second three times as fast, a third as tall):
/// one alone is visibly a sine.
#[must_use]
pub fn wobble(seed: u32, t: f32) -> f32 {
    octave(seed, t) * 0.75 + octave(seed ^ 0x9e37_79b9, t * 3.0) * 0.25
}

/// A fresh value in `0.0..1.0` for this seed, this tick and this stream.
///
/// **Independent per tick, where [`wobble`] is smooth by design**: a gate
/// driven off `wobble` would fire in long runs, not at a rate. `stream` keeps
/// unrelated decisions apart (ramming and firing are not one coin twice).
///
/// No state, no clock, and **not a draw from the world's generator**, which
/// would move every later pickup roll and make a driver's decisions depend on
/// how many pickups had been handed out (`Personality::from_pilot` follows the
/// same rule).
#[must_use]
pub fn roll(seed: u32, phase: u32, stream: u32) -> f32 {
    // 24 bits scaled by 2^-24, as `value` does.
    (mix(seed ^ mix(phase ^ mix(stream))) >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// One octave: hash at each end of the step `t` falls in, smoothstep between.
fn octave(seed: u32, t: f32) -> f32 {
    let whole = t.floor();
    let fraction = t - whole;
    // Wrapping, not saturating: the argument only grows, and a race long enough
    // to wrap should keep wobbling.
    let step = whole as i64 as u32;
    let here = value(seed, step);
    let there = value(seed, step.wrapping_add(1));
    // Smoothstep: a corner in the signal is a step in the derivative, which is
    // what the steering loop sees.
    let blend = fraction * fraction * (3.0 - 2.0 * fraction);
    here + (there - here) * blend
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_roll_stays_in_the_unit_interval() {
        for seed in 1..40u32 {
            for phase in 0..200u32 {
                for stream in 0..3u32 {
                    let value = roll(seed, phase, stream);
                    assert!(
                        (0.0..1.0).contains(&value),
                        "{seed}/{phase}/{stream}: {value}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_roll_is_a_pure_function_of_its_arguments() {
        assert_eq!(roll(7, 11, 2), roll(7, 11, 2));
    }

    /// The reason it exists beside `wobble`: a gate wants independence between
    /// ticks, and `wobble` is correlated between them on purpose.
    #[test]
    fn a_roll_changes_every_tick_where_a_wobble_barely_does() {
        let mut roll_steps = 0.0f32;
        let mut wobble_steps = 0.0f32;
        for phase in 0..500u32 {
            roll_steps += (roll(3, phase + 1, 0) - roll(3, phase, 0)).abs();
            let rate = 1.0 / 300.0;
            wobble_steps +=
                (wobble(3, (phase + 1) as f32 * rate) - wobble(3, phase as f32 * rate)).abs();
        }
        assert!(
            roll_steps > wobble_steps * 20.0,
            "a roll should move far more per tick than a drift: {roll_steps} against {wobble_steps}"
        );
    }

    /// Two decisions must not turn out to be one coin landing twice.
    #[test]
    fn two_streams_of_one_seed_do_not_agree() {
        let mut agreements = 0;
        let total = 2_000;
        for phase in 0..total {
            if (roll(9, phase, 0) < 0.5) == (roll(9, phase, 1) < 0.5) {
                agreements += 1;
            }
        }
        let ratio = f64::from(agreements) / f64::from(total);
        assert!(
            (0.45..0.55).contains(&ratio),
            "streams agree {ratio} of the time"
        );
    }

    /// A gate compares against a rate, so the values have to be spread rather
    /// than merely varied.
    #[test]
    fn a_roll_is_uniform_enough_to_threshold_against() {
        let mut buckets = [0u32; 10];
        for phase in 0..10_000u32 {
            buckets[(roll(5, phase, 0) * 10.0) as usize % 10] += 1;
        }
        for (bucket, count) in buckets.iter().enumerate() {
            assert!(
                (800..1200).contains(count),
                "bucket {bucket} held {count} of 10000"
            );
        }
    }
    use super::*;

    /// The published MurmurHash3 finaliser against values from a separate
    /// implementation, as `oag_core::rng`'s reference vector is: it fails loudly
    /// if the mixer quietly changes.
    #[test]
    fn the_mixer_matches_the_published_finaliser() {
        assert_eq!(mix(0), 0);
        assert_eq!(mix(1), 0x514e_28b7);
        assert_eq!(mix(2), 0x30f4_c306);
        assert_eq!(mix(0xffff_ffff), 0x81f1_6f39);
    }

    #[test]
    fn it_stays_in_range() {
        for step in 0..2_000 {
            let t = step as f32 * 0.037;
            for seed in [1u32, 7, 0xdead_beef, 0x1234_5678] {
                let value = wobble(seed, t);
                assert!((-1.0..=1.0).contains(&value), "{value} at {t}, seed {seed}");
            }
        }
    }

    /// The point of it: consecutive samples are close together, so a driver
    /// following it is not asked to jump.
    #[test]
    fn it_is_smooth() {
        let rate = 1.0 / 300.0;
        let mut last = wobble(11, 0.0);
        let mut worst = 0.0f32;
        for tick in 1..10_000 {
            let now = wobble(11, tick as f32 * rate);
            worst = worst.max((now - last).abs());
            last = now;
        }
        assert!(worst < 0.02, "biggest jump between ticks was {worst}");
    }

    /// And it has to actually move, or the wobble is a very expensive zero.
    #[test]
    fn it_covers_ground() {
        let rate = 1.0 / 300.0;
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        for tick in 0..10_000 {
            let value = wobble(11, tick as f32 * rate);
            low = low.min(value);
            high = high.max(value);
        }
        assert!(low < -0.5, "never went far negative: {low}");
        assert!(high > 0.5, "never went far positive: {high}");
    }

    /// Two craft must not wobble together, which is the whole reason there is a
    /// seed.
    #[test]
    fn different_seeds_are_uncorrelated() {
        let rate = 1.0 / 300.0;
        let mut agreed = 0;
        for tick in 0..6_000 {
            let t = tick as f32 * rate;
            if (wobble(1, t) - wobble(2, t)).abs() < 0.05 {
                agreed += 1;
            }
        }
        assert!(
            agreed < 600,
            "seeds 1 and 2 agreed on {agreed} of 6000 ticks"
        );
    }

    #[test]
    fn it_is_a_pure_function_of_its_arguments() {
        assert_eq!(wobble(5, 12.25), wobble(5, 12.25));
    }
}
