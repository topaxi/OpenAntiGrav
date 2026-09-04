//! The direction a Pulse shadow is cast along.
//!
//! One constant, and it is **this title's**: the same three immediates appear
//! once each in both shipped Pulse executables and in neither Pure build - see
//! [`AUTHORED_AXIS`]. A title package is where a value like that belongs
//! ([ADR-0022](../../../docs/architecture/adr/0022-title-packages.md)), rather
//! than in the renderer, where the next title's answer would land beside it as
//! a second literal in the same file.

/// The local axis every occluder projects its shadow along, before its own
/// world matrix rotates it.
///
/// **`(1, -10, 2)` normalized**, and the ratios are the tell: `z / x` is
/// *bit-exactly* `2.0` - the two floats differ by one exponent step and
/// nothing else - and `y / x` is `-10.0000003`. Read from
/// `Shadow_RegisterClass` (`0x08923518`), which writes it into
/// `g_shadow_direction` (`0x08b62540`) from four immediates in the same
/// function that registers vex class `0x3cb`. Full evidence, including the PRX
/// relocation that hid the address, is on
/// [`shadow-occluder.md`](../../../docs/ghidra/functions/psp-pulse-usa/shadow-occluder.md#the-projection-direction-is-normalize05--5-1).
///
/// **A local axis, not a light.** `Shadow_RenderOccluderVolume` transforms it
/// by the occluder's own world matrix and normalizes, so a craft's shadow
/// direction tilts with the craft. Pulse has no light rig to take a direction
/// from, which this is the other half of.
///
/// **Two builds carry it and Pure carries none.** The three immediates
/// (`0x3dc7dd06`, `0xbf79d448`, `0x3e47dd06`) appear exactly once each, paired
/// with their `ori` halves, in `pulse-psp-usa` (`0x0892351c`) and
/// `pulse-psp-eu` (`0x08922ff8`) - and **zero times** in either Pure
/// executable, which is a converged negative rather than a gap: whatever Pure
/// does for a shadow, it does not do it along this vector. Reproduce with
/// [`scripts/psp-reloc.py`](../../../scripts/psp-reloc.py).
///
/// **It is `4.8e-6` short of unit, uniformly, and that is a fact about how it
/// was made rather than noise to round away.** Its length is `0.9999952`, and
/// each component is the same `4.8e-6` relative distance from the exact
/// normalization - the signature of one normalize done with a *fast*
/// reciprocal square root rather than an exact one, which is what the
/// hardware this shipped on offers
/// ([determinism.md](../../../docs/architecture/determinism.md) records the
/// VFPU's `rsqrt` as the reason this project cannot be bit-identical with the
/// original). The shipped bits are kept here rather than a recomputed
/// `normalize(1, -10, 2)`: what the original projects along is this vector,
/// not the mathematically exact one.
///
/// Confidence 88.
pub const AUTHORED_AXIS: [f32; 3] = [0.097_589_54, -0.975_895_4, 0.195_179_08];

/// The whole-number direction [`AUTHORED_AXIS`] is the normalization of.
///
/// Kept beside it because it is the thing someone typed: one right, ten down,
/// two forward. Nothing reads it - the shipped constant is already
/// normalized - and it is here so the next reader sees why those decimals.
pub const AUTHORED_AXIS_RATIO: [f32; 3] = [1.0, -10.0, 2.0];

#[cfg(test)]
mod tests {
    use super::{AUTHORED_AXIS, AUTHORED_AXIS_RATIO};

    /// The ratios, which is where the exactness actually lives.
    #[test]
    fn the_axis_carries_its_whole_number_direction() {
        let [x, y, z] = AUTHORED_AXIS;
        // Bit-exact: the two floats differ by one exponent step.
        assert_eq!(z / x, 2.0, "z is exactly twice x");
        assert!((y / x + 10.0).abs() < 1e-5, "y / x is {}", y / x);
        for axis in 0..3 {
            assert_eq!(
                AUTHORED_AXIS[axis].signum(),
                AUTHORED_AXIS_RATIO[axis].signum()
            );
        }
    }

    /// And the length, which is where it does not: the shipped vector is
    /// `4.8e-6` short of unit, and this pins that rather than rounding it away.
    #[test]
    fn the_axis_is_a_fast_normalize_not_an_exact_one() {
        let length = AUTHORED_AXIS.iter().map(|c| c * c).sum::<f32>().sqrt();
        assert!(length < 1.0, "length {length}");
        assert!((length - 1.0).abs() < 1e-5, "length {length}");
        let down = AUTHORED_AXIS[1];
        assert!(down < -0.9, "it points down: {down}");
        // The exact normalization, for the record: every component is the same
        // relative distance from it, which is one normalize done imprecisely
        // rather than three values typed by hand.
        let exact = AUTHORED_AXIS_RATIO
            .iter()
            .map(|c| c * c)
            .sum::<f32>()
            .sqrt();
        let errors: Vec<f32> = (0..3)
            .map(|axis| {
                let want = AUTHORED_AXIS_RATIO[axis] / exact;
                ((AUTHORED_AXIS[axis] - want) / want).abs()
            })
            .collect();
        for error in &errors {
            assert!((error - errors[0]).abs() < 1e-7, "{errors:?}");
            assert!(*error < 1e-5, "{errors:?}");
        }
    }
}
