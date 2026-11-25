//! What the Detonator craft's two plasma shells show: `Charge1` and `Charge2`, the two channels
//! `Ship_UpdateDetonatorHullParams` smooths toward the EMP's charge
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-ammo-gauge.md`, "`Charge1`, `Charge2`").

use oag_title::DetonatorHullCharge;

/// The two channels, and the scene time they were last stepped at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shown {
    /// `Charge1`, then `Charge2`.
    pub charge: [f32; 2],
    at: f32,
}

impl Shown {
    /// The channels' settled value for the EMP's charge `v`: what they converge to, and where a
    /// first draw starts (a comparison that charges the EMP wants the picture the original had
    /// settled to).
    #[must_use]
    pub fn settled(v: f32, at: f32) -> Self {
        let (first, second) = target(v);
        Self {
            charge: [first, second],
            at,
        }
    }

    /// The channels at `seconds`: each moves toward its target by `rate * dt` and stops on it.
    #[must_use]
    pub fn step(self, law: &DetonatorHullCharge, v: f32, seconds: f32) -> Self {
        let dt = (seconds - self.at).clamp(0.0, 0.1);
        let (first, second) = target(v);
        let mut charge = self.charge;
        for ((value, target), rate) in charge.iter_mut().zip([first, second]).zip(law.rates) {
            let step = rate * dt;
            *value = if *value < target {
                (*value + step).min(target)
            } else {
                (*value - step).max(target)
            };
        }
        Self {
            charge,
            at: seconds,
        }
    }
}

/// The `v` the craft update hands `Ship_UpdateDetonatorHullParams`: `0` below the ready count `lo`,
/// else the charge mapped linearly from `law.ready_level` at `lo` to `1` at `hi` and held there
/// (`0x000eb414`-`0x000eb4b0`, `0x002a3718`). The HUD's gauge is `0` at `lo`, so reading it
/// lights the first shell one charge late.
#[must_use]
pub fn level(law: &DetonatorHullCharge, n: u32, lo: u32, hi: u32) -> f32 {
    if n < lo {
        return 0.0;
    }
    let span = hi.saturating_sub(lo);
    if span == 0 {
        return 1.0;
    }
    let t = (n - lo) as f32 / span as f32;
    (law.ready_level + (1.0 - law.ready_level) * t).min(1.0)
}

/// The targets for the EMP's charge `v`: both `1` at `v >= 1`, `Charge1` alone over `0`, neither
/// otherwise (`0x00107ef8`-`0x00107f18`).
fn target(v: f32) -> (f32, f32) {
    if v >= 1.0 {
        (1.0, 1.0)
    } else if v > 0.0 {
        (1.0, 0.0)
    } else {
        (0.0, 0.0)
    }
}

impl crate::Race {
    /// This frame's plasma-shell charges on the player's craft, or `None` where the title draws
    /// none. Steps what the last frame left; the first frame starts settled.
    pub(crate) fn hull_charge(&self, seconds: f32) -> Option<Shown> {
        let law = self.view.detonator_hull_charge?;
        let (n, lo, hi) = self.core.sim.detonator_emp_counts()?;
        let v = level(&law, n, lo, hi);
        let mut held = crate::drawable::lock(&self.view.hull_charge_shown);
        let shown = match *held {
            Some(shown) => shown.step(&law, v, seconds),
            None => Shown::settled(v, seconds),
        };
        *held = Some(shown);
        Some(shown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HD: DetonatorHullCharge = oag_hd::race::DETONATOR_HULL_CHARGE;

    /// Below the ready threshold neither shell shows; between ready and full the first does; at full
    /// both do.
    #[test]
    fn the_shells_light_at_ready_and_at_full() {
        for (v, expect) in [
            (0.0, [0.0, 0.0]),
            (0.3, [1.0, 0.0]),
            (0.99, [1.0, 0.0]),
            (1.0, [1.0, 1.0]),
        ] {
            assert_eq!(Shown::settled(v, 0.0).charge, expect, "v {v}");
        }
    }

    /// The ready count itself lights the first shell and only the full count the second; below
    /// the ready count neither shows (read live on RPCS3: `n` 3 `(0, 0)`, 4 `(1, 0)`, 5 `(1, 0)`,
    /// 8 `(1, 1)` with the disc's `4` and `8`). The HUD gauge, `0` at `n = 4`, would leave the
    /// first shell dark there.
    #[test]
    fn the_first_shell_lights_at_the_ready_count_itself() {
        for (n, expect) in [
            (0, [0.0, 0.0]),
            (3, [0.0, 0.0]),
            (4, [1.0, 0.0]),
            (5, [1.0, 0.0]),
            (7, [1.0, 0.0]),
            (8, [1.0, 1.0]),
            (12, [1.0, 1.0]),
        ] {
            assert_eq!(
                Shown::settled(level(&HD, n, 4, 8), 0.0).charge,
                expect,
                "n {n}"
            );
        }
    }

    /// Each channel ramps at its own rate (1.3 and 1.6 a second) and stops on its target.
    #[test]
    fn the_channels_ramp_at_their_own_rates() {
        let near = |got: [f32; 2], want: [f32; 2]| {
            assert!(
                got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-5),
                "{got:?} vs {want:?}"
            );
        };
        let shown = Shown::settled(0.0, 0.0).step(&HD, 1.0, 0.1);
        near(shown.charge, [0.13, 0.16]);
        let done = (0..20).fold(shown, |s, i| s.step(&HD, 1.0, 0.2 + 0.1 * i as f32));
        assert_eq!(done.charge, [1.0, 1.0]);
        let fading = done.step(&HD, 0.0, 2.2);
        near(fading.charge, [0.87, 0.84]);
    }
}
