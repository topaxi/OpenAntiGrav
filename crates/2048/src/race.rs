//! What a Wipeout 2048 race loads: the default circuit and team, where its
//! ships live, and which bank each cue is in.
//!
//! Names and defaults in the [ADR-0022] sense, the counterpart to
//! [`oag_hd::race`]. Every entry name here was probed against
//! `PCSF00007/base/PSP2/data.psarc` by asking the manifest for it, the same
//! bar `oag_pure::race` sets: **confidence 90** for a name that resolves to an
//! entry, and no more than that, because nothing here has been watched
//! running.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`oag_hd::race`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/race.rs

/// What a race needs before it has opened anything, as
/// [`oag_title::Title::race`] carries it.
///
/// **Both Zone fields used to be wrong, and both were fixed by playing the
/// mode rather than by reading further disc data - 2026-08-28.**
///
/// `zone_craft` was [`oag_title::ZoneCraft::OwnShip`] (never resolved: it composed
/// `hdships\Zone` under the native [`SHIP_DIR`]), then
/// [`oag_title::ZoneCraft::PlayerShip`] on 2026-08-28 ("2048 has no dedicated
/// Zone ship"), and is now [`oag_title::ZoneCraft::OwnShipAt`] -
/// `Data\art\published\hdships\Zone\Ship.vex` for every craft, rooted in
/// [`HD_SHIP_DIR`]. The v1.04 executable's ship-model loader names that file in
/// its mode-6 case without reading the craft
/// (`docs/ghidra/functions/vita-2048-eu-v104/zone-craft.md`); the 2026-08-28
/// play note was right that the player picks their ship (it picks the livery)
/// and wrong that the hull is theirs.
///
/// `zone` used to be [`oag_title::ZoneCircuit::Separate`], pointed at one of
/// four Zone-named environments `dlc2.psarc` ships - which
/// `docs/formats/track.md` had already shown are Wipeout HD's own dedicated
/// Zone circuits, reshipped verbatim splines and all, so this read as the
/// Pure/HD shape reused. Playing the mode said otherwise: it ran on an
/// ordinary circuit, not a separate environment - consistent with a fact
/// noted here unread since before that guess was made: every circuit in the
/// base package ships a `ZoneMode2048.effectSettings` beside its `track.vex`,
/// which only makes sense if Zone runs on the circuit that file sits next to.
/// See [`oag_title::ZoneCircuit::SameCircuit`]. The four ported `zone_N`
/// environments are real disc content; they are simply not what this axis
/// answers, and nothing here still names them.
pub const DEFAULTS: &oag_title::RaceDefaults = &oag_title::RaceDefaults {
    track: DEFAULT_TRACK,
    team: DEFAULT_TEAM,
    ship_dir: SHIP_DIR,
    handling_dir: HANDLING_DIR,
    effect_dir: EFFECT_DIR,
    effect_dir_by_circuit: &[],
    zone: oag_title::ZoneCircuit::SameCircuit,
    zone_craft: oag_title::ZoneCraft::OwnShipAt {
        root: HD_SHIP_DIR,
        ship: ZONE_SHIP,
        livery_key: "zoneship_zone",
    },
    // Unread: this title's own boost-plume path (if it authors a standalone
    // one at all, off either roster) has not been searched for. `None` here
    // is silence, not the measured absence Pure's own row records.
    boost: None,
    sounds: SOUND_BANKS,
    zone_announcer: Some(ZONE_ANNOUNCER),
    countdown_voice: None,
    // Unread on this title: `data.psarc` is not extracted in this tree, so
    // its own speed-class bank (if it has one) has not been listed.
    zone_class_announcer: None,
    // One copy per circuit, which is the arrangement that makes
    // `ZoneCircuit::SameCircuit` above work: the table sits beside the very
    // circuit a Zone race runs on. All ten base circuits ship a byte-identical
    // copy, checked in `effectsettings_ground_truth.rs`.
    zone_palette: Some(oag_title::ZonePalette::BesideCircuit(ZONE_PALETTE)),
    // The only recovered zone-number-to-stage ladder on any title. See
    // `ZONE_STAGES`.
    zone_stages: Some(ZONE_STAGES),
    // Unread on this title - see `oag_title::ZoneTransition`.
    zone_transition: None,
    zone_stage_textures: None,
    // `None`: no Zone sky swap has been looked for on this title. HD/Fury's is
    // a named file its loader picks by a mode gate; nothing equivalent has been
    // read here.
    zone_sky: None,
    team_variants: Some(&TEAM_VARIANTS),
    guest_roster: Some(&GUEST_ROSTER),
    // 2048's own reskins are `team_variants`' own measured shape too - a
    // numbered subdirectory, not a second file inside one. See
    // `oag_title::race::HullVariant`.
    hull_variants: None,
    // **Unread, not empty.** This title ships no race box at all - no
    // `Track Creation`, no settings page, racing entered from the campaign
    // event grid instead (`docs/formats/race-setup.md`) - so there is no menu
    // to read a class ladder off, and its per-craft `Data\HandlingStats\`
    // tree has no ground-truth test that lists one either. `None` keeps the
    // caller on its own ladder rather than lending this title Pulse's four as
    // if someone had checked. Do not fill this in without reading the files.
    //
    // Note that `ZONE_STAGES` below is a *different* ladder and is not
    // evidence for this one: it maps a zone count onto `MX_CLASS`/`A_CLASS`
    // and friends for the Zone HUD, which is a per-zone escalation rather
    // than the speed class a race is started in.
    fresh_variant: None,
    speed_classes: None,
};

