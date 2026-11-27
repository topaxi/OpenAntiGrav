//! What a title's race path needs before it has opened anything.
//!
//! An axis on the terms of [`crate::boot`] and [`crate::menu`]: it exists because
//! the composition root would otherwise branch on which disc it opened.
//!
//! | | Pulse | Pure |
//! | --- | --- | --- |
//! | default circuit | `Data\Environments\16_Track\track.vex` | `Data\Environments\01_Vineta_K\track.vex` |
//! | default team | `Assegai` | `Assegai` |
//! | Zone's circuit | the race one's directory, `zone_`-prefixed | `Data\Zone\01_Zone\track.vex`, a circuit of its own |
//! | Zone's hull | the player's team, `Zone.vex` | `Data\Ships\Zone_01`, a ship of its own |
//!
//! **Three of the four fields diverge.** The first is why the type exists:
//! `16_Track` resolves on no Pure pressing, so a compile-time `--track` default
//! made `--race` on a Pure disc fail inside the archive on a name that hashes to
//! nothing. The team is here because a caller that must name a circuit before
//! opening anything must name a team too; the discs do not disagree on it.
//!
//! [`ZoneCircuit`] and [`ZoneCraft`] are the other two, measured across all three
//! titles; their docs carry the probes. They split the corpus identically: Pulse
//! hangs Zone off the entities a race already has, Pure and HD give Zone entities
//! of its own.
//!
//! # Deliberately four fields
//!
//! Not a home for everything a race loads. Kept out:
//!
//! - **Shared vocabulary.** `Data\Ships\<Team>\Ship.vex` resolves on both discs,
//!   so the hull's file name is not a title fact.
//! - **The boost plume**, until all three titles were measured (the bar
//!   [`ZoneCraft`] also cleared), now [`ShipPaths::boost`]: Pulse composes a
//!   per-team `shipboost.vex` (confidence 82, `Engine Flare` constructor,
//!   `psp-pulse-usa/exhaust.md`); Pure's executable authors no standalone boost
//!   model (measured, confidence 93, `psp-pure-usa/ship-models.md`); HD's plume
//!   is a subtree of `engineflare.vex` (`docs/rendering/trail-ribbon.md`).
//! - **Absent by construction.** Pure ships no loading-screen wave and no `.mip`
//!   HUD atlas. An axis `Some` for one title and `None` for the other is the
//!   one-example design in disguise.
//!
//! Those stay constants in the title crate that knows them, or a report line.
//!
//! The test for this type is "needed **before** the source can be opened", not
//! "is it about racing".
//!
//! [ADR-0009]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0009-multi-game-fanout.md
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// Which ghost and tag rows of the Game Options page a title's own list offers.
///
/// Pulse's `Settings` screen authors `Ghost Ship Visible` (a ghost ship is drawn or not) and,
/// on the PSP discs, `Auto Load Ghost` (the stored ghost is loaded when a race starts) and
/// `Multiplayer Tags` (a player's name over each other person's craft). A
/// title whose list is unread or lacks a row offers none of it and behaves as the row's
/// default, which is today's behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameOptions {
    /// `Ghost Ship Visible`.
    pub ship_visible: bool,
    /// `Auto Load Ghost`.
    pub auto_load: bool,
    /// `Multiplayer Tags`.
    pub multiplayer_tags: bool,
}

impl GameOptions {
    /// Neither row.
    pub const NONE: Self = Self {
        ship_visible: false,
        auto_load: false,
        multiplayer_tags: false,
    };
}

/// The ship directory Pulse, Pure and HD all spell identically; 2048 does not.
/// See [`RaceDefaults::ship_dir`].
///
/// Restated in `oag_tables::handling`, which cannot see this crate (a format
/// reader must not know which title it is reading); this field is the axis, that
/// one the spelling its three-title convenience wrapper uses.
pub const SHIP_DIR: &str = r"Data\Ships";

/// Where every title but 2048 keeps its `.pob` effects: [`RaceDefaults::effect_dir`].
pub const EFFECT_DIR: &str = r"Data\Psys";

/// Where one title keeps the models a craft is made of.
///
/// The fields travel together (a hull path is `dir\<team>\<stem>.vex`, and
/// [`ZoneCraft`] decides the team and stem on a Zone run), so they are one value
/// rather than parameters threaded through six functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipPaths {
    /// See [`RaceDefaults::ship_dir`].
    pub dir: &'static str,
    /// See [`RaceDefaults::ship_hull`].
    pub hull: &'static str,
    /// See [`RaceDefaults::zone_craft`].
    pub zone: ZoneCraft,
    /// See [`Detonator::craft`]; `None` on a title that does not race Detonator.
    pub detonator: Option<DetonatorCraft>,
    /// See [`RaceDefaults::boost`].
    pub boost: Option<&'static str>,
}

/// The archive entry name of one of a team's models under a ship directory:
/// `dir\team\model.vex`, assembled the way the loader assembles it, with
/// backslashes, which is what the name hash needs.
#[must_use]
pub fn entry_name_in(dir: &str, team: &str, model: &str) -> String {
    format!(r"{dir}\{team}\{model}.vex")
}

impl ShipPaths {
    /// The archive entry name of a team's hull, the `.vex` every mode without a craft of its
    /// own draws ([`Self::hull`]) or that mode's ([`Self::own_craft`]).
    ///
    /// `craft` is the race's own craft: [`Self::own_craft`] for the mode being raced, `None`
    /// for a mode that flies the team's ordinary hull.
    ///
    /// **A Zone run flies a different hull, and where that hull lives is a title
    /// fact** - [`ZoneCraft`], measured on all three titles. Pulse keeps it inside
    /// the player's own team directory as `Zone.vex`; Pure and HD give Zone a ship
    /// directory of its own, so the player's team stops reaching the hull at all.
    ///
    /// Pulse's is the branch with a recovered selector rather than a name probe:
    /// `Ship_LoadModel` (`0x08843258`) switches on the same
    /// `DAT_08ab07e3 == 0 && DAT_08b31048 == 6` expression already established as
    /// the Zone selector (see `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`),
    /// and only that case builds the `%s\Zone.vex` path. Every team's `Zone.vex`
    /// decodes to the same 1213 vertices / 1149 triangles / 8 meshes, so the hull
    /// itself is shared there - only the livery painted on it still varies by team,
    /// and that paint is the texture block embedded in each team's own `Zone.vex`
    /// (the eight blocks differ, measured in `zone-mode.md`), not a `PI_ModelSkin`:
    /// mode 6 applies none. On the other two there is no per-team Zone hull to share.
    ///
    /// **`hull_variant` is Pulse's Normal/Concept axis, and only applies here.**
    /// [`HullVariant::stem`] names a file inside the *same* directory the hull
    /// already resolves in, so a craft of its own ignores it: that selection is a
    /// title fact of its own ([`ZoneCraft`]), unaffected by which model variant the
    /// RACE page has picked, and every Pulse team's `Zone.vex` is one shared hull
    /// regardless. `None` keeps the title's own hull, [`Self::hull`].
    #[must_use]
    pub fn hull_entry_name(
        self,
        team: &str,
        craft: Option<ZoneCraft>,
        hull_variant: Option<&str>,
    ) -> String {
        let Some(craft) = craft else {
            return entry_name_in(self.dir, team, hull_variant.unwrap_or(self.hull));
        };
        entry_name_in(
            craft.root(self.dir),
            craft.directory(team),
            craft.hull().unwrap_or(self.hull),
        )
    }

