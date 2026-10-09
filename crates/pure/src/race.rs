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
    countdown_voice: None,
    launch_hover: None,
    hover_rig: None,
    craft_laws: None,
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
        names: &["VECTOR", "VENOM", "FLASH", "RAPIER", "PHANTOM"],
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

#[cfg(test)]
mod tests {
    use super::{DEFAULT_TEAM, DEFAULT_TRACK};

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
