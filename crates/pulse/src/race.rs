//! What a Pulse race loads: the default circuit and team, and how a ship's
//! model entry is spelled.
//!
//! Names and defaults, in the [ADR-0022] sense. The mode that picks between a
//! Zone hull and a normal one is `oag_game::race`'s, because `Mode` is the
//! engine's type and a title package must not grow one; what this file owns is
//! the four file names it picks between and the path they go into.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The three things a race needs before it has opened anything, as
/// [`oag_title::Title::race`] carries them.
///
/// The constants below are the same values, kept as named items because that is
/// where their evidence is written down; this is the table the engine reads.
pub const DEFAULTS: &oag_title::RaceDefaults = &oag_title::RaceDefaults {
    track: DEFAULT_TRACK,
    team: DEFAULT_TEAM,
    zone: oag_title::ZoneCircuit::Prefixed(ZONE_TRACK_PREFIX),
};

/// What Pulse puts on the front of a circuit's file name to reach the Zone
/// variant of the same circuit.
///
/// `Data\Environments\16_Track\track.vex` beside
/// `Data\Environments\16_Track\zone_track.vex`, and `track_reversed.vex` beside
/// `zone_track_reversed.vex` - the four names per circuit that
/// [`docs/formats/track.md`] derives from the binary's own `%s\%strack%s.vex`
/// template. This constant is that template's **first** `%s`.
///
/// **A Zone circuit is a different file, not a filter on the race one.** It
/// ships its own meshes, lights, `fogCube` and `Skycube`, and the sky is where
/// the difference is plainest: of the 40 skies on the PSP disc, the twelve with
/// a single material are exactly the Zone variants, against five or six for
/// every race circuit. So Zone's look is loaded rather than computed, which is
/// what `CLAUDE.md`'s rule about not inventing what the assets author asks for.
///
/// **Sixteen of the disc's twenty-four `PI_Track` entries have one**, and which
/// sixteen is declared rather than guessed - see
/// `oag_game::catalogue::Track::available_in_zone`.
///
/// Confidence **94**: every one of the 24 entries was probed by name against
/// `pulse-psp-usa.chd` and the presence of the file matches the declaration
/// 24 times out of 24. `pulse-ps2-eu.chd` carries the same arrangement, so this
/// is a Pulse fact rather than a PSP one. The **selector inside the executable
/// is unread** - the branch that chooses the prefix, the analogue of
/// `Ship_LoadModel`'s `case 6` for the hull, has not been found - which is what
/// caps this below the confidence the hull swap carries.
///
/// [`docs/formats/track.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/track.md
pub const ZONE_TRACK_PREFIX: &str = "zone_";

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

/// The shell a fired Shield pickup draws around the craft, when no team's own
/// is on the source.
///
/// `Data\Weapons\shield.vex`, and **it is on both Pulse's disc and Pure's**,
/// byte-identical in tree and texture to Pulse's per-team
/// [`ships::SHIELD`]. Pure carries no per-team shell at all - no
/// `Data\Ships\<Team>\shipshield.vex` and no `shipboost.vex` either - so on that
/// title this is the only shield model there is, and on Pulse it is a leftover.
///
/// Pulse's own loader never assembles this name: `ShipShield_Construct`
/// (`0x0885db38`) builds `%s\%sshield.vex` from a directory *and* a prefix, so
/// the bare form cannot come out of it. It is here as the fallback a source
/// without the per-team model gets, which is what makes the shield visible on
/// Pure without reading Pure's own executable. See
/// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
pub const SHARED_SHIELD: &str = r"Data\Weapons\shield.vex";

/// The cockpit variant of the shield shell, `Data\Weapons\vr_shield_cockpit.vex`.
///
/// A noise-textured sphere the original draws **instead of** the hull-shaped
/// shell when the camera is inside the craft, at `1.8` times the shell's scale.
/// Also on both discs. Named here rather than left out because it is recovered
/// and because naming it is what stops the next reader concluding the shield has
/// one model; nothing loads it until this engine has an internal camera.
pub const COCKPIT_SHIELD: &str = r"Data\Weapons\vr_shield_cockpit.vex";

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

    /// The shell a fired Shield pickup draws around the craft.
    ///
    /// **Not a name this project chose.** `ShipShield_Construct` (`0x0885db38`)
    /// builds `%s\%sshield.vex` from the team's own directory and a prefix it
    /// reads from the `FE_TeamModel` config key, whose literal default in the
    /// binary is `"ship"` - so the assembled name is exactly this. All eight
    /// playable teams carry one, confirmed by hashing the assembled name against
    /// `Data.wad`'s directory. See
    /// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
    ///
    /// **No Zone variant exists**, unlike [`BOOST`]/[`ZONE_BOOST`]: neither
    /// `Zoneshield` nor `Zoneshipshield` hashes to anything on the disc. So a
    /// Zone craft draws this same shell, which is what the format string does
    /// too - the prefix comes from a config key, not from the game mode.
    pub const SHIELD: &str = "shipshield";

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
