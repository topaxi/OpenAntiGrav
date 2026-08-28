//! What a title's race path needs before it has opened anything.
//!
//! The third axis a second corpus has forced, on the same terms as [`crate::boot`]
//! and [`crate::menu`]: it exists because the composition root would otherwise
//! have to branch on which disc it opened, not because a race obviously needs a
//! table.
//!
//! | | Pulse | Pure |
//! | --- | --- | --- |
//! | default circuit | `Data\Environments\16_Track\track.vex` | `Data\Environments\01_Vineta_K\track.vex` |
//! | default team | `Assegai` | `Assegai` |
//! | Zone's circuit | the race one's directory, `zone_`-prefixed | `Data\Zone\01_Zone\track.vex`, a circuit of its own |
//! | Zone's hull | the player's team, `Zone.vex` | `Data\Ships\Zone_01`, a ship of its own |
//!
//! **Three fields of the four actually diverge**, and the first of them is the
//! reason this type exists at all: `16_Track` resolves on no Pure pressing, so
//! `oag-game`'s `--track` carrying it as a compile-time default made `--race` on
//! a Pure disc fail inside the archive with a message about a name that hashes
//! to nothing. The team is here beside it because a caller that must name a
//! circuit before opening anything must name a team too, not because the two
//! discs disagree - they do not.
//!
//! [`ZoneCircuit`] and [`ZoneCraft`] are the other two, and they are the ones
//! measured across all three titles rather than two; their own docs carry the
//! probes. They split the corpus identically, which is the interesting part:
//! Pulse hangs Zone off the entities a race already has, Pure and HD give Zone
//! entities of its own.
//!
//! # Deliberately four fields
//!
//! Not a home for everything a race loads. Three kinds of thing are kept out,
//! for three different reasons:
//!
//! - **Shared vocabulary.** `Data\Ships\<Team>\Ship.vex` resolves on both
//!   discs, so the hull's file name is not a title fact and a field for it would
//!   be a table with one value in it.
//! - **Unrecovered, not different.** The boost plume is on Pure somewhere; what
//!   that disc calls it is unread. A field would be a `None` designed from one
//!   example, which is the failure [ADR-0009] named and [ADR-0022] does not
//!   license. **The Zone hull was on this list until 2026-08-19 and has come
//!   off it**, which is worth reading as a worked example rather than a
//!   correction: it was excluded while Pulse and Pure were measured and HD was
//!   not, because two measurements and a hole is the shape being refused. HD's
//!   was then measured - `/data/ships/zone/ship.vex`, off the manifest the
//!   `oag_hd::names::MODE_SHIPS` roster already listed - and with three titles
//!   and no hole it became [`ZoneCraft`]. The bar moved because the evidence
//!   did, not because the design changed its mind.
//! - **Absent by construction.** Pure ships no loading-screen wave and no `.mip`
//!   HUD atlas at all. An axis that is `Some` for one title and `None` for the
//!   other is the same one-example design in a different disguise.
//!
//! All three stay as constants in the title crate that knows them, or as an
//! honest report line, exactly as this crate's own module docs say.
//!
//! What is here is the set a caller needs **before** it can open the source: the
//! command line has to name a circuit and a team, and both are per-title. That
//! is the test for this type rather than "is it about racing".
//!
//! [ADR-0009]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0009-multi-game-fanout.md
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The ship directory Pulse, Pure and Wipeout HD all spell identically.
///
/// Not a default so much as the value three of the four titles hold; Wipeout
/// 2048 is the one that does not. See [`RaceDefaults::ship_dir`].
///
/// **Restated in `oag_formats::handling`**, which cannot see this crate - a
/// format reader must not know which title it is reading, so the constant sits
/// on both sides of that boundary rather than being threaded across it. The
/// axis is this field; that one is the spelling its three-title convenience
/// wrapper uses.
pub const SHIP_DIR: &str = r"Data\Ships";

/// Where one title keeps the models a craft is made of.
///
/// The two fields travel together everywhere - a hull path is
/// `dir\<team>\<stem>.vex` and [`ZoneCraft`] is what decides the team and the
/// stem on a Zone run - so they are one value rather than two parameters
/// threaded side by side through six functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipPaths {
    /// See [`RaceDefaults::ship_dir`].
    pub dir: &'static str,
    /// See [`RaceDefaults::zone_craft`].
    pub zone: ZoneCraft,
}