    /// The craft a race flies when it is the mode's own, or `None` for the team's ordinary
    /// hull: Zone's ([`Self::zone`]) when `zone`, Detonator's ([`DetonatorCraft::hull`]) when
    /// `detonator` and this title ships one.
    ///
    /// The two flags are `oag_race::Mode`'s, which this crate does not name;
    /// `oag_livery::entry::own_craft` is the call that takes the mode.
    #[must_use]
    pub fn own_craft(self, zone: bool, detonator: bool) -> Option<ZoneCraft> {
        if zone {
            Some(self.zone)
        } else if detonator {
            self.detonator.map(|craft| craft.hull)
        } else {
            None
        }
    }
}

mod announcer;
mod fresh;
mod lineage;
pub mod sound;
mod transition;
mod variants;
mod zone_circuit;
mod zone_frame;
pub use crate::speed::SpeedClasses;
pub use announcer::{
    CountdownEvent, CountdownVoice, SequenceTick, StartCue, ZoneAnnouncer, ZoneClassAnnouncer,
    ZoneClassTone,
};
pub use fresh::{FreshTeam, FreshVariant};
pub use lineage::CircuitLineage;
pub use sound::{
    CircuitBanks, Crossfade, SoundBanks, TrackBanks, VrBanks, WwiseBank, WwiseCue, WwiseCues,
    WwiseState, WwiseTeamSwitch,
};
pub use transition::ZoneTransition;
pub use variants::{
    GuestRoster, HullVariant, LIVERY_SEPARATOR, TeamVariant, TeamVariants, VariantJoin,
    livery_variant, split_livery,
};
pub use zone_circuit::ZoneCircuit;
pub use zone_frame::ZoneFrame;