/// Wipeout 2048's zone-number to speed-class ladder, read out of the
/// executable rather than fitted to anything.
///
/// # The evidence
///
/// `Hud_UpdateZoneSpeedClassWidget` (`0x81197d6c`, confidence 82) walks
/// seventeen 16-byte records at `0x8151faf8`, each record's first word a
/// descending threshold and its second a pointer into the class-name blob at
/// `0x8148a4a0`. The thresholds and the pointers below were read with
/// `read_memory` directly, not inferred: the pointers step 12 bytes apart
/// through the twelve-byte names and 8 apart through the four short ones,
/// resolving one-for-one onto `MX_CLASS`, `M1_9_CLASS` .. `M1_1_CLASS`,
/// `M1_CLASS`, `AP_CLASS`, `A_CLASS`, `B_CLASS`, `C_CLASS`, `D_CLASS`.
///
/// The value walked against these is `*(*(widget + 0xc) + 0x20)`, which the
/// same block `sprintf`s as the first `%d` of `"%d/%d"` into the scene node
/// `Hud_InitZoneSpeedClassWidget` (`0x81197c24`) looked up by the authored
/// name **`ZoneNumber`** - so it is the zone counter the HUD shows, and the
/// second `%d` is `FUN_812c4e0c`'s event-type-keyed target. The same block
/// also writes the matched record's own threshold and the record above it
/// into the race state (`+0x640`/`+0x644`), which is a progress bar *between
/// two class boundaries* and only makes sense if the walked value climbs in
/// the thresholds' own units. **Confidence 78** on "these are zone numbers":
/// four converging reads, no traced increment of the field itself.
///
/// # What the numbers say, and what the play lead said
///
/// A class holds for a band of zones - `0`-`1`, `2`-`8`, `9`-`16`, `17`-`32`,
/// `33`-`39`, then every five to `90`. That is escalation **every few zones**,
/// which is what the recollection this work started from described. What the
/// mechanism does *not* do is single out the named stages: the boundaries
/// alternate `Sub`/named all the way up, one class step each, nothing skipped.
/// Recorded as corroborated on the first half and not on the second, rather
/// than reshaped to fit either.
///
/// # The sentinel
///
/// Record `0`'s threshold `9999` is unreachable and shares record `1`'s name
/// pointer. `docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`'s
/// fourth pass read that duplicate as the table not making sense and stopped;
/// it is a guard entry, and it is transcribed here as it is stored so the row
/// count matches the executable's own `while (i < 0x11)`.
pub const ZONE_STAGES: &oag_title::ZoneStages = &oag_title::ZoneStages {
    records: &[
        (9999, "MX_CLASS"),
        (90, "MX_CLASS"),
        (85, "M1_9_CLASS"),
        (80, "M1_8_CLASS"),
        (75, "M1_7_CLASS"),
        (70, "M1_6_CLASS"),
        (65, "M1_5_CLASS"),
        (60, "M1_4_CLASS"),
        (55, "M1_3_CLASS"),
        (50, "M1_2_CLASS"),
        (45, "M1_1_CLASS"),
        (40, "M1_CLASS"),
        (33, "AP_CLASS"),
        (17, "A_CLASS"),
        (9, "B_CLASS"),
        (2, "C_CLASS"),
        (0, "D_CLASS"),
    ],
};

