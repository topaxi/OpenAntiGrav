//! [`Reconstruction`]: what resolves the frame onto the surface.
//!
//! Its own file rather than a block in `display.rs` for the 1,000-line rule in
//! `scripts/check-file-size.py`, the same reason `motion_blur.rs` is. It
//! replaces `anti_aliasing.rs` and `upscaler.rs`, which held one axis cut in
//! the wrong place twice - see
//! [ADR-0041](../../../../docs/architecture/adr/0041-one-row-for-what-resolves-the-frame.md).

use serde::{Deserialize, Serialize};

/// What resolves the frame onto the surface.
///
/// **One axis, because every value answers one question.** A spatial
/// post-process pass, a spatial upscaler and a temporal reconstruction all
/// read the scene target and decide what reaches the presentation target, so
/// at most one of them can be doing it. Holding them as one enum makes every
/// wrong pairing unrepresentable rather than something a menu warning has to
/// describe after the fact - five of `menu.toml`'s seven warnings existed only
/// to say "these two rows disagree".
///
/// **MSAA is deliberately not here.** It is a rasterizer sample count baked
/// into every scene pipeline at `race::Scene::new`, not a choice about what
/// reads the resolved result, and it composes with [`Self::Fsr1`] and
/// [`Self::Off`] rather than competing with them. See [`super::Msaa`], and
/// [ADR-0013](../../../../docs/architecture/adr/0013-anti-aliasing-architecture.md)
/// for how the three classes cost, look and compose - that analysis is
/// unchanged, only the row shape moved.
///
/// There is no `Taa` variant. Temporal reconstruction is [`Self::Fsr3`], which
/// is exactly what ADR-0013 asked for: *"do not expose `Taa` and `Fsr3` as
/// separate user-facing concepts unless a real reason turns up to"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Reconstruction {
    /// The blit's own single bilinear tap, and no anti-aliasing beyond
    /// whatever the render scale already does.
    ///
    /// Spelled `off` rather than `bilinear` because "off" is what a player
    /// choosing nothing means. There is no third state hiding behind the name -
    /// something has to resample a frame that is not the size of the rectangle
    /// it goes into, and the blit's sampler is bilinear - so `off` and
    /// "bilinear" are one option, not two.
    ///
    /// **The default**, settled 2026-09-02; the account of what settled it is
    /// in `docs/overview/modern-features.md`.
    #[default]
    Off,
    /// Fast Approximate Anti-Aliasing: one lightweight fullscreen pass that
    /// blurs along detected edges. Cheapest of the lot, softens the image the
    /// most. See [`oag_render::post::fxaa`].
    ///
    /// **Only ever right when nothing else is reconstructing**, which is why
    /// it is a value here rather than a toggle beside this row. Before
    /// [`Self::Fsr3`] it destroys the sub-pixel detail accumulation exists to
    /// gather; before [`Self::Fsr1`] it blurs the edges the edge-adaptive
    /// resample reasons about. ADR-0041 records both cases.
    Fxaa,
    /// Subpixel Morphological Anti-Aliasing: edge detection, then blending
    /// weights from a precomputed area/search lookup, then a neighbourhood
    /// blend. Three fullscreen passes against FXAA's one, and keeps detail
    /// FXAA would soften away. See [`oag_render::post::smaa`].
    Smaa,
    /// AMD FidelityFX Super Resolution 1: EASU, then RCAS.
    ///
    /// Spatial, so it costs two fullscreen passes and needs nothing from the
    /// renderer - no motion vectors, no jitter, no history. **A magnifier**:
    /// `upscale::magnifies` declines to run it where the frame is not drawn
    /// smaller than it is presented, because asked to minify its taps
    /// undersample. See [`oag_render::post::fsr1`].
    Fsr1,
    /// AMD FidelityFX Super Resolution 3.1: the temporal upscaler, and this
    /// row's anti-aliaser.
    ///
    /// Reconstructs each frame from several previous ones, so unlike
    /// [`Self::Fsr1`] it has something to do at every render scale - a 100 %
    /// frame still gets more samples per pixel than one frame carries, which
    /// is what upstream calls its native-AA mode. It is also the only choice
    /// here that reaches back into the renderer: depth, per-object motion
    /// vectors and camera jitter are all inputs, and the jitter is turned on
    /// *by this setting* rather than independently. See
    /// [`oag_render::post::fsr3`] and [fsr3.md](../../../../docs/rendering/fsr3.md).
    ///
    /// **Greys the MSAA row**, per ADR-0041: it anti-aliases the same frame,
    /// and MSAA on top costs a rasterization pass and a resolve for a
    /// marginally cleaner colour input.
    ///
    /// **Selecting it is a request, not a guarantee.** An adapter with no
    /// compute shaders falls back to [`Self::Fsr1`] and logs it, per
    /// [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)'s
    /// requirement that a missing capability degrade rather than fail to boot.
    /// The setting keeps saying `fsr3` either way, so the same settings file
    /// does the right thing on a machine that can run it.
    Fsr3,
}

impl Reconstruction {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Fxaa => "fxaa",
            Self::Smaa => "smaa",
            Self::Fsr1 => "fsr1",
            Self::Fsr3 => "fsr3",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 5] = [Self::Off, Self::Fxaa, Self::Smaa, Self::Fsr1, Self::Fsr3];

    /// Whether this reconstructs from previous frames, and therefore wants the
    /// camera jittered and the history kept.
    ///
    /// The one question the rest of the game asks about the choice - see
    /// `crate::upscale::jitter_phases`. A spatial resampler must *not* be
    /// handed a jittered frame: it has no history to resolve the offset
    /// against, so the offset is just a wobble
    /// ([ADR-0039](../../../../docs/architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)).
    #[must_use]
    pub fn is_temporal(self) -> bool {
        matches!(self, Self::Fsr3)
    }

    /// Whether this is a spatial post-process pass - `Fxaa` or `Smaa` - the
    /// class that reads the resolved scene once and produces no resampling of
    /// its own. See ADR-0013.
    #[must_use]
    pub fn is_spatial_post_process(self) -> bool {
        matches!(self, Self::Fxaa | Self::Smaa)
    }

    /// Whether this resamples at all, rather than leaving the blit to do it.
    ///
    /// `Fxaa` and `Smaa` are anti-aliasing and nothing else: a frame drawn
    /// smaller than its rectangle still reaches the surface through the blit's
    /// bilinear tap when either is selected.
    #[must_use]
    pub fn is_upscaler(self) -> bool {
        matches!(self, Self::Fsr1 | Self::Fsr3)
    }
}

impl std::str::FromStr for Reconstruction {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        // `bilinear` was `off`'s name for two commits and is in settings files
        // already written; it means the same thing and is accepted silently,
        // with the canonical rewrite normalising it on the next run.
        if text.eq_ignore_ascii_case("bilinear") {
            return Ok(Self::Off);
        }
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a reconstruction; try off, fxaa, smaa, fsr1 or fsr3")
            })
    }
}

impl std::fmt::Display for Reconstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for Reconstruction {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Reconstruction> for String {
    fn from(mode: Reconstruction) -> Self {
        mode.to_string()
    }
}
