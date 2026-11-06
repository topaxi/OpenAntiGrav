//! What a title asks a player to call themselves: a name, and on some titles a short tag.
//!
//! One name is kept for every title; what differs is whether a title shows it at all and how
//! many characters its own entry allowed. Per-title behaviour is data here, not a comparison on
//! a title's name (ADR-0058). The table is `docs/formats/fexml.md`, "Which titles keep a name
//! and a tag".

use crate::pre_race::Sourced;

/// The name and tag entries of one title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    /// How many characters the title's own name entry holds, or `None` when the title keeps no
    /// name (2048 and Omega take the online ID of the console's account instead).
    pub name_length: Option<Sourced<u8>>,
    /// How many characters the title's tag entry holds, or `None` when the title has no tag.
    pub tag_length: Option<Sourced<u8>>,
}

impl Profile {
    /// A name and a tag, as Pulse and Pure author them: `length="10"` and `length="3"`
    /// (`docs/formats/fexml.md`, "Which titles keep a name and a tag"; read from executable
    /// strings and shipped XML, no decompile).
    pub const NAME_AND_TAG: Self = Self {
        name_length: Some(Sourced::measured(10)),
        tag_length: Some(Sourced::measured(3)),
    };

    /// A name and no tag, as HD/Fury's executable names only `Create Profile Setup->Name`.
    pub const NAME_ONLY: Self = Self {
        name_length: Some(Sourced::measured(10)),
        tag_length: None,
    };

    /// Neither: no `Create Profile` string in the executable.
    pub const NEITHER: Self = Self {
        name_length: None,
        tag_length: None,
    };

    /// Whether a first-run prompt asks this title's player for a name.
    #[must_use]
    pub const fn asks_for_name(&self) -> bool {
        self.name_length.is_some()
    }

    /// Whether this title uses a tag.
    #[must_use]
    pub const fn asks_for_tag(&self) -> bool {
        self.tag_length.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_name_and_tag_shape_asks_for_a_tag() {
        assert!(Profile::NAME_AND_TAG.asks_for_tag());
        assert!(!Profile::NAME_ONLY.asks_for_tag());
        assert!(!Profile::NEITHER.asks_for_tag());
    }

    #[test]
    fn a_title_with_no_name_asks_for_nothing() {
        assert!(!Profile::NEITHER.asks_for_name());
        assert!(Profile::NAME_ONLY.asks_for_name());
    }
}
