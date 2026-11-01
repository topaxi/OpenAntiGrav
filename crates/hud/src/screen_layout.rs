//! What every screen-layout file shares: the schema kind, the `screen` it names, the
//! anchor vocabulary and the two-number pair.
//!
//! A screen layout is a TOML file of this project's (`schemas/v1/screen-layout.tosd`)
//! read through the asset layers, so a player's override beats it and the disc's
//! authored one, where there is one, sits between. One file is one screen: the
//! top-level `screen` key says which. The screens so far are the split-screen HUD
//! ([`crate::split_hud`], which places a title's own widgets by name) and a loading
//! screen ([`crate::loading_layout`], which places the primitives it is built from).
//! `docs/formats/screen-layout.md` is the format.

use serde::Deserialize;

/// The schema kind a screen-layout file declares in its `[toml-schema]` header.
pub const SCHEMA_KIND: &str = "screen-layout";
/// The one schema major this build reads.
pub const SCHEMA_MAJOR: u32 = 1;

/// Which screen a file lays out: its top-level `screen`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Screen {
    /// The split-screen HUD: groups of the title's own widgets in a view.
    SplitHud,
    /// A race-load screen: placed images, text, dots and a clip.
    Loading,
}

impl Screen {
    /// The name the file spells it with.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SplitHud => "split-hud",
            Self::Loading => "loading",
        }
    }
}

/// A point of a rectangle: where a group is pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Anchor {
    /// Top left corner.
    TopLeft,
    /// Middle of the top edge.
    Top,
    /// Top right corner.
    TopRight,
    /// Middle of the left edge.
    Left,
    /// Centre.
    Centre,
    /// Middle of the right edge.
    Right,
    /// Bottom left corner.
    BottomLeft,
    /// Middle of the bottom edge.
    Bottom,
    /// Bottom right corner.
    BottomRight,
}

impl Anchor {
    /// The point as fractions of a rectangle's width and height.
    #[must_use]
    pub const fn fraction(self) -> [f32; 2] {
        match self {
            Self::TopLeft => [0.0, 0.0],
            Self::Top => [0.5, 0.0],
            Self::TopRight => [1.0, 0.0],
            Self::Left => [0.0, 0.5],
            Self::Centre => [0.5, 0.5],
            Self::Right => [1.0, 0.5],
            Self::BottomLeft => [0.0, 1.0],
            Self::Bottom => [0.5, 1.0],
            Self::BottomRight => [1.0, 1.0],
        }
    }
}

/// Exactly two numbers: a three-element array deserialises into `[f32; 2]` without
/// complaint, which the schema rightly refuses.
pub(crate) fn pair<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<[f32; 2]>, D::Error> {
    match Option::<Vec<f32>>::deserialize(deserializer)? {
        None => Ok(None),
        Some(numbers) => match numbers.as_slice() {
            &[a, b] => Ok(Some([a, b])),
            other => Err(serde::de::Error::invalid_length(
                other.len(),
                &"two numbers",
            )),
        },
    }
}
