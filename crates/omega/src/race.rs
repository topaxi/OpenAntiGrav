//! Where a race looks for its craft and circuit, and which of that is measured.
//!
//! **A race starts on this title's own data**, given archives that were
//! extracted whole - see `docs/formats/omega-status.md`'s "Racing". Nothing
//! here is machinery of its own; every reader was already in the workspace and
//! Omega needed a change to the *shape* it accepts, not a new format:
//!
//! - the circuit is a version-6 little-endian `track.vex` whose `WO Track` node
//!   [`oag_vex::track`] reads unmodified (`tech_de_ra`: 2 paths, 2 junctions,
//!   835 points, and `encoded_len` equals the payload's length to the byte);
//! - its collision is a `track_col.col` in 2048's container with a 19-byte
//!   k-d node in place of 24 ([`oag_vex::kdcol::NodeLayout::Packed`]);
//! - its geometry is a `track.final.rcsmodel` (the `.final` infix is Omega's;
//!   `oag_mesh::mesh::rcs::sibling_name_cooked`) in 2048's `.rcsmodel`
//!   container with 64-bit pointers, whose materials name `.gnf` textures;
//! - a craft is `hdships\<team>\Ship.vex` beside `ship.rcsmodel` and
//!   `handlingstats.xml`, the same three files 2048's HD-derived roster has.
//!
//! **What this module still does not know**, and fills with an honest
//! "unmeasured" (`None`, or a variant that names no path) rather than a
//! guess: Zone mode, the boost plume, every per-team variant table, the
//! speed classes, and the announcer. Racing is not *complete* on this title
//! - see the report in `omega-status.md` for what is drawn and what is not.

/// The HD-derived ship directory, lowercase, confirmed by direct listing on
/// `data00.psarc` (`Data/art/published/hdships/ag_systems/...`). 2048's own
/// scheme for its HD-derived roster, not HD's `Data\Ships`.
pub const SHIP_DIR: &str = r"Data\art\published\hdships";

/// The directory under [`SHIP_DIR`] Zone mode's one hull sits in.
pub const ZONE_SHIP: &str = "Zone";

