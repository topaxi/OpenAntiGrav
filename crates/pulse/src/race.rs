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
    // The directory three of the four titles spell identically; Wipeout 2048 is
    // the one that does not. See `oag_title::RaceDefaults::ship_dir`.
    ship_dir: oag_title::race::SHIP_DIR,
    // The same directory: this title keeps a team's tuning beside its models.
    handling_dir: oag_title::race::SHIP_DIR,
    zone: oag_title::ZoneCircuit::Prefixed(ZONE_TRACK_PREFIX),
    // The one variant with a recovered selector behind it rather than a name
    // probe: `Ship_LoadModel`'s `case 6`. See `ships::ZONE_HULL`.
    zone_craft: oag_title::ZoneCraft::ModelsInTeam {
        hull: ships::ZONE_HULL,
        boost: ships::ZONE_BOOST,
    },
    // The one title with a standalone plume: `%s\%sboost.vex`, recovered at
    // `docs/ghidra/functions/psp-pulse-usa/exhaust.md`. `ships::BOOST` stays
    // where it is - see that constant's own doc comment - this just reads it
    // into the axis every other title answers `None` on.
    boost: Some(ships::BOOST),
    sounds: SOUND_BANKS,
    zone_announcer: Some(ZONE_ANNOUNCER),
    // Pulse's own `speech_zone.bnk` was listed alongside Pure's and HD's -
    // see `docs/formats/psp-audio.md`'s bank table - and names no `MR_*`-shaped
    // speed-class cues at all, only the numbered ladder.
    zone_class_announcer: None,
    // Searched, not skipped: two independently generated candidate lists
    // resolved no `.effectSettings`-shaped entry on this disc at all - see
    // `oag_title::ZonePalette`. A Zone race here escalates its speed alone,
    // with no colour grade to climb.
    zone_palette: None,
    // No stage table to index into, so nothing to map a zone number onto.
    zone_stages: None,
    // Unread on this title - see `oag_title::ZoneTransition`.
    zone_transition: None,
    zone_stage_textures: None,
    // `None` on the same terms as the table above: Pulse ships no Zone
    // effectSettings and no located Zone sky.
    zone_sky: None,
    // Pulse authors one roster and no *second directory* for any team in it.
    team_variants: None,
    guest_roster: None,
    // It does author a second hull *file* per team, inside that one
    // directory - see `oag_title::race::HullVariant`'s own doc comment for
    // the evidence (`crates/game/examples/pulse_variant_probe.rs`).
    hull_variants: Some(HULL_VARIANTS),
    // Four rungs, and the four are the *per-team* file's, not the global
    // one's. Pulse's `Data\XML\HandlingStats.xml` authors five `<GlobalClass>`
    // blocks with `VECTOR` first, and every one of the eight teams'
    // `Data\Ships\<Team>\handlingstats.xml` authors exactly four `<Class>`
    // rungs with no `VECTOR` among them - checked on both pressings. The
    // original's own parser discards the global `VECTOR` block (it matches
    // `name` against a four-entry table and leaves the previous index in
    // place), so four is what this title really has. See
    // `docs/formats/handling-stats.md`.
    speed_classes: Some(oag_title::SpeedClasses {
        names: oag_title::SpeedClasses::PULSE_LADDER,
    }),
};

/// Pulse's Zone milestone announcer.
///
/// `Data\Sound\speech_zone.bnk` (`zone_vo`, `e66cdc25`) names these thirteen
/// cues in ascending order, on both the USA and EU discs - `oag-wad sounds
/// 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' --bank vo`. See
/// `docs/formats/psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title`
/// and [`oag_title::ZoneAnnouncer`] for the confidence split between "the bank
/// holds these" (95) and "the counter reaching this number is the trigger"
/// (75, traced on this title's own executable).
pub const ZONE_ANNOUNCER: &oag_title::ZoneAnnouncer = &oag_title::ZoneAnnouncer {
    bank: r"Data\Sound\speech_zone.bnk",
    milestones: &[5, 10, 15, 20, 25, 30, 40, 50, 60, 70, 80, 90, 100],
};

