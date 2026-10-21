//! What a Pure race loads: the default circuit and team, and how a ship's model
//! entry is spelled.
//!
//! Names and defaults, in the [ADR-0022] sense, and the counterpart to
//! `oag_pulse::race`. Everything here was probed against `pure-psp-eu.chd` on
//! 2026-08-12 by asking the archive for the name and seeing whether an entry
//! answered; confidence **94**, since a name that resolves to a decodable entry
//! of the right shape is about as direct as this gets short of watching the
//! original load it.
//!
//! # Pure's per-craft models are its own, and there are five of them
//!
//! **This section said "there is no `ships` module here" until 2026-09-06**, on
//! the strength of probing Pulse's five names against this disc and finding one.
//! That probe was right about Pulse's names and wrong about the conclusion: Pure
//! spells four models Pulse has no counterpart for, so the miss was a vocabulary
//! gap rather than an absence. See [`ships`], and the table below for what the
//! Pulse-name probe actually established.
//!
//! | Pulse name | On Pure |
//! | --- | --- |
//! | `Data\Ships\<Team>\Ship.vex` | **present** - and so are four models Pulse has no name for |
//! | `Data\Ships\<Team>\shipboost.vex` | absent, and **this title ships no boost-plume asset at all** - see [`ships`] |
//! | `Data\Ships\<Team>\Zone.vex` | absent - **and explained**, see below |
//! | `Data\Ships\<Team>\Zoneboost.vex` | absent, for both reasons at once |
//! | `<environment>\track_reversed.vex` | absent, on every circuit |
//! | `<environment>\zone_track.vex` | absent - **and explained**, see below |
//!
//! The `track_reversed` row is corroborated by the disc's own plugin definition,
//! which carries **no `Reversed` attribute on any `PI_Track`** - so Pure ships no
//! reversed circuits at all, where a third of Pulse's menu is them.
//!
//! **Two of these rows stopped being holes on 2026-08-19**, and the reason is
//! the same for both: Zone is *shaped differently* on this title rather than
//! spelled differently. Pulse hangs Zone off the race circuit and the player's
//! own team - one extra file in each directory. Pure declares Zone's circuits
//! and Zone's craft as first-class entries in its plugin definition, `type="Zone"`
//! against the `type="Race"` everything else carries: four circuits under
//! `Data\Zone\`, and one team, `Data\Ships\Zone_01`, whose `handlingstats.xml`
//! opens `<Stats team="ZoneMode">`. So `Zone.vex` and `zone_track.vex` are
//! absent here not because their spelling is unrecovered but because this disc
//! has nowhere to put them. See [`DEFAULT_ZONE_TRACK`] and [`ZONE_TEAM`].
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
    // `false`: Pure's own menu offers exactly its four `type="Zone"`
    // circuits and nothing else, checked directly against the disc in
    // `pure_declares_zone_circuits_of_its_own_and_no_prefixed_file`. Unlike
    // HD, there is no play-based lead suggesting ordinary circuits belong
    // here too.
    zone: oag_title::ZoneCircuit::Separate(ZONE_TRACKS, false),
    zone_craft: oag_title::ZoneCraft::OwnShip(ZONE_TEAM),
    // Measured absent, not merely unfound: Pure's own executable composes no
    // boost-plume path template and no candidate name resolves against
    // `Data.wad` on either pressing, at confidence 93 -
    // `docs/ghidra/functions/psp-pure-usa/ship-models.md`. The always-on
    // `Engine Flare` billboard still grows when boosting - Pure's own
    // `Exhaust_Update` carries the identical `boost_timer * 8.0` half-size
    // term on both pressings (confidence 88,
    // `docs/ghidra/functions/psp-pure-usa/exhaust-sound.md`) - it just has no
    // separate model to reveal alongside it.
    boost: None,
    sounds: SOUND_BANKS,
    zone_announcer: Some(ZONE_ANNOUNCER),
    // Measured on PSP USA, Time Trial (2026-10-19): see `COUNTDOWN_VOICE`.
    countdown_voice: Some(COUNTDOWN_VOICE),
    launch_hover: None,
    hover_rig: None,
    zone_hover_rig: None,
    zone_handling: None,
    craft_laws: Some(&CRAFT_LAWS),
    autopilot_engine: oag_title::autopilot_engine::MEASURED,
    opponent_autopilot: oag_title::opponent_autopilot::BY_CLASS_AND_DIFFICULTY,
    // The campaign's opponents fly Pure's own sibling driver
    // ([`ORIGINAL_DRIVER`]); every other mode keeps this project's.
    original_opponent: true,
    original_driver: Some(&ORIGINAL_DRIVER),
    // Pure's own `speech_zone.bnk` was listed alongside Pulse's and HD's -
    // see `docs/formats/psp-audio.md`'s bank table - and names no `MR_*`-shaped
    // speed-class cues at all, only the numbered ladder and a medal set.
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
    zoom_ring: None,
    borrows_zoom_ring: true,
    zone_stage_textures: None,
    // `None` for the same reason as Pulse's.
    zone_sky: None,
    // Pure authors one roster and no numbered variant of anything in it.
    team_variants: None,
    guest_roster: None,
    // Nor a second hull file: `Data\Plugins\PI001\Definition.xml` authors no
    // `PI_TeamModel` at all - measured, zero occurrences. See
    // `crates/game/examples/pulse_variant_probe.rs`, which the constant
    // Pulse's own `HULL_VARIANTS` cites is the same probe run against this
    // title's own disc.
    hull_variants: None,
    // **Five**, and this is the disagreement that licenses the axis at all.
    // Measured 2026-09-05 on `pure-psp-eu.chd`: seven of Pure's eight race
    // teams' `Data\Ships\<Team>\handlingstats.xml` were read directly and
    // every one authors five `<Class>` rungs with `VECTOR` among them, where
    // Pulse's author four. `crate::race::handling_stats`' own doc comment
    // records the wider sweep - ten of eleven ship directories, the eleventh
    // being `Zone_01`, which authors no `<Class>` ladder at all.
    //
    // Pure's front end agrees from a completely separate file: it authors five
    // `<Menu name="Class">` entries on its own `Class Selection` screen, with
    // a per-class stat graph for each. See `docs/formats/race-setup.md`.
    //
    // `VECTOR` is listed because Pure authors it; it is not *offered* yet,
    // because nothing downstream can name it. See
    // `oag_title::SpeedClasses::VECTOR`, which is where that is explained and
    // where the filter lives.
    fresh_variant: None,
    speed_classes: Some(oag_title::SpeedClasses {
        names: oag_title::SpeedClasses::PURE_LADDER,
        remix_only: &[oag_title::SpeedClasses::VECTOR],
    }),
};

