//! The shape the game is drawn at inside its window.
//!
//! Split out of `display.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, in the change that gave [`Aspect`] a
//! free-form `w:h` shape - the same seam `msaa`, `shadows`, `motion_blur` and
//! `reconstruction` already sit on.

use serde::{Deserialize, Serialize};

/// The shape the game is drawn at inside its window.
///
/// **These name shapes, not machines**, and the reason is a measurement: the
/// Vita's 960x544 is *exactly* the PSP's 480x272 reduced - both are 30:17 - so
/// a `Vita` variant would be a second spelling of [`Aspect::Psp`] returning an
/// identical `f32`. 720p and 1080p are likewise one shape, [`Aspect::Wide`],
/// and it is 0.7% wider than the PSP's rather than the same. Naming targets
/// instead of shapes would have produced variants that compare equal by ratio
/// and unequal by name, which is the bug this ordering avoids.
///
/// [`Aspect::Ratio`] is the escape hatch for everything unnamed. It is not
/// offered on a menu row - there is no finite list to offer - but a settings
/// file or `--aspect` may spell any shape as `w:h`, which is what lets a target
/// nobody has thought of yet work without an enum variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Aspect {
    /// 30:17 - the PSP's 480x272, the Vita's 960x544, and the shape every
    /// camera value on the disc was authored for.
    ///
    /// The default, and not merely out of deference: `<ExternalCameraFar>`'s
    /// field of view is only defined at this ratio, so it is the one shape where
    /// what a player sees is what the original framed. See
    /// `oag_raceplay::AUTHORED_ASPECT`.
    #[default]
    Psp,
    /// 4:3, the PS2's television - not the shape its own artwork wants.
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
    /// 16:9 - 720p, 1080p, and Wipeout HD's own 1920x1080.
    ///
    /// 0.7% wider than [`Aspect::Psp`], which is small enough to look like a
    /// rounding error and large enough to move a HUD element off a rounded
    /// corner, so it is its own shape rather than folded into the PSP's.
    Wide,
    /// Any shape at all, spelled `w:h`.
    ///
    /// For a target none of the above names. Neither side may be zero; see
    /// [`FromStr`](std::str::FromStr).
    Ratio(u32, u32),
}

impl Aspect {
    /// The ratio this shape asks for, or `None` for [`Aspect::Free`], which asks
    /// for whatever it is given.
    #[must_use]
    pub fn ratio(self) -> Option<f32> {
        match self {
            Self::Psp => Some(crate::space::SCREEN.0 / crate::space::SCREEN.1),
            Self::Ps2 => Some(4.0 / 3.0),
            Self::Wide => Some(16.0 / 9.0),
            Self::Ratio(w, h) => Some(w as f32 / h as f32),
            Self::Free => None,
        }
    }

    /// The spelling used in a settings file and on a menu row.
    ///
    /// [`Aspect::Ratio`] has no `&'static str` to return, so this is a
    /// `String`; every other variant is one of [`Aspect::ALL`]'s fixed
    /// spellings and allocates a short one.
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::Psp => "psp".to_owned(),
            Self::Ps2 => "ps2".to_owned(),
            Self::Wide => "wide".to_owned(),
            Self::Free => "free".to_owned(),
            Self::Ratio(w, h) => format!("{w}:{h}"),
        }
    }

    /// Every *named* shape, for the menus and for error messages.
    ///
    /// [`Aspect::Ratio`] is deliberately absent: it is not a choice a menu row
    /// can offer, because there is no finite list of them.
    pub const ALL: [Self; 4] = [Self::Psp, Self::Ps2, Self::Wide, Self::Free];
}

impl std::str::FromStr for Aspect {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if let Some(aspect) = Self::ALL
            .into_iter()
            .find(|aspect| aspect.name().eq_ignore_ascii_case(text))
        {
            return Ok(aspect);
        }

        // Anything unnamed is `w:h`. Both sides must be non-zero: a zero
        // denominator is an infinite ratio and a zero numerator a flat one, and
        // neither is a picture. Rejecting them here means `ratio()` never has
        // to return a `NaN` or an `inf` a viewport would then divide by.
        if let Some((w, h)) = text.split_once(':')
            && let (Ok(w), Ok(h)) = (w.trim().parse::<u32>(), h.trim().parse::<u32>())
            && w > 0
            && h > 0
        {
            return Ok(Self::Ratio(w, h));
        }

        Err(format!(
            "{text:?} is not an aspect ratio; try psp, ps2, wide, free, or a \
             shape like 21:9"
        ))
    }
}

impl Serialize for Aspect {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.name())
    }
}

impl<'de> Deserialize<'de> for Aspect {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for Aspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}
