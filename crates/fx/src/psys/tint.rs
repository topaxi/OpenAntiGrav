//! A per-instance hue swap: [`Tint`].
//!
//! A burst authored in one colour (Wipeout HD's blue trail-hit sparks) can be played in another
//! colour's trail. A plain multiply by the new colour would only darken a white-hot additive
//! core, so the swap keeps each vertex's luminance and replaces the hue of its chroma only: a
//! white or grey vertex stays as it was, a saturated one keeps its brightness and its strength
//! of colour and takes the new direction.

use oag_mesh::mesh::GpuVertex;

/// Rec. 709 luma weights; they sum to 1, so a chroma vector (rgb minus its luma) has none.
const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

fn luma(rgb: [f32; 3]) -> f32 {
    rgb[0] * LUMA[0] + rgb[1] * LUMA[1] + rgb[2] * LUMA[2]
}

/// The hue a particle is drawn in instead of the one it was authored in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tint {
    /// The target colour's chroma direction: unit length, zero luma. All zero for a grey
    /// target, which desaturates.
    direction: [f32; 3],
}

impl Tint {
    /// The tint toward `colour`'s hue. Only the hue is read: its brightness and its strength of
    /// colour are not, so `[0.0, 0.03, 0.5]` and `[0.0, 0.3, 5.0]` are the same tint.
    #[must_use]
    pub fn towards(colour: [f32; 3]) -> Self {
        let y = luma(colour);
        let chroma = [colour[0] - y, colour[1] - y, colour[2] - y];
        let length = (chroma[0] * chroma[0] + chroma[1] * chroma[1] + chroma[2] * chroma[2]).sqrt();
        let direction = if length > 1e-6 {
            [chroma[0] / length, chroma[1] / length, chroma[2] / length]
        } else {
            [0.0; 3]
        };
        Self { direction }
    }

    /// `rgb` with the same luminance and the same chroma magnitude along the tint's direction.
    ///
    /// A channel that would go negative (a subtraction in an additive blend) shrinks the chroma
    /// instead of being clamped, which would move the luminance. Components above 1 are legal:
    /// a vertex colour carries the emitter's gain.
    #[must_use]
    pub fn apply(self, rgb: [f32; 3]) -> [f32; 3] {
        let y = luma(rgb);
        let chroma = [rgb[0] - y, rgb[1] - y, rgb[2] - y];
        let mut strength =
            (chroma[0] * chroma[0] + chroma[1] * chroma[1] + chroma[2] * chroma[2]).sqrt();
        let floor = self.direction.iter().copied().fold(0.0, f32::min);
        if floor < 0.0 && y > 0.0 {
            strength = strength.min(y / -floor);
        }
        [
            y + strength * self.direction[0],
            y + strength * self.direction[1],
            y + strength * self.direction[2],
        ]
    }

    /// Recolours the vertices `from..` of `list`, alpha untouched.
    pub(super) fn recolour(self, list: &mut [GpuVertex], from: usize) {
        for vertex in &mut list[from..] {
            let [r, g, b] = self.apply([vertex.colour[0], vertex.colour[1], vertex.colour[2]]);
            vertex.colour[..3].copy_from_slice(&[r, g, b]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIOLET: [f32; 3] = [1.432, 0.855, 1.812];

    #[test]
    fn white_and_grey_stay_as_they_were() {
        let tint = Tint::towards(VIOLET);
        for grey in [[1.0; 3], [0.4; 3], [2.5; 3], [0.0; 3]] {
            let out = tint.apply(grey);
            for c in 0..3 {
                assert!((out[c] - grey[c]).abs() < 1e-5, "{grey:?} -> {out:?}");
            }
        }
    }

    #[test]
    fn a_blue_takes_the_target_hue_at_equal_luminance() {
        let blue = [0.1, 0.2, 1.0];
        let out = Tint::towards(VIOLET).apply(blue);
        assert!((luma(out) - luma(blue)).abs() < 1e-5, "{out:?}");
        assert!(out.iter().all(|c| *c >= 0.0), "{out:?}");
        // The chroma now points the way the target's does: red up, green down, blue up.
        let y = luma(out);
        let target = Tint::towards(VIOLET).direction;
        let chroma = [out[0] - y, out[1] - y, out[2] - y];
        let dot: f32 = (0..3).map(|c| chroma[c] * target[c]).sum();
        let length = chroma.iter().map(|c| c * c).sum::<f32>().sqrt();
        assert!((dot - length).abs() < 1e-4, "{chroma:?} vs {target:?}");
    }

    #[test]
    fn two_targets_of_one_hue_are_one_tint_and_two_hues_are_not() {
        let (small, large) = (
            Tint::towards([0.0, 0.03, 0.5]).direction,
            Tint::towards([0.0, 0.3, 5.0]).direction,
        );
        assert!(
            (0..3).all(|c| (small[c] - large[c]).abs() < 1e-5),
            "{small:?} {large:?}"
        );
        let blue = [0.1, 0.2, 1.0];
        assert_ne!(
            Tint::towards([1.0, 0.1, 0.1]).apply(blue),
            Tint::towards([0.1, 1.0, 0.1]).apply(blue)
        );
    }

    #[test]
    fn a_dark_vertex_shrinks_its_chroma_rather_than_going_negative() {
        let out = Tint::towards([1.0, 0.0, 0.0]).apply([0.0, 0.0, 0.2]);
        assert!(out.iter().all(|c| *c >= -1e-6), "{out:?}");
        assert!((luma(out) - luma([0.0, 0.0, 0.2])).abs() < 1e-5);
    }
}
