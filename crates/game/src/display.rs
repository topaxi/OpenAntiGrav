//! What shape the game is drawn at, and what kind of window it is drawn in.
//!
//! Three settings live here and they are three different questions, which is
//! why they are three settings:
//!
//! - [`WindowMode`] is what the compositor is asked for.
//! - [`Size`] is how big a *windowed* window is, and means nothing in the other
//!   two modes, where the display decides.
//! - [`Aspect`] is the shape the game is drawn at **inside** whatever it got,
//!   with the leftover bars left black.
//!
//! The arithmetic is [`viewport`], a pure function with tests, because none of
//! this is checkable from the gate: it is all window and GPU, and a screenshot
//! per mode is the only empirical check there is. Keeping the fit in a function
//! means the part that can be wrong in a way nobody notices is the part that is
//! tested.

use serde::{Deserialize, Serialize};

/// The shape the game is drawn at inside its window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aspect {
    /// 480x272, the PSP's own framebuffer, and the shape every camera value on
    /// the disc was authored for.
    ///
    /// The default, and not merely out of deference: `<ExternalCameraFar>`'s
    /// field of view is only defined at this ratio, so it is the one shape where
    /// what a player sees is what the original framed. See
    /// [`crate::race::AUTHORED_ASPECT`].
    #[default]
    Psp,
    /// 4:3, which is what the PS2 release output.
    Ps2,
    /// Whatever the window is.
    ///
    /// **The field of view does not widen with it.** `Race::projection` caps at
    /// the authored aspect and fits the view inside anything wider, so a wide
    /// window shows the same amount of track rather than more of it. That is a
    /// deliberate reading of "free" as *fill the window* and not as *see more* -
    /// nothing on the disc says what the original would have done with a 21:9
    /// screen, and inventing a wider field of view would be inventing gameplay.
    Free,
}

impl Aspect {
    /// The ratio this shape asks for, or `None` for [`Aspect::Free`], which asks
    /// for whatever it is given.
    #[must_use]
    pub fn ratio(self) -> Option<f32> {
        match self {
            Self::Psp => Some(crate::frontend::SCREEN.0 / crate::frontend::SCREEN.1),
            Self::Ps2 => Some(4.0 / 3.0),
            Self::Free => None,
        }
    }

    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Psp => "psp",
            Self::Ps2 => "ps2",
            Self::Free => "free",
        }
    }

    /// Every shape, for the menus and for error messages.
    pub const ALL: [Self; 3] = [Self::Psp, Self::Ps2, Self::Free];
}

impl std::str::FromStr for Aspect {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|aspect| aspect.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not an aspect ratio; try psp, ps2 or free"))
    }
}

impl std::fmt::Display for Aspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// What kind of window the game asks the compositor for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    /// A window of [`Size`], with decorations.
    #[default]
    Windowed,
    /// A borderless window filling the monitor it is on.
    ///
    /// This is what "fullscreen" means on a modern desktop: no mode switch, no
    /// resolution change, and alt-tab does not black the screen. It also cannot
    /// fail, which exclusive fullscreen can.
    Borderless,
}

impl WindowMode {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Windowed => "windowed",
            Self::Borderless => "borderless",
        }
    }

    /// Every mode this build offers.
    ///
    /// **Exclusive fullscreen is deliberately not here.** It needs a `VideoMode`
    /// enumerated off the monitor, which is a row of its own and a failure path
    /// of its own, and on every compositor this project is developed against it
    /// buys nothing over borderless. Half-wiring it would put a row on the menu
    /// that sometimes does nothing.
    pub const ALL: [Self; 2] = [Self::Windowed, Self::Borderless];
}

impl std::str::FromStr for WindowMode {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a window mode; try windowed or borderless"))
    }
}

impl std::fmt::Display for WindowMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// A window size, spelled `1440x816`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Size {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

