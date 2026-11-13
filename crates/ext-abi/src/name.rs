//! The name of an asset of the sending mod's own: a key of its manifest's `[icons]`,
//! `[shaders]`, `[effects]` or `[sounds]`. A command names the asset it means, never its
//! place in the table, so a file added to the manifest cannot renumber the others.

use crate::HudStr;

/// A key of one of the mod's own manifest tables, at most `MAX_HUD_TEXT` bytes. Which
/// table it is a key of is the command that carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModName(HudStr);

impl ModName {
    /// The name `name`, if it fits.
    #[must_use]
    pub fn new(name: &str) -> Option<Self> {
        HudStr::new(name).map(Self)
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) const fn from_string(name: HudStr) -> Self {
        Self(name)
    }
}