/// Race defaults: two chosen entries, the directories, and unmeasured
/// placeholders where nothing has been read.
pub const DEFAULTS: &oag_title::RaceDefaults = &oag_title::RaceDefaults {
    // **Chosen, not measured.** `tech_de_ra` is a real circuit whose `WO Track`
    // reads, whose collision decodes and whose geometry draws (all measured);
    // that it is *this* title's default is not - no menu order or track
    // plugin definition was read to say which circuit is "first". It was
    // picked because it is the value this constant already held, and
    // `--track` names any other of the 22.
    track: r"Data\environments\tech_de_ra\track.vex",
    // **Chosen, not measured.** `ag_systems` is a real team whose `Ship.vex`,
    // `ship.rcsmodel` and `handlingstats.xml` all read (the last as team "AG
    // Systems", class `venom`); which team this title defaults to is not
    // something any table read here says. (An earlier revision of this
    // comment recorded "no `.rcsmodel`/handling file located" - that was a
    // listing of an extraction with most entries missing from its manifest,
    // see `omega-status.md`.)
    team: "ag_systems",
    // **2048's ship-directory scheme, not HD's `Data\Ships\<Team>\`** -
    // confirmed by direct listing: Omega's HD-derived roster lives at
    // `Data\art\published\hdships\<Team>\`, lowercase ids, the same
    // directory 2048's own `oag_title::race::SHIP_DIR` names for its
    // HD-derived craft (`crates/2048/src/lib.rs`'s module doc, "The asset
    // tree is Wipeout HD's, and the ship directory is not"). New finding for
    // Omega, not carried from `omega-frontend.md`, which never opened this
    // tree. **Not [`oag_title::race::SHIP_DIR`]** - that constant is
    // `Data\Ships`, the path Pulse/Pure/HD share and Omega does not.
    ship_dir: SHIP_DIR,
    // **Measured.** `hdships\ag_systems\handlingstats.xml` is there and
    // `oag_tables::handling` reads it unmodified, so the handling file sits
    // beside the hull, as it does for every title but 2048's native roster.
    handling_dir: SHIP_DIR,
    // **Omega ships `Data/particles` (the HD-era set) and `Data/particles2048`
    // and picks one at runtime** - `Data/Particles/%s` when the u32 at
    // `0x01f99bc0` is 1, `Data/Particles2048/%s` otherwise
    // (`docs/ghidra/functions/ps4-omega-eu/particle-paths.md`, measured). That
    // flag is set by a launch-argument handler and by the level-select routine
    // from a per-entry byte at `+0x24d`; **which circuit sets it was not
    // read.** **Chosen, not measured:** `environments2048\*` circuits take
    // the 2048 set and the HD-heritage `environments\*` ones (and Zone) the
    // `Data\particles` set, which is the era each circuit's own directory
    // already names.
    effect_dir: r"Data\particles",
    effect_dir_by_circuit: &[(r"Data\environments2048\", r"Data\particles2048")],
    // **Zero-fabrication placeholder, not a finding.** `SameCircuit` is the
    // one `ZoneCircuit` variant that names no path at all - 2048's own shape,
    // picked here only because it requires inventing nothing. Omega's Zone
    // mode has not been read; this is not a claim that it works like 2048's.
    zone: oag_title::ZoneCircuit::SameCircuit,
    // `hdships\Zone\Ship.vex` for every craft in Zone mode: the ship-model
    // loader's mode-6 case (`FUN_01301ba0`, confidence 80,
    // `docs/ghidra/functions/ps4-omega-eu/zone-craft.md`) names it without
    // reading the craft. The player's pick selects a livery on it.
    zone_craft: oag_title::ZoneCraft::OwnShipAt {
        root: SHIP_DIR,
        ship: ZONE_SHIP,
        livery_key: "zoneship_team",
    },
    // Not searched for: `None` is silence, not a finding.
    boost: None,
    sounds: SOUND_BANKS,
    zone_announcer: None,
    countdown_voice: None,
    launch_hover: None,
    hover_rig: None,
    craft_laws: None,
    zone_class_announcer: None,
    zone_palette: None,
    zone_stages: None,
    zone_transition: None,
    zone_stage_textures: None,
    zone_sky: None,
    team_variants: None,
    guest_roster: Some(&ERA_2048_ROSTER),
    hull_variants: None,
    fresh_variant: None,
    speed_classes: None,
};

/// Where Omega keeps the models of its **2048-era** craft: 2048's own tree,
/// `Data\art\published\ships\<Team>2048\<n>\`, which is not [`SHIP_DIR`].
///
/// Omega's `Data\plugins\teams\Definition.xml` declares each team's own
/// directory as its `PI_Team` `location` - the five `*2048` teams point here
/// and the fourteen HD-era ones at [`SHIP_DIR`] (read by
/// `crates/game/tests/omega_2048_era_craft_ground_truth.rs` on every run), and
/// the ship-model loader's non-special-mode case builds `%s\ship.vex` off the
/// craft entry's own directory (`Ship_LoadModelSet`, `0x01301ba0`, confidence
/// 75, `docs/ghidra/functions/ps4-omega-eu/zone-craft.md`).
pub const ERA_2048_SHIP_DIR: &str = r"Data\art\published\ships";

/// Where those craft's **tuning** sits, one tree away from the models, as on
/// 2048 itself: `Data\handlingstats\<Team>2048\<n>\handlingstats.xml` for
/// all twenty. **Measured**: the teams definition authors it per craft as
/// `handlingstatslocation="Data\HandlingStats\feisar2048\3"` beside the
/// craft's `modellocation`. Which of the two reads the loader makes of it was
/// not decompiled (`Ship_LoadModelSet` reads only the model directory).
pub const ERA_2048_HANDLING_DIR: &str = r"Data\handlingstats";

/// 2048's four craft per team, joined as a numbered subdirectory
/// (`Feisar2048\3`). **Duplicated from `oag_2048::race::TEAM_VARIANTS`**,
/// not borrowed: a title package does not depend on another, and the test
/// checks every combined id against Omega's own archive.
pub const ERA_2048_VARIANTS: oag_title::TeamVariants = oag_title::TeamVariants {
    teams: &[
        "AG_Systems2048",
        "Auricom2048",
        "Feisar2048",
        "Piranha2048",
        "Qirex2048",
    ],
    variants: &[
        oag_title::TeamVariant {
            suffix: "1",
            label: "fighter",
        },
        oag_title::TeamVariant {
            suffix: "2",
            label: "agility",
        },
        oag_title::TeamVariant {
            suffix: "3",
            label: "speed",
        },
        oag_title::TeamVariant {
            suffix: "4",
            label: "prototype",
        },
    ],
    join: oag_title::VariantJoin::Subdirectory,
};

/// [`oag_title::RaceDefaults::guest_roster`] for Omega: its five 2048-era
/// teams, whose directory and tuning tree are not the HD-era ones. Not a
/// reshipped *copy* in the HD sense - these are Omega's own re-textured
/// (`.gnf`) versions of 2048's craft - so [`oag_title::GuestRoster::reships`]
/// names the title they come from.
pub const ERA_2048_ROSTER: oag_title::GuestRoster = oag_title::GuestRoster {
    dir: ERA_2048_SHIP_DIR,
    handling_dir: Some(ERA_2048_HANDLING_DIR),
    variants: &ERA_2048_VARIANTS,
    reships: "Wipeout 2048",
    alongside_label: Some("Wipeout 2048 (Omega)"),
    origin: oag_title::Origin::Measured,
};

/// Sound banks, confirmed present by name in `data00.psarc`'s listing -
/// **not** copied from [`oag_hd::race::SOUND_BANKS`], whose paths
/// (`Data\Sound\...`) do not exist on Omega at all. Omega's own tree is
/// `Data\audio\sound\...`, lowercase, a different directory this project has
/// not seen before. `hud` reuses the same bank `weapons` does, on the same
/// unverified assumption HD's own table makes for the same reason - no
/// separate `hud.bnk`-named file was found.
pub const SOUND_BANKS: &oag_title::SoundBanks = &oag_title::SoundBanks {
    hud: r"Data\audio\sound\weapons.bnk",
    ship: r"Data\audio\sound\shipHD.bnk",
    ship_zone: r"Data\audio\sound\shipHD.bnk",
    weapons: r"Data\audio\sound\weapons.bnk",
    speech: r"Data\audio\sound\speech.bnk",
    frontend: None,
    // Wwise banks (`BKHD`), which `oag_formats::sblk` does not read, so
    // nothing resolves yet. The directory is the thread's archive census
    // (`env12_techdera.bnk` and `env9_talonsjunction.bnk` read under
    // `Data\audio\sound\`, not beside the track): set so that the load report
    // names the real cause, a format the reader does not know, and not a
    // missing file. `shared` stays empty: which file carries `env_tec` and
    // `techder` is unread. `docs/formats/2048-audio.md`.
    track: oag_title::TrackBanks {
        shared: &[],
        circuit: oag_title::CircuitBanks::Directories(&[r"Data\audio\sound"]),
        origin: oag_title::Origin::Measured,
    },
    crossfade: None,
};

#[cfg(test)]
mod effect_dir_tests {
    #[test]
    fn a_2048_circuit_plays_the_2048_set_and_every_other_the_hd_era_one() {
        assert_eq!(
            super::DEFAULTS.effect_dir_for(r"Data\environments2048\tower\track.vex"),
            r"Data\particles2048"
        );
        for circuit in [
            super::DEFAULTS.track,
            r"Data\environments\zone_1\track.vex",
            r"Data\environments\01_vineta_k\track.vex",
        ] {
            assert_eq!(super::DEFAULTS.effect_dir_for(circuit), r"Data\particles");
        }
    }
}