/// The circuit and team a race falls back to on one title.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceDefaults {
    /// Archive entry of the `.vex` a race loads when the caller names none: a full
    /// entry name because a circuit is a *file* (Pulse ships `track.vex` and
    /// `track_reversed.vex` side by side, two menu entries).
    pub track: &'static str,
    /// The team id a race uses when the caller names none.
    /// The team id a race uses when the caller names none: the folder under
    /// [`Self::ship_dir`], not the name a player reads (`oag_raceplay::catalogue`).
    pub team: &'static str,
    /// The archive directory a team's hull, plume, shield and handling stats all
    /// sit under, one subdirectory per team id.
    ///
    /// **An axis because a third title disagreed** ([ADR-0022]). Pulse, Pure and
    /// HD spell it `Data\Ships\<Team>\`; 2048's HD-derived roster lives at
    /// `Data\art\published\hdships\<Team>\` (confirmed against the manifest), and
    /// its own five-team roster elsewhere. Carried here because a format crate
    /// must not know which title it is reading: the directory is passed *in* to
    /// `oag_tables::handling::entry_name_in` and its siblings.
    ///
    /// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
    pub ship_dir: &'static str,
    /// The model stem of the hull every mode but Zone draws, under a team's own
    /// directory in [`Self::ship_dir`]: `Ship` on every title so far, each one's
    /// own spelling (Pulse's `%s\Ship.vex` at `0x08a7a5f4`'s sibling, Pure's at
    /// `0x08a7a5f4`). A [`HullVariant`] names an alternate stem in the same place.
    pub ship_hull: &'static str,
    /// The archive directory a team's `handlingstats.xml` sits under, one
    /// subdirectory per team id.
    ///
    /// **Equal to [`Self::ship_dir`] on three titles and one of 2048's two
    /// rosters**, hence a separate field: 2048's HD-derived craft keep models and
    /// tuning together under `Data\art\published\hdships\<Team>\`; **its own five
    /// teams do not**, keeping models at `Data\art\published\Ships\<team>\<1..4>\`
    /// and tuning at `Data\HandlingStats\<team>\<1..4>\`. One string cannot
    /// address both trees.
    pub handling_dir: &'static str,
    /// The archive directory this title's `.pob` effects sit in: [`EFFECT_DIR`]
    /// except 2048's `Data\Particles2048` (`docs/formats/pob.md`).
    pub effect_dir: &'static str,
    /// Per-circuit overrides of [`Self::effect_dir`]; see [`Self::effect_dir_for`].
    pub effect_dir_by_circuit: &'static [(&'static str, &'static str)],
    /// The hull file stem every craft of a circuit flies, keyed by the circuit's own
    /// `PI_Track` `type` in the plugin definition: `(type, stem)`. See
    /// [`Self::hull_for_track_type`]. Empty where no `type` swaps the model.
    pub hull_by_track_type: &'static [(&'static str, &'static str)],
    /// Circuit families raced under another game's HUD and lap counts; see
    /// [`Self::circuit_lineage`]. Empty on a title with one game in it.
    pub circuit_lineages: &'static [CircuitLineage],
    /// How this title names the circuit a Zone race runs on. See [`ZoneCircuit`].
    pub zone: ZoneCircuit,
    /// Where this title keeps the craft a Zone race flies. See [`ZoneCraft`].
    pub zone_craft: ZoneCraft,
    /// Whether a Zone race goes through this title's team step, and what it shows
    /// there. See [`ZoneTeamStep`].
    pub zone_team_step: ZoneTeamStep,
    /// The non-Zone boost plume's model stem, under [`Self::ship_dir`] beside the
    /// hull: `Some("shipboost")` on Pulse, `None` where the title's executable
    /// names no such standalone model.
    ///
    /// **`None` is not one shape.** Pure's is measured absent (confidence 93,
    /// `psp-pure-usa/ship-models.md`). HD has a plume: `EF_Boost` is a subtree of
    /// `engineflare.vex`, loaded by `flare::per_team`. 2048's and Omega's is
    /// unread, as for [`Self::zone_announcer`].
    ///
    /// [`ZoneCraft::boost`] is the Zone-mode sibling.
    pub boost: Option<&'static str>,
    /// Which `.bnk` each of this title's race cues is looked up in. See [`SoundBanks`].
    pub sounds: &'static SoundBanks,
    /// The Zone-mode announcer this title ships, when read off the disc. See
    /// [`ZoneAnnouncer`]. `None` rather than a guessed ladder for an unread bank.
    pub zone_announcer: Option<&'static ZoneAnnouncer>,
    /// The `PI_ModelSkin` name every craft wears in an Eliminator race, on the
    /// plain hull, whatever the player picked, or `None` where no title measured
    /// one.
    ///
    /// Pulse's `Ship_LoadModel` (`0x08843258`) switches on `g_game_mode`: modes 8
    /// and `0x12` build `%s\Ship.vex` (not the team-model file) and always apply
    /// the skin `Eliminator`, no unlock consulted; modes 3, 4, 5, 7, 9, 10 and
    /// 0xb read the picked `FE_ModelSkin`
    /// (`docs/ghidra/functions/psp-pulse-usa/ship-skin.md`).
    pub eliminator_skin: Option<&'static str>,
    /// Measured `ready`/`go` start-voice banks, or `None` (plays nothing): [`CountdownVoice`].
    pub countdown_voice: Option<&'static CountdownVoice>,
    /// The modes, by `oag_race::Mode::name`, whose HUD draws nothing until the race is released
    /// (the `go` step): the whole HUD, grid and countdown alike. HD's Zone race manager holds the
    /// HUD's draw gate shut from the grid and opens it in the released phase; a single race
    /// leaves it open through the countdown
    /// (`docs/ghidra/functions/ps3-hdfury-eu/hud-start-gate.md`). Empty: every mode draws its HUD
    /// from the first tick the flyby allows.
    pub hud_waits_for_go: crate::pre_race::Sourced<&'static [&'static str]>,
    /// The hover target the craft holds on the grid and releases after the green
    /// light, or `None` where not measured: [`crate::launch_hover::LaunchHover`].
    pub launch_hover: Option<&'static crate::launch_hover::LaunchHover>,
    /// The craft's hover probe set, `None` for Pulse's two-point law: [`crate::hover_rig::HoverRig`].
    pub hover_rig: Option<&'static crate::hover_rig::HoverRig>,
    /// The hover probe set a Zone race flies instead of [`Self::hover_rig`], or
    /// `None` where the title's Zone hover was not read. Pulse's
    /// `Ship_UpdateHover` picks `Ship_HoverFourCorner` on game mode 6.
    pub zone_hover_rig: Option<&'static crate::hover_rig::HoverRig>,
    /// The handling file a Zone race reads in place of the team's, or `None` where
    /// the race flies the team's own class (Pulse, read; the rest, unread):
    /// [`crate::zone_handling::ZoneHandling`].
    pub zone_handling: Option<&'static crate::zone_handling::ZoneHandling>,
    /// The craft's inertia mass and steering clamp, `None` for Pulse's: [`crate::craft_laws::CraftLaws`].
    pub craft_laws: Option<&'static crate::craft_laws::CraftLaws>,
    /// The drawn hull's random shake, faded out with speed, `None` for a title not
    /// read to have one: [`crate::hull_shake::HullShake`].
    pub hull_shake: Option<crate::pre_race::Sourced<crate::hull_shake::HullShake>>,
    /// What the Autopilot pickup does to the player's engine while it runs:
    /// [`crate::autopilot_engine::AutopilotEngine`].
    pub autopilot_engine: crate::autopilot_engine::AutopilotEngine,
    /// Whether an opponent on this project's driver flies the Autopilot it holds,
    /// per class and difficulty: [`crate::opponent_autopilot::OpponentAutopilot`].
    pub opponent_autopilot: crate::opponent_autopilot::OpponentAutopilot,
    /// The campaign's opponents fly the original's own driver
    /// (`oag_ai::original`, AI mode 3). `true` where that driver was recovered off
    /// this title's executable: Pulse, HD, 2048, Omega and Pure (a sibling design,
    /// [`crate::original_driver::AiTables::LegacyBlock`]); see
    /// `docs/gameplay/ai.md`, "The original driver, recovered". `false` keeps this
    /// project's driver.
    pub original_opponent: bool,
    /// Where this title's original driver differs from Pulse's:
    /// [`crate::original_driver::OriginalDriver`]. `None` flies Pulse's.
    pub original_driver: Option<&'static crate::original_driver::OriginalDriver>,
    /// The Zone-mode **speed-class** announcer this title ships, when read off the
    /// disc. See [`ZoneClassAnnouncer`].
    ///
    /// A different ladder from [`Self::zone_announcer`]'s (a zone *count*, "Zone
    /// 5", versus a speed *class*, "Venom"), though both live in the same family
    /// of banks on every title measured.
    pub zone_class_announcer: Option<&'static ZoneClassAnnouncer>,
    /// Where this title keeps the per-stage colour grade a Zone race escalates
    /// through. See [`ZonePalette`]. `None` for Pulse and Pure, which ship no such
    /// table (see that type for how thoroughly each disc was searched).
    pub zone_palette: Option<ZonePalette>,
    /// How this title turns a live zone number into a stage of that table. See
    /// [`ZoneStages`]. `None` where unrecovered, every title but 2048, HD/Fury
    /// included: its stage index is read from a per-craft field whose writer has
    /// not been found.
    pub zone_stages: Option<&'static ZoneStages>,
    /// How this title's Zone stage change moves through the world once a stage
    /// steps. See [`ZoneTransition`]. `None` where unread, every title but
    /// HD/Fury: 2048 fades by a factor nothing traced writes, and Pulse and Pure
    /// ship no stage table.
    pub zone_transition: Option<&'static ZoneTransition>,
    /// What this title's engine does to a Zone race's frame beyond the stage table: the
    /// light its chunks are drawn under and the `HDR and Bloom` values it runs with, over the
    /// circuit's own. See [`ZoneFrame`]. `None` where unread, every title but HD/Fury.
    pub zone_frame: Option<&'static ZoneFrame>,
    /// HD/Fury's zoom-streak ring (see [`crate::ZoomRing`]); `None` on every other title.
    pub zoom_ring: Option<&'static crate::ZoomRing>,
    /// Whether the player may switch on a borrowed copy of HD's zoom-streak ring
    /// (`graphics.boost_zoom`, off by default): true where [`Self::zoom_ring`] is
    /// `None` because the title draws no such ring, **chosen, not measured**. See
    /// [`crate::ZoomRing`].
    pub borrows_zoom_ring: bool,
    /// The ghost and tag rows this title's own Game Options list offers. See [`GameOptions`].
    pub game_options: GameOptions,
    /// Where this title keeps the per-stage textures a Zone race swaps as it
    /// escalates. See [`ZoneStageTextures`]. `None` where not located, every title
    /// but HD/Fury.
    pub zone_stage_textures: Option<&'static ZoneStageTextures>,
    /// What a Zone race takes on a circuit whose own environment tree carries no Zone table: the
    /// ported circuits of another title. See [`ZoneFallback`]. `None` where the title's Zone
    /// table is title-wide ([`Self::zone_palette`]'s `TitleWide`) or not located.
    pub zone_fallback: Option<&'static ZoneFallback>,
    /// The entry name of the cubemap a Zone race draws **instead of** the circuit's
    /// own `sky.gtf`.
    ///
    /// A replacement, not a tint, read off the original's control flow: HD/Fury's
    /// per-race environment loader picks between the circuit's sibling `sky.gtf`
    /// and this one on a single byte, and both branches converge on the same
    /// handle slot and sampler-state patch. See
    /// `docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md`. `None` where no swap is
    /// located, every title but HD/Fury.
    pub zone_sky: Option<&'static str>,
    /// This title's Detonator mode, or `None` where the title offers none: the menu
    /// lists `oag_race::Mode::Detonator` only where this is `Some`. See [`Detonator`].
    pub detonator: Option<&'static Detonator>,
    /// Team ids that carry more than one selectable directory, and what each is
    /// called and how it combines with the team's own id. See [`TeamVariants`].
    ///
    /// `None` on Pulse and Pure. 2048's own five-team roster (not its HD-derived
    /// one, see [`Self::handling_dir`]) and HD/Fury's twelve are the two measured
    /// shapes; [`VariantJoin`] says why they combine differently.
    pub team_variants: Option<&'static TeamVariants>,
    /// A second roster this title carries besides its own, reusing another title's
    /// team identities verbatim. See [`GuestRoster`]. `None` on every title but
    /// 2048, whose twelve HD-derived teams are the one measured case.
    pub guest_roster: Option<&'static GuestRoster>,
    /// A team's own alternate hull **file**, universal across the roster rather
    /// than scoped to a subset: [`HullVariant`].
    ///
    /// A third axis beside [`Self::team_variants`] and [`Self::guest_roster`],
    /// which fold a suffix into the team's *identity* (a second directory with its
    /// own tuning). Pulse authors no second directory but does author a second
    /// hull *file* inside the team's own, unlock-gated, which is why
    /// [`Self::has_team_variants`] folds this source in too.
    ///
    /// `None` on Pure, which authors no `PI_TeamModel` (zero occurrences in its
    /// `Definition.xml`), and on HD/2048, whose reskins are [`Self::team_variants`].
    pub hull_variants: Option<&'static [HullVariant]>,
    /// The speed classes this title's **own data** authors, slowest first. See
    /// [`SpeedClasses`].
    ///
    /// `None` where the per-team handling files have not been read (2048, which
    /// ships no race box, `docs/formats/race-setup.md`, and has no handling
    /// ground-truth test). The caller supplies its own ladder rather than
    /// borrowing another title's measurement; an empty ladder would claim "this
    /// title authors no classes", which nothing has established.
    pub speed_classes: Option<SpeedClasses>,
    /// The model Ship Select opens on while the player has not picked one, or
    /// `None` when the stored variant (empty) stands. See [`FreshVariant`].
    pub fresh_variant: Option<FreshVariant>,
    /// The team the list opens on while the player has not picked one, or
    /// `None` when the stored team ([`Self::team`] on a fresh profile) stands.
    /// See [`FreshTeam`].
    pub fresh_team: Option<FreshTeam>,
}