/// Wipeout 2048's Zone milestone announcer.
///
/// **The bank-path gate was read, and settled on `speech_zone_NGP.bnk`.** The
/// track-construction function's dispatch (`FUN_812b5890`, `FUN_812b0880` -
/// `docs/ghidra/functions/vita-2048-eu-v104/zone-audio.md`) turns out to
/// check which numbered *pack* the current circuit belongs to, `0` through
/// `4`; `data/audio/sound/speech_zone_NGP.bnk` is what pack `0`'s branch
/// opens, and [`DEFAULT_TRACK`] carries no DLC path prefix, so pack `0` -
/// "no expansion pack" - is what a base-package circuit like Altima resolves
/// to. `data/audio/DLC1/speech_zone.bnk` is the other live path this same
/// dispatch reaches, for a track that does belong to a pack; this port does
/// not select between the two per-circuit yet, the same one-value-per-title
/// simplification [`SOUND_BANKS`]' own doc comment already accepts for
/// `shipHD.bnk`.
///
/// **The milestone ladder is the same string-table evidence it always was.**
/// Fifteen `zone_N` cues sit in the executable, `5` through `100`, the same
/// numbers Wipeout HD's own `speech_zone.bnk` carries - and despite an
/// exhaustive sweep of every reader of the Zone speech bank handle
/// (`DAT_818c4df4`) in this pass, none of them turned out to be the function
/// that walks this specific table. So this is on the same footing Pure's and
/// HD's own ladders are: real data, no traced call site on *this* title's
/// executable - not a step below them, a step *above* 2048's bank path used
/// to be, which had no decompiled dispatch behind it at all until this pass.
pub const ZONE_ANNOUNCER: &oag_title::ZoneAnnouncer = &oag_title::ZoneAnnouncer {
    bank: r"Data\audio\sound\speech_zone_NGP.bnk",
    milestones: &[5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 60, 70, 80, 90, 100],
    tick: oag_title::SequenceTick::Unknown,
};

/// Where this title keeps the **tuning** for the roster [`SHIP_DIR`] holds the
/// models of.
///
/// A second directory, and 2048 is the only title that needs one: its own five
/// teams keep their models under [`SHIP_DIR`] and their `handlingstats.xml`
/// here, one tree apart. Its HD-derived roster keeps both together under
/// `Data\art\published\hdships\<Team>\`, the way every other title does -
/// so the split is a fact about *this roster*, not about the title.
pub const HANDLING_DIR: &str = r"Data\HandlingStats";

/// Where this title keeps the `.pob` effects a race plays.
///
/// The executable builds an effect's path in `FUN_812a6a2e` (v1.04 eboot): a
/// game-mode id **below 23** gets `Data/Particles/%s` (HD's authoring), any
/// other id `Data/Particles2048/%s`. A campaign or multiplayer event's id is
/// `FUN_810016ba(name)` of its own `m_name` (`FUN_812b0d52`, called from
/// `CampaignEventCard_HandleInput`); a name outside the 23-entry table
/// (`Arcade`, `Zone`, ...) falls to a CRC-32, and none of the 577 instance names
/// in `SP.xml` and `MP.xml` is in the table or hashes below 23. So **2048's own
/// events read `Data/Particles2048`**, which is this directory. The named
/// modes (HD-lineage content) read `Data/Particles`; this port plays none of
/// them, so nothing here reaches that side. Confidence 70: the launch path is
/// read, whether a later writer overwrites `state+0xe4` was not walked
/// (`docs/ghidra/functions/vita-2048-eu-v104/particle-paths.md`).
pub const EFFECT_DIR: &str = r"Data\Particles2048";

