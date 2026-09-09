//! Where a title keeps the table its weapons are tuned by.
//!
//! An axis in the [ADR-0022] sense rather than a constant, because the two
//! answers differ in **shape** and not only in spelling: Pulse and Wipeout HD
//! ship two tables and pick between them by race mode, and Pure ships one that
//! every mode reads. A single `&str` could carry the spelling and could not
//! carry that.
//!
//! Recovered from each title's own executable. Pulse's two names are in
//! `oag_tables::weapons`' own constants, taken off `BOOT.BIN`'s string table;
//! Pure's is `Data\XML\weaponstats.xml` at `0x08a445a0` in
//! `/psp-pure-usa/BOOT.BIN`, three strings before the `"WeaponStats"` and
//! `"Weapon"` element names the parser matches - which is what says it is the
//! file this parser reads rather than some other table.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The tables a title tunes its weapons from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weapons {
    /// The table a race reads, and the only one on a title that ships one.
    pub race: &'static str,
    /// The table Eliminator reads instead, or `None` for a title with one table.
    ///
    /// **`None` is a measurement.** `WeaponStats_Elimination.xml` is how Pulse
    /// makes the same weapon hit differently in Eliminator - the global
    /// `DAT_08b32428` selects the file and not the speed class, recovered on
    /// `docs/ghidra/functions/psp-pulse-usa/missile.md`. Pure's single
    /// `weaponstats.xml` is the whole of its tuning, so there is no second file
    /// for a mode to reach for rather than one this build has not found.
    ///
    /// Nothing reads it yet: `oag_game::race::load` opens [`Self::race`] for
    /// every mode. It is carried because the axis is about which files exist,
    /// and recording that Pure has one is the point of the axis.
    pub elimination: Option<&'static str>,
}