/// Pure's Zone milestone announcer.
///
/// `Data\Sound\speech_zone.bnk` names ten numbered cues, on both the USA and EU
/// discs - `oag-wad sounds 'data/images/pure-psp-eu.chd:PSP_GAME/USRDIR/Data.wad'
/// --bank vo`. **A different ladder from Pulse's**, not the same file re-read:
/// Pure stops naming every five zones after 30 and jumps to 75, and carries a
/// `zone_bronze`/`silver`/`gold` medal set and a `ship_destroyed` cue this port
/// does not read. See
/// `docs/formats/psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title`.
/// No call site has been read on Pure's own executable - this ladder is
/// attributed by the same three-titles-and-no-hole standing
/// [`oag_title::ZoneCraft`] already has, not by a traced `Zone_Update`.
pub const ZONE_ANNOUNCER: &oag_title::ZoneAnnouncer = &oag_title::ZoneAnnouncer {
    bank: r"Data\Sound\speech_zone.bnk",
    milestones: &[5, 10, 15, 20, 25, 30, 40, 50, 75, 100],
    tick: oag_title::SequenceTick::Unknown,
};

/// The five models a Pure craft is built from, as leaf names under a team's own
/// directory.
///
/// **Recovered 2026-09-06**, and the counterpart to `oag_pulse::race::ships` -
/// which this module's own docs said for three weeks did not need one. Pure's
/// executable holds all five as `printf` templates in one contiguous `.rodata`
/// block, inside the run of strings belonging to
/// `c:/Work/Wipeout/Code/Backend/Ships/Ship.cpp`:
///
/// | Address (`psp-pure-usa`) | Template |
/// | --- | --- |
/// | `0x08a7a5d4` | `%s\VR\Ship.vex` |
/// | `0x08a7a5e4` | `%s\Phantom.vex` |
/// | `0x08a7a5f4` | `%s\Ship.vex` |
/// | `0x08a7a600` | `%s\Phantom_shipwreck.vex` |
/// | `0x08a7a61c` | `%s\Shipwreck.vex` |
///
/// The `%s` is a `<PI_Team>` `location` out of
/// `Data\Plugins\PI001\Definition.xml` (eleven of them on this disc), not a bare
/// team name, which is why the leaf names here are joined to a location rather
/// than to [`DEFAULT_TEAM`].
///
/// **Confidence 96.** Three independent things agree: the templates are read
/// byte for byte at the addresses above (`read_memory`, not a decompile); every
/// composed name hash-resolves in `Data.wad` on **both** pressings; and each
/// blob that comes back carries its *own* authoring path - `Z:/Data/Ships/
/// Feisar/Phantom.mb` inside `Data\Ships\Feisar\Phantom.vex` - so the payload
/// spells the recovered name back in the exporter's words. Pinned by
/// `crates/pure/tests/ship_models_ground_truth.rs`.
///
/// # There is no boost plume on this disc
///
/// **Pure ships no boost-plume asset, and that is a finding rather than a name
/// still unrecovered.** This module used to say the opposite - "Pure certainly
/// has one, and what it calls that file is unread" - which was an assumption
/// carried from Pulse. Three axes say otherwise, each with its Pulse half
/// alongside so the negative is a measurement:
///
/// 1. **No template.** Pulse composes its plume from `%s\%sboost.vex` at
///    `0x08a84ccc` on `psp-pulse-usa`. Pure's five templates above are the whole
///    block, and none of them is a plume. Searching the entire 3.6 MiB
///    executable case-insensitively for `boost` returns exactly **two** strings,
///    neither a path: `HUD_Perfect Boost!` (`0x08a46398`, a Zone HUD message)
///    and `StartBoostSpeed` (`0x08a79bc8`, a tuning key in the AI stat block
///    beside `LeadZone`/`TailZone`/`AIThrust`). Same two on the EU pressing.
/// 2. **No entry.** `<location>\shipboost.vex` hash-resolves against none of the
///    eleven declared locations, on either pressing, while the same probe
///    resolves on Pulse.
/// 3. **No anchor.** Pulse's `Data\Ships\Feisar\Ship.vex` carries `boost_flare`
///    and `boost_flare1` locator nodes - the anchors
///    `docs/ghidra/functions/psp-pulse-usa/exhaust.md` documents the plume
///    riding. Pure's carries `engine_flare` and no node naming `boost` at all.
///
/// **Settled 2026-09-23: Pure's boost is not visually inert.** The asset is
/// absent; the *effect* is not. Pure's own `Exhaust_Update` and
/// `Exhaust_UpdateEngineSound` (`docs/ghidra/functions/psp-pure-usa/exhaust-sound.md`,
/// confidence 88, both pressings) carry Pulse's identical
/// `half_size = ((i * 0.6 + 0.4) * 2.5 + boost_timer * 8.0)` term and `0.8s`
/// arm bit for bit, so the always-on `engine_flare` billboard grows on boost
/// exactly as Pulse's does - `oag_fx::exhaust::Exhaust` already
/// reproduces it, title-agnostically. Only the separate `<Team>boost.vex`
/// plume mesh never existed.
pub mod ships {
    /// The hull every mode draws, `%s\Ship.vex` at `0x08a7a5f4`.
    ///
    /// The one name of the five that Pulse spells identically
    /// (`oag_pulse::race::ships::HULL`), which is why the pre-2026-09-06 probe
    /// found it and stopped.
    pub const HULL: &str = "Ship";