/// Where this title keeps its roster - **the one axis 2048 forced into
/// existence**.
///
/// `Data\Ships\<Team>\` is what Pulse, Pure and Wipeout HD all spell, and
/// [`oag_title::race::SHIP_DIR`]'s own docs used to argue it was shared
/// vocabulary rather than a title fact. 2048 disagrees: its fourteen
/// HD-derived teams are at `Data\art\published\hdships\<Team>\`, carrying
/// `ship.vex`, `handlingstats.xml`, `shipshield.vex`, `Engineflare.vex`,
/// `Locators.vex` and four LODs each - the same file set under the same names,
/// one directory up and three deep.
///
/// **2048's own five teams**, one directory each: `ag_systems2048`,
/// `auricom2048`, `feisar2048`, `piranha2048` and `qirex2048`, each holding
/// four numbered craft - see [`SHIP_TYPES`]. Every one of the twenty carries a
/// `Ship.vex` and a `ship.rcsmodel`, checked against the manifest.
///
/// The team id this build hands to the loader is therefore **two levels**,
/// `feisar2048\3` rather than `feisar2048`, which the path template joins with
/// a separator exactly as it joins any other id. That works because the id is
/// opaque to everything that composes a path with it.
///
/// **What it costs is the HD-derived roster** - see [`HD_SHIP_DIR`].
pub const SHIP_DIR: &str = r"Data\art\published\Ships";

/// Where 2048's **HD-derived** twelve teams keep both their models and their
/// tuning, the way every other title keeps a roster.
///
/// Not [`SHIP_DIR`] - a second tree, reached per-team through
/// [`GUEST_ROSTER`] rather than through [`oag_title::RaceDefaults::ship_dir`],
/// which holds one string and cannot address both of this title's rosters at
/// once. Every file is here - `Ship.vex`, `ship.rcsmodel`,
/// `handlingstats.xml`, `Engineflare.vex`, four LODs - for all twelve teams
/// and their `_c1`/`_n1` Fury variants; see [`GUEST_TEAM_VARIANTS`] for the
/// twelve spelled out and confirmed against the manifest.
pub const HD_SHIP_DIR: &str = r"Data\art\published\hdships";

/// The directory under [`HD_SHIP_DIR`] Zone mode's one hull sits in, for
/// every craft: [`oag_title::ZoneCraft::OwnShipAt`].
pub const ZONE_SHIP: &str = "Zone";

/// What the numbered directory under a native team selects: **the four craft
/// each of 2048's five teams flies**.
///
/// Recovered from the liveries each one ships, which name their own type:
/// `qirex2048\1\Textures_1\Qirex_Fighter_Livery.gxt`, then `_Agility_`,
/// `_Speed_` and `_Prototype_` in order. Confirmed on two teams independently,
/// and corroborated by the front end's own locked-ship art
/// (`data/FE/NewImages/lockedships/locked_Feisar2048_speed.gxt` and its three
/// siblings) - which is the same four names from a different direction.
///
/// **Feisar spells the first one `Combat` and Qirex spells it `Fighter`**, on
/// the same slot; the game's own word for it is fighter. Confidence **90**: two
/// independent naming schemes on the disc agree on the set and the order, and
/// nothing has been watched running.
pub const SHIP_TYPES: [&str; 4] = ["fighter", "agility", "speed", "prototype"];

/// [`SHIP_TYPES`]' own labels, as string-table ids - **confirmed on the
/// disc**, `english/entries.xml`: `FE_SHIP_COMBAT` -> `"FIGHTER"`,
/// `FE_SHIP_AGILITY` -> `"AGILITY"`, `FE_SHIP_SPEED` -> `"SPEED"`,
/// `FE_SHIP_PROTO` -> `"PROTOTYPE"` - the same four words in the same order,
/// this title's own name for the axis rather than a label invented for the
/// front end's Team screen. Confidence 92, the disc's own string table.
pub const SHIP_TYPE_LABELS: [&str; 4] = [
    "FE_SHIP_COMBAT",
    "FE_SHIP_AGILITY",
    "FE_SHIP_SPEED",
    "FE_SHIP_PROTO",
];

/// The five native teams, spelled exactly as `Data\Plugins\teams\Definition.xml`
/// declares their `PI_Team` `location` - which is not [`SHIP_DIR`]'s own
/// lowercase folder names (`feisar2048` against `Feisar2048`). Both spellings
/// resolve against the manifest, which case-folds, the same way
/// [`oag_title::RaceDefaults::team`]'s own doc comment records for HD's
/// `assegai`/`Assegai`. Kept in the plugin's own case here because this is
/// matched against `crate::catalogue::teams`' output, which reads the
/// plugin.
pub const NATIVE_TEAMS: [&str; 5] = [
    "AG_Systems2048",
    "Auricom2048",
    "Feisar2048",
    "Piranha2048",
    "Qirex2048",
];