/// The circuit and team a race falls back to on one title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaceDefaults {
    /// Archive entry of the `.vex` a race loads when the caller names none.
    ///
    /// A full entry name rather than a directory, because a circuit is a *file*:
    /// Pulse ships `track.vex` and `track_reversed.vex` side by side and the
    /// menu treats them as two entries.
    pub track: &'static str,
    /// The team id a race uses when the caller names none.
    ///
    /// An **id** - the folder under [`Self::ship_dir`] - and not the name a
    /// player reads, which comes from the string table. See
    /// `oag_game::catalogue`.
    pub team: &'static str,
    /// The archive directory a team's hull, plume, shield and handling stats
    /// all sit under, one subdirectory per team id.
    ///
    /// **An axis because a third title disagreed**, which is the bar
    /// [ADR-0022] sets. This module's own docs argued at length that
    /// `Data\Ships\<Team>\` was shared vocabulary rather than a title fact,
    /// and for Pulse, Pure and Wipeout HD it is - all three spell it
    /// identically. Wipeout 2048 does not: its HD-derived roster lives at
    /// `Data\art\published\hdships\<Team>\`, confirmed against the
    /// manifest, and its own five-team roster somewhere else again.
    ///
    /// Carried here rather than in `oag-formats` because a format crate must
    /// not know which title it is reading; the directory is passed *in* to
    /// [`oag_formats::handling::entry_name_in`] and its siblings.
    ///
    /// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
    pub ship_dir: &'static str,
    /// The archive directory a team's `handlingstats.xml` sits under, one
    /// subdirectory per team id.
    ///
    /// **Equal to [`Self::ship_dir`] on three of the four titles, and on one of
    /// Wipeout 2048's two rosters** - which is exactly why it is a separate
    /// field rather than the same one. 2048's HD-derived craft keep their
    /// models and their tuning together under
    /// `Data\art\published\hdships\<Team>\`; **its own five teams do not**,
    /// keeping models at `Data\art\published\Ships\<team>\<1..4>\` and
    /// tuning at `Data\HandlingStats\<team>\<1..4>\`. One string cannot
    /// address both trees, and folding them would mean picking a roster.
    pub handling_dir: &'static str,
    /// How this title names the circuit a Zone race runs on. See
    /// [`ZoneCircuit`].
    pub zone: ZoneCircuit,
    /// Where this title keeps the craft a Zone race flies. See [`ZoneCraft`].
    pub zone_craft: ZoneCraft,
    /// Which `.bnk` each of this title's race cues is looked up in. See
    /// [`SoundBanks`].
    pub sounds: &'static SoundBanks,
    /// The Zone-mode announcer this title ships, when it has been read off the
    /// disc. See [`ZoneAnnouncer`].
    ///
    /// `None` rather than a guessed ladder for a title whose bank has not been
    /// read - see [`ZoneAnnouncer`]'s own docs for which that is today.
    pub zone_announcer: Option<&'static ZoneAnnouncer>,
}

impl RaceDefaults {
    /// Where this title keeps the models a craft is made of, as one value.
    #[must_use]
    pub fn ships(&self) -> ShipPaths {
        ShipPaths {
            dir: self.ship_dir,
            zone: self.zone_craft,
        }
    }
}

