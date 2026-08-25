//! [`AntiAliasing`]: which anti-aliasing the scene draws with, if any.
//!
//! Split out of `display.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests
//! stay in `display/tests.rs` with the rest of the module's.

use serde::{Deserialize, Serialize};

/// Which anti-aliasing the scene draws with, if any.
///
/// **Not one dial.** MSAA, a spatial post-process pass and a temporal one act
/// at three different points in the frame and cost, look and compose
/// differently - see
/// [ADR-0013](../../../../docs/architecture/adr/0013-anti-aliasing-architecture.md).
/// This enum holds the two classes that are actually built: rasterization
/// (`Msaa4x`) and spatial post-process (`Fxaa`, `Smaa`). There is no `Taa`
/// variant - temporal reconstruction needs motion vectors, jitter and a
/// history buffer that do not exist yet, and the ADR is explicit that a row
/// for infrastructure that is not there is worse than no row. There is no
/// `Msaa2x` either: sample count 2 is only available behind
/// `wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`, which nothing
/// in this codebase requests, so a pipeline built at `sample_count: 2` fails
/// device validation on every adapter this game runs on, not just some - see
/// the ADR's Consequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum AntiAliasing {
    /// No anti-aliasing beyond whatever the render scale and the upscaler
    /// already do.
    #[default]
    Off,
    /// Fast Approximate Anti-Aliasing: one lightweight fullscreen pass that
    /// blurs along detected edges. Cheapest of the three, softens the image
    /// the most. See [`oag_render::post::fxaa`].
    Fxaa,
    /// Subpixel Morphological Anti-Aliasing: edge detection, then blending
    /// weights from a precomputed area/search lookup, then a neighbourhood
    /// blend. Costs three fullscreen passes against FXAA's one, and keeps
    /// detail FXAA would soften away. See [`oag_render::post::smaa`].
    Smaa,
    /// Multisample anti-aliasing at 4 samples per pixel: the rasterizer
    /// itself supersamples triangle edges, resolved to a single sample before
    /// anything downstream sees it.
    Msaa4x,
}

impl AntiAliasing {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Fxaa => "fxaa",
            Self::Smaa => "smaa",
            Self::Msaa4x => "msaa4x",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 4] = [Self::Off, Self::Fxaa, Self::Smaa, Self::Msaa4x];

    /// How many samples the rasterizer runs at: 1 for everything but `Msaa4x`.
    ///
    /// Baked into every scene pipeline at the moment it is built - see
    /// `race::Scene::new` - so this is read once, when a race starts, rather
    /// than every frame the way [`Upscaler`] is.
    #[must_use]
    pub fn msaa_samples(self) -> u32 {
        match self {
            Self::Msaa4x => 4,
            Self::Off | Self::Fxaa | Self::Smaa => 1,
        }
    }

    /// Whether this is a spatial post-process pass - `Fxaa` or `Smaa` - the
    /// class that reads the resolved scene once and can fight an
    /// edge-adaptive upscaler reading the same pixels. See the ADR.
    #[must_use]
    pub fn is_spatial_post_process(self) -> bool {
        matches!(self, Self::Fxaa | Self::Smaa)
    }
}

impl std::str::FromStr for AntiAliasing {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not an anti-aliasing mode; try off, fxaa, smaa or msaa4x")
            })
    }
}

impl std::fmt::Display for AntiAliasing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for AntiAliasing {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<AntiAliasing> for String {
    fn from(mode: AntiAliasing) -> Self {
        mode.to_string()
    }
}