/// Where a title keeps the two per-stage texture sets a Zone race indexes by
/// stage number, and which of the two carries the art.
///
/// # Two sets, only one a picture
///
/// HD/Fury ships **two** fifteen-entry sets. Decoded through
/// `oag_texture::gtf`, all fifteen of the "general" set are byte-identical flat
/// white; all fifteen of the "track" set are distinct greyscale images whose alpha
/// pattern changes along the ladder. [`Self::track_entry`] carries the picture;
/// [`Self::general_entry`] is bound because the original binds it to every chunk
/// whose `.rcsmodel` render-block flags lack the track bit, beside the `Scene.*`
/// colours (`oag_rcs::rcsmodel::Mesh::is_track`).
///
/// # The trap
///
/// The obvious function to copy binds the *blank* set. `Scene_PrepareFrame`
/// (`ps3-hdfury-eu`) publishes only the general set to the `zoneTexInner` and
/// `zoneTexOuter` shader parameters; seven other publishers bind the track set to
/// the same parameters in a second block. Following the first function found
/// loads fifteen white squares and reads as a wiring bug. Confidence 80 on the
/// two-publisher-families reading, 82 on the sampler-to-array join
/// (`docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`, twenty-third pass).
///
/// # Not here
///
/// The original aliases the **Detonator** set into the same fifteen slots when
/// Detonator loaded last; that is a property of its storage. This engine keeps
/// Detonator's set apart, in [`Detonator::stage_textures`].
///
/// `zoneTexVis`, the third texture the shader takes, is **not a file**: the
/// original builds a 256x1 colour ramp at runtime, to be generated when something
/// draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneStageTextures {
    /// The entry name each "general" texture is numbered under, without the
    /// stage number or the extension; empty where the title ships no such set
    /// (Omega), in which case the loader binds the flat white HD's set measured as.
    pub general: &'static str,
    /// The same for the "track" set, the one that carries the art.
    pub track: &'static str,
    /// The extension both sets use, leading dot included.
    pub extension: &'static str,
    /// How many stages the sets are numbered for: fifteen on HD/Fury, matching its
    /// [`ZonePalette`] table.
    pub stages: u32,
}

impl ZoneStageTextures {
    /// The entry name of the "general" set's texture for `stage`.
    ///
    /// A flat white on every stage HD/Fury ships, sampled by the chunks without
    /// the track bit.
    #[must_use]
    pub fn general_entry(&self, stage: u32) -> String {
        format!("{}{stage}{}", self.general, self.extension)
    }

    /// The entry name of the "track" set's texture for `stage`, the one with the art.
    #[must_use]
    pub fn track_entry(&self, stage: u32) -> String {
        format!("{}{stage}{}", self.track, self.extension)
    }

    /// Every stage this title numbers its sets for.
    pub fn stages(&self) -> impl Iterator<Item = u32> + use<> {
        0..self.stages
    }
}

/// A title's ladder from the zone number a Zone race has reached to the stage of
/// its [`ZonePalette`] table that should be showing.
///
/// # Recovered, not chosen
///
/// 2048's Zone HUD widget (`Hud_UpdateZoneSpeedClassWidget`, `0x81197d6c`) walks
/// a 17-record table of **descending** thresholds, stops at the first record the
/// zone number has reached, and writes `17 - i` into the per-craft field
/// `Zone_UpdateStage` (`0x81044cfc`) then clamps to the stage table's last row.
/// [`Self::records`] is that table, read out of the executable at `0x8151faf8`;
/// [`Self::stage_for`] is that arithmetic.
///
/// So escalation is **not** one stage per zone: a class holds for a band of zones,
/// wide at the bottom and five apart across most of it
/// (`docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`).
///
/// # An axis, not one shared rule
///
/// The two titles that ship a stage table disagree on where the index comes from.
/// 2048 reads this threshold table. HD/Fury's `Environment_UpdateStageBlend`
/// (`0x003da540`) takes it from a per-craft field (`+0x640`) nothing found writes,
/// so HD carries `None` rather than 2048's thirteen-stage ladder.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneStages {
    /// The threshold table, in the order the executable stores it: descending by
    /// zone number, each paired with the speed-class name the record points at.
    ///
    /// **The first record is a sentinel**: seventeen records, sixteen class names;
    /// record `0` carries the unreachable threshold `9999` and points at the
    /// *same* name string as record `1`. Every record below it has one name apiece.
    pub records: &'static [(u16, &'static str)],
}

impl ZoneStages {
    /// The stage index showing at zone number `zone`, before the stage table's own
    /// clamp.
    ///
    /// `None` when no record matches, as in the original: its loop runs all
    /// seventeen records and, finding nothing, leaves the previous class standing.
    /// A shipped table ends at threshold `0`, so a real one cannot hit this.
    #[must_use]
    pub fn stage_for(&self, zone: u16) -> Option<u32> {
        let index = self.records.iter().position(|&(at, _)| zone >= at)?;
        u32::try_from(self.records.len() - index).ok()
    }