/// Which sound bank a title keeps each race cue in.
///
/// # Why this is an axis rather than a constant
///
/// The banks and the cue strings inside them are **shared across the lineage** -
/// Pulse, Pure and Wipeout HD all carry `SPEEDUPPAD`, `.COLLISIONS`, `ABSORB`,
/// `~SHIELD` and `shieldactive`, spelled identically. What moves is which file
/// they live in, and only on HD:
///
/// | Cue | Pulse and Pure | Wipeout HD |
/// | --- | --- | --- |
/// | `SPEEDUPPAD` | `hud.bnk` | **`weapons.bnk`** - HD has no `hud.bnk` |
/// | `.COLLISIONS` | `ship.bnk` | **`shiphd.bnk`** |
/// | `ABSORB`, `~SHIELD` | `weapons.bnk` | `weapons.bnk` |
/// | `shieldactive` | `speech.bnk` | `speech.bnk` |
///
/// So this is a five-field path table, measured per title, and not a
/// re-derivation of anything. `oag_game::audio::sfx::Cue` maps a cue to a field
/// here; nothing else knows a filename.
///
/// # A missing cue is an ordinary state
///
/// HD has **no `~ENGINE` at all** - its ship audio is a per-event `c_*` set
/// (`c_CShipWall`, `c_ElecArcA`..`D`) rather than one held loop, which is a
/// different design and not a renamed cue. There is no field for "the bank a
/// cue this title has not got lives in": the loader reports the miss and the
/// engine voice never opens. See `docs/formats/psp-audio.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundBanks {
    /// Where `SPEEDUPPAD` is. `hud.bnk` everywhere but HD.
    pub hud: &'static str,
    /// Where `~ENGINE` and `.COLLISIONS` are.
    pub ship: &'static str,
    /// The Zone counterpart of [`Self::ship`].
    ///
    /// Equal to `ship` on a title that ships no separate Zone bank, which is
    /// what HD does - the field is not an `Option` because "the same bank" is
    /// the honest answer there rather than "none".
    pub ship_zone: &'static str,
    /// Where `ABSORB` and `~SHIELD` are.
    pub weapons: &'static str,
    /// Where `shieldactive` is: the announcer, not an effect.
    pub speech: &'static str,
}

/// A title's Zone-mode announcer: the milestone numbers it names a voice line
/// for, and the bank that line is in.
///
/// # Every title's ladder is its own disc data, not one shared table
///
/// `Data\Sound\speech_zone.bnk` is one path all three measured titles carry -
/// see `docs/formats/psp-audio.md`'s bank table - but what it names differs:
/// Pulse announces every five zones to 30 then every ten to 100 (thirteen
/// cues), Pure stops naming every five after 30 and jumps straight to 75
/// (ten cues, plus a `bronze`/`silver`/`gold` medal set this port does not
/// read), and Wipeout HD keeps naming every five all the way to 50 before
/// switching to tens (fifteen cues, plus eleven speed-class lines this port
/// also does not read - see the same doc page). A milestone number is
/// therefore a title fact and not an engine constant, the same way
/// [`ZoneCircuit`] and [`ZoneCraft`] are.
///
/// # The cue name is always `zone_<n>`
///
/// Measured on all three: every numbered cue in every title's own
/// `speech_zone.bnk` is spelled exactly that way, so [`Self::cue_name`] is one
/// function rather than a per-title table of names.
///
/// # Confidence
///
/// **95** for "the bank exists, holds these cues, under this name" - read
/// directly off the shipped audio with `oag-wad sounds`, not inferred. **75**
/// for "the run-time zone counter reaching this number is what plays this
/// cue" - traced end to end only on Pulse's executable
/// (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md#the-ten-second-step`),
/// where the milestone table's own thresholds and the bank's own cue order
/// agree ordinally but the instruction that finally selects a waveform sits
/// outside this project's Ghidra database. Pure and HD's own executables have
/// not been read for this at all; their ladders are attributed by the pattern
/// three-titles-and-no-hole already established for the other two Zone axes,
/// not by a second traced call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneAnnouncer {
    /// `Data\Sound\speech_zone.bnk` on every title measured so far.
    pub bank: &'static str,
    /// The zone numbers this title's bank names a `zone_<n>` cue for, ascending.
    pub milestones: &'static [u16],
}

impl ZoneAnnouncer {
    /// The cue name for one of [`Self::milestones`], e.g. `"zone_5"`.
    #[must_use]
    pub fn cue_name(milestone: u16) -> String {
        format!("zone_{milestone}")
    }
}

