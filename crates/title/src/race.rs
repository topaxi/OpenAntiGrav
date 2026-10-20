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
//! - **The boost plume sat here too, as "unrecovered, not different" - it no longer does.** Pulse
//!   composes a per-team `shipboost.vex` (confidence 82, its `Engine Flare` constructor,
//!   `psp-pulse-usa/exhaust.md`); Pure's executable authors no standalone boost model at all,
//!   measured rather than merely unfound (confidence 93, `psp-pure-usa/ship-models.md`); HD's
//!   plume is a third shape again, a subtree of `engineflare.vex`
//!   (`docs/rendering/trail-ribbon.md`). Three titles measured and no hole, the bar [`ZoneCraft`]
//!   cleared below, so this is [`ShipPaths::boost`] now, on the precedent **the Zone hull already
//!   set**: it was excluded while Pulse and Pure were measured and HD was not, because two
//!   measurements and a hole is the shape being refused. HD's was then measured -
//!   `/data/ships/zone/ship.vex`, off the manifest the `oag_hd::names::MODE_SHIPS` roster already
//!   listed - and with three titles and no hole it became [`ZoneCraft`]. The bar moved because the
//!   evidence did, not because the design changed its mind.
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
/// **Restated in `oag_tables::handling`**, which cannot see this crate - a
/// format reader must not know which title it is reading, so the constant sits
/// on both sides of that boundary rather than being threaded across it. The
/// axis is this field; that one is the spelling its three-title convenience
/// wrapper uses.
pub const SHIP_DIR: &str = r"Data\Ships";

/// Where every title but 2048 keeps its `.pob` effects: [`RaceDefaults::effect_dir`].
pub const EFFECT_DIR: &str = r"Data\Psys";

/// Where one title keeps the models a craft is made of.
///
/// The fields travel together everywhere - a hull path is
/// `dir\<team>\<stem>.vex` and [`ZoneCraft`] is what decides the team and the
/// stem on a Zone run - so they are one value rather than parameters threaded
/// side by side through six functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipPaths {
    /// See [`RaceDefaults::ship_dir`].
    pub dir: &'static str,
    /// See [`RaceDefaults::zone_craft`].
    pub zone: ZoneCraft,
    /// See [`RaceDefaults::boost`].
    pub boost: Option<&'static str>,
}

mod announcer;
mod fresh;
pub mod sound;
mod transition;
mod variants;
mod zone_circuit;
pub use crate::speed::SpeedClasses;
pub use announcer::{
    CountdownEvent, CountdownVoice, SequenceTick, StartCue, ZoneAnnouncer, ZoneClassAnnouncer,
};
pub use fresh::FreshVariant;
pub use sound::{CircuitBanks, Crossfade, SoundBanks, TrackBanks};
pub use transition::ZoneTransition;
pub use variants::{GuestRoster, HullVariant, TeamVariant, TeamVariants, VariantJoin};
pub use zone_circuit::ZoneCircuit;

