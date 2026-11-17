//! What a title's water setting has to offer: whether its own law differs from `off`, and the
//! surfaces this project's `enhanced` law is drawn on.
//!
//! Per title (ADR-0058). The WATER row is hidden where nothing it offers changes the picture,
//! and the sea is found by name on a title that ships no water program
//! (`docs/rendering/water.md`, "Enhanced"); this crate cannot name the setting type, so the
//! game maps these onto the row.

/// The surfaces `enhanced` draws water on in one circuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Circuit {
    /// The circuit's directory name in its `track.vex` entry (`03_Track`), compared
    /// case-insensitively against a path component.
    pub directory: &'static str,
    /// Embedded texture names, with or without the extension, of the batches that are the
    /// sea. Each is confirmed on a frame (`docs/rendering/water.md`, "Enhanced"); a name that
    /// is only a census match is not listed.
    pub textures: &'static [&'static str],
}

/// A title's water.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Water {
    /// Whether the title's own law (`original`) draws something `off` does not.
    pub original_differs: bool,
    /// The circuits `enhanced` is drawn on; empty where it is not offered.
    pub enhanced: &'static [Circuit],
}

impl Water {
    /// A title whose original draws the same as `off` and which offers no `enhanced`.
    pub const NONE: Self = Self {
        original_differs: false,
        enhanced: &[],
    };

    /// Whether `enhanced` is offered on this title.
    #[must_use]
    pub const fn offers_enhanced(self) -> bool {
        !self.enhanced.is_empty()
    }

    /// The water textures of the circuit `track` (a `track.vex` entry name, either path
    /// separator) belongs to, or `None` where the title lists none for it.
    #[must_use]
    pub fn textures_of(self, track: &str) -> Option<&'static [&'static str]> {
        self.enhanced
            .iter()
            .find(|circuit| {
                track
                    .split(['\\', '/'])
                    .any(|part| part.eq_ignore_ascii_case(circuit.directory))
            })
            .map(|circuit| circuit.textures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATER: Water = Water {
        original_differs: false,
        enhanced: &[Circuit {
            directory: "03_Track",
            textures: &["NewSeaTest"],
        }],
    };

    #[test]
    fn a_circuit_is_found_by_its_directory_in_either_separator() {
        assert_eq!(
            WATER.textures_of(r"Data\Environments\03_Track\track.vex"),
            Some(&["NewSeaTest"][..])
        );
        assert!(
            WATER
                .textures_of("Data/Environments/03_track/track.vex")
                .is_some()
        );
        assert_eq!(
            WATER.textures_of(r"Data\Environments\04_Track\track.vex"),
            None
        );
        assert_eq!(Water::NONE.textures_of("03_Track"), None);
    }

    #[test]
    fn enhanced_is_offered_only_where_a_circuit_is_listed() {
        assert!(WATER.offers_enhanced());
        assert!(!Water::NONE.offers_enhanced());
    }
}