    /// The lowest zone number strictly above `zone` that changes the class, or
    /// `None` on the top rung.
    ///
    /// The threshold of the record **above** the one [`Self::stage_for`] picks.
    /// HD's Zone HUD needs it: its "next speed class" bar sits beside the zone
    /// number the upcoming class starts at.
    #[must_use]
    pub fn next_zone(&self, zone: u16) -> Option<u16> {
        let index = self.records.iter().position(|&(at, _)| zone >= at)?;
        Some(self.records[index.checked_sub(1)?].0)
    }

    /// The speed-class name the original would show at zone number `zone`, from the
    /// record [`Self::stage_for`] picks. For the load report and tests; nothing
    /// drawn reads it.
    #[must_use]
    pub fn class_for(&self, zone: u16) -> Option<&'static str> {
        let index = self.records.iter().position(|&(at, _)| zone >= at)?;
        Some(self.records[index].1)
    }
}

/// What a title's Detonator race loads beside the circuit: its own stage grade, its
/// own per-stage textures and its chase-camera scale.
///
/// HD/Fury's Detonator escalates through fourteen stages, one per lap, on a table of
/// its own (`detonatormode.effectsettings`, with a DLC3 revision on the four Zone
/// circuits) and a texture set of its own, picked against Zone's by the same mode test
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mode.md`,
/// `zone-effectsettings-loader.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Detonator {
    /// The stage grade, chosen per circuit the way [`RaceDefaults::zone_palette`] is, or
    /// `None` where this title's grade is not drawn.
    pub palette: Option<ZonePalette>,
    /// The table tried when [`Self::palette`]'s file is not in the archive set. 2048's
    /// `Environment_Load` reads the circuit's own table and, for a circuit with no
    /// environment tree of its own (HD's ported circuits), HD/Fury's title-wide one
    /// (`docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`).
    pub fallback_palette: Option<ZonePalette>,
    /// The per-stage texture sets, or `None` where none is located.
    pub stage_textures: Option<&'static ZoneStageTextures>,
    /// The per-stage texture sets of [`Self::fallback_palette`]'s circuits alone. 2048 loads
    /// `DetonatorModeTrack<n>.gxt` inside `Environment_LoadHDFuryContent`, the branch HD's ported
    /// circuits take, so a circuit with an environment tree of its own keeps its own materials
    /// and takes none (`docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`);
    /// the counterpart of [`ZoneFallback::stage_textures`].
    pub fallback_stage_textures: Option<&'static ZoneStageTextures>,
    /// The chase rig's craft scale in this mode, in place of the `0.75` every other
    /// mode uses: HD's craft constructors write `0.7` on mode `14`
    /// (`docs/ghidra/functions/ps3-hdfury-eu/camera.md`).
    pub camera_scale: f32,
    /// The scale the drawn hull's world matrix carries in this mode, in place of the `0.75` of
    /// `TARGET_GLOBAL_SCALE` every other mode draws it at: the craft
    /// constructors that write [`Self::camera_scale`] write `0.9` beside it, and the hull's world
    /// matrix reads `0.7 * 0.9`. `None` where it is not measured, the hull is drawn at the
    /// model's own scale (`docs/ghidra/functions/ps3-hdfury-eu/camera.md`, "The drawn hull").
    pub hull_scale: Option<f32>,
    /// The two files that say what Detonator lays on a circuit, or `None` where a title's are
    /// not located.
    pub layouts: Option<DetonatorLayouts>,
    /// The hull and livery a Detonator race flies, or `None` where a title's are not located.
    pub craft: Option<DetonatorCraft>,
    /// Whether the pulse cannon's round and muzzle flash submit point lights to the vertex light
    /// list: read on HD (`PulseBullet`'s render queue, `detonator-cannon.md`, "The cannon's three
    /// point lights"); 2048's and Omega's queues are unread.
    pub cannon_lights: bool,
    /// Whether the EMP wave submits its `(0, 5, 10)` point light, range `50`, at each front:
    /// HD's, 2048's and Omega's `EMP_DrawLights` all read
    /// (`detonator-emp.md`, "2048 and Omega draw it").
    pub emp_light: bool,
    /// The Detonator HUD layout a source authored in the PS2's 640x448 grid reads in place of
    /// the title's own, for a layout of ours whose coordinates are the PSP's. `None` where the
    /// title's layout serves every source.
    pub ps2_hud: Option<&'static str>,
    /// For a title with no EMP of its own: a fired EMP also draws the Repulsor's opening, its
    /// screen flash, its blast riding the firer and its field ring
    /// (`docs/gameplay/detonator-pulse.md`, "The EMP").
    pub emp_borrows_repulsor: bool,
    /// How fast a laid mine turns about its up axis, radians per second of the mine's age: its
    /// football and spikes are drawn on the world axes turned by `rate * age`, not on a frame
    /// built on the up axis. `None` where a title's mine animation is unread, and the mine keeps
    /// an upright, unturned frame (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-draw.md`,
    /// "The spin").
    pub mine_spin: Option<f32>,
    /// The cannon gauge on the craft's back, or `None` where a title's law is not read.
    pub ammo_gauge: Option<DetonatorAmmoGauge>,
    /// The plasma shells on the craft's wings that the EMP's charge lights, or `None` where a
    /// title's law is not read.
    pub hull_charge: Option<DetonatorHullCharge>,
    /// The handling file a Detonator race flies in place of the team's, or `None` where a
    /// title's is not located.
    pub handling: Option<crate::zone_handling::ZoneHandling>,
    /// The models a Detonator race lays and draws in place of the ordinary Mine, Bomb and pads,
    /// or `None` where a title's are not located.
    pub objects: Option<DetonatorObjects>,
    /// What a speed pad and a weapon pad do in Detonator.
    pub pads: DetonatorPads,
    /// Whether a Detonator race flies the Zone variant of the picked circuit
    /// ([`RaceDefaults::zone`]) rather than the race circuit itself.
    pub races_zone_circuit: bool,
    /// The EMP's two announcer lines and the bank that holds them, or `None` where a title's
    /// are not located.
    pub voices: Option<DetonatorVoices>,
    /// Whether the HUD's `EmpReady` lamp is bound by the executable: where it is not, the
    /// layout's widget is dropped (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hud.md`).
    pub binds_emp_ready: crate::pre_race::Sourced<bool>,
    /// Whether the HUD's three medal models (`DetonatorGoldMedal`, `Silver`, `Bronze`) are drawn
    /// as a campaign race's score passes the cell's targets. Where it is `false` the layout's
    /// widgets are dropped.
    pub shows_medals: crate::pre_race::Sourced<bool>,
    /// Whether a score add raises the floating `+N` (`ScoreFeedBack0`-`15`) at the scored object
    /// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hud.md`, "The floating score"). Where it
    /// is `false` the layout's widgets stay empty.
    pub floats_score: crate::pre_race::Sourced<bool>,
}

/// What drives the Detonator craft's cannon gauge, the hull's `detonator_bulletgaugesegments`
/// segments: `Detonator_UpdateHud` writes a fill and a colour into the material's constant block
/// while the craft is faster than [`Self::speed_gate`]
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-ammo-gauge.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetonatorAmmoGauge {
    /// The count a full gauge shows: the fill is the clip (or the reload's refill) over this.
    pub rounds: f32,
    /// The colour while the cannon can fire.
    pub ready: [f32; 3],
    /// The colour during a reload lockout, while the refill climbs.
    pub locked: [f32; 3],
    /// The colour and fill the block holds from the hull's bind until the first update.
    pub bind: ([f32; 3], f32),
    /// The craft speed the update needs to exceed; at or below it the gauge holds what it showed.
    pub speed_gate: f32,
}