/// Where a title keeps the hull a Zone race flies.
///
/// The companion of [`ZoneCircuit`], and it splits the corpus the same way -
/// which is the thing worth noticing about it. Pulse hangs Zone off the entities
/// a race already has: the player's own team directory gains a second model, the
/// race circuit's directory gains a second `.vex`. Pure and HD give Zone
/// entities of its own: its own ship directory, its own circuits. One decision
/// per title, showing up twice.
///
/// | Title | Zone's hull | Measured on |
/// | --- | --- | --- |
/// | Pulse | `Data\Ships\<Team>\Zone.vex` - the player's team, a different file | `pulse-psp-usa.chd` |
/// | Pure | `Data\Ships\Zone_01\Ship.vex` - a ship directory of its own | `pure-psp-eu.chd` |
/// | HD / Fury | `/data/ships/zone/ship.vex` - a ship directory of its own | `hdfury-ps3-eu-dec.iso` |
/// | 2048 | the player's team, unchanged - no Zone-specific model at all | played, see [`Self::PlayerShip`] |
///
/// Pulse's is the one with a recovered *selector* behind it rather than a name
/// probe: `Ship_LoadModel` (`0x08843258`) `case 6` builds `%s\Zone.vex` under
/// the established Zone expression, confidence 84 - see
/// `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`. The other two are name
/// resolution against shipped archives at 94, and neither title's executable has
/// been read. 2048's is the odd one out procedurally as well as in shape: no
/// name was probed at all, because there is no name to probe - see
/// [`Self::PlayerShip`].
///
/// # Every title ships a `ZoneMode` handling file, and nothing reads it
///
/// The finding that does **not** fit this enum, recorded here because this is
/// where the next reader will look for it. All three titles carry a dedicated
/// Zone-mode craft *directory* - `Data\Ships\Zone_01` on both PSP titles,
/// `/data/ships/zone` on HD - whose `handlingstats.xml` opens
/// `<Stats team="ZoneMode">` and authors **no `<Class>` block at all**, where
/// every team file authors four or five.
///
/// So the shipped answer to "what handling does a Zone craft have" is *one
/// block, shared by every team* rather than the player's own - which is
/// consistent with the mode overriding exactly the parts a `<Class>` would
/// carry: [`zone-mode.md`] has the engine replaced by the auto-speed law, the
/// brakes disabled and a four-corner hover variant, all three selected by the
/// Zone expression rather than read from a team.
///
/// **This engine does not read it.** `oag_gameplay::handling_for` needs a
/// `<Class>` and panics without one, so a Zone race still takes its per-class
/// handling from the player's own team, and `oag_game::race::load` says so in
/// the report. Closing that is a physics question - which of those blocks the
/// mode is actually meant to supply - not a naming one, and no title's Zone
/// handling path has been read in any executable.
///
/// Note the near-miss beside it on Pulse: `Data\Ships\Zone` is `<Stats
/// team="Zone">` **with** class blocks, and is the unlockable Zone *livery*, a
/// raceable team. `Zone_01` is the mode. The two differ by a suffix and are not
/// the same thing.
///
/// [`zone-mode.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/ghidra/functions/psp-pulse-usa/zone-mode.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneCraft {
    /// The player keeps their team and the *model files* change, to these
    /// stems. Pulse.
    ///
    /// The hull is shared geometry with a per-team paint: all eight teams'
    /// `Zone.vex` decode to the same 1213 vertices / 1149 triangles, so the team
    /// still chooses the livery and no longer chooses the shape.
    ModelsInTeam {
        /// Model stem for the hull, in place of the usual one.
        hull: &'static str,
        /// Model stem for the boost plume, in place of the usual one.
        boost: &'static str,
    },
    /// Zone has a ship directory of its own and the player's team choice does
    /// not reach the hull at all. Pure and HD.
    ///
    /// The model stems inside it are the ordinary ones, so only the directory
    /// changes - which is why this carries one string where
    /// [`Self::ModelsInTeam`] carries two.
    OwnShip(&'static str),
    /// Zone flies the player's own ship, exactly as any other mode would.
    /// 2048.
    ///
    /// The third shape, and not a variant of either of the other two: not
    /// [`Self::ModelsInTeam`], because no model file changes at all, and not
    /// [`Self::OwnShip`], because there is no Zone ship directory to name -
    /// 2048 is the first title in the lineage with no dedicated Zone craft.
    ///
    /// **Source: played, not decompiled or name-probed.** `oag_2048::race`
    /// used to guess a Zone ship directory shaped like Pure's and HD's, named
    /// `hdships\Zone` - which resolves under
    /// [`RaceDefaults::ship_dir`]'s *sibling* tree (2048's HD-derived
    /// roster, not its native one) and so never loaded, failing every Zone
    /// race on this title. Confirmed by play instead: 2048 lets the player
    /// choose their ship for Zone exactly as for any other mode - see
    /// `oag_2048::race::DEFAULTS`. Confidence 90 on the same bar the rest of
    /// `oag_2048::race` holds itself to for an unread executable: a direct
    /// behavioural observation of the shipped game, not yet corroborated
    /// against a decompiled selector.
    PlayerShip,
}