/// The circuit and team a race falls back to on one title.
// `PartialEq` without `Eq` since 2026-09-15: `zone_transition` carries floats.
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// player reads, which comes from the string table. See `oag_raceplay::catalogue`.
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
    /// `oag_tables::handling::entry_name_in` and its siblings.
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
    /// The archive directory this title's `.pob` effects sit in: [`EFFECT_DIR`]
    /// everywhere but 2048's own `Data\Particles2048` (`docs/formats/pob.md`).
    pub effect_dir: &'static str,
    /// Per-circuit overrides of [`Self::effect_dir`]; see [`Self::effect_dir_for`].
    pub effect_dir_by_circuit: &'static [(&'static str, &'static str)],
    /// How this title names the circuit a Zone race runs on. See [`ZoneCircuit`].
    pub zone: ZoneCircuit,
    /// Where this title keeps the craft a Zone race flies. See [`ZoneCraft`].
    pub zone_craft: ZoneCraft,
    /// The non-Zone boost plume's model stem, under [`Self::ship_dir`] beside
    /// the hull - `Some("shipboost")` on Pulse, `None` where this title's own
    /// executable names no such standalone model.
    ///
    /// **`None` is not one shape.** Pure's is measured absent (confidence 93,
    /// `psp-pure-usa/ship-models.md`). HD's is not a claim HD has no plume -
    /// `EF_Boost` is a subtree of `engineflare.vex` rather than a `.vex` of
    /// its own, so it never had a stem to name here; `flare::per_team` loads
    /// it, untouched by this field. 2048's and Omega's is neither of those:
    /// simply unread, [`Self::zone_announcer`]'s own rule below.
    ///
    /// [`ZoneCraft::boost`] is this field's Zone-mode sibling - see that
    /// method's own doc for how the two combine.
    pub boost: Option<&'static str>,
    /// Which `.bnk` each of this title's race cues is looked up in. See [`SoundBanks`].
    pub sounds: &'static SoundBanks,
    /// The Zone-mode announcer this title ships, when it has been read off the
    /// disc. See [`ZoneAnnouncer`].
    ///
    /// `None` rather than a guessed ladder for a title whose bank has not been
    /// read - see [`ZoneAnnouncer`]'s own docs for which that is today.
    pub zone_announcer: Option<&'static ZoneAnnouncer>,
    /// Measured `ready`/`go` start-voice banks, or `None` (plays nothing): [`CountdownVoice`].
    pub countdown_voice: Option<&'static CountdownVoice>,
    /// The hover target the craft holds on the grid and releases after the green light, or `None`
    /// where the title was not measured to do it: [`crate::launch_hover::LaunchHover`].
    pub launch_hover: Option<&'static crate::launch_hover::LaunchHover>,
    /// The craft's hover probe set, `None` for Pulse's two-point law: [`crate::hover_rig::HoverRig`].
    pub hover_rig: Option<&'static crate::hover_rig::HoverRig>,
    /// The hover probe set a Zone race flies instead of [`Self::hover_rig`], or `None` where the
    /// title's Zone hover was not read (the race keeps [`Self::hover_rig`]). Pulse's
    /// `Ship_UpdateHover` picks `Ship_HoverFourCorner` on game mode 6: [`crate::hover_rig::HoverRig`].
    pub zone_hover_rig: Option<&'static crate::hover_rig::HoverRig>,
    /// The handling file a Zone race reads in place of the team's, or `None` where the Zone race
    /// flies the team's own class (Pulse, read; the rest, unread):
    /// [`crate::zone_handling::ZoneHandling`].
    pub zone_handling: Option<&'static crate::zone_handling::ZoneHandling>,
    /// The craft's inertia mass and steering clamp, `None` for Pulse's: [`crate::craft_laws::CraftLaws`].
    pub craft_laws: Option<&'static crate::craft_laws::CraftLaws>,
    /// What the Autopilot pickup does to the player's engine while it runs:
    /// [`crate::autopilot_engine::AutopilotEngine`].
    pub autopilot_engine: crate::autopilot_engine::AutopilotEngine,
    /// The campaign's opponents fly the original's own driver
    /// (`oag_ai::original`, the maintainer's AI mode 3). `true` where that
    /// driver was recovered off this title's executable: Pulse, HD, 2048,
    /// Omega and Pure (Pure's is a sibling design,
    /// [`crate::original_driver::AiTables::LegacyBlock`]); see
    /// `docs/gameplay/ai.md`, "The original driver, recovered". `false` keeps
    /// this project's driver.
    pub original_opponent: bool,
    /// Where this title's original driver differs from Pulse's:
    /// [`crate::original_driver::OriginalDriver`]. `None` flies Pulse's.
    pub original_driver: Option<&'static crate::original_driver::OriginalDriver>,
    /// The Zone-mode **speed-class** announcer this title ships, when it has
    /// been read off the disc. See [`ZoneClassAnnouncer`].
    ///
    /// A different ladder from [`Self::zone_announcer`]'s: that one calls a
    /// zone *count* ("Zone 5"), this one calls a speed *class* ("Venom"). Both
    /// happen to live in the same family of banks on every title measured so
    /// far, which is why they read as one axis until the cue names are
    /// actually listed.
    pub zone_class_announcer: Option<&'static ZoneClassAnnouncer>,
    /// Where this title keeps the per-stage colour grade a Zone race escalates
    /// through. See [`ZonePalette`].
    ///
    /// `None` for a title that ships no such table, which is Pulse and Pure -
    /// see that type's docs for how thoroughly each disc was searched before
    /// answering `None` rather than pointing at a plausible entry.
    pub zone_palette: Option<ZonePalette>,
    /// How this title turns a live zone number into a stage of that table. See
    /// [`ZoneStages`].
    ///
    /// `None` where the mapping is unrecovered, which is every title but 2048,
    /// HD/Fury included: its stage index is read from a per-craft field whose
    /// writer has not been found. See [`ZoneStages`]' own docs.
    pub zone_stages: Option<&'static ZoneStages>,
    /// How this title's Zone stage change moves through the world once a
    /// stage steps. See [`ZoneTransition`].
    ///
    /// `None` where the law is unread, which is every title but HD/Fury: 2048
    /// fades by a factor nothing traced writes, and Pulse and Pure ship no
    /// stage table to transition between.
    pub zone_transition: Option<&'static ZoneTransition>,
    /// HD/Fury's zoom-streak ring (see [`crate::ZoomRing`]); `None` on every other title.
    pub zoom_ring: Option<&'static crate::ZoomRing>,
    /// Whether the player may switch on a borrowed copy of HD's zoom-streak ring
    /// (`graphics.boost_zoom`, off by default) on this title: true where
    /// [`Self::zoom_ring`] is `None` because the title draws no such ring of its
    /// own, **chosen, not measured**. See [`crate::ZoomRing`].
    pub borrows_zoom_ring: bool,
    /// Where this title keeps the per-stage textures a Zone race swaps as it
    /// escalates. See [`ZoneStageTextures`].
    ///
    /// `None` for a title whose set has not been located, which is every title
    /// but HD/Fury.
    pub zone_stage_textures: Option<&'static ZoneStageTextures>,
    /// The entry name of the cubemap a Zone race draws **instead of** the
    /// circuit's own `sky.gtf`.
    ///
    /// A replacement, not a tint, and read off the original's own control flow
    /// rather than inferred from the look: HD/Fury's per-race environment
    /// loader picks between the circuit's sibling `sky.gtf` and this one on a
    /// single byte, and the two branches converge on the same handle slot and
    /// the same sampler-state patch, so nothing downstream can tell them
    /// apart. See `docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md`.
    ///
    /// `None` for a title with no such swap located, which is every title but
    /// HD/Fury.
    pub zone_sky: Option<&'static str>,
    /// Team ids that carry more than one selectable directory, and what each
    /// one is called and how it combines with the team's own id. See
    /// [`TeamVariants`].
    ///
    /// `None` on Pulse and Pure, neither of which authors a second directory
    /// for any team. Wipeout 2048's own five-team roster - not its
    /// HD-derived one, which [`Self::handling_dir`]'s own doc comment already
    /// separates out - and Wipeout HD/Fury's twelve are the two measured
    /// shapes; see [`VariantJoin`] for why they combine differently rather
    /// than sharing one join rule.
    pub team_variants: Option<&'static TeamVariants>,
    /// A second roster this title carries besides its own, reusing another
    /// title's team identities verbatim. See [`GuestRoster`].
    ///
    /// `None` on every title but Wipeout 2048, whose twelve HD-derived teams
    /// are the one measured case.
    pub guest_roster: Option<&'static GuestRoster>,
    /// A team's own alternate hull **file**, universal across the whole
    /// roster rather than scoped to a subset - see [`HullVariant`].
    ///
    /// A third, structurally different axis from [`Self::team_variants`] and
    /// [`Self::guest_roster`], both of which combine a suffix into the
    /// team's own *identity* (a second directory, carrying its own tuning).
    /// This does not: Pulse authors no second directory for any team - the
    /// doc comment above is correct as far as it goes - but it does author a
    /// second hull *file* inside the team's own one, unlock-gated, and
    /// [`Self::has_team_variants`] folds this source in beside the other two
    /// for exactly that reason.
    ///
    /// `None` on Pure, which authors no `PI_TeamModel` at all - measured
    /// against its own `Definition.xml`, zero occurrences - and on HD/2048,
    /// whose reskins are [`Self::team_variants`]' own measured shape instead.
    pub hull_variants: Option<&'static [HullVariant]>,
    /// The speed classes this title's **own data** authors, slowest first. See
    /// [`SpeedClasses`].
    ///
    /// `None` for a title whose per-team handling files have not been read -
    /// Wipeout 2048 today, which ships no race box at all
    /// (`docs/formats/race-setup.md`) and has no handling ground-truth test.
    /// The caller then supplies its own ladder rather than borrowing another
    /// title's measurement, exactly as [`crate::MenuSkin::row_extra_leading`]
    /// is handled. An empty ladder would say something different and stronger
    /// - "this title authors no classes" - which nothing has established.
    pub speed_classes: Option<SpeedClasses>,
    /// The model Ship Select opens on while the player has not picked one, or
    /// `None` when the stored variant (empty) stands. See [`FreshVariant`].
    pub fresh_variant: Option<FreshVariant>,
}

