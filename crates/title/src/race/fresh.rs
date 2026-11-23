//! The model a profile that has never picked one opens Ship Select on.

use crate::effects::Origin;

/// A title's fresh-profile model, with where it came from. See
/// [`super::RaceDefaults::fresh_variant`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshVariant {
    /// The variant suffix, as the title's own team directories spell it.
    pub variant: &'static str,
    /// Where it came from.
    pub origin: Origin,
}

/// A title's fresh-profile team: the row its team list rests on while the
/// player has never picked one, with where it came from. See
/// [`super::RaceDefaults::fresh_team`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshTeam {
    /// The team id, as the title's own plugin definition spells it.
    pub team: &'static str,
    /// Where it came from.
    pub origin: Origin,
}