impl ZoneCraft {
    /// Which ship directory a Zone race's models come out of.
    ///
    /// `player` is the team the caller would otherwise have used, and is the
    /// answer for [`Self::ModelsInTeam`]: on Pulse the Zone hull lives inside
    /// whichever team the player picked, which is what keeps the livery theirs.
    #[must_use]
    pub fn directory(self, player: &str) -> &str {
        match self {
            Self::ModelsInTeam { .. } | Self::PlayerShip => player,
            Self::OwnShip(ship) => ship,
        }
    }

    /// The hull's model stem, or `None` for "whatever a race would have used".
    ///
    /// `None` on [`Self::OwnShip`] rather than a spelling of `Ship`, so that the
    /// one place that knows how a hull is named stays the title package that
    /// knows it - this crate deliberately holds no game constant. See the module
    /// docs.
    #[must_use]
    pub fn hull(self) -> Option<&'static str> {
        match self {
            Self::ModelsInTeam { hull, .. } => Some(hull),
            Self::OwnShip(_) | Self::PlayerShip => None,
        }
    }

    /// The boost plume's model stem, on the same terms as [`Self::hull`].
    #[must_use]
    pub fn boost(self) -> Option<&'static str> {
        match self {
            Self::ModelsInTeam { boost, .. } => Some(boost),
            Self::OwnShip(_) | Self::PlayerShip => None,
        }
    }
}