/// Where a title keeps the two per-stage texture sets a Zone race indexes by
/// stage number, and which of the two carries the art.
///
/// # Two sets, and only one of them is a picture
///
/// HD/Fury ships **two** fifteen-entry sets, and they are not
/// interchangeable. Decoded through `oag_texture::gtf`, all fifteen of the
/// "general" set are byte-identical to each other and hold a flat, uniform
/// white with nothing in it; all fifteen of the "track" set are mutually
/// distinct and hold a greyscale image whose alpha pattern visibly changes
/// along the ladder. So [`Self::track_entry`] is the one with a picture in
/// it, and [`Self::general_entry`] is bound because the original binds it
/// too: to every chunk whose `.rcsmodel` render-block flags lack the track
/// bit, beside the `Scene.*` colours - `oag_rcs::rcsmodel::Mesh::is_track`.
///
/// # The trap this type exists to stop a port walking into
///
/// The obvious function to copy binds the *blank* set. `Scene_PrepareFrame`
/// (`ps3-hdfury-eu`) publishes only the general set to the `zoneTexInner` and
/// `zoneTexOuter` shader parameters; seven other publishers bind the track set
/// to those same parameters in a second block. A reimplementation that follows
/// the first function it finds loads fifteen white squares and draws nothing,
/// which reads as a wiring bug and is not one. Confidence 80 on the
/// two-publisher-families reading, 82 on the sampler-to-array join - see
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
/// twenty-third pass.
///
/// # What is deliberately not here
///
/// The original aliases the **Detonator** set into the same fifteen slots when
/// Detonator mode loaded last, so the parameter named for Zone can carry
/// Detonator's art. That sharing is a property of the original's storage, not
/// of the files, and nothing in this port needs it yet - a Detonator entry set
/// belongs here when Detonator does.
///
/// `zoneTexVis`, the third texture the same shader takes, is **not a file at
/// all**: the original builds a 256x1 colour ramp at runtime. It is not
/// nameable here and must be generated when something draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneStageTextures {
    /// The entry name each "general" texture is numbered under, without the
    /// stage number or the extension.
    pub general: &'static str,
    /// The same for the "track" set - the one that carries the art.
    pub track: &'static str,
    /// The extension both sets use, leading dot included.
    pub extension: &'static str,
    /// How many stages the sets are numbered for. Fifteen on HD/Fury, matching
    /// its [`ZonePalette`] table's own fifteen stages exactly.
    pub stages: u32,
}

