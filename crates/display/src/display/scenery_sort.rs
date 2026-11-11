//! [`ScenerySort`]: how the circuit's blended list (trees, translucent floor
//! patches, glass) decides which tree is in front of which.

use serde::{Deserialize, Serialize};

/// How the circuit's blended list (list B, the trees and the translucent floor
/// patches and glass a circuit authors) is put in front-to-back order.
///
/// **Only meaningful where the circuit is the PSP preset** (Pulse and Pure,
/// whose list-B meshes the original queues at a per-mesh key); HD/Fury, Omega
/// and 2048 keep their own order and offer no choice.
/// `[render_profiles.<title> (<platform>)] scenery_sort`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ScenerySort {
    /// The original's order, and the default: each list-B mesh is one queue key
    /// from its box centre's view depth (`Mesh_SubmitNode`) and the list draws
    /// far to near by it, with no depth written. A strip of trees is one mesh, so
    /// a tree in it can be drawn over a nearer one of another strip, and which
    /// one it is changes as the camera pans and the keys cross
    /// (`docs/rendering/draw-order.md`, "Trees overlapping trees on Platinum
    /// Rush White").
    #[default]
    Original,
    /// The original's order, then the foliage's own depth on top: a blended
    /// alpha-over mesh of the circuit lays down the depth of its solid texels
    /// (alpha at least one half) before the list draws, and the list tests
    /// against it, so a nearer tree is over a farther one at any camera angle.
    /// **Chosen, not measured**: the original writes no depth there. Translucent
    /// floor patches, glass fades and additive meshes stay as they were.
    Depth,
}

impl ScenerySort {
    /// The spelling used in a settings file, on a menu row and on `--scenery-sort`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Depth => "depth",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Original, Self::Depth];

    /// Whether foliage writes its own depth.
    #[must_use]
    pub fn writes_foliage_depth(self) -> bool {
        matches!(self, Self::Depth)
    }
}

impl std::str::FromStr for ScenerySort {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|sort| sort.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a scenery sort; try original or depth"))
    }
}

impl std::fmt::Display for ScenerySort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for ScenerySort {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<ScenerySort> for String {
    fn from(sort: ScenerySort) -> Self {
        sort.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_is_the_default_and_names_round_trip() {
        assert_eq!(ScenerySort::default(), ScenerySort::Original);
        for sort in ScenerySort::ALL {
            assert_eq!(sort.name().parse::<ScenerySort>(), Ok(sort));
        }
        assert!(ScenerySort::Depth.writes_foliage_depth());
        assert!(!ScenerySort::Original.writes_foliage_depth());
        assert!("nearest".parse::<ScenerySort>().is_err());
    }
}