/// Where a title keeps the environment a Zone race flies through.
///
/// **Zone's whole look is authored, not computed.** A Zone circuit ships its own
/// meshes, its own textures, its own lights, its own `fogCube` and its own
/// `Skycube` - and the sky is the giveaway, because the twelve one-material
/// skies `docs/formats/skycube.md` counts across the PSP disc are exactly the
/// Zone variants against five or six materials for every race circuit. So there
/// is nothing to tint, desaturate or otherwise invent: loading the file the disc
/// authors *is* the aesthetic, per `CLAUDE.md`'s rule about not authoring a
/// stand-in for what the data already carries.
///
/// **Where that file lives is the part that diverges**, and unlike the two
/// fields beside it this one was measured on all three PSP/PS3 titles rather
/// than two:
///
/// | Title | Zone's environment | Measured on |
/// | --- | --- | --- |
/// | Pulse | `Data\Environments\16_Track\zone_track.vex` - the race circuit's own directory, one extra file | `pulse-psp-usa.chd`, `pulse-ps2-eu.chd` |
/// | Pure | `Data\Zone\01_Zone\track.vex` - four circuits of its own, declared `type="Zone"` | `pure-psp-eu.chd` |
/// | HD / Fury | `/data/environments/zone_1/track.vex` - four circuits of its own | `hdfury-ps3-eu-dec.iso` |
/// | 2048 | the race circuit itself, unchanged - no environment swap at all | played, see [`Self::SameCircuit`] |
///
/// Two shapes across the three disc-backed titles, so neither variant was
/// designed from one example and there was no third state for a title nobody
/// had looked at - which is what [ADR-0022] licenses and what the `Option`
/// fields this module's docs refuse do not have. 2048 later added a third
/// shape once it had a measurement of its own; see [`Self::SameCircuit`].
///
/// **The selector in the executable is unread.** `docs/formats/track.md` records
/// the binary's own `%s\%strack%s.vex` template, whose two `%s`s are exactly the
/// prefix and the `_reversed` suffix [`Self::Prefixed`] composes - but the
/// branch that puts `zone_` in the first one has not been found, the way
/// `Ship_LoadModel`'s `case 6` was found for the hull. Everything here is name
/// resolution against shipped archives, which is why the confidence is the 94 of
/// a direct probe and not higher.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneCircuit {
    /// A Zone race runs the *same* circuit, out of a second file in its
    /// directory whose name is the race one with this prefix on the front.
    ///
    /// Pulse, on both its platforms: `track.vex` beside `zone_track.vex`, and
    /// `track_reversed.vex` beside `zone_track_reversed.vex`. The prefix is
    /// carried rather than hardcoded because it is the first `%s` of the
    /// binary's own path template, and a title that spelled it differently would
    /// change this string and nothing else.
    ///
    /// **Not every circuit has one**, which is a load-correctness fact and not a
    /// menu one - see `oag_game::catalogue::Track::available_in_zone`.
    Prefixed(&'static str),
    /// A Zone race runs a circuit of its own, sharing nothing with the race
    /// ones, and this is the one it opens when the caller names none.
    ///
    /// Pure and HD. There is deliberately **no mapping** from a race circuit to
    /// a Zone one here, because the discs author none: Pure ships eight race
    /// circuits and four Zone circuits, HD sixteen environments of which four
    /// are Zone. Asking "what is Vineta K's Zone variant" is a question with no
    /// answer in the data, so this variant answers the only question that does
    /// have one.
    ///
    /// **Every Zone circuit this title has, first entry first.** Not one
    /// string: a caller can still name any of the four directly (Pure's own
    /// four `type="Zone"` circuits, HD's `zone_1`..`zone_4`), and
    /// [`Self::variant_of`] needs the whole list to tell "the player picked a
    /// Zone circuit" from "the player picked a race circuit and this is Zone
    /// mode" - the two cases a menu that always names *something* cannot
    /// distinguish on its own. The first entry is what [`Self::default_track`]
    /// answers with, on the same "the disc's own ordering is the best
    /// available answer" terms as [`RaceDefaults::track`]'s own docs give.
    ///
    /// Never empty on a title actually wired up this way - see the
    /// `oag_pure`/`oag_hd`/`oag_2048` construction sites, each with its own
    /// evidence for what belongs in the list.
    Separate(&'static [&'static str]),
    /// A Zone race runs the exact circuit a race would have, and nothing is
    /// derived or substituted at all. 2048.
    ///
    /// The third shape, and not a variant of either of the other two: not
    /// [`Self::Prefixed`], because there is no second `.vex` beside the race
    /// one to derive a name for, and not [`Self::Separate`], because there is
    /// no dedicated Zone circuit to point at - 2048 runs Zone on whichever
    /// circuit the player already picked.
    ///
    /// **Source: played, not decompiled or name-probed.** This used to be
    /// [`Self::Separate`], pointed at `zone_1` of four Zone-named environments
    /// `dlc2.psarc` ships - `docs/formats/track.md` had already shown those
    /// four are Wipeout HD's own dedicated Zone circuits, reshipped verbatim
    /// splines and all, which read as the Pure/HD shape reused. Playing the
    /// game said otherwise: 2048's Zone mode ran on an ordinary circuit, not
    /// a separate environment. That agrees with a fact [`RaceDefaults::zone`]
    /// already carried unread on this title - every circuit in the base
    /// package ships a `ZoneMode2048.effectSettings` beside its `track.vex` -
    /// which only makes sense if Zone runs on the circuit that file sits
    /// next to. The four ported `zone_N` environments are real disc content;
    /// they are simply not what this axis answers. Confidence 90, on the same
    /// bar the rest of `oag_2048::race` holds itself to for an unread
    /// executable: a direct behavioural observation, not yet corroborated
    /// against a decompiled selector or a read `effectSettings` format.
    SameCircuit,
}

