//! [`Lod`]: whether an authored `LodGroup`'s children are all drawn.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// Whether [`super::build_with_textures`] draws every child of an authored
/// `LodGroup` (class `0x2ee`), or only the first.
///
/// **This is not a quality tier and it does not switch by distance.** The
/// original PSP binary never does either: it registers the class but never
/// reads its `child_count` or switch-distance fields, and its generic tree
/// walker draws every child of every node unconditionally, always, with no
/// live re-evaluation against the camera - see `docs/formats/vex.md`,
/// "`LodGroup`: authored, but never switched at runtime". Ten of the eleven
/// `LodGroup` instances on `16_Track` carry two children with real,
/// differently-detailed mesh geometry in both, so [`Self::Both`] (matching
/// the original) means genuinely overlapping duplicate geometry, not a
/// "higher quality" picture. [`Self::Single`] is a one-time choice made when
/// the model is built, not a live switch: it keeps the higher-detail tier and
/// permanently discards the other, removing that duplication at the cost of
/// no longer matching the original.
///
/// A real distance-based switch (the original's authored switch-distance
/// value, re-checked against the camera every frame) would need `Model` to
/// carry per-node bounds and the render loop to re-evaluate them each frame -
/// the same live-camera mechanism the roadmap's unimplemented frustum-culling
/// entry needs, and not yet built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lod {
    /// Draw every child of every `LodGroup`, exactly as the original does -
    /// including the duplicate geometry that results.
    #[default]
    Both,
    /// Draw only the first child of a two-child `LodGroup` - the
    /// higher-triangle-count tier in all ten measured cases - and skip the
    /// rest. A permanent, load-time choice, not a live switch. Removes real
    /// duplicate geometry the original always draws twice, at the cost of no
    /// longer matching it.
    Single,
}

impl Lod {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Single => "single",
        }
    }

    /// Every mode, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Both, Self::Single];
}

impl std::str::FromStr for Lod {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|lod| lod.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a level-of-detail mode; try both or single"))
    }
}

impl std::fmt::Display for Lod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
