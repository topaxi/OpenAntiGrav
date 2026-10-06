//! What a Pulse race loads: the default circuit and team, and how a ship's
//! model entry is spelled.
//!
//! Names and defaults, in the [ADR-0022] sense. The mode that picks between a
//! Zone hull and a normal one is `oag_raceplay`'s, because `Mode` is the
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
    effect_dir: oag_title::race::EFFECT_DIR,
    effect_dir_by_circuit: &[],
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
    countdown_voice: Some(COUNTDOWN_VOICE),
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
    fresh_variant: None,
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
    tick: oag_title::SequenceTick::Psp,
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
    crossfade: None,
};

/// The banks Pulse's `ready` and `go` are read from in the two modes that do
/// not use `speech.bnk`.
///
/// Read out of `World_LoadTrack` (`0x08883fa0`): the Zone branch opens
/// `Data\Sound\speech_zone.bnk`, the Eliminator branch
/// `Data\Sound\speech_elim.bnk`, and each stores the opened bank's slot where
/// `RaceMode_SetState` and `RaceMode_UpdateCountdown` read it back. Eliminator
/// confirmed live (the cue-start log names `elim_vo` at the countdown's two
/// cues); Zone from the loader alone. See
/// `docs/ghidra/functions/psp-pulse-usa/countdown-voice.md`.
pub const COUNTDOWN_VOICE: &oag_title::CountdownVoice = &oag_title::CountdownVoice {
    eliminator_bank: r"Data\Sound\speech_elim.bnk",
    zone_bank: r"Data\Sound\speech_zone.bnk",
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
/// `oag_raceplay::catalogue::Track::available_in_zone`.
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
/// force-law failure and is a track-selection one. See `oag_raceplay`'s own
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

    /// The shell the **PS2** build draws, `extrashield`: the `%s\%sshield.vex`
    /// format of [`SHIELD`] with the literal `"extra"` as its prefix.
    ///
    /// **Read, then seen formed.** `ShipShield_Construct` (`0x00169168` in
    /// `SCES_547.48`) passes the string at `0x002a7920`, which reads `extra`,
    /// where the PSP build passes the `FE_TeamModel` config key's `"ship"`. A
    /// PCSX2 run shows the assembled `Data\Ships\Assegai\extrashield.vex` and
    /// `Data\Ships\AG_Systems\extrashield.vex` in EE RAM and no `shipshield`
    /// anywhere, and a GS dump of the raised shell draws this model's
    /// `pulse_shield_extra_ADD` texture rather than the `grid_GLOW` lattice
    /// `shipshield.vex` carries. See
    /// `docs/ghidra/functions/ps2-pulse-eu/shield-pickup.md`.
    pub const PS2_SHIELD: &str = "extrashield";

    /// The model the craft becomes once it has blown up: `%s\%swreck.vex` with
    /// the same `"ship"` prefix [`SHIELD`] takes, so `shipwreck`.
    ///
    /// `Ship_LoadModel` (`0x08843258`) reads it into `entity+0x8b8` beside the
    /// hull at `+0x8b4`, and `Ship_SetState`'s case 5 makes it the live model
    /// - see `docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md`.
    pub const WRECK: &str = "shipwreck";

    /// The wreck a Zone craft becomes: the `local_38 == 6` guard that swaps the
    /// hull for `Zone.vex` swaps this in for [`WRECK`] too
    /// (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md`).
    pub const ZONE_WRECK: &str = "zonewreck";

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
/// `oag_raceplay`'s own `ROCKET_MODEL_ENTRY` and friends, which stay as
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
    // `Repulser_Construct` (`0x08875008`), format string `0x08a7ccdc`.
    repulser_field: Some(r"Data\Weapons\pulse_repulsorwave.vex"),
    // `MagFloorFx_Construct` (`0x088590a8`), strings `0x08a7bfa8`/`0x08a7bfcc`.
    mag_floor: Some([
        r"Data\visual_effects\MagEffect1.vex",
        r"Data\visual_effects\MagEffect2.vex",
    ]),
    magstrip_wake: None,
    magstrip_pob: false,
    leachbeam_ball: None,
    // `None` is this title's own reading - see the field's doc comment.
    cannon_look: None,
    // `MeshNode_Ghost_LoadStaticGlow` (`0x08910ef8`) names it.
    ghost_static: Some(r"Data\Tex\staticglow.mip"),
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

/// Where the track-description screen is authored: `InGame_Definition.xml`, the file `Skin.xml`'s
/// `LoadXML` list names last. It holds `InGameTrackDescriptionScreen`, the panel the original
/// puts over the pre-race flyby - see `docs/gameplay/race-intro.md`.
pub const TRACK_DESCRIPTION_DEFINITION: &str = r"Data\Plugins\PI001\GUI\InGame_Definition.xml";

/// The screen name in [`TRACK_DESCRIPTION_DEFINITION`].
pub const TRACK_DESCRIPTION_SCREEN: &str = "InGameTrackDescriptionScreen";

/// The panel's two `Text` widgets: the circuit's name, and its paragraph. Both author
/// `string=""`; the race constructor (`RaceManager_Construct`, `0x08829124`) fills them.
pub const TRACK_NAME_WIDGET: &str = "InGameTrackName";

/// See [`TRACK_NAME_WIDGET`].
pub const TRACK_TEXT_WIDGET: &str = "InGameTrackText";

/// The string-table id of a circuit's paragraph: `MSC_TRACK_%.2s` of its `PI_Track` id, so
/// `16_Track` and `32_Track` read `MSC_TRACK_16` and `MSC_TRACK_32`. The name is the id itself,
/// looked up in the same table (`16_Track` is "Talon's Junction White").
///
/// Read from the format string at `0x08a7a720` and its one caller, `RaceManager_Construct`
/// (`0x08829124`, the references at `0x088296a0` and `0x088296ec`), which passes the current race's circuit id (`*(DAT_08b310b4 + 0x74)`) to the name widget unchanged
/// and to `sprintf` for the paragraph. `None` for an id shorter than two characters.
#[must_use]
pub fn track_description_id(track_id: &str) -> Option<String> {
    let digits: String = track_id.chars().take(2).collect();
    (digits.chars().count() == 2).then(|| format!("MSC_TRACK_{digits}"))
}

#[cfg(test)]
mod track_description_tests {
    use super::*;

    #[test]
    fn the_paragraph_id_is_the_first_two_characters_of_the_circuit_id() {
        assert_eq!(
            track_description_id("16_Track").as_deref(),
            Some("MSC_TRACK_16")
        );
        assert_eq!(
            track_description_id("32_Track").as_deref(),
            Some("MSC_TRACK_32")
        );
        assert_eq!(track_description_id("7"), None);
    }
}