impl ZoneCircuit {
    /// The circuit a Zone race opens when the caller named one.
    ///
    /// [`Self::Prefixed`] rewrites it, because on such a title the name the
    /// caller gave is a race circuit and its Zone twin is derivable.
    ///
    /// [`Self::Separate`] hands it back untouched **only when it already names
    /// one of this title's own Zone circuits** - checked case- and
    /// separator-insensitively, the same fold a PSARC container applies before
    /// it looks a path up, so `--track` and a menu-built entry name agree on
    /// what counts as a match. Anything else - which is what a menu always
    /// hands this when the player has not touched the Circuit row since
    /// picking Zone mode, because that row is filled with race circuits - falls
    /// back to the title's own default instead of being raced as though it
    /// were a Zone environment.
    ///
    /// **This used to hand back whatever it was given, unconditionally.** On a
    /// title with no per-circuit Zone mapping to derive, that read as "nothing
    /// to derive, so trust the caller" - but the caller is a menu that always
    /// names *some* circuit, Zone-shaped or not, and the untouched name it
    /// hands back most of the time is a race circuit's own environment. A Zone
    /// race on Wipeout HD or Wipeout Pure then flew the picked race circuit
    /// under Zone's rules rather than any Zone environment at all, and the
    /// load report said `racing {track} as named` - which reads as success.
    /// Trusting the caller was right; trusting it to have named a Zone circuit
    /// specifically was not, and this is the fix.
    ///
    /// **Idempotent under `Prefixed`**: a name already carrying the prefix comes
    /// back unchanged, so `--track ...\zone_track.vex --mode zone` asks for
    /// `zone_track.vex` rather than `zone_zone_track.vex`. `Separate` is
    /// idempotent too, for the same caller-facing reason: a name already in the
    /// list is recognised on the second pass exactly as on the first.
    #[must_use]
    pub fn variant_of(self, track: &str) -> String {
        match self {
            Self::Prefixed(prefix) => {
                let split = track.rfind(['\\', '/']).map_or(0, |i| i + 1);
                let (directory, file) = track.split_at(split);
                if file.starts_with(prefix) {
                    return track.to_string();
                }
                format!("{directory}{prefix}{file}")
            }
            Self::Separate(tracks) => {
                debug_assert!(
                    !tracks.is_empty(),
                    "a title wired up as Separate must name at least its own default"
                );
                if tracks.iter().any(|&own| paths_match(own, track)) {
                    track.to_string()
                } else {
                    tracks[0].to_string()
                }
            }
            Self::SameCircuit => track.to_string(),
        }
    }

    /// The circuit a Zone race opens when the caller named none.
    ///
    /// `race_default` is [`RaceDefaults::track`], which is what a race would
    /// have opened. [`Self::Prefixed`] derives its Zone twin from it, so the
    /// title's opening circuit stays the opening circuit in both modes;
    /// [`Self::Separate`] ignores it and answers with its own first entry,
    /// because a race circuit is not a Zone one on those titles and opening it
    /// would be a normal race wearing the Zone rules.
    #[must_use]
    pub fn default_track(self, race_default: &str) -> String {
        match self {
            Self::Prefixed(_) => self.variant_of(race_default),
            Self::Separate(tracks) => tracks[0].to_string(),
            Self::SameCircuit => race_default.to_string(),
        }
    }
}

/// Whether two entry names address the same archive entry.
///
/// Mirrors the fold [`oag_assets::psarc`](oag_assets)'s own `normalise`
/// applies before it looks a path up - leading `/` trimmed, `\` folded to `/`,
/// ASCII-lowercased - so [`ZoneCircuit::Separate`]'s membership check agrees
/// with the archive about which spellings name the same file. Repeated here
/// rather than shared: `oag-title` describes a title and does not depend on
/// `oag-assets`, which reads one.
fn paths_match(a: &str, b: &str) -> bool {
    fn normalise(path: &str) -> String {
        path.trim_start_matches('/')
            .replace('\\', "/")
            .to_ascii_lowercase()
    }
    normalise(a) == normalise(b)
}

#[cfg(test)]
mod tests {
    use super::{ZoneCircuit, ZoneCraft};

    const PULSE: ZoneCircuit = ZoneCircuit::Prefixed("zone_");
    const PURE: ZoneCircuit = ZoneCircuit::Separate(&[
        r"Data\Zone\01_Zone\track.vex",
        r"Data\Zone\02_Zone\track.vex",
        r"Data\Zone\03_Zone\track.vex",
        r"Data\Zone\04_Zone\track.vex",
    ]);