/// [`oag_title::RaceDefaults::team_variants`] for 2048's native roster - not
/// its HD-derived one, which carries none. See [`SHIP_TYPES`] and
/// [`NATIVE_TEAMS`].
pub const TEAM_VARIANTS: oag_title::TeamVariants = oag_title::TeamVariants {
    teams: &NATIVE_TEAMS,
    variants: &[
        oag_title::TeamVariant {
            suffix: "1",
            label: SHIP_TYPES[0],
        },
        oag_title::TeamVariant {
            suffix: "2",
            label: SHIP_TYPES[1],
        },
        oag_title::TeamVariant {
            suffix: "3",
            label: SHIP_TYPES[2],
        },
        oag_title::TeamVariant {
            suffix: "4",
            label: SHIP_TYPES[3],
        },
    ],
    join: oag_title::VariantJoin::Subdirectory,
};

/// The twelve teams 2048 reships from Wipeout HD/Fury's own roster,
/// spelled exactly as `Data\Plugins\teams\Definition.xml` declares their
/// `PI_Team` `location` - which is the same spelling
/// [`oag_hd::race::TEAMS`](https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/race.rs)
/// carries, confirmed against 2048's own manifest rather than assumed from
/// the name. **Not fourteen** - an earlier doc comment on this file
/// overcounted before all twelve were confirmed one by one; `detonator`,
/// which sits alongside them under [`HD_SHIP_DIR`], is Detonator mode's own
/// craft, not a thirteenth team.
pub const GUEST_TEAMS: [&str; 12] = [
    "AG_Systems",
    "Assegai",
    "Auricom",
    "EGX",
    "Feisar",
    "Goteki",
    "Harimau",
    "Icaras",
    "Mirage",
    "Piranha",
    "Qirex",
    "Triakis",
];

/// [`oag_title::GuestRoster::variants`] for 2048's own copy of Wipeout
/// HD/Fury's roster - **duplicated rather than borrowed**: this crate does
/// not depend on `oag-hd`, so this is 2048's own measurement of the same
/// suffix scheme, not a reference to `oag_hd::race::TEAM_VARIANTS`. Confirmed
/// to resolve identically under [`HD_SHIP_DIR`]:
/// `crates/game/examples/hd_fury_variant_probe.rs`'s sibling reproducer,
/// `team_variants_probe.rs`, covers this title too.
pub const GUEST_TEAM_VARIANTS: oag_title::TeamVariants = oag_title::TeamVariants {
    teams: &GUEST_TEAMS,
    variants: &[
        oag_title::TeamVariant {
            suffix: "",
            label: "HD",
        },
        oag_title::TeamVariant {
            suffix: "_c1",
            label: "Fury Concept",
        },
        oag_title::TeamVariant {
            suffix: "_n1",
            label: "Fury Nitro",
        },
    ],
    join: oag_title::VariantJoin::Suffix,
};

/// [`oag_title::RaceDefaults::guest_roster`] for this title: the twelve
/// HD-derived teams, under [`HD_SHIP_DIR`], on [`GUEST_TEAM_VARIANTS`]' own
/// terms.
pub const GUEST_ROSTER: oag_title::GuestRoster = oag_title::GuestRoster {
    dir: HD_SHIP_DIR,
    handling_dir: None,
    variants: &GUEST_TEAM_VARIANTS,
    // `docs/formats/2048-status.md`: 2048 reships HD/Fury's twelve teams under
    // a tree of its own. Spelled out rather than read from `oag-hd`, which a
    // title package does not depend on (a test pins it to `oag_hd::TITLE.name`).
    reships: "Wipeout HD",
    alongside_label: None,
    origin: oag_title::Origin::Measured,
};

