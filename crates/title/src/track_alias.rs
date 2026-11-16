//! The key a title's event cards name a circuit by, where the executable keeps
//! a table of its own for it: [ADR-0058]'s per-title behaviour as data.
//!
//! Wipeout 2048 titles an HD-lineage circuit's event card not by the circuit's
//! own `PI_Track` name but by the string table entry of an HD track id
//! (`NN_Track`) that a static alias table in the executable pairs with it, one
//! row per direction (`docs/ghidra/functions/vita-2048-eu-v104/circuit-name-key.md`,
//! "The card's key: the track alias table"). A title with no such table lists
//! no rows and its cards name a circuit the way its pickers do.
//!
//! [ADR-0058]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md

/// One row of a title's track alias table: a circuit's id, which direction it
/// is, and the string table key its event card is titled by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackAlias {
    /// The circuit's `PI_Track` id, spelled as the table spells it.
    pub name: &'static str,
    /// Whether the row is the circuit's reverse. Not read by the lookup (the
    /// `_R` id carries its own row); kept because the table authors it.
    pub reversed: bool,
    /// The string table key, in the plugin's own spelling (`01_Track`); the
    /// table's entries are cased differently, so a caller folds the case.
    pub key: &'static str,
}

/// The `key` of the row whose `name` or `key` is `id`, compared byte for byte.
///
/// Matching either spelling is the original's own rule: it makes the map
/// idempotent on a key. `None` when no row matches, which is where the
/// original falls to `99_Track`; a caller that has no such row to answer for a
/// circuit keeps the circuit's ordinary name instead.
#[must_use]
pub fn key_for<'a>(rows: &'a [TrackAlias], id: &str) -> Option<&'a str> {
    rows.iter()
        .find(|row| row.name == id || row.key == id)
        .map(|row| row.key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: &[TrackAlias] = &[
        TrackAlias {
            name: "Vineta_K",
            reversed: false,
            key: "01_Track",
        },
        TrackAlias {
            name: "Vineta_K_R",
            reversed: true,
            key: "09_Track",
        },
    ];

    #[test]
    fn a_row_is_found_by_name_or_by_key_and_only_byte_for_byte() {
        assert_eq!(key_for(ROWS, "Vineta_K"), Some("01_Track"));
        assert_eq!(key_for(ROWS, "Vineta_K_R"), Some("09_Track"));
        assert_eq!(key_for(ROWS, "01_Track"), Some("01_Track"));
        assert_eq!(key_for(ROWS, "vineta_k"), None);
        assert_eq!(key_for(&[], "Vineta_K"), None);
    }
}