impl Size {
    /// The sizes the menus offer, at the PSP's own aspect.
    ///
    /// Multiples of 480x272 and then a few common desktop heights at the same
    /// ratio, so the default shape needs no letterboxing at any of them. A
    /// player on a different aspect gets bars, which is what [`Aspect`] is for.
    pub const OFFERED: [Self; 6] = [
        Self::new(960, 544),
        Self::new(1280, 720),
        Self::new(1440, 816),
        Self::new(1600, 900),
        Self::new(1920, 1080),
        Self::new(2560, 1440),
    ];

    /// A size, with both dimensions forced to at least one.
    ///
    /// Zero is what a minimised window reports and what a surface cannot be
    /// configured at, so it is clamped here rather than at each of the several
    /// places that would otherwise have to.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width: if width == 0 { 1 } else { width },
            height: if height == 0 { 1 } else { height },
        }
    }
}

impl Default for Size {
    /// 1440x816: three times the PSP's own framebuffer, and what the window
    /// opened at before any of this was configurable.
    fn default() -> Self {
        Self::new(1440, 816)
    }
}

impl std::str::FromStr for Size {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (width, height) = text
            .split_once(['x', 'X'])
            .ok_or_else(|| format!("{text:?} is not a size; try 1440x816"))?;
        let parse = |value: &str, which: &str| {
            value
                .trim()
                .parse::<u32>()
                .map_err(|e| format!("{text:?}: {which} is not a number: {e}"))
        };
        let size = Self::new(parse(width, "width")?, parse(height, "height")?);
        if size.width < 2 || size.height < 2 {
            return Err(format!("{text:?} is too small to draw into"));
        }
        Ok(size)
    }
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

impl TryFrom<String> for Size {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Size> for String {
    fn from(size: Size) -> Self {
        size.to_string()
    }
}

/// How much of the viewport rectangle the game is actually rendered at, as a
/// percentage.
///
/// Below 100 this is the usual internal-resolution knob; above it, it is
/// supersampling. A percentage rather than an absolute resolution because a
/// percentage has no invalid values - see [`crate::upscale`] for the argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Scale(u32);

impl Scale {
    /// Rendering at exactly the size it is displayed at.
    pub const FULL: Self = Self(100);

    /// The narrowest and widest a scale may be.
    ///
    /// Not preferences: below the floor a 480-wide viewport renders at 120
    /// pixels and the menus stop being readable at all, and above the ceiling
    /// the target runs past what an adapter will allocate on an ordinary
    /// window. [`crate::upscale::target_size`] clamps the actual pixels too,
    /// because the ceiling here is not a per-device answer.
    pub const RANGE: std::ops::RangeInclusive<u32> = 25..=200;

    /// The percentages the menus offer.
    pub const OFFERED: [Self; 6] = [
        Self(50),
        Self(75),
        Self(100),
        Self(125),
        Self(150),
        Self(200),
    ];

    /// The multiplier this percentage means.
    #[must_use]
    pub fn factor(self) -> f32 {
        self.0 as f32 / 100.0
    }

    /// The percentage itself.
    #[must_use]
    pub fn percent(self) -> u32 {
        self.0
    }
}

impl Default for Scale {
    fn default() -> Self {
        Self::FULL
    }
}

impl std::str::FromStr for Scale {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let percent: u32 = text
            .trim()
            .trim_end_matches('%')
            .parse()
            .map_err(|e| format!("{text:?} is not a render scale: {e}"))?;
        Self::try_from(percent)
    }
}

impl TryFrom<u32> for Scale {
    type Error = String;

    fn try_from(percent: u32) -> Result<Self, Self::Error> {
        if Self::RANGE.contains(&percent) {
            Ok(Self(percent))
        } else {
            Err(format!(
                "a render scale of {percent} is outside {}-{}",
                Self::RANGE.start(),
                Self::RANGE.end()
            ))
        }
    }
}

impl From<Scale> for u32 {
    fn from(scale: Scale) -> Self {
        scale.0
    }
}