/// The circuit a race loads when the caller names none.
///
/// **Altima, the circuit 2048's own campaign opens on**, and the one whose
/// `WO Track` payload the version `0x107` control-point layout was recovered
/// against - 6 paths, 4 junctions, 2,029 control points, `encoded_len()`
/// landing on the payload length to the byte. See `docs/formats/track.md`.
///
/// Its drivable geometry is another matter: `track.rcsmodel` is 17.4 MiB of a
/// container this build cannot decode, so a race here comes up as the derived
/// ribbon rather than the authored surface, exactly as a Wipeout HD race does.
/// The per-stage colour grade a Zone race climbs, one copy per circuit.
///
/// **Per-circuit rather than title-wide**, and the reason
/// [`oag_title::ZoneCircuit::SameCircuit`] above reads the way it does: this
/// file sits beside the `track.vex` of whichever ordinary circuit the player
/// picked, which only makes sense if Zone runs on that circuit. All ten base
/// circuits ship a byte-identical 43,812-byte copy naming thirteen stages -
/// HD's fifteen with `Sub Venom`/`Venom` dropped - checked against the disc in
/// `effectsettings_ground_truth.rs`.
///
/// **The title-wide `ZoneMode2048default.effectSettings` this title's own
/// loader also reaches for does not exist on any of the three packages**, nor
/// do the four `ZoneEnvironmentHDFury\*` paths its DLC fallback names; see
/// `docs/formats/effectsettings.md`. So this is the only copy there is.
pub const ZONE_PALETTE: &str = "ZoneMode2048.effectSettings";

pub const DEFAULT_TRACK: &str = r"Data\art\published\environments\altima\track.vex";

/// The team whose `handlingstats.xml` a race uses by default.
///
/// **Feisar's speed craft, and this is the one default that deliberately does
/// *not* match the rest of the lineage.** Pulse, Pure and Wipeout HD all
/// default to Assegai so that a race is directly comparable with the PPSSPP
/// captures under `data/traces/`, every one of which was flown with it. There
/// is no 2048 capture to match, and Assegai on this title is a *ported* craft:
/// defaulting to it would mean a Wipeout 2048 race that flies a Wipeout HD
/// ship, which is the wrong first impression of what this title is.
///
/// Feisar because it is the lineage's starter team, and `3` because that is
/// the speed craft - see [`SHIP_TYPES`] for how the numbering was recovered.
pub const DEFAULT_TEAM: &str = r"feisar2048\3";

/// Where each race cue lives.
///
/// **Measured off the manifest rather than inherited**, the way
/// [`oag_hd::race::SOUND_BANKS`] was: the archive holds 29 distinct `.bnk`
/// names and, like HD, **no `hud.bnk`** - so `SPEEDUPPAD` is looked for in
/// `weapons.bnk` here too. The ship bank is `shipHD.bnk`, HD's spelling
/// exactly, and there is a `Ship_NGP.bnk` and a `Ship_NGP_Zone.bnk` beside it
/// that are presumably 2048's own ("NGP" being the Vita's development name).
///
/// **Which of the two pairs the original loads is unread**, and this points at
/// HD's, because HD's is the one whose cue strings this build has actually
/// decoded. That is a choice made for a reason, not a measurement; a race
/// reports every cue it cannot find.
///
/// [`oag_hd::race::SOUND_BANKS`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/race.rs
pub const SOUND_BANKS: &oag_title::SoundBanks = &oag_title::SoundBanks {
    hud: r"Data\audio\sound\weapons.bnk",
    ship: r"Data\audio\sound\shipHD.bnk",
    ship_zone: r"Data\audio\sound\shipHD.bnk",
    weapons: r"Data\audio\sound\weapons.bnk",
    speech: r"Data\audio\sound\speech.bnk",
    // `None` for the reason HD's is, and 2048 follows HD everywhere in this
    // table. See [`oag_hd::race::SOUND_BANKS`].
    track_general: None,
    // The engine does not play out of `shipHD.bnk`. The five `<team>2048`
    // tables name their layers by cue index, and `Ship_NGP.bnk` is the only
    // bank where those indices bind looping waveforms; Zone's three tables
    // resolve the same way in `Ship_NGP_Zone.bnk`. The executable's loader
    // (`FUN_8121bce2`) names both banks and `FUN_81263f6c` builds the table
    // names. `docs/formats/2048-xfx.md`.
    crossfade: Some(oag_title::Crossfade {
        ship: r"Data\audio\sound\Ship_NGP.bnk",
        ship_zone: r"Data\audio\sound\Ship_NGP_Zone.bnk",
        zone_infix: Some("ZONE_"),
    }),
};