    /// The wreckage a destroyed craft leaves, `%s\Shipwreck.vex` at
    /// `0x08a7a61c`.
    ///
    /// On all eleven declared locations. Pulse composes its own from
    /// `%s\%swreck.vex`, a two-part template, so the recovered leaf differs even
    /// though the concept does not.
    pub const WRECK: &str = "Shipwreck";

    /// The Phantom-class hull, `%s\Phantom.vex` at `0x08a7a5e4`.
    ///
    /// **A model swapped in for one speed class**, which **Pulse** does not do -
    /// its own recovered template set is mode-keyed (`Ship`/`Zone`/`shipboost`/
    /// `Zoneboost`/`shipshield`), not class-keyed. **HD and 2048 were not
    /// checked**, so this is a two-title comparison and not a lineage claim.
    /// The identification of *what* `Phantom` is rests on the
    /// title's own vocabulary and not on the word alone: `PHANTOM` is the top
    /// rung of the five [`DEFAULTS`] carries, the executable names
    /// `Unlock Phantom Class` (`0x08a7c848`) and `PhantomStats` (`0x08a79afc`),
    /// and the model's textures are `feis_phantom1_shinemap.tga` and friends -
    /// a separate paint set, not a re-skin of the base hull. Confidence 90.
    ///
    /// **What selects it is mostly read, not fully.** The call site
    /// (`FUN_08927dac`/`FUN_08927694`) reads one flag byte out of a synced
    /// ad-hoc-multiplayer network object (`sceNetAdhoc*` calls confirmed on
    /// the object's own init routine) - not a class comparison - but which
    /// send-side write decides *this* player's own flag before it is
    /// broadcast is still unread. See "`Phantom` is the speed class, and what
    /// selects the model is now partly read" in `ship-models.md`. Nothing
    /// here should be wired to a class comparison on the strength of the
    /// name.
    ///
    /// Present for the **eight core racing teams only** - not `Medievil`, not
    /// `Zone`, not `Zone_01`, all three of which are unlockable or Zone-mode
    /// craft. That split is asserted, not assumed, in
    /// `ship_models_ground_truth.rs`.
    pub const PHANTOM_HULL: &str = "Phantom";

    /// The wreckage [`PHANTOM_HULL`] leaves, `%s\Phantom_shipwreck.vex` at
    /// `0x08a7a600`. Same eight teams, same reasoning.
    pub const PHANTOM_WRECK: &str = "Phantom_shipwreck";

    /// `%s\VR\Ship.vex` at `0x08a7a5d4` - the one template with a directory in
    /// it.
    ///
    /// On ten of the eleven declared locations; `Zone_01`, the Zone-mode craft,
    /// has none. Each is a genuinely separate model with its own texture set
    /// (`Data\Ships\<Team>\VR\Textures\vr_<team>_01_shinemap.tga`) sharing one
    /// environment map, `Data\Tex\vr_env.tga`.
    ///
    /// **The name is recovered; what `VR` means is not.** The prefix recurs
    /// across the disc - `Data\Weapons\vr_bomb.vex`, `vr_mine.vex`,
    /// `vr_shield.vex`, `Data\Tex\EngineFlare\vr_engine_noise.mip` - so it names
    /// a whole alternate presentation set rather than one model, but no code
    /// path that selects it has been read. Pulse carries exactly one member of
    /// that set (`%s\vr_shield_cockpit.vex`) and no `VR\` craft directory at
    /// all. Recording the spelling is safe; guessing the mode is not.
    pub const VR_HULL: &str = r"VR\Ship";

