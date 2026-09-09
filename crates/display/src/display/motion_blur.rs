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
