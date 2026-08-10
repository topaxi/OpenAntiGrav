//! What a Pulse race loads: the default circuit and team, and how a ship's
//! model entry is spelled.
//!
//! Names and defaults, in the [ADR-0022] sense. The mode that picks between a
//! Zone hull and a normal one is `oag_game::race`'s, because `Mode` is the
//! engine's type and a title package must not grow one; what this file owns is
//! the four file names it picks between and the path they go into.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The circuit a race loads when the caller names none.
///
/// **`16_Track` is Talon's Junction**, and the mapping is not guessable from the
/// directory number: the circuits are named in `Data\Plugins\PI001\Definition.xml`
/// and two entries can name one environment, differing only by `Reversed`. A
/// trace comparison run against the wrong circuit seeds the original's recorded
/// position into a different track's collision geometry, which reads as a
/// force-law failure and is a track-selection one. See `oag_game::race`'s own
/// note on [`DEFAULT_TRACK`] for the full account of that day.
pub const DEFAULT_TRACK: &str = r"Data\Environments\16_Track\track.vex";

/// The team whose `handlingstats.xml` and model a race uses by default.
///
/// **Assegai, not Feisar** - chosen to match the reference scenario used for
/// every PPSSPP capture this project has taken (Time Trial, Venom, Talon's
/// Junction White, Assegai; see
/// `docs/reverse-engineering/ppsspp-debugger.md`'s "reference scenario"
/// section), so `just play --race`'s defaults and a captured trace are
/// directly comparable without passing `--team`/`--class` every time.
pub const DEFAULT_TEAM: &str = "Assegai";

/// The four per-team `.vex` models a race can ask for, and how the path is built.
pub mod ships {
    /// The hull every mode but Zone draws.
    pub const HULL: &str = "Ship";

    /// The hull Zone mode draws instead.
    ///
    /// Not a livery swap of convenience: `Ship_LoadModel` (`0x08843258`)
    /// switches on the same `DAT_08ab07e3 == 0 && DAT_08b31048 == 6` expression
    /// already established as the Zone selector (see
    /// `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`), and only that case
    /// builds the `%s\Zone.vex` path. Every team's `Zone.vex` decodes to the
    /// same 1213 vertices / 1149 triangles / 8 meshes, so the hull itself is
    /// shared - only the livery painted on it still varies by team.
    pub const ZONE_HULL: &str = "Zone";

    /// The boost plume that goes with [`HULL`].
    pub const BOOST: &str = "shipboost";

    /// The boost plume that goes with [`ZONE_HULL`].
    ///
    /// Every team ships one beside its `Zone.vex`, so the pairing exists in the
    /// data; loading [`BOOST`] in a Zone race was a real mismatch rather than a
    /// simplification.
    pub const ZONE_BOOST: &str = "Zoneboost";

    /// The archive entry name of one of a team's models.
    ///
    /// Assembled the way the loader assembles it, with backslashes, which is
    /// what the name hash needs.
    #[must_use]
    pub fn entry_name(team: &str, model: &str) -> String {
        format!(r"Data\Ships\{team}\{model}.vex")
    }
}

#[cfg(test)]
mod tests {
    use super::ships;

    /// The template is the loader's, backslashes included, so a forward slash
    /// here would hash to nothing and find no entry.
    #[test]
    fn a_model_entry_is_spelled_the_way_the_name_hash_needs() {
        assert_eq!(
            ships::entry_name("Feisar", ships::HULL),
            r"Data\Ships\Feisar\Ship.vex"
        );
        assert_eq!(
            ships::entry_name("Feisar", ships::ZONE_BOOST),
            r"Data\Ships\Feisar\Zoneboost.vex"
        );
    }
}