    /// `%s\VR\Phantom.vex` at `0x08a7cb64`.
    ///
    /// **Named in the executable and resolving nowhere** - under the same `%s`
    /// the five templates above take. The composed name hash-matches nothing in
    /// `Data.wad`, `FE.wad` or `FEData.wad`, on either pressing.
    ///
    /// **The name reads that absence as "authored and cut", and that is one of
    /// two readings.** This template is in a *different* `.rodata` block from
    /// the five above (`0x08a7cb54`/`0x08a7cb64`, beside `Unlock Phantom
    /// Class`), and the five are known to take a `<PI_Team>` location as their
    /// `%s` only *because they resolve*. Whether this block's `%s` is a team
    /// location at all is unread, so "composed with the wrong argument" is not
    /// excluded.
    ///
    /// Kept either way, because someone re-probing this name later should find
    /// the answer written down instead of re-deriving it. Nothing reads this
    /// constant, and nothing should try to load it.
    pub const VR_PHANTOM_HULL_UNSHIPPED: &str = r"VR\Phantom";
}

/// Where each race cue lives - **Pulse's spelling exactly**.
///
/// Pure's `BOOT.BIN` names only three of these (`frontend.bnk`, `hud.bnk`,
/// `generaltrack.bnk`), but all five hash to real entries on both pressings and
/// every cue the game fires resolves inside them. That agreement is the finding
/// rather than the assumption: this project recorded for months that Pure
/// shipped no `SBlk` bank at all, on the strength of a magic scan run at the
/// wrong offset. See `docs/formats/pure-status.md`.
pub const SOUND_BANKS: &oag_title::SoundBanks = &oag_title::SoundBanks {
    hud: r"Data\Sound\hud.bnk",
    ship: r"Data\Sound\ship.bnk",
    ship_zone: r"Data\Sound\ship_zone.bnk",
    weapons: r"Data\Sound\weapons.bnk",
    speech: r"Data\Sound\speech.bnk",
    frontend: Some(oag_title::FrontEndSounds {
        bank: r"Data\Sound\frontend.bnk",
        cues: oag_title::MenuCues::PULSE,
        // Pure keeps `gold_med`.. in `speech.bnk` and has no place lines; its call sites were not read.
        results: None,
        voices: None,
    }),
    // Resolves on both Pure pressings and holds an `SBlk` whose own label is
    // `GENTRAK` - Pulse's `gentrak` in Pure's upper case. Name resolution
    // against a shipped archive, like the five above; Pure's executable has
    // not been read for it, and whether Pure's circuits author the `.vex`
    // audio classes at all is unswept.
    track: oag_title::TrackBanks {
        shared: &[r"Data\Sound\generaltrack.bnk"],
        circuit: oag_title::CircuitBanks::BesideTrack,
        origin: oag_title::Origin::Measured,
    },
    crossfade: None,
};

/// The circuit a Zone race loads when the caller names none.
///
/// **Pure's Zone circuits are circuits of their own**, and this is the one thing
/// about Zone that this title and Pulse disagree on rather than merely spell
/// differently. Pulse authors a second `.vex` inside each race circuit's own
/// directory (`oag_pulse::race::ZONE_TRACK_PREFIX`); Pure authors four separate
/// environments under `Data\Zone\` and declares them in its own plugin
/// definition as `PI_Track` entries with `type="Zone"`:
///
/// ```xml
/// <PI_Track name="Zone 1">
///   <Values type="Zone" location="Data\Zone\01_Zone"></Values>
/// </PI_Track>
/// ```
///
/// Four of them, `01_Zone` through `04_Zone`, each unlocked by a medal on the
/// one before. Each holds a plain `track.vex` - **no `zone_track.vex` exists
/// anywhere on this disc**, and no race circuit carries a Zone variant either.
/// So there is no mapping from a race circuit to a Zone one to be had, which is
/// why [`oag_title::ZoneCircuit::Separate`] carries a circuit rather than a
/// rewrite rule.
///
/// `01_Zone` because it is the first the definition declares and the only one
/// not gated behind a medal on another - the same "the disc's own ordering is
/// the best available answer" reasoning as [`DEFAULT_TRACK`].
///
/// Confidence **94**, on the same terms as everything else in this module:
/// `Data\Zone\01_Zone\track.vex` resolves to a decodable version-4 `.vex` on
/// `pure-psp-eu.chd`, probed 2026-08-19, and `Data\Zone\01_Zone\zone_track.vex`
/// resolves to nothing.
pub const DEFAULT_ZONE_TRACK: &str = r"Data\Zone\01_Zone\track.vex";

/// The rest of [`DEFAULT_ZONE_TRACK`]'s siblings, on the same terms: `02_Zone`
/// through `04_Zone`, each declared `type="Zone"` beside it and each holding a
/// plain `track.vex`, probed 2026-08-27 against `pure-psp-usa.chd`.
pub const ZONE_TRACK_2: &str = r"Data\Zone\02_Zone\track.vex";
/// See [`ZONE_TRACK_2`].
pub const ZONE_TRACK_3: &str = r"Data\Zone\03_Zone\track.vex";
/// See [`ZONE_TRACK_2`].
pub const ZONE_TRACK_4: &str = r"Data\Zone\04_Zone\track.vex";

