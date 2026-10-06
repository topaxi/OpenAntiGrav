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