impl ZoneStageTextures {
    /// The entry name of the "general" set's texture for `stage`.
    ///
    /// A flat white on every stage HD/Fury ships, sampled by the chunks
    /// without the track bit - see the type's own docs.
    #[must_use]
    pub fn general_entry(&self, stage: u32) -> String {
        format!("{}{stage}{}", self.general, self.extension)
    }

    /// The entry name of the "track" set's texture for `stage` - the set with the art in it.
    #[must_use]
    pub fn track_entry(&self, stage: u32) -> String {
        format!("{}{stage}{}", self.track, self.extension)
    }

    /// Every stage this title numbers its sets for.
    pub fn stages(&self) -> impl Iterator<Item = u32> + use<> {
        0..self.stages
    }
}

/// A title's ladder from the zone number a Zone race has reached to the stage
/// of its [`ZonePalette`] table that should be showing.
///
/// # What this is, recovered rather than chosen
///
/// Wipeout 2048's Zone HUD widget (`Hud_UpdateZoneSpeedClassWidget`,
/// `0x81197d6c`) walks a 17-record table of **descending** thresholds, stops at
/// the first record whose threshold the zone number has reached, and writes
/// `17 - i` - the record's index counted from the bottom - into the per-craft
/// field `Zone_UpdateStage` (`0x81044cfc`) then clamps to the stage table's own
/// last row. [`Self::records`] is that table, read straight out of the
/// executable at `0x8151faf8`; [`Self::stage_for`] is that arithmetic.
///
/// So the escalation is **not** one stage per zone: a class holds for a band of
/// zones, wide at the bottom of the ladder and five zones apart across most of
/// it. See `docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`.
///
/// # Why this is an axis and not one shared rule
///
/// The two titles that ship a stage table disagree about where the index comes
/// from. 2048 reads it off this threshold table. HD/Fury's own
/// `Environment_UpdateStageBlend` (`0x003da540`) takes Zone's index from a
/// per-craft field (`+0x640`) instead, and nothing found writes it - so HD
/// carries `None` here rather than 2048's numbers, which name a
/// thirteen-stage ladder HD does not have.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneStages {
    /// The threshold table, in the order the executable stores it: descending
    /// by zone number, each paired with the speed-class name the same record
    /// points at.
    ///
    /// **The first record is a sentinel.** There are seventeen records and only
    /// sixteen class names; record `0` carries the unreachable threshold
    /// `9999` and points at the *same* name string as record `1`. That
    /// duplicate is what an earlier pass read as the table being inconsistent
    /// and left unresolved - it is a guard entry, and every record below it is
    /// strictly one name apiece.
    pub records: &'static [(u16, &'static str)],
}