/// Every Zone circuit this title has, [`DEFAULT_ZONE_TRACK`] first.
///
/// What [`DEFAULTS`] hands [`oag_title::ZoneCircuit::Separate`]. A caller
/// naming one of these four directly gets it honoured in Zone mode; naming
/// anything else - which is what a menu that has not touched the Circuit row
/// since picking Zone mode always does, since that row is filled with race
/// circuits - falls back to [`DEFAULT_ZONE_TRACK`]. See
/// [`oag_title::ZoneCircuit::variant_of`].
pub const ZONE_TRACKS: &[&str] = &[DEFAULT_ZONE_TRACK, ZONE_TRACK_2, ZONE_TRACK_3, ZONE_TRACK_4];

/// The team a Zone race flies on this title.
///
/// **Recovered 2026-08-19, and it closes the "Pure's Zone hull is unrecovered"
/// item this module's own docs used to carry.** The answer is that Pure does not
/// express the Zone craft the way Pulse does at all. Pulse keeps a second model
/// file inside every team's directory (`Data\Ships\<Team>\Zone.vex`, picked by
/// `Ship_LoadModel`'s `case 6`); Pure declares an entire **team** for it, in the
/// same plugin definition as the circuits:
///
/// ```xml
/// <PI_Team name="Zone_01">
///   <Values type="Zone" location="Data\Ships\Zone_01"></Values>
/// </PI_Team>
/// ```
///
/// `type="Zone"`, where every raceable team is `type="Race"` - so the definition
/// itself says what this team is for. `Data\Ships\Zone_01\Ship.vex` resolves,
/// and the directory's `handlingstats.xml` opens `<Stats team="ZoneMode">`,
/// which is the disc naming the mode in its own words rather than this project
/// inferring it. Confidence **94** for the identification; the two together are
/// about as direct as a name probe gets.
///
/// **Read by [`DEFAULTS`] since 2026-08-19**, through
/// [`oag_title::ZoneCraft::OwnShip`]. It sat here with no reader for exactly as
/// long as HD's answer was unknown - the craft axis then had Pulse and Pure
/// measured and a hole, which is the two-of-three shape [ADR-0022] does not
/// license. HD's turned out to be `/data/ships/zone`, the same shape as this
/// one, so the axis reached three measurements and became a type. The constant
/// did not change; what changed is that there was something to fill in.
///
/// Not to be confused with the `PI_Team name="Zone"` beside it, which is
/// `type="Race"` and is the unlockable *livery* a player earns by taking gold on
/// all four Zone circuits - a raceable team, not the Zone-mode craft.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
pub const ZONE_TEAM: &str = "Zone_01";

/// The circuit a race loads when the caller names none.
///
/// **Vineta K**, the first `PI_Track` the disc's own definition declares and the
/// first circuit of the `Alpha` league - so it is this title's own opening
/// circuit rather than an arbitrary pick, which is the same reasoning that put
/// Talon's Junction in `oag_pulse::race::DEFAULT_TRACK` (there it was the
/// reference-capture circuit; here there are no captures yet and the disc's own
/// ordering is the best available answer).
///
/// Unlike Pulse's, this path is **not** numbered independently of its name:
/// Pure's environment directories carry the circuit's name (`01_Vineta_K`), so
/// the mapping Pulse needs a comment to explain does not arise here.
pub const DEFAULT_TRACK: &str = r"Data\Environments\01_Vineta_K\track.vex";

/// The team whose `handlingstats.xml` and model a race uses by default.
///
/// **Assegai, which is Pulse's default too - and the reason is Pulse's, not
/// Pure's.** It is the team every PPSSPP capture under `data/traces/` was taken
/// with, so keeping it here means the two titles' `--race` defaults stay
/// directly comparable rather than differing in two things at once.
///
/// The only claim being made *about Pure* is that the disc carries the team:
/// `Data\Ships\Assegai\handlingstats.xml` resolves on `pure-psp-eu.chd`,
/// checked 2026-08-12, as do ten other ship directories. Nothing about Assegai
/// is Pure's own preference - its definition declares Feisar first - so change
/// this freely; no measurement is calibrated to it.
pub const DEFAULT_TEAM: &str = "Assegai";

/// Pure's opponent, the sibling Pulse's grew out of, as read off `BOOT.BIN`
/// (`docs/ghidra/functions/psp-pure-usa/ai-controls.md`): the legacy block's
/// cruise throttle (measured on 1,688 of 1,689 live samples) and the ringless
/// planner (209 of 209 live calls) come with [`AiTables::LegacyBlock`]; the
/// rail and the pitch differ from Pulse's as below. The formation fields are
/// never read by the cruise throttle.
///
/// [`AiTables::LegacyBlock`]: oag_title::original_driver::AiTables::LegacyBlock
pub const ORIGINAL_DRIVER: oag_title::original_driver::OriginalDriver =
    oag_title::original_driver::OriginalDriver {
        // Pulse's formation terms: no reader on Pure's legacy path.
        finished_spread: oag_title::pre_race::Sourced::chosen(50.0),
        finished_wander_to_place: oag_title::pre_race::Sourced::chosen(false),
        eliminator_drops_when_behind: oag_title::pre_race::Sourced::chosen(false),
        // `AiManager_Update` (`0x0891b8d0`) increments, then flags; moot under
        // `every_tick`.
        round_robin_pre_increment: oag_title::pre_race::Sourced::measured(true),
        halved_lookahead: &[],
        // `AiManager_SnapshotRacers` flags every racer every frame (measured,
        // eight drivers a frame).
        every_tick: oag_title::pre_race::Sourced::measured(true),
        // The ringless planner's two bumps and Pulse's search, read whole.
        event_ring_rate: oag_title::pre_race::Sourced::measured(false),
        coin_ordered_search: oag_title::pre_race::Sourced::measured(false),
        preferred_follows_choice: oag_title::pre_race::Sourced::measured(false),
        event_ai: None,
        tables: oag_title::original_driver::AiTables::LegacyBlock,
        // `Ai_SnapToLane` (`0x08917b80`) hands `Body_SetPosition` the position
        // alone; the turned velocity stays on the stack.
        rail_turns_velocity: oag_title::pre_race::Sourced::measured(false),
        // `Ai_ComputePitch` (`0x0891892c`) times `FUN_08917744(d, 1030, 1180, 1, 0)`.
        pitch_fade: Some((1030.0, 1180.0)),
    };

