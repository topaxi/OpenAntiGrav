//! [`Msaa`]: how many samples the rasterizer takes.
//!
//! An axis of its own since
//! [ADR-0041](../../../../docs/architecture/adr/0041-one-row-for-what-resolves-the-frame.md),
//! where it used to be a value of the anti-aliasing enum competing with two
//! fullscreen passes it has nothing in common with.

use serde::{Deserialize, Serialize};

/// How many samples the rasterizer takes per pixel.
///
/// **Not on [`super::Reconstruction`]'s axis**, and the split is the whole
/// point of ADR-0041: this is read once, at `race::Scene::new`, and baked into
/// every scene pipeline, while a reconstruction is read fresh every frame and
/// decides what reads the *resolved* result. The two act at different points
/// in the frame and compose rather than compete - `menu.toml` records that
/// MSAA "resolves before any of this runs and composes with FSR 1 rather than
/// fighting it".
///
/// The one exception is `Reconstruction::Fsr3`, which anti-aliases the same
/// frame temporally; the MSAA row is greyed there rather than warned about.
/// See ADR-0041 for why greyed and not warned.
///
/// There is no `X2`: sample count 2 is only available behind
/// `wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`, which nothing
/// in this codebase requests, so a pipeline built at `sample_count: 2` fails
/// device validation on every adapter this game runs on, not just some - see
/// [ADR-0013](../../../../docs/architecture/adr/0013-anti-aliasing-architecture.md)'s
/// Consequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Msaa {
    /// One sample per pixel: the rasterizer supersamples nothing.
    #[default]
    Off,
    /// Four samples per pixel, resolved to one before anything downstream
    /// sees it.
    X4,
}

impl Msaa {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::X4 => "4x",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Off, Self::X4];

    /// How many samples the rasterizer runs at.
    ///
    /// Baked into every scene pipeline at the moment it is built - see
    /// `race::Scene::new` - so this is read once, when a race starts, rather
    /// than every frame the way [`super::Reconstruction`] is.
    #[must_use]
    pub fn samples(self) -> u32 {
        match self {
            Self::Off => 1,
            Self::X4 => 4,
        }
    }
}

impl std::str::FromStr for Msaa {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        // `msaa4x` is what the pre-ADR-0041 `anti_aliasing` row spelled this,
        // and a settings file written before the split can still carry it on
        // the migrated key. Accepted silently, normalised on the next write.
        if text.eq_ignore_ascii_case("msaa4x") {
            return Ok(Self::X4);
        }
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not an MSAA level; try off or 4x"))
    }
}

impl std::fmt::Display for Msaa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for Msaa {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Msaa> for String {
    fn from(mode: Msaa) -> Self {
        mode.to_string()
    }
}