impl ZoneStages {
    /// The stage index showing at zone number `zone`, before the stage table's
    /// own clamp.
    ///
    /// `None` when no record matches, which reproduces the original's own
    /// behaviour: its loop runs the full seventeen records and, finding
    /// nothing, writes no class at all and leaves the previous one standing.
    /// A shipped table ends at threshold `0`, so this cannot happen for a real
    /// one - the case exists because the loop's exit does.
    #[must_use]
    pub fn stage_for(&self, zone: u16) -> Option<u32> {
        let index = self.records.iter().position(|&(at, _)| zone >= at)?;
        u32::try_from(self.records.len() - index).ok()
    }

    /// The lowest zone number strictly above `zone` that changes the class, or
    /// `None` when `zone` is already on the top rung.
    ///
    /// The threshold of the record **above** the one [`Self::stage_for`] picks,
    /// which is where the next speed class begins. HD's Zone HUD needs it
    /// directly: its "next speed class" bar is drawn beside the zone number the
    /// upcoming class starts at, so the row it sits on is this number minus the
    /// current zone.
    #[must_use]
    pub fn next_zone(&self, zone: u16) -> Option<u16> {
        let index = self.records.iter().position(|&(at, _)| zone >= at)?;
        Some(self.records[index.checked_sub(1)?].0)
    }

    /// The speed-class name the original would show at zone number `zone`.
    ///
    /// Read off the same record [`Self::stage_for`] picks, so the two cannot
    /// disagree. For the load report and for tests; nothing drawn reads it.
    #[must_use]
    pub fn class_for(&self, zone: u16) -> Option<&'static str> {
        let index = self.records.iter().position(|&(at, _)| zone >= at)?;
        Some(self.records[index].1)
    }
}