/// The eight racing teams every Pure disc carries a full craft set for, and `Medievil`, a Pure
/// team with no counterpart elsewhere. `Zone` and `Zone_01` are craft, not teams.
pub const TEAM_IDENTITIES: oag_title::team::Roster = &[
    oag_title::team::team("AG_Systems", "ag_systems"),
    oag_title::team::team("Assegai", "assegai"),
    oag_title::team::team("Auricom", "auricom"),
    oag_title::team::team("Feisar", "feisar"),
    oag_title::team::team("Harimau", "harimau"),
    oag_title::team::team("Piranha", "piranha"),
    oag_title::team::team("Qirex", "qirex"),
    oag_title::team::team("Triakis", "triakis"),
    oag_title::team::team("Medievil", "medievil"),
];

/// The start-of-race voice's banks: the Zone bank is the one the Zone announcer reads, Eliminator
/// reads the ordinary speech bank.
///
/// **Measured on Pure PSP (USA), Time Trial**, 2026-10-19: `Scream_StartSound` (`0x0883defc`)
/// starts `SPEECH` cue 12 (`ready`) and, 180.0 frames later, cue 13 (`go`), in two runs, and no
/// other cue of the start is a countdown voice; the craft's launch gate flips in the first update
/// after `go`. The same law as Pulse's. Zone and Eliminator were not captured on Pure, so their
/// banks are the title's own names for them. See `docs/ghidra/functions/psp-pure-usa/countdown-voice.md`.
pub const COUNTDOWN_VOICE: &oag_title::CountdownVoice = &oag_title::CountdownVoice {
    eliminator_bank: r"Data\Sound\speech.bnk",
    zone_bank: ZONE_ANNOUNCER.bank,
    origin: oag_title::Origin::Measured,
    tick: oag_title::SequenceTick::Unknown,
    hashed: false,
    event: None,
    beat: None,
};

/// Pure's compiled-in pitch: `Ship_UpdatePitch` (`0x0892caf8`) adds `pitch * 0.1` airborne and
/// `pitch * 0.4` grounded, `Ship_ApplyAngularDamping` (`0x0892cc2c`) damps pitch at `1.5` while
/// racing and `0.6` in the grid state (`3 / c`, `c` the roll coefficient `-2` or `-5`).
/// **Measured live** on Pure PSP (USA), 2026-10-20: input `-100` adds `-10` and `-40`, and the
/// damping reproduces `-1.5 * omega.x` racing and `-0.6 * omega.x` on the grid to `2e-6`
/// (`docs/physics/pure-physics.md`).
pub const CODE_PITCH: oag_title::craft_laws::CodePitch = oag_title::craft_laws::CodePitch {
    air: 0.1,
    ground: 0.4,
    damping: 1.5,
    grid_damping: 0.6,
};

/// Pure's launch boost, compiled in where Pulse authors `<StartBoost>`.
///
/// `Race_UpdateLaunchGrade` (`0x0880f870`) grades each craft's first throttle edge against a
/// clock that starts at GO: before `0.1` s a stall, `0.1`-`0.4` s perfect, `0.4`-`0.5` s a
/// stall, later normal (its last window ends at `1.3`, after the multiplier's). Then
/// `Ship_UpdateStartBoost` (`0x089252e8`) holds `craft+0x24c` at `0.8`, `1.2` or `4.2` until the
/// craft's own state-1 clock reaches `1.0`, and writes `1.0` up to `1.5`: Pulse's shape exactly,
/// with the windows and multipliers as code literals. **Measured live** on Pure PSP (USA),
/// 2026-10-20: a first press at `0.200` s read grade `2` and `4.2` until `1.0` s, then `1.0`; a
/// press held through the countdown read `0.8`. The original applies it to the player's engine
/// only (`0x0892f60c`), and so does ours: the field launches unboosted.
pub const START_BOOST: oag_title::craft_laws::CodeStartBoost =
    oag_title::craft_laws::CodeStartBoost {
        window_start: 0.1,
        window_end: 0.4,
        stall_end: 0.5,
        overall_duration: 1.0,
        stall_mul: 0.8,
        normal_mul: 1.2,
        boost_mul: 4.2,
        player_only: true,
    };

/// Pure's sideshift force, compiled in where Pulse authors `<Airbrake sideshift>`:
/// `Ship_UpdateAirbrakes` (`0x0892f64c`) pushes `right * 200.0` through `Body_AddForceWorld`
/// while a shift's timer runs and the craft is grounded. **Measured live** on Pure PSP (USA),
/// 2026-10-20: `body+0x140` gained `right * 200.000` on every update the shift flag was set.
pub const SIDESHIFT: f32 = 200.0;

