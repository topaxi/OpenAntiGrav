//! [`Shadows`]: what casts a shadow, and what draws it.
//!
//! Its own file rather than a block in `display.rs` for the 1,000-line rule
//! in `scripts/check-file-size.py`, the same reason `motion_blur.rs` and
//! `reconstruction.rs` are.

use serde::{Deserialize, Serialize};

/// What casts a shadow, and what draws it.
///
/// **Variants named for the technique**, following
/// [`super::Reconstruction`] and deliberately not [`super::MotionBlur`]:
/// [`docs/rendering/shadows.md`](../../../../docs/rendering/shadows.md)
/// settled that shape before anything was built, because the ladder is *not*
/// monotonic in quality. A flat authored hull projected onto the road can
/// read worse than a soft circle on a track whose floor curves under the
/// craft, so a Low/Medium/High dial would be claiming an ordering that does
/// not exist. Same argument as
/// [ADR-0013](../../../../docs/architecture/adr/0013-anti-aliasing-architecture.md)'s
/// "not one dial", and worse here: a blob quad, a projected occluder hull and
/// a shadow map act at different points in the frame and compose differently.
///
/// **Two of the design's four tiers are missing on purpose.** `original` -
/// Pulse's 129 `Dynamic Shadow Occluder` hulls, HD's shadow-map jobs, 2048's
/// track-proximity pair - and `mapped`, this project's own cascaded shadow
/// map, are both designed and neither is built. They are not offered until
/// they exist, per ADR-0013's own rule that a row for infrastructure that is
/// not there is worse than no row. The enum grows when they land; a settings
/// file written today keeps meaning what it says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Shadows {
    /// Nothing casts, and no shadow pass runs at all.
    ///
    /// **The default, and it stays the default until `original` exists.**
    /// Every title on the discs authors its own shadow mechanism (or, for
    /// Pure, authors none and says so), so a generated falloff as the default
    /// would be standing an invention in front of data this project already
    /// has - see `docs/rendering/shadows.md`. It is also the comparison
    /// setting, the same role [`super::MotionBlur::Off`] plays.
    #[default]
    Off,
    /// A quad under each craft, laid on the surface below it.
    ///
    /// The shape comes from the disc where the disc has one: Wipeout HD ships
    /// nine `ambient_shadow.gtf`, one per team, decoded already
    /// ([`gtf.md`](../../../../docs/formats/gtf.md)). Only where a title has
    /// no such asset is a radial falloff generated instead, which is the
    /// narrow case `CLAUDE.md`'s never-invent rule allows - a substitute for
    /// the missing asset alone, never an override of data we do have.
    ///
    /// **Not a reimplementation of anything.** No title in the lineage draws a
    /// blob: `blob` `0x3e0` and `textureBlob` `0x3df` are authored zero times
    /// across all 415 `.vex` files on the Pulse disc. It is offered as the
    /// cheap tier a weaker machine can afford, on the same footing as
    /// [`super::MotionBlur`] and FSR 1.
    Blob,
}

impl Shadows {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Blob => "blob",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Off, Self::Blob];

    /// Whether anything is drawn at all - the one question the frame asks
    /// before building a shadow pass.
    ///
    /// A method rather than a `!= Off` at each call site so that the tiers
    /// still to come (`original`, `mapped`) join it by changing this file
    /// instead of every caller.
    #[must_use]
    pub fn draws(self) -> bool {
        !matches!(self, Self::Off)
    }
}

impl std::str::FromStr for Shadows {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a shadow setting; try off or blob"))
    }
}

impl std::fmt::Display for Shadows {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for Shadows {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Shadows> for String {
    fn from(mode: Shadows) -> Self {
        mode.to_string()
    }
}