/// Where each race cue lives.
///
/// Every one of these is a literal string in the PSP executable and hashes to a
/// real archive entry on **both** Pulse pressings and on the PS2 disc - see
/// `docs/formats/psp-audio.md`'s table, which lists the `BOOT.BIN` address of
/// each. Nothing here is a guessed spelling.
pub const SOUND_BANKS: &oag_title::SoundBanks = &oag_title::SoundBanks {
    hud: r"Data\Sound\hud.bnk",
    ship: r"Data\Sound\ship.bnk",
    ship_zone: r"Data\Sound\ship_zone.bnk",
    weapons: r"Data\Sound\weapons.bnk",
    speech: r"Data\Sound\speech.bnk",
    // A literal string in the executable too, at the same confidence as the
    // five above, and the bank 568 of the 1,164 authored `sound` nodes name.
    track_general: Some(r"Data\Sound\generaltrack.bnk"),
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

/// The eight teams the base disc ships, one `handlingstats.xml` and one ship
/// directory each. **Not the whole roster** - see [`DLC_TEAMS`].
///
/// **Moved from `oag_tables::handling::TEAMS`, 2026-09-01.** A format crate
/// describing a game's own roster by name was the ADR-0022 smell, named and
/// never acted on; `oag_hd::names::TEAMS` already keeps HD's roster the same
/// way this keeps Pulse's. All eight files were located by hashing candidate
/// names built from this list; none of them appears as a string in the
/// executable, because the loader builds the path from a template. See
/// `docs/formats/fexml.md`.
pub const TEAMS: [&str; 8] = [
    "AG_Systems",
    "Assegai",
    "EGX",
    "Feisar",
    "Goteki",
    "Piranha",
    "Qirex",
    "Triakis",
];

/// The four teams Pulse's downloadable packs add, by **id** - the folder
/// under `Data\Ships\`, not the display name a player reads.
///
/// **Confidence 94.** `Mantis` is the id the Mirage pack's own manifest, the
/// PS2 disc and the PSP string tables all agree on; `Mirage` is shipped text
/// and was never a folder. See
/// [`docs/formats/dlc-pack.md`](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/dlc-pack.md#the-roster-and-the-name-that-hid-it).
/// Each of the four ships in its own pack, so which of these resolve on a
/// given source depends on which packs are mounted - unlike [`TEAMS`], none
/// of these is guaranteed present.
pub const DLC_TEAMS: [&str; 4] = ["Auricom", "Harimau", "Icaras", "Mantis"];

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

/// Every team's own alternate hull *file* - `Ship.vex`, the baseline every
/// mode but Zone draws, and `extra.vex`, the `PI_TeamModel name="Concept"`
/// declares on every one of Pulse's eight base teams. See
/// [`oag_title::race::HullVariant`]'s own doc comment for the evidence and for
/// why `PI_TeamModel name="Zone"`'s own `zone01.vex` is deliberately not a
/// third entry here.
pub const HULL_VARIANTS: &[oag_title::race::HullVariant] = &[
    oag_title::race::HullVariant {
        stem: ships::HULL,
        label: "Normal",
    },
    oag_title::race::HullVariant {
        stem: "extra",
        label: "Concept",
    },
];

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

    /// The archive entry name of one of a team's models, under the ship
    /// directory this title keeps its roster in.
    ///
    /// Assembled the way the loader assembles it, with backslashes, which is
    /// what the name hash needs. `dir` comes from
    /// [`oag_title::RaceDefaults::ship_dir`] - see that field for why it is an
    /// axis and not the constant this function used to spell inline.
    #[must_use]
    pub fn entry_name_in(dir: &str, team: &str, model: &str) -> String {
        format!(r"{dir}\{team}\{model}.vex")
    }

    /// [`entry_name_in`] under
    /// [`oag_title::race::SHIP_DIR`](oag_title::race::SHIP_DIR), which is where
    /// Pulse, Pure and Wipeout HD all keep their rosters.
    #[must_use]
    pub fn entry_name(team: &str, model: &str) -> String {
        entry_name_in(oag_title::race::SHIP_DIR, team, model)
    }
}

/// Where Pulse keeps each weapon's own body model.
///
/// **`oag_pure::TITLE` restates these same five literals rather than
/// depending on this crate at runtime** - a title package does not depend on
/// another outside `[dev-dependencies]`, per ADR-0022; `oag_2048::TITLE`
/// carries no restatement at all, since 2048's tree is Vita/PSP2 and none of
/// these paths is checked to resolve there. Moved here 2026-09-17 off
/// `oag_game::race`'s own `ROCKET_MODEL_ENTRY` and friends, which stay as
/// `pub use` aliases so no doc link or call site moved - see that module's
/// own doc comment for the confidence and evidence each entry carries; only
/// the location changed, not the reading.
pub const WEAPON_MODELS: &oag_title::weapons::WeaponModels = &oag_title::weapons::WeaponModels {
    rocket: Some(r"Data\Weapons\Rocket.vex"),
    mine: Some(r"Data\Weapons\Pulse_Mine.vex"),
    bomb: Some(r"Data\Weapons\Pulse_Bomb.vex"),
    cannon: Some(r"Data\Weapons\pulse_muzzleflash.vex"),
    // Pulse rides the charge/travel glow alone; it authors no bolt-head mesh.
    plasma_ball: None,
    plasma_blast_pulse: Some(oag_title::weapons::PulsePlasmaBlast {
        halo: r"Data\Weapons\pulse_plasma_halo1.vex",
        hemisphere2: r"Data\Weapons\pulse_plasma_hemisphere2.vex",
        hemisphere1: r"Data\Weapons\pulse_plasma_hemisphere1.vex",
    }),
    plasma_blast_hd: None,
    // `BombBlast_Construct`'s own load order - see
    // `docs/ghidra/functions/psp-pulse-usa/mine.md`.
    bomb_blast_pulse: Some(oag_title::weapons::PulseBombBlast {
        hemisphere: r"Data\Weapons\explosion_hemisphere.vex",
        shockwave: r"Data\Weapons\Bomb_Shockwave.vex",
    }),
    leachbeam_ball: None,
};

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
