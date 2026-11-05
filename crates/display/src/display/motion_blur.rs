//! [`MotionBlur`]: how hard the finished frame is smeared along the camera's
//! motion, if at all.
//!
//! Its own file rather than a block in `display.rs` for the 1,000-line rule
//! in `scripts/check-file-size.py`, the same reason `anti_aliasing.rs` is.

use serde::{Deserialize, Serialize};

/// How hard the finished frame is smeared along the camera's motion, if at
/// all.
///
/// **Strength, not technique** - the shape
/// [`docs/rendering/motion-blur.md`](../../../../docs/rendering/motion-blur.md)
/// settled before anything was built: each live tier is a shutter fraction of
/// one simulation tick, and the technique underneath is free to improve
/// without a settings migration. It already has, once: camera reprojection
/// shipped first
/// ([ADR-0028](../../../../docs/architecture/adr/0028-camera-motion-blur-first.md))
/// and the design's per-object velocity buffer replaced it under this same
/// row ([ADR-0030](../../../../docs/architecture/adr/0030-velocity-buffer-motion-blur.md)).
/// Every draw now writes its measured screen motion, so a rival holding
/// station stays sharp because it *is* sharp, not because a mask says so.
///
/// An enhancement of this project's, not a recovery: neither PSP build
/// renders motion blur, so `Off` is the default and the comparison setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum MotionBlur {
    /// No blur, and the frame never even reaches the pass.
    #[default]
    Off,
    /// A quarter of the tick's travel - a 90-degree shutter.
    Low,
    /// Half the tick's travel - the 180-degree shutter film runs at, and
    /// most renderers default to.
    Medium,
    /// Three quarters of the tick's travel: past what a physical shutter
    /// gives, for whoever wants the smear to read as a speed effect.
    High,
}

impl MotionBlur {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Medium, Self::High];

    /// The shutter fraction: how much of one tick's camera travel the smear
    /// spans. Zero is off.
    #[must_use]
    pub fn shutter(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Low => 0.25,
            Self::Medium => 0.5,
            Self::High => 0.75,
        }
    }
}

impl std::str::FromStr for MotionBlur {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a motion blur strength; try off, low, medium or high")
            })
    }
}

impl std::fmt::Display for MotionBlur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for MotionBlur {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<MotionBlur> for String {
    fn from(mode: MotionBlur) -> Self {
        mode.to_string()
    }
}

/// At what resolution the blur gathers its smear.
///
/// **A cost against a look, so the player's to make.** `Full` gathers every
/// pixel; `Half` gathers a quarter of them and blends the smear back over the
/// full-size frame by depth - about a third of the chain's cost on an
/// integrated GPU, at the price of a grainier smear. Full is the default: a
/// GPU with the headroom should not pay in picture for a saving it does not
/// need. `Quarter` gathers a sixteenth of them, only where the extent's
/// ceiling gathers at least [`QUARTER_MIN_LINES`] lines, and draws at no more
/// than `low` strength: see [`Self::for_lines`] and [`Self::shutter_cap`]. See
/// `docs/rendering/motion-blur.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum BlurResolution {
    /// Every pixel gathered.
    #[default]
    Full,
    /// A quarter of the pixels gathered, blended back at full size.
    Half,
    /// A sixteenth of the pixels gathered, blended back at full size.
    Quarter,
}

/// The fewest lines a quarter gather may have: 3840x2160 gathers 960x540,
/// which is what half gathers at 1080p, the coarsest gather a player had been
/// shown when this was chosen. Judged on the extent's ceiling: under it the
/// gather stays at half, and a dynamic resolution falling beneath it keeps
/// quarter, which sheds cost before pixels.
pub const QUARTER_MIN_LINES: u32 = 540;

impl BlurResolution {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Half => "half",
            Self::Quarter => "quarter",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 3] = [Self::Full, Self::Half, Self::Quarter];

    /// What this choice gathers at under a render extent ceiling `lines`
    /// high: `Quarter` is held at `Half` where it would fall under
    /// [`QUARTER_MIN_LINES`].
    ///
    /// The render extent's ceiling and not the window, because an upscaler's
    /// lines are the ones the gather divides; and not the extent now, or a
    /// fall would draw half again and the blur would get dearer as the extent
    /// falls.
    #[must_use]
    pub fn for_lines(self, lines: u32) -> Self {
        if self == Self::Quarter && lines / 4 < QUARTER_MIN_LINES {
            Self::Half
        } else {
            self
        }
    }

    /// The most shutter this gather draws with, whatever strength the player
    /// chose: `low` at `Quarter`, where a longer smear bleeds across a hull's
    /// silhouette. A cap and never a floor: a lower strength stays lower.
    #[must_use]
    pub fn shutter_cap(self) -> f32 {
        match self {
            Self::Full | Self::Half => f32::INFINITY,
            Self::Quarter => MotionBlur::Low.shutter(),
        }
    }
}

impl std::str::FromStr for BlurResolution {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a motion blur resolution; try full, half or quarter")
            })
    }
}

impl std::fmt::Display for BlurResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for BlurResolution {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<BlurResolution> for String {
    fn from(mode: BlurResolution) -> Self {
        mode.to_string()
    }
}