/// What drives the Detonator craft's `Charge1` and `Charge2`, the alpha of its two plasma-shell
/// materials: `Ship_UpdateDetonatorHullParams` moves each toward its target by `rate * dt`, the
/// targets being `1` and `1` while the EMP's charge `v` is full, `1` and `0` while it is over zero,
/// `0` and `0` otherwise (`docs/ghidra/functions/ps3-hdfury-eu/detonator-ammo-gauge.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetonatorHullCharge {
    /// Per second, `Charge1` then `Charge2`.
    pub rates: [f32; 2],
    /// `v` at the EMP's ready count: the charge is mapped from `ready_level` there to `1` at the
    /// full count, so the first shell lights at the ready count itself.
    pub ready_level: f32,
}

/// What Detonator's pads do (`docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md`, "Pads
/// charge, they do not boost"). Where a field is `false` the pad keeps its ordinary race
/// behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetonatorPads {
    /// A speed pad gives no boost and charges the EMP once per entry.
    pub speed_pad_charges: crate::pre_race::Sourced<bool>,
    /// A weapon pad hands out nothing and charges the EMP once per pad, again only after another
    /// pad or the next start-line crossing.
    pub weapon_pad_charges: crate::pre_race::Sourced<bool>,
    /// The circuit's own speed-pad and weapon-pad models are not drawn under the Detonator pad:
    /// `WorldManager_Construct` deletes the circuit's pad importers before it loads
    /// `padReplacement` and the `Detonator_Pad` model.
    pub replaces_circuit_pads: crate::pre_race::Sourced<bool>,
    /// For a title that ships no Detonator pad model: every pad is drawn as the race circuit's
    /// own first `Weapon Pad`, held at this key of the weapon pad's ready cycle, in place of the
    /// circuit's own pads. `None` where [`DetonatorObjects::pad`] carries the model.
    pub circuit_weapon_pad_key: Option<usize>,
}

/// The EMP's announcer lines: `Detonator_UpdateRace` raises `EMPREADY` and `EMPFULL` through
/// `Sound_QueueAnnouncerCue` (`detonator-emp.md`, "The charge and the gauge").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetonatorVoices {
    /// The speech bank that holds them.
    pub bank: &'static str,
    /// The ready line's name.
    pub ready: &'static str,
    /// The full line's name.
    pub full: &'static str,
}

/// The archive entries of Detonator's own object models
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`, "The Detonator objects' models").
///
/// `DetonatorMine_Construct` loads `mine_rita_football`, `mine_rita_spikes` and
/// `Electric_Visualiser`. `DetonatorBomb_Construct` loads three bomb parts, and
/// `WorldManager_Construct` a `Detonator_Pad` for every speed pad and weapon pad;
/// `PulseBullet_Construct` the pulse cannon's round and muzzle flash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetonatorObjects {
    /// A laid mine's two parts, `[mine_rita_football, mine_rita_spikes]`: the shell and its spikes.
    pub mine: [&'static str; 2],
    /// The mine's `Electric_Visualiser`: a camera-facing ring of music-driven lightning around the
    /// shell. `None` where the title's own electricity program is unread, so the ring is not
    /// drawn rather than drawn as a stand-in.
    pub mine_visualiser: Option<&'static str>,
    /// A laid bomb's three parts, `[detonator_bomb_final, _final_equalisers, _glowedges]`.
    pub bomb: [&'static str; 3],
    /// The model carried by every speed pad and every weapon pad.
    pub pad: &'static str,
    /// The pulse cannon's `[round, muzzle flash]` (`detonator_cannonbolt`,
    /// `hd_muzzleflash_detonator`): `PulseBullet_Construct`'s two models.
    pub cannon: [&'static str; 2],
}

/// The craft a Detonator race flies: one hull of its own, whatever team the player picked, with
/// the pick choosing the livery on it
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`, "The hull").
///
/// The same shape as Zone's ([`ZoneCraft::OwnShipAt`]): HD's `Ship_ReloadModelForSkin` takes the
/// same `zoneship_zone` key to the team's `PI_TeamModel name="detonator"` `texturelocation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetonatorCraft {
    /// Where the hull sits and which path component its livery replaces.
    pub hull: ZoneCraft,
    /// The livery a craft whose team authors no `PI_TeamModel name="detonator"` wears, where the
    /// original has a literal for it; `None` keeps the hull's default skin.
    pub livery_fallback: Option<&'static str>,
}

