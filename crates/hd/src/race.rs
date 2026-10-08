//! What an HD race loads before it has opened anything: the default circuit and
//! team.
//!
//! The counterpart of `oag_pulse::race` and `oag_pure::race`, and the shortest
//! of the three, because the interesting result is how little needed saying.
//!
//! # Pulse's entry-name spellings resolve here unchanged
//!
//! A PSARC stores real paths where a WAD stores a name hash, so the two look
//! incompatible. They are not, because
//! [`oag_assets::psarc::Archive`](oag_assets::psarc) folds separators, case and
//! a leading `/` before it looks a path up. So
//! `oag_tables::handling::entry_name("assegai")` produces
//! `Data\Ships\assegai\handlingstats.xml`, which normalises to
//! `data/ships/assegai/handlingstats.xml`, which is what the manifest stores -
//! and `handling::GLOBAL_ENTRY`'s `Data\XML\HandlingStats.xml` reaches
//! `/data/xml/handlingstats.xml` the same way.
//!
//! That is why there is no `handling_stats` helper here and no HD-specific
//! spelling of one. Adding it would be a second copy of a string that already
//! works.
//!
//! **The circuit is the one thing that could not carry over**, and not because
//! of the container: HD calls Talon's Junction `talons_junction` where Pulse
//! calls the same circuit `16_Track`. A name, not a path shape.

/// The two things a race needs before it has opened anything.
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
    // `true`: unverified, see `ZONE_TRACKS`'s own docs and
    // `oag_title::ZoneCircuit::Separate`'s - a play-based lead says ordinary
    // HD/Fury circuits belong in the Zone picker too, not just these four.
    zone: oag_title::ZoneCircuit::Separate(ZONE_TRACKS, true),
    zone_craft: oag_title::ZoneCraft::OwnShip(ZONE_SHIP),
    // **Not a claim HD has no boost plume** - HD ships `EF_Boost`, a subtree
    // inside `engineflare.vex` rather than a standalone `.vex`, so there is
    // no per-team stem to name here at all. It loads and draws through
    // `livery::flare::per_team`/`authored_flare`, which this field neither
    // reaches nor gates - see `docs/rendering/trail-ribbon.md`.
    boost: None,
    sounds: SOUND_BANKS,
    zone_announcer: Some(ZONE_ANNOUNCER),
    countdown_voice: None,
    // Read in the craft update and measured on three RPCS3 boots (2026-10-08): the hover target is
    // clamped low through the flyby and countdown and released after the green light. See
    // `docs/ghidra/functions/ps3-hdfury-eu/hover-target.md`.
    launch_hover: Some(&LAUNCH_HOVER),
    hover_rig: Some(&HOVER_RIG),
    zone_class_announcer: Some(ZONE_CLASS_ANNOUNCER),
    // One title-wide table, layered over whichever circuit races - the shape
    // `ZoneCircuit::Separate(_, true)` above already implies, since ordinary
    // circuits are offered in the Zone picker too. `zonemodedlc3` is a second
    // revision this does not point at; what selects it is unread.
    zone_palette: Some(oag_title::ZonePalette::TitleWide(ZONE_PALETTE)),
    // Recovered 2026-08-31, and the writer of `craftArray[n]->+0x640` with it.
    // See `ZONE_STAGES`.
    zone_stages: Some(ZONE_STAGES),
    // Read off `Environment_UpdateStageBlend` and reproduced live - see
    // `ZONE_TRANSITION`.
    zone_transition: Some(ZONE_TRANSITION),
    zone_stage_textures: Some(ZONE_STAGE_TEXTURES),
    zone_sky: Some(ZONE_SKY),
    team_variants: Some(&TEAM_VARIANTS),
    guest_roster: None,
    // HD/Fury's own reskins are `team_variants`' own measured shape - a
    // second directory, not a second file inside one - so there is nothing
    // for this third axis to add. See `oag_title::race::HullVariant`.
    hull_variants: None,
    // Four, the same shape Pulse has and for the same reason. HD's global
    // `handlingstats.xml` authors five `<GlobalClass>` rungs with `VECTOR`
    // first - identical to both Pulse pressings - and **no HD team file
    // carries a `<Class name="VECTOR">`** either. So on the one title whose
    // ladder was supposed to begin at Vector, the per-team shape is Pulse's
    // exactly. See `docs/formats/handling-stats.md`.
    // Chosen, not measured: the definition of "fresh" is this project's. See
    // `FRESH_PROFILE_VARIANT`.
    fresh_variant: Some(oag_title::race::FreshVariant {
        variant: FRESH_PROFILE_VARIANT,
        origin: oag_title::Origin::Chosen,
    }),
    speed_classes: Some(oag_title::SpeedClasses {
        names: oag_title::SpeedClasses::PULSE_LADDER,
    }),
};

