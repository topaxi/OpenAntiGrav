//! A hue swap: [`Tint`], and [`Effect::recoloured`].
//!
//! A burst authored in one colour (Wipeout HD's blue trail-hit sparks) can be redrawn in another
//! colour's trail. A plain multiply by the new colour would only darken an additive core, so the
//! swap keeps each colour's luminance and replaces the hue of its chroma only: a white or grey
//! stays as it was, a saturated colour keeps its brightness and its strength of colour and takes
//! the new direction. It is applied to the **sprite texels**, because that is where these effects
//! carry their colour: the vertex colour only multiplies the texel, so a blue texel stays blue
//! whatever the vertex colour is turned into.

use std::sync::Arc;

use super::{Effect, sprite::Sprite};

/// Rec. 709 luma weights; they sum to 1, so a chroma vector (rgb minus its luma) has none.
const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

fn luma(rgb: [f32; 3]) -> f32 {
    rgb[0] * LUMA[0] + rgb[1] * LUMA[1] + rgb[2] * LUMA[2]
}

/// A hue to draw in instead of the authored one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tint {
    /// The target colour's chroma direction: unit length, zero luma. All zero for a grey
    /// target, which desaturates.
    direction: [f32; 3],
}

impl Tint {
    /// The tint with no hue: every colour becomes the grey of its own luminance.
    pub const GREY: Self = Self {
        direction: [0.0; 3],
    };

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
    /// A channel that would go negative (a subtraction in an additive blend) or above `ceiling`
    /// (a texel byte) shrinks the chroma instead of being clamped, which would move the
    /// luminance.
    #[must_use]
    pub fn apply_within(self, rgb: [f32; 3], ceiling: f32) -> [f32; 3] {
        let y = luma(rgb);
        let chroma = [rgb[0] - y, rgb[1] - y, rgb[2] - y];
        let mut strength =
            (chroma[0] * chroma[0] + chroma[1] * chroma[1] + chroma[2] * chroma[2]).sqrt();
        let floor = self.direction.iter().copied().fold(0.0, f32::min);
        if floor < 0.0 && y > 0.0 {
            strength = strength.min(y / -floor);
        }
        let peak = self.direction.iter().copied().fold(0.0, f32::max);
        if peak > 0.0 && ceiling.is_finite() {
            strength = strength.min((ceiling - y).max(0.0) / peak);
        }
        [
            y + strength * self.direction[0],
            y + strength * self.direction[1],
            y + strength * self.direction[2],
        ]
    }

    /// [`Self::apply_within`] with no ceiling: a palette entry may exceed 1 (an emitter's gain).
    #[must_use]
    pub fn apply(self, rgb: [f32; 3]) -> [f32; 3] {
        self.apply_within(rgb, f32::INFINITY)
    }

    /// `sprite` with every texel's colour swapped, alpha untouched.
    #[must_use]
    pub fn sprite(self, sprite: &Sprite) -> Sprite {
        Sprite {
            width: sprite.width,
            height: sprite.height,
            rgba: self.texels(&sprite.rgba),
            chain: sprite
                .chain
                .as_ref()
                .map(|levels| levels.iter().map(|level| self.texels(level)).collect()),
        }
    }

    /// RGBA8 texels with every colour swapped, alpha untouched.
    fn texels(self, source: &[u8]) -> Arc<[u8]> {
        let mut rgba = source.to_vec();
        for texel in rgba.as_chunks_mut::<4>().0 {
            let rgb = [texel[0], texel[1], texel[2]].map(|c| f32::from(c) / 255.0);
            for (byte, value) in texel.iter_mut().zip(self.apply_within(rgb, 1.0)) {
                *byte = (value * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
        Arc::from(rgba)
    }
}

impl Effect {
    /// A copy of this effect drawn in `tint`'s hue at its own brightness.
    ///
    /// An emitter that samples its sprite takes the hue in the sprite's texels, and its colour
    /// table goes grey (its luminance), so the product of the two is the tint's hue and not that
    /// hue compounded with the table's. An emitter drawn with the procedural profile has only its
    /// colour table, which takes the hue.
    #[must_use]
    pub fn recoloured(&self, tint: Tint) -> Self {
        let mut out = self.clone();
        for spec in &mut out.emitters {
            let sprite = spec
                .sprite
                .as_ref()
                .filter(|_| spec.samples_sprite())
                .map(|sprite| tint.sprite(sprite));
            let palette = if sprite.is_some() { Tint::GREY } else { tint };
            for entry in spec.palette.iter_mut() {
                let [r, g, b] = palette.apply([entry[0], entry[1], entry[2]]);
                entry[..3].copy_from_slice(&[r, g, b]);
            }
            if sprite.is_some() {
                spec.sprite = sprite;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIOLET: [f32; 3] = [1.432, 0.855, 1.812];

    /// The emitter's sprite carries the hue and its colour table goes grey, so their product is
    /// the target hue once, not twice; an emitter with no sprite takes the hue in its table.
    #[test]
    fn an_effect_takes_the_hue_in_its_sprite_or_else_in_its_table() {
        let target = Tint::towards(VIOLET);
        let mut effect = (*crate::psys::tests::effect("blue", false, 1.0)).clone();
        effect.emitters.push(effect.emitters[0].clone());
        for spec in &mut effect.emitters {
            *spec.palette = [[0.6, 0.5, 1.0, 0.75]; 256];
        }
        effect.emitters[0].render = crate::psys::Render::Billboard;
        effect.emitters[0].sprite = Some(Sprite {
            width: 1,
            height: 1,
            rgba: Arc::from(vec![14, 78, 129, 200]),
            chain: None,
        });
        effect.emitters[1].sprite = None;

        let out = effect.recoloured(target);

        let texel = &out.emitters[0].sprite.as_ref().expect("kept").rgba;
        let want = target
            .apply_within([14.0 / 255.0, 78.0 / 255.0, 129.0 / 255.0], 1.0)
            .map(|c| (c * 255.0).round() as u8);
        assert_eq!(&texel[..3], &want, "the texel takes the hue");
        assert!(
            texel[0] > texel[1],
            "red above green: purple, not cyan-blue"
        );
        assert_eq!(texel[3], 200, "alpha untouched");
        let table = out.emitters[0].palette[0];
        let grey = luma([0.6, 0.5, 1.0]);
        assert!(
            (0..3).all(|c| (table[c] - grey).abs() < 1e-5),
            "{table:?} is grey"
        );
        assert_eq!(table[3], 0.75);
        let plain = out.emitters[1].palette[0];
        assert_eq!(
            [plain[0], plain[1], plain[2]],
            target.apply([0.6, 0.5, 1.0]),
            "no sprite: the table takes the hue"
        );
    }

    #[test]
    fn a_texel_that_would_pass_white_shrinks_its_chroma() {
        let bright = [0.0, 0.7, 1.0];
        let out = Tint::towards(VIOLET).apply_within(bright, 1.0);
        assert!(
            out.iter().all(|c| (-1e-6..=1.0 + 1e-6).contains(c)),
            "{out:?}"
        );
        assert!((luma(out) - luma(bright)).abs() < 1e-5);
    }

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