/// The craft-body laws. The box mass and the steering ramp are Pulse's, as before this
/// table existed; the in-air `weight_distribution` pitch torque is off: Pure authors no
/// `<Misc weight_distribution>`.
///
/// **Measured on Pure PSP (USA)**, 2026-10-20 (`docs/physics/pure-physics.md`): the engine does
/// not double its output (`Ship_UpdateEngine`, `0x0892f1f0`, live on 374 frames to `3e-6`), its
/// grounded drag is `-0.0025`, half Pulse's (`0x0892cba0`, every grounded frame), and its
/// two-point hover's bank-to-yaw gain is `50`, not `30` (`0x0892dea8`, 129 banked frames). The
/// speed pair, the pitch, the sideshift and the launch are on; the bank-to-yaw gain is held at
/// Pulse's until our AI flies it.
pub const CRAFT_LAWS: oag_title::craft_laws::CraftLaws = oag_title::craft_laws::CraftLaws {
    inertia_mass: oag_title::pre_race::Sourced::inherited(0.9, "Pulse"),
    steer_ramp_clamped: oag_title::pre_race::Sourced::inherited(false, "Pulse"),
    air_pitch_bias: oag_title::pre_race::Sourced::chosen(false),
    // Pure's `Ship_UpdatePitch` (`0x0892caf8`) has no `Ship_TestPitchAlignment` call
    // (`docs/physics/pure-physics.md`).
    pitch_gate: oag_title::pre_race::Sourced::measured(false),
    // No masked read of `craft+0x1c0` against `0x2000` anywhere in Pure USA's image
    // (`psp-relocate.py masked 0x1c0 0x2000`: zero), where Pulse has two.
    eliminator_wrong_way_kill: oag_title::pre_race::Sourced::measured(false),
    // Read, not watched: Pure's sideshift function tests `craft+0x1c0 & 2` (`0x0892b784`) and
    // its engine reads `0x200` four times in Pulse's places, with a pad clock at `craft+0x284`
    // (`docs/ghidra/functions/psp-pulse-usa/craft-flags.md`, "Cross-title").
    sideshift_grid_gate: oag_title::pre_race::Sourced::measured(true),
    turbo_running_laws: oag_title::pre_race::Sourced::measured(true),
    class_gravity_on_flight: oag_title::pre_race::Sourced::inherited(false, "Pulse"),
    // Pulse's world-axis frame: Pure's `Body_Integrate` (`0x089345e8`) builds the angular rate
    // from the body's inertia block with Pulse's own instruction sequence, registers renamed
    // (`docs/physics/pure-physics.md`). Read, on Pure's own executable.
    inertia_rotates: oag_title::pre_race::Sourced::measured(false),
    // Measured: Pure's engine does not double and its grounded drag is `-0.0025`, half Pulse's
    // acceleration at the same top speed (`docs/physics/pure-physics.md`).
    engine_output_double: oag_title::pre_race::Sourced::measured(false),
    drag_ground: oag_title::pre_race::Sourced::measured(-0.0025),
    // Measured `50` (`oag_physics::hover::BANK_TO_YAW_GAIN_PURE`), live on 129 banked frames
    // (`docs/physics/pure-physics.md`). On since 2026-10-21: the original's law is the default,
    // and our AI adapts to it.
    bank_to_yaw: oag_title::pre_race::Sourced::measured(50.0),
    // Pure's four-corner hover (`0x0892e5b8`) loads `50`, as Pulse's does
    // (`oag_physics::hover::BANK_TO_YAW_GAIN_PURE`); static read.
    bank_to_yaw_zone: oag_title::pre_race::Sourced::measured(50.0),
    // Pure's resolver is not read: Pulse's two-sided contact.
    wall_one_sided: oag_title::pre_race::Sourced::inherited(false, "Pulse"),
    wall_steer_away: oag_title::pre_race::Sourced::inherited(false, "Pulse"),
    wall_point_rate_body_local: oag_title::pre_race::Sourced::inherited(false, "Pulse"),
    // Pure's binary is not read for the cage; its own headers clear the flag on all 16 tracks.
    cage_from_track_header: oag_title::pre_race::Sourced::inherited(true, "Pulse"),
    // The player camera update (`0x08849970`) copies the tripod matrix into the camera node and
    // the GE view matrix keeps it: right = forward x up with length `cos(5.71 deg) = 0.995`, up
    // = the craft's own (`docs/ghidra/functions/psp-pure-usa/camera.md`, "The view matrix is a
    // shear"). Read live on PPSSPP at the grid.
    chase_view_sheared: oag_title::pre_race::Sourced::measured(true),
    // Measured (`CODE_PITCH`), in place of Pulse's `<pitch>` stand-in, with its grid damping
    // (`docs/physics/pure-physics.md`).
    code_pitch: Some(oag_title::pre_race::Sourced::measured(CODE_PITCH)),
    // Measured (`SIDESHIFT`): Pure's files author no sideshift, its airbrake code pushes at `200`
    // (`docs/physics/pure-physics.md`).
    code_sideshift: Some(oag_title::pre_race::Sourced::measured(SIDESHIFT)),
    // Measured (`START_BOOST`, the player's engine alone). Its multipliers apply to an engine
    // with no doubling, so it rides on `engine_output_double` (`docs/physics/pure-physics.md`).
    code_start_boost: Some(oag_title::pre_race::Sourced::measured(START_BOOST)),
    // Pure's `FUN_08925054` is Pulse's drawn-lean filter literal for literal (`0.01`, `0.005`,
    // `0.5`/`0.3`, `5.0`, `3.0`), composed at `0x0892718c` as `lean * 0.5 + roll * 6.28`
    // (`docs/ghidra/functions/psp-pure-usa/`); no `<Tilt>`, so Pulse's gains.
    tilt_steer_lean: None,
    tilt_camera_lean: None,
};