/// The entries of Detonator's layout files, global to every circuit
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-placement.md`, "Where the data comes from"),
/// read by `oag_tables::detonator_layout`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetonatorLayouts {
    /// `PatternFile.xml`: the groups of clusters.
    pub pattern_file: &'static str,
    /// `GamePlayValues.xml`: the per-stage mine counts and the placement's constants.
    pub gameplay_values: &'static str,
    /// `DetonatorScoring.xml`: what a shot pays (`detonator-cannon.md`, "The score").
    pub scoring: &'static str,
}

/// What a title's Zone race takes on a circuit that ships no Zone table of its own.
///
/// 2048's `Environment_Load` reads the circuit's own `ZoneMode2048.effectSettings` and, for a
/// circuit with no environment tree (HD's ported circuits), HD/Fury's title-wide table instead
/// (`docs/ghidra/functions/vita-2048-eu-v104/zone-environment-fallback.md`). Those circuits'
/// materials carry HD's Zone variants (`zoneColourTint`, `zoneEffectInner`, `zoneTexInner`), so
/// the per-stage textures that feed them come with the fallback table and a circuit on its own
/// table draws no Zone surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneFallback {
    /// The table a ported circuit takes when [`RaceDefaults::zone_palette`]'s file is not in the
    /// archive set.
    pub palette: ZonePalette,
    /// The per-stage texture sets the fallback table's surface samples.
    pub stage_textures: &'static ZoneStageTextures,
    /// The zone-number ladder that steps the fallback table's fifteen stages, which is not
    /// [`RaceDefaults::zone_stages`]'s (that one grades the circuit's own table).
    pub stages: &'static ZoneStages,
    /// A directory whose circuits keep their own Zone model and table and take none of this: a
    /// race on one draws no Zone grade where this build does not draw that model. Matched
    /// ignoring case and separators. `None` where no circuit is held back.
    pub own_model_under: Option<&'static str>,
}

impl ZoneFallback {
    /// Whether the circuit `track` (its `.vex` entry name) is one this fallback does not grade.
    #[must_use]
    pub fn holds_back(&self, track: &str) -> bool {
        let fold = |name: &str| {
            name.trim_start_matches(['/', '\\'])
                .replace('\\', "/")
                .to_ascii_lowercase()
        };
        self.own_model_under
            .is_some_and(|dir| fold(track).starts_with(&fold(dir)))
    }
}

/// Where a title keeps its `.effectSettings` stage table, the per-stage colour
/// grade a Zone race climbs as its speed class rises.
///
/// **An axis because the two titles that ship one disagree on scope** ([ADR-0022]).
/// HD/Fury ships a single title-wide file layered over whichever circuit is
/// racing; 2048 ships a byte-identical copy of one table in each circuit's own
/// directory. Neither derives from the other, and neither belongs in
/// `oag-formats`.
///
/// **`None` on Pulse and Pure is a searched answer.** Two independently generated
/// candidate lists (`scripts/mine-names.py`'s 1,409 for Pulse and 1,509 for Pure,
/// plus 322 hand-guessed stage-name combinations) matched against each disc's
/// `Data.wad` directory resolved nothing but geometry, hulls, audio and HUD
/// layouts. A hash-named WAD cannot be proven empty, only searched; see
/// `docs/formats/effectsettings.md`.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZonePalette {
    /// One table for the whole title, at a fixed entry name, layered over whichever
    /// circuit the race runs on, except on the title's own Zone circuits, which
    /// take a second table. HD/Fury: `entry` is
    /// `/data/environments/zonemode.effectsettings`, and the four `zone_N`
    /// circuits take `zonemodedlc3.effectsettings`.
    ///
    /// The split is HD's: `Environment_LoadRaceScene` (`0x003f4084`-`0x003f51b8`)
    /// compares the race's track id against `25_Track`..`28_Track` and loads the
    /// DLC3 table on a match
    /// (`docs/ghidra/functions/ps3-hdfury-eu/zone-palette-select.md`).
    TitleWide {
        /// The table an ordinary circuit takes.
        entry: &'static str,
        /// The circuits' own `.vex` entry names that take `zone_circuit_entry`
        /// instead. Matched ignoring case, separator and a leading slash, as an
        /// archive folds names.
        zone_circuits: &'static [&'static str],
        /// The table the circuits in `zone_circuits` take.
        zone_circuit_entry: &'static str,
    },
    /// One copy per circuit, under the circuit's own directory, named by the file
    /// name here. 2048: `ZoneMode2048.effectSettings`, a byte-identical copy in
    /// each of the ten base circuits.
    BesideCircuit(&'static str),
}

impl ZonePalette {
    /// Whether `track` (the circuit's own `.vex` entry name) is one of
    /// [`Self::TitleWide`]'s Zone circuits. Always `false` for
    /// [`Self::BesideCircuit`], which names none.
    #[must_use]
    pub fn on_zone_circuit(&self, track: &str) -> bool {
        let Self::TitleWide { zone_circuits, .. } = self else {
            return false;
        };
        let fold = |name: &str| {
            name.trim_start_matches(['/', '\\'])
                .replace('\\', "/")
                .to_ascii_lowercase()
        };
        let track = fold(track);
        zone_circuits.iter().any(|c| fold(c) == track)
    }

    /// The archive entry this title's table reads from for a race on `track`, the
    /// circuit's own `.vex` entry name.
    ///
    /// [`Self::BesideCircuit`] rewrites the sibling path as `.envsettings` is found
    /// beside a `track.vex`; [`Self::TitleWide`] asks whether the circuit is one of
    /// the title's Zone circuits.
    #[must_use]
    pub fn entry_for(&self, track: &str) -> Option<String> {
        match self {
            Self::TitleWide {
                entry,
                zone_circuit_entry,
                ..
            } => Some(
                if self.on_zone_circuit(track) {
                    zone_circuit_entry
                } else {
                    entry
                }
                .to_string(),
            ),
            Self::BesideCircuit(file) => {
                let at = track.rfind(['/', '\\'])?;
                Some(format!("{}{}", &track[..=at], file))
            }
        }
    }
}

impl RaceDefaults {
    /// Where this title keeps the models a craft is made of, as one value. The
    /// per-team siblings ([`Self::team_variants_for`], [`Self::ships_for`],
    /// [`Self::handling_dir_for`]) are in `variants.rs`.
    #[must_use]
    pub fn ships(&self) -> ShipPaths {
        ShipPaths {
            dir: self.ship_dir,
            hull: self.ship_hull,
            zone: self.zone_craft,
            detonator: self.detonator.and_then(|detonator| detonator.craft),
            boost: self.boost,
        }
    }
}

/// Whether a Zone race goes through a title's team step (Ship Select, the RACE
/// page's TEAM row) like any other mode, and what that step shows.
///
/// **Why it matters though every craft flies one Zone hull**: the team is the one
/// the race and its loyalty credit use (`Race_ComputeLoyaltyAward` banks a Zone
/// race's award to it), and where the Zone hull is shared geometry in a per-team
/// paint ([`ZoneCraft::ModelsInTeam`], [`ZoneCraft::OwnShipAt`]) it is the paint.
///
/// | Title | Value | Source |
/// | --- | --- | --- |
/// | Pulse | [`Self::OfferedAtFullRatings`] | measured: `TeamSelection_Update`'s mode-6 branch, 84 |
/// | Pure | [`Self::Skipped`] | measured: its own chain, 94 |
/// | HD/Fury, 2048, Omega | [`Self::Offered`] | not measured: Pulse's rule, minus the ratings |
///
/// Pulse's `Team Selection` runs in Zone with the livery cycler replaced by a
/// fixed `Zone` label, the team list and Loyalty bar intact, and the four rating
/// bars forced to 10 (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md`). Pure's
/// chain goes `Zone Track Selection` straight to `Launch Game`
/// (`docs/formats/race-setup.md`, "Pure differs in shape"). The other three were
/// not read in Zone, so they take Pulse's rule without its ratings, one more
/// measured fact about one title's screen (`docs/ui/selection-screens.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneTeamStep {
    /// A Zone race launches on the team already stored: no ship screen, no live TEAM row.
    Skipped,
    /// The team step runs as in any mode and shows the team's own ratings.
    Offered,
    /// The team step runs as in any mode, and every rating bar reads full: the
    /// craft is one shared hull, so the team's ratings say nothing about it.
    OfferedAtFullRatings,
}

impl ZoneTeamStep {
    /// Whether the player picks the team for a Zone race.
    #[must_use]
    pub const fn offered(self) -> bool {
        !matches!(self, Self::Skipped)
    }

    /// Whether the rating bars read full in a Zone race.
    #[must_use]
    pub const fn full_ratings(self) -> bool {
        matches!(self, Self::OfferedAtFullRatings)
    }
}