/// The twelve teams a Wipeout HD/Fury race can pick, spelled the way
/// `Data\Plugins\PI001\Definition.xml`'s own `PI_Team` locations do -
/// confirmed against the manifest, case-folded the same way
/// [`DEFAULT_TEAM`]'s own lowercase spelling is.
pub const TEAMS: [&str; 12] = [
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

/// Every one of [`TEAMS`] ships three directories, not one - confirmed
/// against the manifest for all twelve:
/// `crates/game/examples/hd_fury_variant_probe.rs`. The unsuffixed one is
/// the classic HD hull; `_c1` and `_n1` are Fury's two reskins - "the
/// concept skin" and "the nitro one" respectively, in the words
/// `race/load.rs`'s own `hd_trail_red` already used for the flag that reads
/// these exact suffixes to decide a craft's trail colour.
///
/// **Not `detonator`** - a thirteenth directory beside these thirty-six,
/// and Detonator mode's own craft rather than a variant of any of the
/// twelve; excluded from [`TEAMS`] and from this table both.
pub const TEAM_VARIANTS: oag_title::TeamVariants = oag_title::TeamVariants {
    teams: &TEAMS,
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

/// The model row Ship Select opens on for a profile that has never chosen
/// one - the `_c1` directory, the `concept1` model.
///
/// **Measured on RPCS3, 2026-09-29, confidence 90**: with `savedata` empty
/// (the first-boot dialogs `EpilepsyWarning` and `FirstPlay` answered), both
/// the Fury campaign's `Team Selection` and Racebox's open on the third
/// `HexSelection` row with `Feisar` reading `080`/`085`/`*`/`080`, which is
/// `concept1`'s `8/8.5/10/8` and not `normal`'s `7/8/10/8`. Two routes, one
/// boot each, on top of the earlier fresh-profile frame. **What is not
/// measured**: a profile that already holds a save opened on `normal` on the
/// same disc, so the original remembers something - what, and where, is
/// unread - and the classic hull cannot be the default everywhere.
/// `oag_game::settings::Race::opening_variant` applies this to a profile
/// that has not picked a model yet, which is **chosen, not measured** as the
/// definition of "fresh" here.
pub const FRESH_PROFILE_VARIANT: &str = "_c1";

/// Wipeout HD/Fury's zone-number to speed-class ladder, read out of the
/// executable.
///
/// # The evidence
///
/// Full working, addresses and a reproduce script in
/// [zone-speed-class-table.md]. In short: `g_ZoneSpeedClassTable`
/// (`0x00860d44`) is fourteen 8-byte records of
/// `{ u32 zoneThreshold, u32 stringIdPointer }` with the thresholds strictly
/// descending to zero, and `Hud_UpdateZoneSpeedClass` (`0x00049718`) walks it
/// from the top and stops at the first record the zone counter has reached.
/// The record count is a literal `14` in the instruction stream
/// (`ZoneSpeedClassTable_Count`, `0x00083490`), so where the run ends is read
/// rather than inferred, and the fourteen string pointers are the only
/// references to those ids anywhere in the image.
///
/// The names below are **string-table ids**, resolved through the language
/// plugin the same way an `idstring` caption is - `MSC_SVENOM` is `SUB-VENOM`,
/// `Venom` is `VENOM`, `IG_HUD_MACH1` is `MACH 1`. That is the same convention
/// [`oag_2048::race::ZONE_STAGES`] uses for its own `MX_CLASS` family.
///
/// # Why it was found, and what it closed on the way
///
/// The search was for the *HUD*: a Zone race names the current speed class
/// beside the zone number, and nothing here could say which class a zone runs
/// at. The walker turned out to end with `stw r3, 0x640(r29)` where `r3` is
/// `14 - i` - **the writer of the per-craft stage field
/// `Environment_UpdateStageBlend` reads**, which
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md` had
/// recorded as unfound and which was the one thing between this port and a Zone
/// race whose colour grade escalates. So HD has the same architecture 2048 does
/// and for the same reason: **the HUD widget drives the grade**.
///
/// `14 - i` lands on `zonemode.effectsettings`' own fifteen rungs exactly -
/// record 13 (`MSC_SVENOM`) on `1 Sub Venom`, record 0 (`IG_HUD_SUPSON`) on
/// `14 Supersonic`, and all twelve between - which is fourteen independent
/// agreements between a table in `.data` and a table in an asset file.
///
/// # Confidence 88, and three checks come from the running game
///
/// The compared value is a float, so the instruction stream alone does not say
/// what it counts. Three observations of the original pin it, on different rows:
/// a Zone frame the maintainer supplied reads `SUB-VENOM` at zone 1 (record
/// 13's band is `[0, 2)`), and the maintainer's own play names **zone 2 as
/// already Venom, explicitly as the exception to "not every zone is a speed
/// class bump"** - record 12's band is the single zone `2`. A second frame, at
/// zone 8, reads `SUB-RAPIER` on the current row and `RAPIER` four rows down on
/// zone `12`, which is record 9's `[7, 12)` band and record 8's threshold
/// exactly. A one-zone band in the middle of a table whose others run 2 to 15
/// wide is not something a wrong reading lands on, and a five-zone band with its
/// upper boundary named is a check the table was written before anyone had.
///
/// # It is not 2048's table
///
/// [`oag_2048::race::ZONE_STAGES`] is seventeen records with a sentinel and
/// bands of `0`-`1`, `2`-`8`, `9`-`16`, then every five. This is fourteen
/// records, no sentinel, and bands of `0`-`1`, `2`, `3`-`4`, `5`-`6`, `7`-`11`,
/// widening to fifteen at the top. Same character, different numbers; neither
/// is a copy of the other.
///
/// [zone-speed-class-table.md]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/ps3-hdfury-eu/zone-speed-class-table.md
/// [`oag_2048::race::ZONE_STAGES`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/2048/src/race.rs
pub const ZONE_STAGES: &oag_title::ZoneStages = &oag_title::ZoneStages {
    records: &[
        (75, "IG_HUD_SUPSON"),
        (60, "IG_HUD_MACH1"),
        (50, "IG_HUD_SUBSON"),
        (42, "IG_HUD_SUPZEN"),
        (35, "MSC_ZEN"),
        (27, "MSC_SPPHANTOM"),
        (20, "Phantom"),
        (16, "MSC_SPHANTOM"),
        (12, "Rapier"),
        (7, "MSC_SRAPIER"),
        (5, "Flash"),
        (3, "MSC_SFLASH"),
        (2, "Venom"),
        (0, "MSC_SVENOM"),
    ],
};

/// How a Zone stage change sweeps the world on this title: a sphere out of the
/// local craft, and a colour weight beside it.
///
/// **Every number is the executable's own**, none authored on the disc and
/// none chosen here. `Environment_UpdateStageBlend` (`0x003da540`) writes
/// `radius = 0.1f` on the commit and copies the start speed from the entry's
/// `+0x0c`; the `.data` image initialises that field to `0.5f` and the
/// acceleration at `+0x14` to `0.1f` (the two developer-only schema keys
/// `Transition start speed` and `Transition acceleration` would overwrite
/// them and no shipped file authors either); the cap is the literal `20000.0f`
/// at TOC slot `0x008b7c24`, and the weight step the literal `0.01f` at
/// `0x008b7c28`. Read at instruction level (confidence 85) and then matched
/// live on RPCS3 at two frame counts and frame to frame (94) - see
/// [zone-effectsettings-loader.md], passes thirty and thirty-one.
///
/// The one thing not measured is the radius's *unit* against this port's own
/// world: the shader compares it to a world-space distance from the craft,
/// and whether the `.rcsmodel` geometry this port draws is in the same units
/// is unverified. It is handed over unscaled.
///
/// [zone-effectsettings-loader.md]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md
pub const ZONE_TRANSITION: &oag_title::ZoneTransition = &oag_title::ZoneTransition {
    start_radius: 0.1,
    start_speed: 0.5,
    acceleration: 0.1,
    radius_cap: 20_000.0,
    weight_step: 0.01,
};

/// The cubemap a Zone race draws in place of the circuit's own `sky.gtf`.
///
/// **A file swap, read out of the loader's control flow rather than inferred
/// from the look.** The per-race environment loader (`0x003f3fb0`) tests one
/// byte and takes one of two branches: `Data/Tex/ZoneSky.gtf` by its own name,
/// or the `sky.gtf` beside the circuit under a path it builds. Both branches
/// call the same loader, store the handle into the same slot and fall into the
/// same sampler-state patch, so nothing downstream distinguishes them - the
/// picture is the only thing that changes. Confidence 84, capped there because
/// the reading is static; full evidence in
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md`.
///
/// **It is a 64x64 cubemap where `01_vineta_k`'s is 2048x2048** - 12,288 bytes
/// of DXT1 against 12,582,912, 1,024 times fewer texels a face - which is the
/// measured form of the maintainer's own observation that a Zone race's sky
/// reads as solid colours or gradients. It
/// is not itself flat: five of its six faces carry hundreds of distinct
/// colours, in the cyan-teal the Zone palette works in.
///
/// **The four Zone arenas' own `sky.gtf` are dead art under this rule.** The
/// branch is either/or, so `zone_1`..`zone_4/sky.gtf` are never what a Zone
/// race shows - which is worth knowing before anyone reaches for them.
pub const ZONE_SKY: &str = "/data/tex/zonesky.gtf";

/// The two fifteen-entry per-stage texture sets a Zone race indexes.
///
/// **Located from the executable's own control flow, not by a name sweep.**
/// `Environment_LoadStageTextures` (`0x003d6dc8`) loads all thirty from a
/// fully-unrolled filename table and stores the handles into two adjacent
/// fifteen-entry arrays; `Scene_PrepareFrame` indexes the first of them for
/// the `zoneTexInner`/`zoneTexOuter` shader parameters, and seven other
/// publishers index the second for the same two. Confidence 82 on the join -
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`.
///
/// Fifteen of each, matching [`ZONE_PALETTE`]'s own fifteen stages. The
/// entries are lower-case on this disc, which is what
/// `oag_assets::Archives::read_name` folds anyway.
pub const ZONE_STAGE_TEXTURES: &oag_title::ZoneStageTextures = &oag_title::ZoneStageTextures {
    general: "/data/tex/zonemode",
    track: "/data/tex/zonemodetrack",
    extension: ".gtf",
    stages: 15,
};

/// Wipeout HD/Fury's Zone milestone announcer.
///
/// `Data\Sound\speech_zone.bnk` (`DATA01.PSARC`) names fifteen numbered cues -
/// every five zones to 50, then every ten to 100 - a third ladder, not Pulse's
/// or Pure's re-read. It also carries a `ready`/`321_GO`/`go` countdown set and
/// the fourteen `MR_*` speed-class lines [`ZONE_CLASS_ANNOUNCER`] wires - see
/// `docs/formats/psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title`.
/// No call site has been read on this title's own executable, the same
/// standing [`oag_title::ZoneCraft`] and [`oag_title::ZoneCircuit`] already
/// have here.
pub const ZONE_ANNOUNCER: &oag_title::ZoneAnnouncer = &oag_title::ZoneAnnouncer {
    bank: r"Data\Sound\speech_zone.bnk",
    milestones: &[5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 60, 70, 80, 90, 100],
    tick: oag_title::SequenceTick::Ps3,
};

/// Wipeout HD/Fury's Zone **speed-class** announcer - the voice line called
/// on a speed-class step, not a zone-count milestone. See
/// [`oag_title::ZoneClassAnnouncer`] for the evidence and what is and is not
/// established about it.
pub const ZONE_CLASS_ANNOUNCER: &oag_title::ZoneClassAnnouncer = &oag_title::ZoneClassAnnouncer {
    bank: r"Data\Sound\speech_class.bnk",
    classes: &[
        "MR_SVE", "MR_VEN", "MR_SFL", "MR_FLA", "MR_SRA", "MR_RAP", "MR_SPH", "MR_PHA", "MR_SUP",
        "MR_ZEN", "MR_SUZ", "MR_Z_SUB", "MR_Z_M1", "MR_Z_SUP",
    ],
};

/// Where each race cue lives, and the one title where two of them move.
///
/// Measured off the disc rather than assumed from Pulse: the seven PSARCs hold
/// 18 distinct `.bnk` names and **`hud.bnk` is not among them**. `SPEEDUPPAD`
/// is in `weapons.bnk` here, and the ship bank is `shiphd.bnk`.
///
/// Spelled `Data\Sound\...` even though a PSARC stores `/data/sound/...`,
/// because [`oag_assets::psarc`] folds case and separators - so one spelling
/// reaches every container in the lineage and no caller branches on the disc.
///
/// **`ship_zone` is `shiphd.bnk` because HD ships no separate Zone ship bank.**
/// Its Zone content is `env0_zone.bnk` and `speech_zone.bnk`, which are a
/// circuit and a voice-over; there is no `shiphd_zone.bnk` to point at, and
/// "the same bank" is the honest answer rather than a fallback.
///
/// **HD has no `~ENGINE`.** Its ship audio is a per-event set - `c_CShipWall`,
/// `c_CShipShip`, `c_GShipShip`, `c_ElecArcA`..`D`, `c_CrackLoopL/C/R` - which
/// is a different design and not a renamed cue, so no field here can supply it
/// and the loader reports the miss. See `docs/formats/psp-audio.md`.
pub const SOUND_BANKS: &oag_title::SoundBanks = &oag_title::SoundBanks {
    hud: r"Data\Sound\weapons.bnk",
    ship: r"Data\Sound\shiphd.bnk",
    ship_zone: r"Data\Sound\shiphd.bnk",
    weapons: r"Data\Sound\weapons.bnk",
    speech: r"Data\Sound\speech.bnk",
    // `frontend.bnk` is the first bank `0x00301338` loads. The names and the
    // events that play them are `docs/ghidra/functions/ps3-hdfury-eu/menu-sounds.md`.
    frontend: Some(oag_title::FrontEndSounds {
        bank: r"Data\Sound\frontend.bnk",
        cues: oag_title::MenuCues {
            up: "navUp",
            down: "navDown",
            left: "navLeft",
            right: "navRight",
            step_left: "navLeft",
            step_right: "navRight",
            accept: oag_title::StyledCue {
                plain: "accept",
                fury: "accept_fury",
            },
            decline: oag_title::StyledCue {
                plain: "reject",
                fury: "reject_fury",
            },
        },
    }),
    // The banks a circuit's nodes spell besides their own, each named by the
    // executable: the sound manager's constructor loads `generaltrack`,
    // `voppler` and `speech_PreRaceChatter` (label `radios`) for the whole
    // session, and the race-bank table holds `crowd.bnk` beside `ShipHD.bnk`
    // (`docs/formats/hd-audio.md`). They sit in `Data\Sound\` with every other
    // HD bank, not in Pulse's or 2048's directory.
    track: oag_title::TrackBanks {
        shared: &[
            r"Data\Sound\generaltrack.bnk",
            r"Data\Sound\voppler.bnk",
            r"Data\Sound\crowd.bnk",
            r"Data\Sound\speech_preracechatter.bnk",
            r"Data\Sound\shiphd.bnk",
        ],
        circuit: oag_title::CircuitBanks::BesideTrack,
        origin: oag_title::Origin::Measured,
    },
    crossfade: None,
};

/// The ship directory a Zone race flies out of.
///
/// **Shaped like Pure's and not like Pulse's**, which is what gave
/// [`oag_title::ZoneCraft`] its third measurement and so its licence to exist:
/// Zone has a ship directory of its own, and the player's team choice does not
/// reach the hull.
///
/// This name was already on the disc's roster before anything wanted it -
/// `zone` is one of the four [`crate::names::MODE_SHIPS`], read off the manifest
/// beside `detonator`, `zone battle` and `test`, and that listing already
/// recorded the shape of the finding: **the mode ships author no `<Class>` block
/// at all** where every team file authors four.
/// `crates/assets/tests/hd_psarc_ground_truth.rs` asserts exactly that and names
/// `zone` as one of the two classless ones.
///
/// `/data/ships/zone/ship.vex` resolves and decodes - 29 meshes, 29,678
/// triangles through its `.rcsmodel` sibling, probed 2026-08-19. Confidence
/// **94** for the name; **nothing about which mode selects it has been read in
/// this executable**, so the identification rests on the directory's name, its
/// classless handling file and its company in `MODE_SHIPS`, exactly as Pure's
/// does.
///
/// Spelled without a leading slash and capitalised the way Pulse spells a team,
/// because it goes through `oag_pulse::race::ships::entry_name` like every other
/// hull and a PSARC folds case and separators - see this module's own docs.
/// `Data\Ships\zone\Ship.vex` normalises onto `data/ships/zone/ship.vex`, which
/// is what the manifest stores.
pub const ZONE_SHIP: &str = "zone";

/// The per-stage colour grade a Zone race climbs, one file for the whole
/// title.
///
/// **Title-wide rather than per-circuit**, which is what makes it usable on an
/// ordinary circuit as well as on the four dedicated ones - the arrangement
/// [`oag_title::ZoneCircuit::Separate`]'s own `also_race_circuits` flag
/// already asserts for this title. 21,160 bytes, fifteen stages named `Start`
/// through `Supersonic`, parsed by `oag_tables::effectsettings` and checked
/// against the disc in `effectsettings_ground_truth.rs`.
///
/// **This is the base revision, not `zonemodedlc3.effectsettings`.** That file
/// is a second, twice-as-large authoring of the same table; what selects
/// between them is unread, so nothing here selects.
pub const ZONE_PALETTE: &str = "/data/environments/zonemode.effectsettings";

/// The circuit a Zone race loads when the caller names none.
///
/// **HD is shaped like Pure here, not like Pulse**: Zone runs on circuits of its
/// own rather than on a second `.vex` inside a race circuit's directory. Four of
/// them, and this build already knew their names without knowing what they were
/// for - `zone_1` through `zone_4` are four of the sixteen entries in
/// [`crate::names::ENVIRONMENTS`], which `crates/hd/tests/hd_title_ground_truth.rs`
/// re-derives from the manifest rather than trusting. Each holds a plain
/// `track.vex`, the same as every other environment on the disc.
///
/// `zone_1` for the reason [`DEFAULT_TRACK`] gives for Talon's Junction inverted:
/// there is no capture to match here, so the disc's own ordering answers.
///
/// **What is *not* claimed** is which of the four a player reaches first, or how
/// they are unlocked. Pure declares that in its plugin definition and HD's
/// track-selection XML has not been read - see [`crate::names::ENVIRONMENTS`],
/// which says the same about the sixteen. This is a name that resolves, nothing
/// more.
pub const DEFAULT_ZONE_TRACK: &str = "/data/environments/zone_1/track.vex";

/// The rest of [`DEFAULT_ZONE_TRACK`]'s siblings, on the same terms.
///
/// **The pairing is a measurement, not an assumption.** `DATA00`'s own
/// `PI_Track` order names `25_Track`..`28_Track` as HD's four Zone circuits -
/// `docs/formats/hd-frontend.md` records that much already - and reading
/// each one's `location` off the disc pairs them in file order:
/// `25_Track`/`Zone_1`, `26_Track`/`Zone_2`, `27_Track`/`Zone_3`,
/// `28_Track`/`Zone_4`, probed 2026-08-27 against
/// `hdfury-ps3-eu-dec.iso`. Each holds a plain `track.vex`, the same as
/// [`DEFAULT_ZONE_TRACK`]'s own.
///
/// **Not `type="Zone"` the way Pure declares its four.** All four of
/// `25_Track`..`28_Track` are `type="Race"`, carrying a separate
/// `zone="true"` flag instead - confirmed 2026-08-28 against
/// `Data\Plugins\frontend\definition.xml`: exactly four `zone="true"`
/// attributes appear in the file, out of 28 `PI_Track` entries total, and
/// they are these four and only these four. So a caller asking this title's
/// definition for `type="Zone"` entries the way it would ask Pure's gets
/// nothing back; [`oag_title::ZoneCircuit::menu_tracks`] asks by this list's
/// own names instead, which is shape-agnostic and works on both.
///
/// **Not generic "Zone" placeholders either.** Read against `DATA06`'s copy
/// of `entries.xml` - the one that names all 28 `PI_Track` ids, see
/// `oag_game::language::CircuitNames`'s own docs - `25_Track`..`28_Track`
/// resolve to four real, distinct track names: Pro Tozo, Mallavol, Corridon
/// 12 and Syncopia. None of the four ever appears with a `reversed="true"`
/// sibling, unlike every one of the other twelve HD/Fury circuits, which is
/// consistent with genuinely Zone-exclusive content rather than a
/// placeholder directory. Confirmed 2026-08-28.
pub const ZONE_TRACK_2: &str = "/data/environments/zone_2/track.vex";
/// See [`ZONE_TRACK_2`].
pub const ZONE_TRACK_3: &str = "/data/environments/zone_3/track.vex";
/// See [`ZONE_TRACK_2`].
pub const ZONE_TRACK_4: &str = "/data/environments/zone_4/track.vex";

/// Every Zone-exclusive circuit this title has, [`DEFAULT_ZONE_TRACK`] first.
///
/// What [`DEFAULTS`] hands [`oag_title::ZoneCircuit::Separate`]. A caller
/// naming one of these four directly gets it honoured in Zone mode; naming
/// anything else falls back to [`DEFAULT_ZONE_TRACK`] **only when it also
/// names none of HD's ordinary race circuits** - `DEFAULTS.zone` wires
/// `also_race_circuits: true`, so a menu naming Anulpha Pass or Moa Therma
/// (both ordinary, neither in this list) gets that circuit honoured too, on
/// a play-based lead rather than disc evidence. See
/// [`oag_title::ZoneCircuit::variant_of`] and [`oag_title::ZoneCircuit::Separate`]'s
/// own docs for what backs `also_race_circuits` and what does not yet.
pub const ZONE_TRACKS: &[&str] = &[DEFAULT_ZONE_TRACK, ZONE_TRACK_2, ZONE_TRACK_3, ZONE_TRACK_4];

/// The circuit a race loads when the caller names none.
///
/// **Talon's Junction, and the reason is that it is the same circuit as Pulse's
/// `16_Track` in the same world coordinates** - checked in
/// `crates/assets/tests/hd_psarc_ground_truth.rs`, which loads both and compares
/// their bounds and start positions. So an HD run and an existing Pulse capture
/// are directly comparable, which is worth more here than picking the disc's own
/// opening circuit would be.
///
/// It is also the circuit this project has read end to end on HD: 826 nodes, an
/// 862-point spline, 400 collision objects, 18 speedup and 9 weapon pads.
///
/// Spelled as the archive stores it, leading slash and all. Any spelling would
/// resolve - see this module's docs - and the stored one is the one a reader can
/// grep the manifest for.
pub const DEFAULT_TRACK: &str = "/data/environments/talons_junction/track.vex";

/// The team whose `handlingstats.xml` a race uses by default.
///
/// **Assegai, as on both PSP titles, and lowercase, as HD spells it.** Sharing
/// the default across all three titles is deliberate: every PPSSPP capture under
/// `data/traces/` was taken with Assegai, so a default that matched would leave
/// the three `--race` runs differing in one thing rather than two.
///
/// The claim about HD specifically is only that the disc carries the team:
/// `/data/ships/assegai/handlingstats.xml` is in `DATA02`, and it parses with
/// Pulse's schema. Nothing about Assegai is HD's own preference.
pub const DEFAULT_TEAM: &str = "assegai";

/// Where Wipeout HD/Fury keeps each weapon's own body model.
///
/// **Every entry is the executable's own string**, not a guess: `strings -a`
/// over `EBOOT.elf` names every one of these under `Data\Weapons\`, and each
/// resolves on `hdfury-ps3-eu-dec.iso` as a `.vex`/`.rcsmodel` pair (checked
/// against the full entry dump, `/data/weapons/<name>.vex` and
/// `/data/weapons/<name>.rcsmodel` both present, lowercase as every PSARC
/// entry is - see `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`).
/// `mesh::rcs::build` is what a PS3 `.vex`'s external-geometry branch needs
/// for all of them - see `oag_raceplay::load::weapon_models`.
pub const WEAPON_MODELS: &oag_title::weapons::WeaponModels = &oag_title::weapons::WeaponModels {
    rocket: Some(r"Data\Weapons\hd_Rocket.vex"),
    mine: Some(r"Data\Weapons\HD_Mine.vex"),
    bomb: Some(r"Data\Weapons\HD_Bomb.vex"),
    // **On HD the name is the truth**: a flash at the craft's `cannon_flash`
    // locator for the round's first 0.1 s, not a body riding the round the
    // way Pulse's same-named file is - see `CANNON_LOOK` below.
    cannon: Some(r"Data\Weapons\hd_muzzleflash.vex"),
    // `PlasmaManager_Update` places this every tick the bolt is live or
    // charging - see `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`.
    plasma_ball: Some(r"Data\Weapons\HD_plasma_ball.vex"),
    plasma_blast_pulse: None,
    // `WeaponExplosions_Start` (`0x00127cd0`) loads these three, in this
    // order, into `+0x54`/`+0x58`/`+0x5c` - see plasma.md's own "detonation"
    // section for the ease each one is scrubbed by.
    plasma_blast_hd: Some(oag_title::weapons::HdPlasmaBlast {
        ring: r"Data\Weapons\HD_plasma_ring.vex",
        sphere: r"Data\Weapons\HD_plasma_sphere.vex",
        halo: r"Data\Weapons\HD_plasma_halo.vex",
    }),
    bomb_blast_pulse: None,
    // `NormalBomb`'s blast object loads these four, the last eight times
    // over - `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`, 2026-10-07.
    // `halo` is the armed bomb's own model, not the blast's; it is drawn while
    // the bomb is laid, not after it trips.
    bomb_blast_hd: Some(oag_title::weapons::HdBombBlast {
        sphere: r"Data\Weapons\HD_bomb_sphere.vex",
        sphere_white: r"Data\Weapons\HD_bomb_sphere_white.vex",
        bloom_ring: r"Data\Weapons\hd_bomb_sphere_bloomring.vex",
        shockwaves: r"Data\Weapons\hd_bomb_shockwaves.vex",
        halo: r"Data\Weapons\HD_bomb_halo.vex",
    }),
    // `MissileManager`'s explosion pool loads this (`0x00154cf0`); entered only
    // when a missile reaches a craft - `weapons.md`, 2026-10-07 (`hd-weapon-fx`).
    missile_blast_hd: Some(r"Data\Weapons\HD_missile_explosion.vex"),
    // Named, not wired - see `oag_raceplay::load::weapon_models`'s own doc
    // comment for what placing this would need and where reading it stopped.
    // HD hands the Repulser out, but its own field-model law is unread.
    repulser_field: None,
    // `EBOOT.elf` carries no `MagEffect` string at all.
    mag_floor: None,
    // `MagstripWake_Construct` (`0x0010a0c0`) and its two texture literals
    // (`0x007a0d38`, `0x007a0d70`); both files ship in `DATA02.PSARC`. See
    // `docs/ghidra/functions/ps3-hdfury-eu/magstrip-wake.md`. Omega's
    // `WEAPON_MODELS` is still `EMPTY` (it does not race); when it does, it names
    // the same pair as `.gnf`.
    magstrip_wake: Some(oag_title::weapons::MagstripWake {
        atlas: r"Data\Tex\HD_electric_arc_8x8.gtf",
        contact: r"Data\Tex\HD_ElectricArc_Contact.gtf",
    }),
    magstrip_pob: false,
    // `RibbonEffects_LoadOpacityRamp` (`0x002c38a8`) names the ramp; the
    // texture is `hd_rockettrail.rcsmaterial`'s `Texture2`, the one its
    // program samples once the frame clock passes 3.3 s, which is all of a
    // race. See `docs/ghidra/functions/ps3-hdfury-eu/rocket-trail.md`. Omega
    // ships the textures but no ramp, so it stays unwired.
    rocket_trail: Some(oag_title::weapons::RocketTrail {
        texture: "Data/RibbonEffects/textures/smoke_trails_frame2_alpha.gtf",
        opacity_ramp: "Data/RibbonEffects/textures/smoke_trails_opacity_ramp.tga",
    }),
    // `leachbeam_triangle.rcsmodel`'s material names the glow and the noise;
    // the live draw binds a 128x128 on unit 0 and a 256x256 on unit 1. See
    // `docs/ghidra/functions/ps3-hdfury-eu/leach-beam-strips.md`, "hd-leach-draw".
    leach_strip: Some(oag_title::weapons::LeachStrip {
        glow: "Data/RibbonEffects/textures/hd_leechbeam_glow.gtf",
        noise: "Data/RibbonEffects/textures/hd_waketrail_clouds.gtf",
    }),
    leachbeam_ball: Some(r"Data\Weapons\hd_leachbeam_ball_bloomring.vex"),
    shuriken: None,
    cannon_look: Some(CANNON_LOOK),
    // No `staticglow` string in HD's EBOOT: it never binds a ghost static.
    ghost_static: None,
};

/// How HD draws a Cannon round, off `CannonBullet`'s own constructor, update
/// and draw - see `docs/ghidra/functions/ps3-hdfury-eu/cannon.md`.
///
/// - **Textures**: the constructor (`0x001322c8`) loads
///   `Data\Weapons\Textures\Cannon_bolt` and `..\Cannon_muzzle_flash`
///   (strings at `0x00784478`/`0x007844a0`, no extension - the PS3 texture
///   loader's own `.gtf`), which resolve on `DATA02` as
///   `/data/weapons/textures/cannon_bolt.gtf`/`cannon_muzzle_flash.gtf`.
///   Confidence 85.
/// - **Bolt**: half-width `0.25` (`0x008aa700`, read at the draw site,
///   `0x00132bac`) against Pulse's `0.35`, and the streak runs the **whole**
///   previous-to-current segment - the draw adds the full difference back
///   with no fraction, where Pulse leaves the near fifth uncovered.
///   Confidence 80.
/// - **Flash**: half-size rolled from `(0.25, 1.0)` every tick of the
///   `0.1` s window (`0x00132210`, bounds `0x008aa700` and a `lis 0x3f80`),
///   against Pulse's `(0.65, 1.3)`. Confidence 80.
/// - **Body**: `hd_muzzleflash` is not a round body on HD. The update
///   (`0x00131478`) re-places it at the craft's `cannon_flash` locator
///   while the round is under `0.1` s old, scaled by the same roll as the
///   flash quad and stretched along Z by a second roll from `(0.7, 1.3)`
///   (`0x008aa670`/`0x008aa6e0`), and hides it after. Confidence 80.
pub const CANNON_LOOK: oag_title::weapons::CannonLook = oag_title::weapons::CannonLook {
    bolt_texture: r"Data\Weapons\Textures\Cannon_bolt.gtf",
    flash_texture: r"Data\Weapons\Textures\Cannon_muzzle_flash.gtf",
    bolt_half_width: 0.25,
    bolt_near_fraction: 0.0,
    flash_size_range: (0.25, 1.0),
    body: oag_title::weapons::CannonBody::Muzzle {
        stretch_range: (0.7, 1.3),
    },
};

/// HD's grid hover: [`oag_title::launch_hover::LaunchHover`].
pub const LAUNCH_HOVER: oag_title::launch_hover::LaunchHover =
    oag_title::launch_hover::LaunchHover {
        // The mean of the original's `3.0 + rand8 * 0.0003`; the mean is chosen.
        grid_cap: oag_title::pre_race::Sourced::chosen(3.03825),
        release_rate: oag_title::pre_race::Sourced::measured(1.0),
    };

/// HD's hover probe set: [`oag_title::hover_rig::HoverRig`].
///
/// Four probes at `(+/-1.5, -1.125, +/-4.5)` (`craft+0xe0..0x110`, read live, and the same four
/// local offsets at `craft+0x1a0..0x1d0` the damper crosses), `0.15` of the load each
/// (`0x008a917c`), all four marched every frame by `Craft_IntegrateHull` (`0x000ef450`), each
/// spring along its hit normal and the normals summed times `0.25` by `Craft_HoverFourPoint`
/// (`0x000ede88`). Confidence 85: `docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md`.
/// HD's probe order (left-front, left-rear, right-front, right-rear in its own frame, whose
/// row 0 points left and row 2 forward) is kept, converted to this engine's frame.
pub const HOVER_RIG: oag_title::hover_rig::HoverRig = oag_title::hover_rig::HoverRig {
    probes: oag_title::pre_race::Sourced::measured(&[
        [-1.5, -1.125, -4.5],
        [-1.5, -1.125, 4.5],
        [1.5, -1.125, -4.5],
        [1.5, -1.125, 4.5],
    ]),
    spring_share: oag_title::pre_race::Sourced::measured(0.15),
    cast_every_probe: oag_title::pre_race::Sourced::measured(true),
    along_hit_normal: oag_title::pre_race::Sourced::measured(true),
    quarter_sum_normal: oag_title::pre_race::Sourced::measured(true),
};

#[cfg(test)]
mod tests {
    use super::*;

    /// **The two Zone tables cannot drift apart.**
    ///
    /// [`ZONE_STAGES`] is indexed by zone and [`crate::hud::ZONE_SPEED_CLASSES`]
    /// by rung, and both carry the same fourteen string ids because both were
    /// read off the same fourteen records. `oag_hud` reads the second and
    /// the colour grade the first, so a drift would put a class name on screen
    /// that disagrees with the palette the circuit is graded with.
    #[test]
    fn the_two_zone_tables_name_the_same_class_at_every_zone() {
        for zone in 0..=120u16 {
            let stage = ZONE_STAGES
                .stage_for(zone)
                .expect("a shipped table ends at 0");
            assert_eq!(
                crate::hud::ZONE_SPEED_CLASSES.id_for(stage),
                ZONE_STAGES.class_for(zone),
                "zone {zone} on rung {stage}"
            );
        }
    }

    /// The fourteen records, as the executable stores them: descending, no
    /// sentinel, ending at zero.
    #[test]
    fn the_ladder_is_the_fourteen_records_the_executable_holds() {
        assert_eq!(ZONE_STAGES.records.len(), 14);
        assert_eq!(ZONE_STAGES.records[0], (75, "IG_HUD_SUPSON"));
        assert_eq!(ZONE_STAGES.records[13], (0, "MSC_SVENOM"));
        for pair in ZONE_STAGES.records.windows(2) {
            assert!(pair[0].0 > pair[1].0, "{pair:?} is not descending");
        }
        // Fourteen distinct names: unlike 2048's, this table has no guard entry
        // sharing a name with its neighbour.
        let mut names: Vec<_> = ZONE_STAGES.records.iter().map(|r| r.1).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 14);
    }

    /// **The two rungs the running game pins.** The reference frame reads
    /// `SUB-VENOM` at zone 1, and the maintainer's play names zone 2 as already
    /// Venom - the one-zone band that rules out one rung per zone.
    #[test]
    fn the_bands_match_what_the_original_shows() {
        assert_eq!(ZONE_STAGES.class_for(0), Some("MSC_SVENOM"));
        assert_eq!(ZONE_STAGES.class_for(1), Some("MSC_SVENOM"));
        assert_eq!(ZONE_STAGES.class_for(2), Some("Venom"));
        assert_eq!(ZONE_STAGES.class_for(3), Some("MSC_SFLASH"));
        assert_eq!(ZONE_STAGES.stage_for(1), Some(1));
        assert_eq!(ZONE_STAGES.stage_for(2), Some(2));
        // `14 - i`, the executable's own arithmetic, tops out on the fifteenth
        // `.effectSettings` rung.
        assert_eq!(ZONE_STAGES.stage_for(75), Some(14));
        assert_eq!(ZONE_STAGES.stage_for(999), Some(14));
    }

    /// The next bump, which is what places the HUD's next-class bar.
    #[test]
    fn the_next_bump_is_the_threshold_above_the_current_band() {
        assert_eq!(ZONE_STAGES.next_zone(1), Some(2));
        assert_eq!(ZONE_STAGES.next_zone(2), Some(3));
        assert_eq!(ZONE_STAGES.next_zone(7), Some(12));
        // The top rung has nothing above it.
        assert_eq!(ZONE_STAGES.next_zone(75), None);
        assert_eq!(ZONE_STAGES.next_zone(999), None);
    }
}