    /// The prefix goes on the *file*, not the front of the whole entry name,
    /// and both separators the corpus uses are directory separators.
    #[test]
    fn the_prefix_lands_on_the_file_name() {
        assert_eq!(
            PULSE.variant_of(r"Data\Environments\16_Track\track.vex"),
            r"Data\Environments\16_Track\zone_track.vex"
        );
        assert_eq!(
            PULSE.variant_of("/data/environments/talons_junction/track.vex"),
            "/data/environments/talons_junction/zone_track.vex"
        );
    }

    /// The `_reversed` suffix is part of the file name, so prefixing composes
    /// with it and produces the fourth of the four names a circuit ships.
    #[test]
    fn the_prefix_composes_with_the_reversed_suffix() {
        assert_eq!(
            PULSE.variant_of(r"Data\Environments\16_Track\track_reversed.vex"),
            r"Data\Environments\16_Track\zone_track_reversed.vex"
        );
    }

    /// `--track` naming a Zone circuit outright must not be prefixed twice.
    #[test]
    fn prefixing_an_already_prefixed_name_changes_nothing() {
        let zone = r"Data\Environments\16_Track\zone_track.vex";
        assert_eq!(PULSE.variant_of(zone), zone);
    }

    /// A title whose Zone circuits are their own has nothing to derive from a
    /// race circuit: naming one of its *own* Zone circuits stands, and the
    /// default is the title's own first Zone circuit rather than a rewrite of
    /// its race one.
    #[test]
    fn separate_circuits_are_never_rewritten() {
        let named = r"Data\Zone\03_Zone\track.vex";
        assert_eq!(PURE.variant_of(named), named);
        assert_eq!(
            PURE.default_track(r"Data\Environments\01_Vineta_K\track.vex"),
            r"Data\Zone\01_Zone\track.vex"
        );
        assert_eq!(
            PULSE.default_track(r"Data\Environments\16_Track\track.vex"),
            r"Data\Environments\16_Track\zone_track.vex"
        );
    }

    /// **The regression this type exists to close.** A menu that switches to
    /// Zone mode without touching the Circuit row hands `variant_of` a *race*
    /// circuit, not one of the title's own Zone ones - and until this test
    /// existed, `Separate` passed it straight through, so the "zone race"
    /// loaded whichever race environment happened to be selected.
    #[test]
    fn separate_substitutes_its_own_default_for_a_race_circuit() {
        let race_circuit = r"Data\Environments\01_Vineta_K\track.vex";
        assert_eq!(
            PURE.variant_of(race_circuit),
            r"Data\Zone\01_Zone\track.vex"
        );
    }

    /// The membership check folds case and separators the same way a PSARC
    /// container does before it looks a path up, so a caller that spells its
    /// own Zone circuit differently from the list still gets it honoured
    /// rather than silently substituted.
    #[test]
    fn separate_recognises_its_own_circuit_however_it_is_spelled() {
        let differently_spelled = "data/zone/03_zone/track.vex";
        assert_eq!(
            PURE.variant_of(differently_spelled),
            differently_spelled,
            "a name that matches one of PURE's own Zone circuits case- and \
             separator-insensitively should come back exactly as given"
        );
    }

    /// **2048's shape.** Unlike [`ZoneCraft::ModelsInTeam`] and
    /// [`ZoneCraft::OwnShip`], nothing changes at all: the directory stays the
    /// player's team and both model stems fall back to whatever a race would
    /// have used, which is the point - see [`ZoneCraft::PlayerShip`]'s own
    /// docs for why.
    #[test]
    fn player_ship_resolves_to_nothing_but_the_players_own_team() {
        let craft = ZoneCraft::PlayerShip;
        assert_eq!(craft.directory("feisar2048\\3"), "feisar2048\\3");
        assert_eq!(craft.hull(), None);
        assert_eq!(craft.boost(), None);
    }

    /// **2048's circuit shape, the companion of the craft one above.** Neither
    /// method derives or substitutes anything - see
    /// [`ZoneCircuit::SameCircuit`]'s own docs for why.
    #[test]
    fn same_circuit_resolves_to_exactly_what_it_was_given() {
        let zone = ZoneCircuit::SameCircuit;
        let track = r"Data\art\published\environments\altima\track.vex";
        assert_eq!(zone.variant_of(track), track);
        assert_eq!(zone.default_track(track), track);
    }
}
