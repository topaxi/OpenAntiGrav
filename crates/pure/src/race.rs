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
//! # There is no `ships` module here, and that is a measurement
//!
//! `oag_pulse::race::ships` names five things. **One of them resolves on this
//! disc and it resolves identically**, so it is shared vocabulary rather than a
//! title fact and `oag_game::race` keeps reading Pulse's spelling of it on both
//! sources. Restating it here would be a second copy of one string.
//!
//! The other four find nothing, and their absence is recorded here rather than
//! left to be rediscovered:
//!
//! | Pulse name | On Pure |
//! | --- | --- |
//! | `Data\Ships\<Team>\Ship.vex` | **present** |
//! | `Data\Ships\<Team>\shipboost.vex` | absent |
//! | `Data\Ships\<Team>\Zone.vex` | absent - **and now explained**, see below |
//! | `Data\Ships\<Team>\Zoneboost.vex` | absent |
//! | `<environment>\track_reversed.vex` | absent, on every circuit |
//! | `<environment>\zone_track.vex` | absent - **and now explained**, see below |
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
//! **The boost plume is still genuinely unrecovered**, and the distinction
//! matters: Pure certainly has one, and **what it calls that file is unread**. So
//! nothing here guesses at a spelling, and no constant records the absence
//! either: `oag_game::race::load` reports a boost model it cannot find and
//! carries on, which makes the consequence a race with no plume rather than no
//! race. An absent entry is a fact; a table saying "Pure's boost is called X"
//! would be an invention.
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
    // `false`: Pure's own menu offers exactly its four `type="Zone"`
    // circuits and nothing else, checked directly against the disc in
    // `pure_declares_zone_circuits_of_its_own_and_no_prefixed_file`. Unlike
    // HD, there is no play-based lead suggesting ordinary circuits belong
    // here too.
    zone: oag_title::ZoneCircuit::Separate(ZONE_TRACKS, false),
    zone_craft: oag_title::ZoneCraft::OwnShip(ZONE_TEAM),
    sounds: SOUND_BANKS,
    zone_announcer: Some(ZONE_ANNOUNCER),
    // Searched, not skipped: two independently generated candidate lists
    // resolved no `.effectSettings`-shaped entry on this disc at all - see
    // `oag_title::ZonePalette`. A Zone race here escalates its speed alone,
    // with no colour grade to climb.
    zone_palette: None,
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
};

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
    /// **Not a style point.** This module exists because `oag_game::race`
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
