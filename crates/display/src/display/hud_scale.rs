//! [`HudScale`]: how a raster HUD is stretched onto a screen several times
//! the size of the one it was drawn for.
//!
//! Its own file for the 1,000-line rule in `scripts/check-file-size.py`, the
//! same reason `motion_blur.rs` is.

use serde::{Deserialize, Serialize};

/// How a HUD authored as raster art (Pulse's 480x272 PSP sheet, Pure's) is
/// stretched to the output, where the original's screen was a few hundred
/// pixels across and ours is a thousand to several thousand.
///
/// **Only the titles whose HUD is raster offer it** (`HudArt::raster`);
/// HD, 2048 and Omega author their HUD at a size a stretch does not blur, so
/// they draw as before whatever this says.
///
/// An enhancement of this project's, not a recovery: the PSP's own panel is
/// 480x272 and shows one texel per pixel, so there is no original behaviour at
/// this scale to match. What is chosen is chosen, not measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum HudScale {
    /// Every texel is an evenly sized block of whole pixels: each glyph and
    /// sprite is drawn at the floor of the HUD's scale factor, anchored where
    /// the layout puts it. Crisp, and smaller than full scale wherever the
    /// factor is not near a whole number (a quarter smaller on a 1280x800 deck).
    Integer,
    /// Nearest to the integer factor, a one pixel linear blend between texels
    /// for the remainder. Full size, and identical to `integer` wherever the
    /// factor is a whole number. The default: it never shrinks the HUD.
    #[default]
    SharpBilinear,
    /// Nearest-neighbour at the fractional factor: full size, with texels
    /// alternating between two widths.
    Nearest,
    /// Plain linear stretch, the HUD as it was drawn before this setting.
    Linear,
}

impl HudScale {
    /// The spelling used in a settings file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::SharpBilinear => "sharp-bilinear",
            Self::Nearest => "nearest",
            Self::Linear => "linear",
        }
    }

    /// Every choice, for error messages.
    pub const ALL: [Self; 4] = [
        Self::Integer,
        Self::SharpBilinear,
        Self::Nearest,
        Self::Linear,
    ];

    /// Whether glyph and sprite textures are sampled without filtering.
    #[must_use]
    pub fn samples_nearest(self) -> bool {
        matches!(self, Self::Integer | Self::Nearest)
    }

    /// Whether the shader blends only the last pixel of each texel.
    #[must_use]
    pub fn is_sharp_bilinear(self) -> bool {
        self == Self::SharpBilinear
    }

    /// Whether quads are resized to whole multiples of a texel.
    #[must_use]
    pub fn snaps_geometry(self) -> bool {
        self == Self::Integer
    }
}

/// How far over a whole factor a texel may be drawn before the factor rounds
/// down instead of up: 2%.
///
/// **Chosen, not measured.** The PSP grid is 480x272 and a 1080p frame is
/// 270 rows of it, so the scale factor is 3.97 and not 4: a strict floor
/// would draw every texel at 3 pixels on the two most common screens, a 25%
/// shrink for the sake of 0.7%. Two percent covers that and the 4K row (7.94).
const OVERSIZE_TOLERANCE: f32 = 0.02;

/// The whole number of screen pixels one texel is drawn as, given how many it
/// covers at the HUD's true scale: its floor, give or take [`OVERSIZE_TOLERANCE`].
///
/// `None` below one pixel per texel: a texel shown smaller than a pixel is a
/// minification, which no whole factor describes, so it is left as laid out.
/// The tolerance also keeps a factor that is a whole number up to rounding
/// (`3.0000002`, or `2.9999998` off the same product) from flipping a texel
/// between two sizes.
#[must_use]
pub fn integer_factor(pixels_per_texel: f32) -> Option<f32> {
    if pixels_per_texel < 1.0 - 1e-3 {
        return None;
    }
    Some(
        (pixels_per_texel * (1.0 + OVERSIZE_TOLERANCE))
            .floor()
            .max(1.0),
    )
}

impl std::str::FromStr for HudScale {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!(
                    "{text:?} is not a HUD scale; try integer, sharp-bilinear, nearest or linear"
                )
            })
    }
}

impl std::fmt::Display for HudScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for HudScale {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<HudScale> for String {
    fn from(mode: HudScale) -> Self {
        mode.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_factor_is_the_floor_and_never_flips_on_rounding() {
        assert_eq!(
            integer_factor(3.97),
            Some(4.0),
            "1080p over the 272-row grid"
        );
        assert_eq!(integer_factor(7.94), Some(8.0), "2160p");
        assert_eq!(
            integer_factor(2.67),
            Some(2.0),
            "a 1280-wide Deck: no rounding up"
        );
        assert_eq!(integer_factor(3.0), Some(3.0));
        assert_eq!(integer_factor(2.9999998), Some(3.0));
        assert_eq!(integer_factor(8.0), Some(8.0));
        assert_eq!(integer_factor(1.4), Some(1.0));
        assert_eq!(integer_factor(0.6), None, "a minified texel is left alone");
    }

    #[test]
    fn every_name_round_trips_and_a_stranger_is_refused() {
        for mode in HudScale::ALL {
            assert_eq!(mode.name().parse::<HudScale>(), Ok(mode));
        }
        assert!("crisp".parse::<HudScale>().is_err());
    }

    #[test]
    fn only_integer_resizes_and_only_the_unfiltered_modes_sample_nearest() {
        assert!(HudScale::Integer.snaps_geometry());
        assert!(!HudScale::Nearest.snaps_geometry());
        assert!(HudScale::Integer.samples_nearest() && HudScale::Nearest.samples_nearest());
        assert!(!HudScale::SharpBilinear.samples_nearest());
        assert!(HudScale::SharpBilinear.is_sharp_bilinear());
        assert!(!HudScale::Linear.samples_nearest() && !HudScale::Linear.is_sharp_bilinear());
    }
}