#[cfg(test)]
mod zone_stage_tests {
    use super::ZONE_STAGES;

    /// The table as the executable stores it: seventeen records, the first a
    /// sentinel, the last matching zone `0` so the walk can never fall off the
    /// end during a real race.
    #[test]
    fn the_table_is_the_seventeen_records_the_executable_holds() {
        assert_eq!(ZONE_STAGES.records.len(), 17);
        assert_eq!(ZONE_STAGES.records[0], (9999, "MX_CLASS"));
        assert_eq!(
            ZONE_STAGES.records[1].1, "MX_CLASS",
            "record 0 is a sentinel"
        );
        assert_eq!(ZONE_STAGES.records[16], (0, "D_CLASS"));
        // Sixteen distinct names across seventeen records.
        let mut names: Vec<_> = ZONE_STAGES.records.iter().map(|r| r.1).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 16);
    }

    /// Strictly descending below the sentinel, which is what makes
    /// "first record the zone has reached" well defined.
    #[test]
    fn the_thresholds_descend() {
        for pair in ZONE_STAGES.records.windows(2) {
            assert!(pair[0].0 > pair[1].0, "{pair:?} is not descending");
        }
    }

    /// `0x11 - i`, read off `Hud_UpdateZoneSpeedClassWidget` at `0x81197d6c`.
    #[test]
    fn a_zone_maps_to_its_records_index_counted_from_the_bottom() {
        assert_eq!(ZONE_STAGES.stage_for(0), Some(1));
        assert_eq!(ZONE_STAGES.stage_for(1), Some(1));
        assert_eq!(ZONE_STAGES.stage_for(2), Some(2));
        assert_eq!(ZONE_STAGES.stage_for(8), Some(2));
        assert_eq!(ZONE_STAGES.stage_for(9), Some(3));
        assert_eq!(ZONE_STAGES.stage_for(17), Some(4));
        assert_eq!(ZONE_STAGES.stage_for(33), Some(5));
        assert_eq!(ZONE_STAGES.stage_for(40), Some(6));
        assert_eq!(ZONE_STAGES.stage_for(70), Some(12));
        assert_eq!(ZONE_STAGES.stage_for(90), Some(16));
        // The sentinel is reachable only by a zone number no run reaches.
        assert_eq!(ZONE_STAGES.stage_for(9999), Some(17));
    }

    /// The class name and the stage come off the same record, so a report
    /// naming one cannot disagree with the picture drawn from the other.
    #[test]
    fn the_class_name_comes_off_the_same_record_as_the_stage() {
        assert_eq!(ZONE_STAGES.class_for(0), Some("D_CLASS"));
        assert_eq!(ZONE_STAGES.class_for(2), Some("C_CLASS"));
        assert_eq!(ZONE_STAGES.class_for(17), Some("A_CLASS"));
        assert_eq!(ZONE_STAGES.class_for(33), Some("AP_CLASS"));
        assert_eq!(ZONE_STAGES.class_for(40), Some("M1_CLASS"));
    }

    /// **The escalation is every few zones, not every zone** - the half of the
    /// play-based lead this session started from that the numbers corroborate.
    /// The other half ("especially the named ones") they do not: every band
    /// boundary is one class step, alternating `Sub`/named all the way up, with
    /// nothing skipped and nothing singled out.
    #[test]
    fn a_class_holds_for_a_band_of_zones() {
        let boundaries: Vec<u16> = (0..=90u16)
            .filter(|&zone| {
                zone == 0 || ZONE_STAGES.stage_for(zone) != ZONE_STAGES.stage_for(zone - 1)
            })
            .collect();
        assert_eq!(
            boundaries,
            vec![0, 2, 9, 17, 33, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90],
        );
    }
}

#[cfg(test)]
mod effect_dir_tests {
    #[test]
    fn this_title_reads_its_own_particle_directory_not_the_shared_one() {
        assert_eq!(super::DEFAULTS.effect_dir, r"Data\Particles2048");
        assert_ne!(super::DEFAULTS.effect_dir, oag_title::race::EFFECT_DIR);
    }
}