/// Where a title keeps its `.effectSettings` stage table - the per-stage
/// colour grade a Zone race climbs as its speed class rises.
///
/// **An axis because the two titles that ship one disagree about its scope**,
/// which is the bar [ADR-0022] sets. HD/Fury ships a single title-wide file
/// that layers over whichever circuit is racing; 2048 ships a byte-identical
/// copy of one table in each circuit's own directory. Neither shape can be
/// derived from the other, and neither belongs in `oag-formats`, which must
/// not know which title it is reading.
///
/// **`None` on Pulse and Pure is a searched answer, not an unchecked one.**
/// Two independently generated candidate lists - `scripts/mine-names.py`'s
/// 1,409 for Pulse and 1,509 for Pure, plus 322 hand-guessed stage-name
/// combinations - were matched against each disc's `Data.wad` directory and
/// resolved nothing but geometry, hulls, audio and HUD layouts. A hash-named
/// WAD cannot be proven empty, only searched; see
/// `docs/formats/effectsettings.md`.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZonePalette {
    /// One table for the whole title, at a fixed entry name, layered over
    /// whichever circuit the race runs on. HD/Fury:
    /// `/data/environments/zonemode.effectsettings`.
    ///
    /// **`zonemodedlc3.effectsettings` is a second, larger revision of the
    /// same table and is not what this points at**: what selects between the
    /// base file and the DLC3 one is unread, so the base file is used and the
    /// selector is left unresolved rather than guessed at.
    TitleWide(&'static str),
    /// One copy per circuit, under the circuit's own directory, named by the
    /// file name here. 2048: `ZoneMode2048.effectSettings`, which every one of
    /// the ten base circuits ships a byte-identical copy of.
    BesideCircuit(&'static str),
}

impl ZonePalette {
    /// The archive entry this title's table reads from for a race on `track`,
    /// where `track` is the circuit's own `.vex` entry name.
    ///
    /// [`Self::BesideCircuit`] rewrites the sibling path the same way
    /// `.envsettings` is found beside a `track.vex`; [`Self::TitleWide`]
    /// ignores the circuit entirely, which is the whole of the difference
    /// between the two shapes.
    #[must_use]
    pub fn entry_for(&self, track: &str) -> Option<String> {
        match self {
            Self::TitleWide(entry) => Some((*entry).to_string()),
            Self::BesideCircuit(file) => {
                let at = track.rfind(['/', '\\'])?;
                Some(format!("{}{}", &track[..=at], file))
            }
        }
    }
}

impl RaceDefaults {
    /// Where this title keeps the models a craft is made of, as one value.
    ///
    /// See `variants.rs`, split out under this crate's 1,000-line ceiling,
    /// for the per-team siblings this has: [`Self::team_variants_for`],
    /// [`Self::ships_for`], [`Self::handling_dir_for`].
    #[must_use]
    pub fn ships(&self) -> ShipPaths {
        ShipPaths {
            dir: self.ship_dir,
            zone: self.zone_craft,
            boost: self.boost,
        }
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
/// | 2048 v1.04 / Omega | `Data\art\published\hdships\Zone\Ship.vex`, one hull for every craft | decompiled, see [`Self::OwnShipAt`] |
///
/// Pulse's is the one with a recovered *selector* behind it rather than a name
/// probe: `Ship_LoadModel` (`0x08843258`) `case 6` builds `%s\Zone.vex` under
/// the established Zone expression, confidence 84 - see
/// `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`. The other two are name
/// resolution against shipped archives at 94, and neither title's executable has
/// been read. 2048 v1.04 and Omega are decompiled: one loader case names the
/// shared hull, see [`Self::OwnShipAt`].
///
/// # Every title ships a `ZoneMode` handling file; Pulse's executable never reads it
///
/// The finding that does **not** fit this enum, recorded here because this is
/// where the next reader will look for it. All three titles carry a dedicated
/// Zone-mode craft *directory* - `Data\Ships\Zone_01` on both PSP titles,
/// `/data/ships/zone` on HD - whose `handlingstats.xml` opens
/// `<Stats team="ZoneMode">` and authors **no `<Class>` block at all**, where
/// every team file authors four or five.
///
/// **On Pulse nothing reads it** (2026-10-02, confidence 82): neither name is a
/// string in the executable, the definition has no team of `type="Zone"`, and
/// the craft's stats come from the player's own team directory and class in
/// every mode - `docs/ghidra/functions/psp-pulse-usa/zone-rest.md`. A Zone race
/// here taking its handling from the player's team is the original's behaviour,
/// not a gap. Pure's definition does mark `Zone_01` as `type="Zone"`, and neither
/// that executable nor HD's has been read.
///
/// Note the near-miss beside it on Pulse: `Data\Ships\Zone` is `<Stats
/// team="Zone">` **with** class blocks, and is the unlockable Zone *livery*, a
/// raceable team. `Zone_01` is the file nothing reads. The two differ by a
/// suffix and are not the same thing.
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
    /// No title today: 2048 was read this way on 2026-08-28 and was wrong, see
    /// [`Self::OwnShipAt`]. Kept as the value a title with an unread Zone
    /// loader names, since it names no path.
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
    /// Zone has one hull of its own that lives **outside the ship tree the
    /// player's team is in**: the directory is `root\ship`, whatever team
    /// the player picked and whichever roster that team belongs to.
    /// Wipeout 2048 v1.04 and the Omega Collection.
    ///
    /// Distinct from [`Self::OwnShip`], which composes under the same
    /// directory as the player's own team and so cannot reach a tree the
    /// player's roster does not sit in: 2048's native craft live under
    /// `Ships\<team>2048\<n>`, the Zone hull under `hdships\Zone`.
    ///
    /// **Source: decompiled, one source tree read in two builds**, so one
    /// call site rather than two independent ones. Both executables'
    /// ship-model loader switches on the game mode and, for mode 6, formats
    /// `Data\art\published\hdships\Zone` and `...\Zone\Ship.vex` without
    /// looking at the craft - see
    /// `docs/ghidra/functions/vita-2048-eu-v104/zone-craft.md`. The player's
    /// pick selects a livery on that hull, not a hull.
    OwnShipAt {
        /// The tree the Zone hull's directory sits in.
        root: &'static str,
        /// The Zone hull's directory name inside `root`.
        ship: &'static str,
        /// The path component the hull's team texture sits under, which the
        /// loader replaces with the craft's livery name: `zoneship_zone` on
        /// 2048 v1.04, `zoneship_team` on Omega. Measured off the hull's own
        /// texture requests (`.../zoneship_team/team.gnf`); the loader's
        /// substitution is confidence 75, `zone-craft.md`.
        livery_key: &'static str,
    },
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
            Self::OwnShip(ship) | Self::OwnShipAt { ship, .. } => ship,
        }
    }

    /// The tree [`Self::directory`] sits in: `default` (the player's own
    /// ship tree) except for [`Self::OwnShipAt`], which names its own.
    #[must_use]
    pub fn root(self, default: &str) -> &str {
        match self {
            Self::OwnShipAt { root, .. } => root,
            _ => default,
        }
    }

    /// The path component a Zone hull's team texture sits under and a craft's
    /// livery replaces, for the one shape that names it. See
    /// [`Self::OwnShipAt`].
    #[must_use]
    pub fn livery_key(self) -> Option<&'static str> {
        match self {
            Self::OwnShipAt { livery_key, .. } => Some(livery_key),
            _ => None,
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
            Self::OwnShip(_) | Self::OwnShipAt { .. } | Self::PlayerShip => None,
        }
    }

    /// The boost plume's model stem, on the same terms as [`Self::hull`].
    ///
    /// `None` on [`Self::OwnShip`]/[`Self::PlayerShip`] means "whatever a
    /// non-Zone race used" ([`ShipPaths::boost`]), not "no plume in Zone".
    #[must_use]
    pub fn boost(self) -> Option<&'static str> {
        match self {
            Self::ModelsInTeam { boost, .. } => Some(boost),
            Self::OwnShip(_) | Self::OwnShipAt { .. } | Self::PlayerShip => None,
        }
    }
}

#[cfg(test)]
mod tests;