#[cfg(test)]
mod tests {
    use super::{COUNTDOWN_VOICE, DEFAULT_TEAM, DEFAULT_TRACK};

    /// Pure's chase view matrix is the sheared one, measured on its GE view and camera tripod;
    /// a title whose camera is not read keeps Pulse's orthonormal look-at.
    #[test]
    fn the_chase_view_is_sheared_and_measured() {
        assert_eq!(
            super::CRAFT_LAWS.chase_view_sheared,
            oag_title::pre_race::Sourced::measured(true)
        );
    }

    /// Pure's engine does not double its output and its grounded drag is `-0.0025`, both
    /// measured live (`docs/physics/pure-physics.md`), and Pure flies them.
    #[test]
    fn the_speed_laws_are_measured_and_on() {
        assert_eq!(
            super::CRAFT_LAWS.engine_output_double,
            oag_title::pre_race::Sourced::measured(false)
        );
        assert_eq!(
            super::CRAFT_LAWS.drag_ground,
            oag_title::pre_race::Sourced::measured(-0.0025)
        );
    }

    /// Pure's two-point hover turns into a bank at `50`, measured live, and Pure flies it.
    #[test]
    fn the_bank_to_yaw_gain_is_measured_and_on() {
        assert_eq!(
            super::CRAFT_LAWS.bank_to_yaw,
            oag_title::pre_race::Sourced::measured(50.0)
        );
    }

    /// Pure integrates its craft with Pulse's world-axis inertia, read off its own
    /// `Body_Integrate`.
    #[test]
    fn the_inertia_frame_is_pulses_world_axis() {
        assert_eq!(
            super::CRAFT_LAWS.inertia_rotates,
            oag_title::pre_race::Sourced::measured(false)
        );
    }

    /// Pure's pitch is four code literals, measured live, and on.
    #[test]
    fn the_pitch_is_measured_and_on() {
        let pitch = super::CODE_PITCH;
        assert_eq!(
            (pitch.air, pitch.ground, pitch.damping, pitch.grid_damping),
            (0.1, 0.4, 1.5, 0.6)
        );
        assert_eq!(
            super::CRAFT_LAWS.code_pitch,
            Some(oag_title::pre_race::Sourced::measured(pitch))
        );
    }

    /// Pure's sideshift pushes at `200`, a code literal measured live, and is on.
    #[test]
    fn the_sideshift_is_measured_and_on() {
        assert_eq!(super::SIDESHIFT, 200.0);
        assert_eq!(
            super::CRAFT_LAWS.code_sideshift,
            Some(oag_title::pre_race::Sourced::measured(200.0))
        );
    }

    /// Pure's launch is its own compiled-in boost, measured live, the player's alone, and on.
    #[test]
    fn the_launch_boost_is_measured_and_on() {
        let boost = super::START_BOOST;
        assert_eq!(
            (
                boost.window_start,
                boost.window_end,
                boost.stall_end,
                boost.overall_duration
            ),
            (0.1, 0.4, 0.5, 1.0)
        );
        assert_eq!(
            (boost.stall_mul, boost.normal_mul, boost.boost_mul),
            (0.8, 1.2, 4.2)
        );
        assert!(boost.player_only);
        assert_eq!(
            super::CRAFT_LAWS.code_start_boost,
            Some(oag_title::pre_race::Sourced::measured(boost))
        );
    }

    /// Pure's start voice is its own measurement, not Pulse's lent: a regression to `Chosen` would
    /// put the "lent" label back on a measured title.
    #[test]
    fn the_start_voice_is_measured_on_pure() {
        assert_eq!(COUNTDOWN_VOICE.origin, oag_title::Origin::Measured);
    }

    /// The circuit is this title's own; the team deliberately is not.
    ///
    /// **Not a style point.** This module exists because `oag_raceplay`
    /// re-exported Pulse's circuit constant, so a Pure race asked the archive
    /// for an environment directory that is not on the disc. The team is the
    /// opposite case and is asserted as such: both discs carry Assegai, so
    /// sharing the default is correct rather than a leftover, and a future
    /// reader should not "fix" it into a divergence.
    #[test]
    fn the_circuit_is_pures_own_and_the_team_is_shared_on_purpose() {
        assert_ne!(
            DEFAULT_TRACK,
            oag_pulse::race::DEFAULT_TRACK,
            "16_Track is on no Pure pressing; if these ever match, someone reached \
             for the other title's table again"
        );
        assert_eq!(
            DEFAULT_TEAM,
            oag_pulse::race::DEFAULT_TEAM,
            "both discs carry Assegai, and sharing it is what keeps the two titles' \
             --race defaults comparable - see this constant's own docs"
        );
    }
}