impl std::fmt::Display for Scale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The centred rectangle of `aspect` inside a `target`-sized viewport.
///
/// Returns `(x, y, width, height)` in physical pixels, which is exactly what
/// `wgpu::RenderPass::set_viewport` takes. The leftover is never drawn into and
/// stays whatever the pass cleared to, which is black.
///
/// Both dimensions come back at least one pixel. A zero-area viewport is a
/// validation error in wgpu and a minimised window is the ordinary way to get
/// one, so the degenerate case is handled here rather than at the call site.
#[must_use]
pub fn viewport(target: (u32, u32), aspect: Aspect) -> (f32, f32, f32, f32) {
    let width = target.0.max(1) as f32;
    let height = target.1.max(1) as f32;
    let Some(wanted) = aspect.ratio() else {
        return (0.0, 0.0, width, height);
    };

    let have = width / height;
    if have > wanted {
        // Too wide: bars down the sides.
        let fitted = (height * wanted).max(1.0);
        ((width - fitted) / 2.0, 0.0, fitted, height)
    } else {
        // Too tall, or exact: bars top and bottom, and nothing when exact.
        let fitted = (width / wanted).max(1.0);
        (0.0, (height - fitted) / 2.0, width, fitted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The aspect of a rectangle `viewport` returned.
    fn ratio(rect: (f32, f32, f32, f32)) -> f32 {
        rect.2 / rect.3
    }

    #[test]
    fn free_fills_whatever_it_is_given() {
        assert_eq!(
            viewport((1920, 1080), Aspect::Free),
            (0.0, 0.0, 1920.0, 1080.0)
        );
        assert_eq!(viewport((100, 900), Aspect::Free), (0.0, 0.0, 100.0, 900.0));
    }

    /// The window this build has always opened at is exactly the PSP's shape,
    /// so the default setting on the default window draws no bars at all.
    #[test]
    fn the_default_window_needs_no_bars_at_the_default_aspect() {
        let size = Size::default();
        let rect = viewport((size.width, size.height), Aspect::Psp);
        assert_eq!(rect.0, 0.0, "{rect:?}");
        assert_eq!(rect.1, 0.0, "{rect:?}");
        assert!(
            (rect.2 - size.width as f32).abs() < 1.0 && (rect.3 - size.height as f32).abs() < 1.0,
            "{rect:?}"
        );
    }

    #[test]
    fn a_wide_window_gets_bars_down_the_sides() {
        let rect = viewport((3440, 1440), Aspect::Psp);
        assert!(rect.0 > 0.0, "no left bar: {rect:?}");
        assert_eq!(rect.1, 0.0, "and no top bar: {rect:?}");
        assert!((rect.3 - 1440.0).abs() < 1e-3, "full height: {rect:?}");
        assert!(
            (ratio(rect) - Aspect::Psp.ratio().expect("psp has a ratio")).abs() < 1e-3,
            "{rect:?}"
        );
        // Centred, so both bars are the same width.
        assert!((rect.0 * 2.0 + rect.2 - 3440.0).abs() < 1e-3, "{rect:?}");
    }

    #[test]
    fn a_tall_window_gets_bars_above_and_below() {
        let rect = viewport((1080, 1920), Aspect::Ps2);
        assert_eq!(rect.0, 0.0, "{rect:?}");
        assert!(rect.1 > 0.0, "{rect:?}");
        assert!((ratio(rect) - 4.0 / 3.0).abs() < 1e-3, "{rect:?}");
        assert!((rect.1 * 2.0 + rect.3 - 1920.0).abs() < 1e-3, "{rect:?}");
    }

    /// The two fixed shapes really are different, or the setting does nothing.
    #[test]
    fn psp_and_ps2_are_not_the_same_shape() {
        let psp = viewport((1920, 1080), Aspect::Psp);
        let ps2 = viewport((1920, 1080), Aspect::Ps2);
        assert!((ratio(psp) - ratio(ps2)).abs() > 0.1, "{psp:?} {ps2:?}");
        assert!(ps2.0 > psp.0, "4:3 is the narrower of the two");
    }

    /// A minimised window reports zero, and a zero-area viewport is a wgpu
    /// validation error rather than a blank frame.
    #[test]
    fn a_degenerate_window_still_gives_a_drawable_rectangle() {
        for target in [(0, 0), (1, 4000), (4000, 1)] {
            for aspect in Aspect::ALL {
                let rect = viewport(target, aspect);
                assert!(
                    rect.2 >= 1.0 && rect.3 >= 1.0,
                    "{target:?} {aspect}: {rect:?}"
                );
                assert!(rect.0.is_finite() && rect.1.is_finite());
            }
        }
    }

    /// The rectangle has to stay inside the surface, or `set_viewport` fails
    /// validation - which is the one way this can be wrong without looking
    /// wrong in a screenshot.
    #[test]
    fn the_rectangle_never_leaves_the_surface() {
        for target in [(1920, 1080), (800, 600), (3440, 1440), (7, 5000)] {
            for aspect in Aspect::ALL {
                let (x, y, w, h) = viewport(target, aspect);
                assert!(x >= 0.0 && y >= 0.0, "{target:?} {aspect}");
                assert!(
                    x + w <= target.0.max(1) as f32 + 1e-3,
                    "{target:?} {aspect}: {x} + {w}"
                );
                assert!(
                    y + h <= target.1.max(1) as f32 + 1e-3,
                    "{target:?} {aspect}: {y} + {h}"
                );
            }
        }
    }

    #[test]
    fn a_size_round_trips_through_its_own_spelling() {
        let size: Size = "1920x1080".parse().expect("parse");
        assert_eq!(size, Size::new(1920, 1080));
        assert_eq!(size.to_string(), "1920x1080");
        assert_eq!("2560X1440".parse::<Size>().expect("parse").width, 2560);
    }

    #[test]
    fn a_malformed_size_says_what_it_wanted() {
        for text in ["1920", "axb", "1920x", "0x0", ""] {
            let error = text.parse::<Size>().expect_err(text);
            assert!(!error.is_empty(), "{text}");
        }
    }

    #[test]
    fn every_offered_size_is_the_psp_shape() {
        let wanted = Aspect::Psp.ratio().expect("psp has a ratio");
        for size in Size::OFFERED {
            let have = size.width as f32 / size.height as f32;
            assert!(
                (have - wanted).abs() < 0.02,
                "{size} is {have}, wanted about {wanted}"
            );
        }
    }

    #[test]
    fn a_scale_round_trips_and_refuses_what_it_cannot_draw() {
        assert_eq!("100".parse::<Scale>(), Ok(Scale::FULL));
        assert_eq!("50%".parse::<Scale>().expect("parse").factor(), 0.5);
        assert_eq!(Scale::FULL.to_string(), "100");
        for bad in ["0", "24", "201", "1000", "half", ""] {
            assert!(bad.parse::<Scale>().is_err(), "{bad}");
        }
    }

    #[test]
    fn every_offered_scale_is_one_this_build_accepts() {
        for scale in Scale::OFFERED {
            assert_eq!(scale.to_string().parse::<Scale>(), Ok(scale));
        }
        assert!(
            Scale::OFFERED.contains(&Scale::FULL),
            "100% must be offered"
        );
    }

    #[test]
    fn modes_and_aspects_round_trip_through_their_names() {
        for mode in WindowMode::ALL {
            assert_eq!(mode.name().parse::<WindowMode>(), Ok(mode));
        }
        for aspect in Aspect::ALL {
            assert_eq!(aspect.name().parse::<Aspect>(), Ok(aspect));
        }
        assert!("exclusive".parse::<WindowMode>().is_err());
        assert!("16:9".parse::<Aspect>().is_err());
    }
}