/// Where a title keeps the hull a Zone race flies.
///
/// The companion of [`ZoneCircuit`], splitting the corpus the same way. Pulse
/// hangs Zone off the entities a race already has (the player's team directory
/// gains a second model, the circuit's directory a second `.vex`); Pure and HD
/// give Zone entities of its own. One decision per title, showing up twice.
///
/// | Title | Zone's hull | Measured on |
/// | --- | --- | --- |
/// | Pulse | `Data\Ships\<Team>\Zone.vex` - the player's team, a different file | `pulse-psp-usa.chd` |
/// | Pure | `Data\Ships\Zone_01\Ship.vex` - a ship directory of its own | `pure-psp-eu.chd` |
/// | HD / Fury | `/data/ships/zone/ship.vex` - a ship directory of its own | `hdfury-ps3-eu-dec.iso` |
/// | 2048 v1.04 / Omega | `Data\art\published\hdships\Zone\Ship.vex`, one hull for every craft | decompiled, see [`Self::OwnShipAt`] |
///
/// Pulse's has a recovered *selector*: `Ship_LoadModel` (`0x08843258`) `case 6`
/// builds `%s\Zone.vex` under the established Zone expression, confidence 84
/// (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md`). Pure and HD are name
/// resolution against shipped archives at 94; neither executable has been read.
/// 2048 v1.04 and Omega are decompiled: one loader case names the shared hull.
///
/// # Every title ships a `ZoneMode` handling file; Pulse's executable never reads it
///
/// All three titles carry a dedicated Zone-mode craft *directory*
/// (`Data\Ships\Zone_01` on both PSP titles, `/data/ships/zone` on HD) whose
/// `handlingstats.xml` opens `<Stats team="ZoneMode">` and authors **no `<Class>`
/// block**, where every team file authors four or five.
///
/// **On Pulse nothing reads it** (confidence 82): neither name is a string in the
/// executable, the definition has no team of `type="Zone"`, and the craft's stats
/// come from the player's own team directory and class in every mode
/// (`docs/ghidra/functions/psp-pulse-usa/zone-rest.md`). A Zone race taking its
/// handling from the player's team is the original's behaviour. Pure's definition
/// marks `Zone_01` as `type="Zone"`; neither that executable nor HD's has been
/// read.
///
/// Near-miss: `Data\Ships\Zone` on Pulse is `<Stats team="Zone">` **with** class
/// blocks, the unlockable Zone *livery*, a raceable team. `Zone_01` is the file
/// nothing reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneCraft {
    /// The player keeps their team and the *model files* change, to these
    /// The player keeps their team and the *model files* change, to these stems.
    /// Pulse.
    ///
    /// Shared geometry with a per-team paint: all eight teams' `Zone.vex` decode
    /// to the same 1213 vertices / 1149 triangles, so the team chooses the livery
    /// and not the shape. The paint is each file's embedded texture block; the
    /// eight differ (`zone-mode.md`).
    ModelsInTeam {
        /// Model stem for the hull, in place of the usual one.
        hull: &'static str,
        /// Model stem for the boost plume, in place of the usual one.
        boost: &'static str,
    },
    /// Zone has a ship directory of its own and the player's team choice does not
    /// reach the hull. Pure.
    ///
    /// The model stems inside are the ordinary ones, so only the directory changes
    /// (one string where [`Self::ModelsInTeam`] carries two).
    OwnShip(&'static str),
    /// Zone flies the player's own ship, as any other mode would. No title today:
    /// 2048 was read this way and was wrong, see [`Self::OwnShipAt`]. Kept as the
    /// value for a title with an unread Zone loader, since it names no path.
    ///
    /// Not a variant of [`Self::ModelsInTeam`] (no model file changes) or
    /// [`Self::OwnShip`] (no Zone ship directory to name).
    ///
    /// **Source: played, not decompiled or name-probed.** A guessed `hdships\Zone`
    /// directory resolves under [`RaceDefaults::ship_dir`]'s *sibling* tree (2048's
    /// HD-derived roster) and never loaded; 2048 lets the player choose their ship
    /// for Zone as for any mode (`oag_2048::race::DEFAULTS`,
    /// `docs/ghidra/functions/vita-2048-eu-v104/zone-craft.md`, "The 2026-08-28 "no
    /// Zone ship" note"). Confidence 90: a direct observation of the shipped game,
    /// not corroborated against a decompiled selector.
    PlayerShip,
    /// Zone has one hull of its own at `root\ship`, whatever team the player
    /// picked and whichever roster it belongs to, and the pick chooses the
    /// livery on it. HD/Fury, 2048 v1.04 and Omega.
    ///
    /// Distinct from [`Self::OwnShip`], which composes under the player's own
    /// team directory and cannot reach another tree: 2048's native craft live
    /// under `Ships\<team>2048\<n>`, the Zone hull under `hdships\Zone`. On
    /// HD `root` is the player's own tree (`Data\Ships`, `ship` is `zone`); what makes
    /// it this shape is the livery key (`docs/ghidra/functions/ps3-hdfury-eu/zone-craft.md`).
    ///
    /// **Source: decompiled, one source tree read in three builds**, so one call
    /// site. The ship-model loaders switch on the game mode and for mode 6 format
    /// `Data\art\published\hdships\Zone` and `...\Zone\Ship.vex` without looking at
    /// the craft (`docs/ghidra/functions/vita-2048-eu-v104/zone-craft.md`). The
    /// player's pick selects a livery on that hull, not a hull.
    OwnShipAt {
        /// The tree the Zone hull's directory sits in.
        root: &'static str,
        /// The Zone hull's directory name inside `root`.
        ship: &'static str,
        /// The path component the hull's team texture sits under, which the loader
        /// replaces with the craft's livery name: `zoneship_zone` on HD/Fury and
        /// 2048 v1.04, `zoneship_team` on Omega. Measured off the hull's own texture
        /// requests (`.../zoneship_team/team.gnf`); the substitution is confidence
        /// 75 on 2048 and Omega, 80 on HD (`zone-craft.md` per title).
        livery_key: &'static str,
    },
}

impl ZoneCraft {
    /// Which ship directory a Zone race's models come out of.
    ///
    /// `player` is the team the caller would otherwise have used, the answer for
    /// [`Self::ModelsInTeam`]: on Pulse the Zone hull lives inside the picked team.
    #[must_use]
    pub fn directory(self, player: &str) -> &str {
        match self {
            Self::ModelsInTeam { .. } | Self::PlayerShip => player,
            Self::OwnShip(ship) | Self::OwnShipAt { ship, .. } => ship,
        }
    }

    /// The tree [`Self::directory`] sits in: `default` (the player's own ship tree)
    /// except for [`Self::OwnShipAt`].
    #[must_use]
    pub fn root(self, default: &str) -> &str {
        match self {
            Self::OwnShipAt { root, .. } => root,
            _ => default,
        }
    }

    /// The path component a Zone hull's team texture sits under and a craft's
    /// livery replaces, for the one shape that names it ([`Self::OwnShipAt`]).
    #[must_use]
    pub fn livery_key(self) -> Option<&'static str> {
        match self {
            Self::OwnShipAt { livery_key, .. } => Some(livery_key),
            _ => None,
        }
    }

    /// The hull's model stem, or `None` for "whatever a race would have used".
    ///
    /// `None` on [`Self::OwnShip`] rather than a spelling of `Ship`, so the one
    /// place that knows how a hull is named stays the title package.
    #[must_use]
    pub fn hull(self) -> Option<&'static str> {
        match self {
            Self::ModelsInTeam { hull, .. } => Some(hull),
            Self::OwnShip(_) | Self::OwnShipAt { .. } | Self::PlayerShip => None,
        }
    }

    /// The boost plume's model stem, as [`Self::hull`].
    ///
    /// `None` on [`Self::OwnShip`]/[`Self::PlayerShip`] means "whatever a non-Zone
    /// race used" ([`ShipPaths::boost`]), not "no plume in Zone".
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
