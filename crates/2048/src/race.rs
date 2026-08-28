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
/// `zone_craft` used to be [`oag_title::ZoneCraft::OwnShip`], reasoning by
/// analogy from Pure and HD's own Zone ship directories: the name it built,
/// `Data\art\published\hdships\Zone\Ship.vex`, sits under [`HD_SHIP_DIR`]
/// rather than this title's native [`SHIP_DIR`], so it never resolved and
/// every Zone race on this title failed to load. 2048 has no dedicated Zone
/// ship at all - Zone flies whichever of the twenty native craft the player
/// picked, exactly as any other mode. See [`oag_title::ZoneCraft::PlayerShip`].
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
    zone: oag_title::ZoneCircuit::SameCircuit,
    zone_craft: oag_title::ZoneCraft::PlayerShip,
    sounds: SOUND_BANKS,
    zone_announcer: Some(ZONE_ANNOUNCER),
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

/// Where 2048's **HD-derived** fourteen teams keep both their models and their
/// tuning, the way every other title keeps a roster.
///
/// Not [`SHIP_DIR`], and that is the limitation worth naming rather than
/// hiding: [`oag_title::RaceDefaults::ship_dir`] holds one string, this title
/// ships two rosters in two trees, and this build races the native one. Every
/// file is here - `Ship.vex`, `ship.rcsmodel`, `handlingstats.xml`,
/// `Engineflare.vex`, four LODs - for all fourteen teams and their `_c1`/`_n1`
/// Fury variants. Reaching them needs the axis to become per-team rather than
/// per-title, which is a change no second title has yet asked for.
pub const HD_SHIP_DIR: &str = r"Data\art\published\hdships";

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
};
