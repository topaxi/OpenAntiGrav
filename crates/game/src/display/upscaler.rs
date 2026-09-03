//! [`Upscaler`]: which resampler carries the offscreen frame onto the surface.
//!
//! Its own file rather than a block in `display.rs` for the 1,000-line rule in
//! `scripts/check-file-size.py`, the same reason `anti_aliasing.rs` and
//! `motion_blur.rs` are. It moved out when FSR 3.1 was added and the file's
//! ratchet refused the growth, which is the rule working as intended.

use serde::{Deserialize, Serialize};

/// Which resampler carries the offscreen frame onto the surface.
///
/// This is the companion to [`Scale`], and only that pairing makes it mean
/// anything: at 100 % there is nothing to upscale and the choice is between a
/// blit and a sharpen. Below 100 % it is the whole point of the render-scale
/// row - how much of what the lower resolution threw away can be argued back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Upscaler {
    /// No upscaler: the blit's own single bilinear tap, which is what this
    /// always did.
    ///
    /// Spelled `off` rather than `bilinear` because the row is called UPSCALER
    /// and "off" is what a player choosing one means. There is no third state
    /// hiding behind the name - something has to resample a frame that is not
    /// the size of the rectangle it goes into, and the blit's sampler is
    /// bilinear - so `off` and "bilinear" are one option, not two.
    ///
    /// **The default, and settled on 2026-09-02 rather than still open** -
    /// the account of what settled it, and of what was play-tested to get
    /// there, is in `docs/overview/modern-features.md`. Note that
    /// [`Scale::default`] is [`Scale::FULL`] and FSR 1 only runs where it is
    /// magnifying, so on a default install this row is inert whichever way it
    /// points; it starts meaning something once a player lowers the render
    /// scale.
    #[default]
    Off,
    /// AMD FidelityFX Super Resolution 1: EASU, then RCAS.
    ///
    /// Spatial, so it costs two fullscreen passes and needs nothing from the
    /// renderer - no motion vectors, no jitter, no history. See
    /// [`oag_render::post::fsr1`].
    Fsr1,
    /// AMD FidelityFX Super Resolution 3.1: the temporal upscaler.
    ///
    /// Reconstructs each frame from several previous ones, so unlike
    /// [`Upscaler::Fsr1`] it has something to do at every render scale - a
    /// 100 % frame still gets more samples per pixel than one frame carries.
    /// It is also the only choice here that reaches back into the renderer:
    /// depth, per-object motion vectors and camera jitter are all inputs, and
    /// the jitter is turned on *by this setting* rather than independently.
    /// See [`oag_render::post::fsr3`] and
    /// [fsr3.md](../../../docs/rendering/fsr3.md).
    ///
    /// **Selecting it is a request, not a guarantee.** An adapter with no
    /// compute shaders falls back to FSR 1 and logs it, per
    /// [ADR-0012](../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)'s
    /// requirement that a missing capability degrade rather than fail to boot.
    /// The setting keeps saying `fsr3` either way, so the same settings file
    /// does the right thing on a machine that can run it.
    Fsr3,
}

impl Upscaler {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Fsr1 => "fsr1",
            Self::Fsr3 => "fsr3",
        }
    }

    /// Whether this upscaler reconstructs from previous frames, and therefore
    /// wants the camera jittered and the history kept.
    ///
    /// The one question the rest of the game asks about the choice - see
    /// `crate::upscale::jitter_phases`. A spatial resampler must *not* be
    /// handed a jittered frame: it has no history to resolve the offset
    /// against, so the offset is just a wobble
    /// ([ADR-0039](../../../docs/architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)).
    #[must_use]
    pub fn is_temporal(self) -> bool {
        matches!(self, Self::Fsr3)
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 3] = [Self::Off, Self::Fsr1, Self::Fsr3];
}

impl std::str::FromStr for Upscaler {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        // `bilinear` was this option's name for two commits and is in settings
        // files already written; it means the same thing and is accepted
        // silently, with the canonical rewrite normalising it on the next run.
        if text.eq_ignore_ascii_case("bilinear") {
            return Ok(Self::Off);
        }
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not an upscaler; try off, fsr1 or fsr3"))
    }
}

impl std::fmt::Display for Upscaler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for Upscaler {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Upscaler> for String {
    fn from(mode: Upscaler) -> Self {
        mode.to_string()
    }
}
