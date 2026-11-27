//! What a title ships, described in types a title package fills in.
//!
//! The *vocabulary* half of [ADR-0022]: it holds no constant that belongs to a
//! game. `oag-pulse` and `oag-pure` supply those and `oag-assets` reads them.
//!
//! # Two questions, only one of them here
//!
//! **"How does this byte stream decode?" is answered by the file**, inside
//! `oag-formats`, from the artifact's own version word. No type here describes a
//! file's contents; that would start a decoder that has to be told which disc it
//! is reading.
//!
//! **"What does this title ship?" is answered here.** Which archives exist and
//! what they are called is a property of the release, not of any file.
//!
//! # Why this is small
//!
//! ADR-0022 licenses an axis only where a second corpus has been measured.
//! `pure-status.md` measured Pure's archive layout, not its HUD atlas or mode
//! set; types for those would be designed from one example (the failure ADR-0009
//! named). They stay plain constants in the title crate until a second title
//! forces their shape.
//!
//! [`boot`] is such an axis: both titles' boot sequences were cold-booted, differ
//! in length as well as content, and neither derives from the other or from its
//! XML. See [ADR-0023], which supersedes ADR-0022 item 4 for that axis alone.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [ADR-0023]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0023-boot-sequence-as-title-data.md

pub mod adverts;
pub mod autopilot_engine;
pub mod boot;
pub mod campaign;
pub mod contact_cue;
pub mod craft_laws;
pub mod demo;
pub mod destroyed;
pub mod effects;
pub mod eliminator;
pub mod endrace;
pub mod engine_effects;
pub mod exhaust;
pub mod flare;
pub mod group_volume;
pub mod hover_rig;
pub mod hud;
pub mod hull_shake;
pub mod language;
pub mod lap_hud;
pub mod launch_hover;
pub mod legend;
pub mod loading;
pub mod loyalty;
pub mod main_menu;
pub mod menu;
pub mod now_playing;
pub mod opponent_autopilot;
pub mod original_driver;
pub mod pause_menu;
pub mod photo;
pub mod pickup_roster;
pub mod pre_race;
pub mod pressing;
pub mod profile;
pub mod prompts;
pub mod race;
pub mod race_song;
pub mod recovery;
pub mod rumble;
pub mod shadow;
pub mod speed;
pub mod team;
pub mod touch;
pub mod track_alias;
pub mod water;
pub mod weapon_pad;
pub mod weapons;
pub mod weather;
pub mod width_throttle;
pub mod wingtip;
pub mod zone_handling;
pub mod zoom;

pub use boot::{BootProfile, BootStep, Provenance};
pub use campaign::{Campaign, CampaignDialect, CardPose, FlowFiles, MainRow, Wording};
pub use effects::{
    Burst, ColourWalk, EffectSpec, Effects, Looks, MATERIAL_LOD_BIAS, NearLaw, Origin, Platforms,
    QuakeLook, QuakeUp, Rule, ShieldPalette, ShieldPalettes, Trigger,
};
pub use endrace::{EndRaceDialect, EndRaceStyle};
pub use hud::{
    BarModels, HudArt, HudLayouts, HudSkin, ResumeCountdown, ZoneScore, ZoneSpeedClasses,
};
pub use language::LanguageManifest;
pub use loading::Loading;
pub use main_menu::{Attached, MainMenu, OriginalRow};
pub use menu::{
    HelpText, ListBlocks, MenuBlocks, MenuGlow, MenuList, MenuSettings, MenuSkin, MenuStrip,
    StripBlocks,
};
pub use oag_disc::Platform;
pub use pause_menu::{
    PauseAction, PauseLayout, PauseMenu, PauseMenuRows, PauseMusicList, PauseRow, PauseScreen,
    PauseTouch,
};
pub use pressing::{Pressing, Pressings};
pub use race::sound::{
    FrontEndSounds, MedalScreen, MenuCues, ResultsLaw, ResultsVoices, StyledCue, TournamentPlace,
};
pub use race::{
    CircuitBanks, CountdownEvent, CountdownVoice, Crossfade, Detonator, DetonatorAmmoGauge,
    DetonatorCraft, DetonatorHullCharge, DetonatorLayouts, DetonatorObjects, DetonatorPads,
    DetonatorVoices, GuestRoster, RaceDefaults, SequenceTick, SoundBanks, SpeedClasses, StartCue,
    TeamVariant, TeamVariants, TrackBanks, VariantJoin, VrBanks, WwiseBank, WwiseCue, WwiseCues,
    WwiseState, WwiseTeamSwitch, ZoneAnnouncer, ZoneCircuit, ZoneClassAnnouncer, ZoneClassTone,
    ZoneCraft, ZoneFallback, ZoneFrame, ZonePalette, ZoneStageTextures, ZoneStages, ZoneTeamStep,
    ZoneTransition,
};
pub use touch::{TouchButton, TouchFrontEnd};
pub use track_alias::TrackAlias;
pub use zoom::ZoomRing;

/// One title's release-level facts.
///
/// A `&'static Title` is chosen once by the composition root. Nothing dispatches
/// through it per call, and it is a struct of tables rather than a trait: the
/// differences between two Wipeout releases are data.
///
/// Not `Eq`: [`menu::MenuSkin`] carries offsets and scales as `f32`, the unit the
/// disc authors them in, and selection is by `&'static` identity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Title {
    /// The title as a person would name it, for reports and error messages.
    pub name: &'static str,
    /// When each platform's release first came out, as `(platform, year, month)`
    /// (the European release): the launcher orders its cards by it.
    pub released: &'static [(Platform, u16, u8)],
    /// Which bulk and companion archives its releases carry.
    pub archives: ArchiveCandidates,
    /// Serials known to belong to a *different* title. See [`ArchiveCandidates`].
    pub foreign_serials: &'static [ForeignSerial],
    /// This title's front end, or `None` when none of it has been recovered. See
    /// [`FrontEnd`] for why layout and boot chain are one field and why `None` is
    /// a measurement.
    pub front_end: Option<&'static FrontEnd>,
    /// Which layout this title's in-race HUD is read from, per mode. See
    /// [`hud::HudLayouts`].
    pub hud: &'static hud::HudLayouts,
    /// How this title's HUD sprites reach the screen. See [`hud::HudArt`].
    pub hud_art: &'static hud::HudArt,
    /// What an Eliminator is won on. See [`eliminator::EliminatorRule`].
    pub eliminator: pre_race::Sourced<eliminator::EliminatorRule>,
    /// The Eliminator flip, `None` where the title has none or nobody has wired it.
    pub eliminator_flip: pre_race::Sourced<Option<eliminator::FlipRule>>,
    /// How a score Eliminator's opponents spend their weapons. See
    /// [`eliminator::ScorePaceTerms`].
    pub score_pace: pre_race::Sourced<eliminator::ScorePaceTerms>,
    /// The team ids this title's roster names and the shared identity each is
    /// kept under, for loyalty. See [`team`].
    pub teams: team::Roster,
    /// How this title's races pay loyalty, or `None` when they pay none. See
    /// [`loyalty::LoyaltyEarning`].
    pub loyalty: Option<loyalty::LoyaltyEarning>,
    /// What a race falls back to when the caller names no circuit or team. See
    /// [`race::RaceDefaults`].
    ///
    /// Hung off `Title` like [`Self::boot`]: it is needed **before** the archives
    /// are open, because a command line has to name a circuit. A caller comparing
    /// titles to pick a constant does not work: a title package declares its
    /// `Title` as a `const`, so `std::ptr::eq` compares promoted temporaries
    /// whose addresses need not be equal.
    pub race: &'static race::RaceDefaults,
    /// Where this title keeps the exhaust ribbon's texture. See
    /// [`exhaust::Exhaust`]: Pulse and Pure name a texture, HD ships a ribbon
    /// template whose material names its own.
    pub exhaust: &'static exhaust::Exhaust,
    /// Where this title keeps the **engine flare**, the light at the nozzle (not
    /// the ribbon [`Self::exhaust`]). See [`flare::Flare`].
    pub flare: &'static flare::Flare,
    /// The wingtip contrails this title's craft lay after a speed pad, or `None` for a title that
    /// has none of its own. See [`wingtip::Wingtip`].
    pub wingtip: Option<&'static wingtip::Wingtip>,
    /// What colours a weapon pad's light bars at run time. See
    /// [`weapon_pad::WeaponPadGlow`]: HD overwrites the material's authored value
    /// with a per-pad cycle, every other title keeps it.
    pub weapon_pad_glow: &'static weapon_pad::WeaponPadGlow,
    /// Which pickups the executable's draw may grant in each mode, or `None` where its gate has
    /// not been read: the draw is then the weapon table's alone. See
    /// [`pickup_roster::PickupRoster`].
    pub pickup_roster: Option<pre_race::Sourced<pickup_roster::PickupRoster>>,
    /// Which of the ways a craft is put back this title has. See [`recovery::ResetEnables`].
    pub reset_enables: recovery::ResetEnables,
    /// How a destroyed craft's sequence runs. See [`destroyed::DestroyedCraft`].
    pub destroyed: pre_race::Sourced<destroyed::DestroyedCraft>,
    /// How a craft's contact with the track or another craft is heard. See
    /// [`contact_cue::ContactCues`].
    pub contact_cues: contact_cue::ContactCues,
    /// The curve from the SFX slider and the flyby fade to the effects bus. See
    /// [`group_volume::GroupVolumeLaw`]. A title whose mix is authored (HD and its lineage) keeps
    /// the bus to that mix and reads `Linear`.
    pub effects_volume_law: pre_race::Sourced<group_volume::GroupVolumeLaw>,
    /// The curve from the Speech slider to the speech bus (the announcer). See
    /// [`group_volume::GroupVolumeLaw`]. Same rule as [`Self::effects_volume_law`]: an authored mix
    /// reads `Linear`. The flyby's voice-over is on the music player and never takes this.
    pub speech_volume_law: pre_race::Sourced<group_volume::GroupVolumeLaw>,
    /// Whether the speech bus is driven by the effects group (the SFX slider times the flyby
    /// fade, through [`Self::speech_volume_law`]) instead of the Speech slider alone. The Speech
    /// slider then multiplies in linearly on top. `false` on an authored mix.
    pub speech_follows_effects: pre_race::Sourced<bool>,
    /// A fixed trim on the whole mix, in decibels, applied ahead of the master limiter. Pulse's
    /// mix is authored for a 16-bit clamp and sums to about twice full scale in a busy race; HD's
    /// authored mix has headroom and reads `0.0`.
    pub master_trim_db: pre_race::Sourced<f32>,
    /// The child of `GlobalAudioConfig.xml`'s `<RearOcclusion>` this title's original reads to
    /// low-pass a voice behind the listener (`Stereo` here: ours is a stereo mix), or `None` when
    /// it does not. The frequency is the file's, never ours. Only a title that sets this plays
    /// the law, even where its file carries the element: Omega ships one nothing reads
    /// (`docs/ghidra/functions/ps4-omega-eu/audio-3d.md`, "`RearOcclusion` is dead on Omega").
    pub rear_occlusion: Option<pre_race::Sourced<&'static str>>,
    /// Which codepoints in this title's strings are controller-button glyphs,
    /// and which control each depicts. See [`prompts::Prompts`].
    pub prompts: &'static prompts::Prompts,
    /// Which name and tag entries this title has. See [`profile::Profile`].
    pub profile: profile::Profile,
    /// Which of photo mode's words this title's disc already says. See [`photo::Strings`].
    pub photo: photo::Strings,
    /// The frames its photo mode accumulates into a shot, or `None` where the original's shot is
    /// the one frame on screen. See [`photo::Accumulation`].
    pub photo_shot: Option<photo::Accumulation>,
    /// The plugin definition declaring what this release carries to race with:
    /// one `PI_Team` node per team, one `PI_Track` per circuit, one `PI_Music`
    /// per soundtrack track.
    ///
    /// **An axis because a third title disagreed**, as [`FrontEnd::root`] is: both
    /// PSP titles number the game plugin (`Data\Plugins\PI001\Definition.xml`) and
    /// HD names it (`Data\Plugins\Frontend\Definition.xml`). A single `oag-pulse`
    /// constant missed on HD, the roster came back empty and an HD grid was eight
    /// copies of the player's craft (`livery::teams_for_slots`).
    ///
    /// The schema is shared: HD's file carries the same three node kinds under the
    /// same attribute names, and `oag_raceplay::catalogue` reads all three off it.
    ///
    /// **Overlaps [`DeclaredTracks::declared_in`] and is deliberately not merged
    /// with it.** The two agree on the titles that carry both, but unmeasured
    /// rather than structurally: `Music` is `None` where music is unlocated and
    /// `tracks` is `None` on Pulse, so routing through it could not answer for
    /// Pulse.
    pub plugin_definition: &'static str,
    /// Where `PI_Track` lives, when it is not [`Self::plugin_definition`].
    ///
    /// **`None` on every title but one.** Pulse, Pure and HD ship one file with all
    /// three node kinds; 2048 splits them into three, so its `plugin_definition`
    /// names the *team* one (a race needs a roster before a circuit list).
    /// `None` means "the same file", never "no circuits".
    pub track_plugin_definition: Option<&'static str>,
    /// What this title puts on screen while it loads, or `None` when it puts up
    /// nothing of its own.
    ///
    /// **`None` is a measurement**, like [`Self::front_end`]'s: Pure ships neither
    /// the tips plugin nor the glow strip in any of its three archives, so the
    /// screen falls back to this build's own counts. [`loading::Loading`]'s module
    /// doc carries the three-way table.
    pub loading: Option<&'static loading::Loading>,
    /// Where this title keeps its music, or `None` when none of it has been
    /// located. See [`Music`].
    pub music: Option<&'static Music>,
    /// Which file this title tunes its weapons from. See [`weapons::Weapons`].
    ///
    /// An axis because the answers differ in shape: Pulse and HD ship two tables
    /// and choose by race mode, Pure ships one. `oag_tables::weapons::RACE_ENTRY`
    /// is Pulse's spelling, so a Pure race reaching for it found no table and
    /// handed out no pickups, with only a report line to say so.
    pub weapons: &'static weapons::Weapons,
    /// Where this title keeps each weapon's own body model. See
    /// [`weapons::WeaponModels`].
    ///
    /// Required rather than `Option`, like [`Self::weapons`]: every `TITLE` const
    /// supplies one (`Default` where nothing is recovered).
    pub weapon_models: &'static weapons::WeaponModels,
    /// What this title throws on each race [`effects::Trigger`], with where each
    /// answer came from. `None` on a trigger means draw nothing. See [`effects`]
    /// and ADR-0058.
    pub effects: &'static effects::Effects,
    /// What this title draws or does in a race beyond its effect triggers, as
    /// per-platform rules with their origin. See [`effects::Looks`].
    pub looks: &'static effects::Looks,
    /// Which campaign reader and draw list this title's front end uses, and what
    /// of the campaign it authors. See [`campaign::Campaign`].
    pub campaign: &'static campaign::Campaign,
    /// The rows of this title's own main menu, as the disc authors them, and how a
    /// row attached to one is drawn. See [`main_menu::MainMenu`].
    pub main_menu: &'static main_menu::MainMenu,
    /// The pause screens this title's disc authors, per game mode, or `None` where none has
    /// been read: pausing then falls through to the front end's main page over the parked
    /// race. Never a stand-in screen. See [`pause_menu::PauseMenu`].
    pub pause_menu: Option<&'static pause_menu::PauseMenu>,
    /// What this title's executable resolves per pressing (the title screen's
    /// wordmark, a localised boot movie), or `None` when it resolves nothing
    /// per pressing. See [`pressing::Pressings`].
    pub pressings: Option<&'static pressing::Pressings>,
    /// The flyby this title plays before its countdown, or `None` where it plays
    /// none or none has been read. See [`pre_race::PreRace`].
    pub pre_race: Option<&'static pre_race::PreRace>,
    /// How this title draws its circuits' billboard adverts, or `None` where they
    /// are not drawn. See [`adverts::Adverts`].
    ///
    /// **`None` is "not measured or not shipped", never a default**: Pulse and HD
    /// each carry their own target size and clip planes, and a title with none
    /// draws no advert rather than borrowing another's.
    pub adverts: Option<&'static adverts::Adverts>,
    /// The mouse pointer drawn over this title's screens, as SVG source.
    ///
    /// Nothing in this lineage was authored for a mouse; each title package draws
    /// one in its own menu palette (`assets/cursors/README.md`). On `Title` rather
    /// than [`MenuSkin`] because a title with no menu skin (front end unread) still
    /// answers a pointer. Rasterised and drawn by `oag_game::cursor`.
    pub cursor: &'static str,
    /// Whether this title's own `original` shadow tier is built, and so whether
    /// the shadow setting defaults to it. See [`shadow::OriginalShadows`].
    pub original_shadows: shadow::OriginalShadows,
    /// Whether a circuit's rain and snow are played, and so whether the weather
    /// setting is offered. See [`weather::Weather`].
    pub weather: weather::Weather,
    /// Whether the water setting has anything to change here, and the surfaces its
    /// `enhanced` law is drawn on. See [`water::Water`].
    pub water: water::Water,
    /// Whether the original plays pad patterns, and so whether the RUMBLE
    /// setting offers `original`. See [`rumble::Rumble`].
    pub rumble: rumble::Rumble,
    /// The original's own throttle on the scene's width, or `None` where it has none or none
    /// has been read: only then does TARGET FPS offer `original`. See
    /// [`width_throttle::Law`].
    pub width_throttle: Option<pre_race::Sourced<width_throttle::Law>>,
    /// The attract demo its main menu starts when left alone. See [`demo::Demo`].
    pub demo: demo::Demo,
}

/// Where a title keeps the music its front end and its races play.
///
/// Both PSP titles build the front end's music path in the executable, and the
/// two differ in shape as well as spelling: Pulse expands a
/// `Data\Music\FEMusic\frontend%d.at3` template, Pure names one literal file.
/// Neither derives from the other, which is what ADR-0022 asks for before an axis
/// becomes a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Music {
    /// The front end's own looping track, by entry name in the bulk archive, or
    /// `None` when not recovered (Omega: no front-end music event or state is as
    /// clear in its bank as the race tracks').
    ///
    /// Not a soundtrack track: it is short, loops, and is addressed by a name the
    /// executable spells out rather than [`Self::track_file`].
    pub front_end: Option<&'static str>,
    /// How the soundtrack tracks are addressed, for a title whose entries are
    /// recovered **by name**. See [`DeclaredTracks`].
    ///
    /// `None` for Pulse is a statement about **this build**, not the disc: Pulse's
    /// `Definition.xml` declares its sixteen tracks as Pure's declares nineteen
    /// and each resolves, but switching would reorder its race playlist and change
    /// which track a PS2 boot's menu plays, a change needing its own evidence.
    /// `oag_sound` finds Pulse's sixteen by what the entries *are* (`HANDOVER.md`).
    pub tracks: Option<DeclaredTracks>,
    /// A soundtrack addressed by a Wwise bank state rather than by file. See
    /// [`StateTracks`]. Exclusive with [`Self::tracks`] in practice.
    pub state_tracks: Option<StateTracks>,
    /// How the playing song is shown during a race, when the title authors one.
    pub now_playing: Option<now_playing::NowPlaying>,
    /// The extension of a spectrum sidecar that sits beside the front end's
    /// track (2048's `.fft`, the band levels its map follows), or `None`.
    pub spectrum_extension: Option<&'static str>,
    /// When the race song begins to sound, and where that came from.
    pub race_song_start: pre_race::Sourced<race_song::RaceSongStart>,
}

impl Music {
    /// Whether the race song waits for the race to leave its countdown.
    #[must_use]
    pub fn song_waits_for_go(&self) -> bool {
        self.race_song_start.value == race_song::RaceSongStart::AtGo
    }
}

/// A soundtrack whose playlist is plugin XML and whose audio is picked by setting
/// Wwise states, as Omega plays one.
///
/// Each `PI_Music` entry's `location` is the `N` of an event
/// ([`Self::set_track_event`]) that sets [`Self::track_group`] to that song's
/// state; with the [`Self::race_flow`] states also set, [`Self::play_event`]
/// walks the bank's music switches down to the song.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateTracks {
    /// The plugin definition whose `PI_Music` nodes list the playlist.
    pub declared_in: &'static str,
    /// The bank whose `HIRC` holds the events, music switches, segments and
    /// tracks.
    pub bank: &'static str,
    /// The directory the loose `<media id>.wem` streams are in.
    pub media_dir: &'static str,
    /// The state group whose states are the songs.
    pub track_group: &'static str,
    /// The event that sets one song's state, `{}` standing for the entry's
    /// `location` number.
    pub set_track_event: &'static str,
    /// The event that plays the music tree.
    pub play_event: &'static str,
    /// The state every group not set stands at.
    pub none_state: &'static str,
    /// `(state group, state)` pairs set for a race.
    pub race_flow: &'static [(&'static str, &'static str)],
    /// `(state group, state)` pairs set for the menus, or empty when not read.
    pub front_end_flow: &'static [(&'static str, &'static str)],
}

/// Where a title declares its soundtrack, and what a declaration points at.
///
/// One `PI_Music` node per track, each naming the directory the track lives in;
/// the audio is [`Self::file`] inside it. Reading the declaration makes the
/// soundtrack order the **disc's own**, so "the first track" is the release's
/// decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclaredTracks {
    /// The plugin definition whose `PI_Music` nodes declare them, by entry name in
    /// the bulk archive.
    pub declared_in: &'static str,
    /// The file inside each declared location.
    pub file: &'static str,
}

/// One of the disc's list screens (a title, a column of rows or lists, a footer) that a comparison with the
/// original's frame draws as a page of the menu model, read off the screen's own XML. The live pages stay
/// the menu tree's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListScreen {
    /// The `--menu-page` name that asks for it.
    pub page: &'static str,
    /// The archive entry that authors it.
    pub file: &'static str,
    /// The screen's own `name` attribute.
    pub screen: &'static str,
    /// The header's string id where the screen authors no title `Text`: Pulse's Racebox screens are
    /// headed by their `AreaController`'s area, whose text the executable draws. **Chosen, not
    /// measured**: the id of the Main Menu row that names the area, which the frames read.
    pub title_id: Option<&'static str>,
    /// Whether the screen has a way back, so the footer legend draws `Back`.
    pub back: bool,
}

/// One title's front end: how it lays menus out, and how it boots into them.
///
/// # Why the two are one field
///
/// They are recovered together and useless apart: a boot chain walks screens
/// whose layout comes from the skin, and a skin with no chain has nothing to open
/// it. Two independent `Option`s would admit three states, two impossible, and
/// `oag-game`'s `load_shell` would need two refusals where one is honest.
///
/// **[ADR-0054](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md)
/// narrows this for one axis.** [`Self::menu`] is
/// `Option<&'static menu::MenuSkin>` because a title's chain can be walkable
/// while its menu vocabulary is not `MenuSkin`-shaped. [`Self::boot`] stays
/// mandatory, so `oag-game` has one refusal for "no front end at all" and a
/// narrower one for "no `MenuSkin`-shaped menu to draw" (see
/// [`Self::menu`]/[`Self::touch`]).
///
/// # Why `None` is a result
///
/// HD ships `/data/plugins/frontend/gui/skin.xml` and a directory of screen
/// definitions beside it, and **not one number out of either has been read**. A
/// [`MenuSkin`](menu::MenuSkin) filled in from Pulse's would attribute
/// measurements to a disc nobody measured (CLAUDE.md, "Never invent what the
/// assets already author"). `None` is a visible absence a caller refuses by name.
/// name and says so, which is a visible absence rather than a wrong picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrontEnd {
    /// The archive entry holding this title's front-end root: every boot screen,
    /// the `FEGlobals` block, and the `LoadXML` list pulling in the rest.
    ///
    /// **An axis because a third title disagreed** (the [ADR-0022] bar). Both PSP
    /// titles keep it at `Data\Plugins\PI001\GUI\Skin.xml`, a *numbered* plugin;
    /// HD names its plugins (`Data\Plugins\Frontend\Gui\Skin.xml`), as it names its
    /// soundtrack plugin.
    ///
    /// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
    pub root: &'static str,
    /// The plugins that carry a language, in the order the disc lists them.
    ///
    /// A *token* joined into `Data\Plugins\{token}\Definition.xml` by the caller,
    /// so the PSP titles' `PI008` and HD's `Languages\English` share one
    /// mechanism (same divergence and fix as [`Self::root`]). A language's own
    /// name is inside its definition, not its path, so this list is what is read
    /// back from the archive rather than trusted.
    pub language_plugins: &'static [&'static str],
    /// What each release of this title offers, keyed by disc serial, read off the
    /// executable's plugin manifest. Empty where no manifest has been read.
    ///
    /// [`Self::language_plugins`] stays the default for a caller with no serial
    /// and a release not listed here, so it is the *superset* a disc may carry,
    /// not an offered list. See [`Self::offered_languages`].
    pub language_manifests: &'static [language::LanguageManifest],
    /// The release a source that reports **no serial** is taken to be, for
    /// [`Self::offered_languages`] alone. `None` where every source names its own
    /// release (every title but Omega: its PS4 extract keeps no `param.sfo`, and
    /// the EU base-plus-patch pair is the only one this project holds). **Chosen,
    /// not measured.**
    pub assumed_release: Option<&'static str>,
    /// The namespace under `assets/ui/strings/disc/` holding this project's
    /// translations of this title's disc text, keyed by its idstrings. `None` for
    /// a title nobody has translated: Pure, HD and Omega reuse id spellings, so a
    /// namespace is never shared between titles. A disc's own language only has
    /// its *gaps* filled from it (an id the disc's table lacks or leaves empty); a
    /// project language is overlaid.
    pub disc_strings: Option<&'static str>,
    /// How this title lays menus out, for a title that authors the
    /// `FEGlobals`/`<Menu>`/`<HorizMenu>` vocabulary [`menu::MenuSkin`] describes.
    ///
    /// Both PSP titles' `Skin.xml` files were read and agree on nothing they
    /// share. Presentation only: the menu tree is this project's, in
    /// `assets/ui/menu.toml`.
    ///
    /// **`Option` since [ADR-0054].** 2048 authors no `<FEGlobals>` block and no
    /// `<Menu>`/`<HorizMenu>` widget, only a touch-icon grid over a persistent
    /// scene, a vocabulary this type cannot hold; placeholder numbers would be an
    /// invented stand-in. `None` here is a measurement about the front end's
    /// idiom that no reading will fill. See [`Self::touch`] and
    /// `docs/formats/2048-frontend.md`, "Why `front_end` stays `None`".
    ///
    /// [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md
    pub menu: Option<&'static menu::MenuSkin>,
    /// [`Self::menu`] again, for a title's PS2 pressing, when read off that
    /// pressing's own `Skin.xml` rather than approximated from the other.
    ///
    /// Scaling `menu` by [`menu::MenuSkin::space`] is an approximation
    /// (`oag_display::space::Space`: 30 of 43 shared `Skin.xml` coordinates land
    /// within a pixel of a uniform 640/480, 448/272 stretch, 13 do not). Pulse's
    /// PS2 `Data\Plugins\PI001\GUI\MainMenu_Definition.xml` is one of the 13:
    /// `<Menu>` `y="53"` (not 52.7), `helptext0` `y="75"` (22 below the row, not
    /// PSP's 18 scaled to 29.6), `row_pitch` 41 (not 33.9), which let that subtitle
    /// overrun `RACEBOX`/`COURSE`. See `oag_pulse::frontend::PS2_MENU_SKIN`.
    ///
    /// `None` for every title but Pulse. Falls back to [`Self::menu`], so filling
    /// this in only narrows an approximation.
    pub menu_ps2: Option<&'static menu::MenuSkin>,
    /// The control this title's PS2 pressing binds the front end's `backward` and `cancel` to, where
    /// that is not the circle every other pressing binds. `None`: circle.
    ///
    /// Pulse's PS2 executable parses both names to the triangle's pad bit and the PSP's to the
    /// circle's (`docs/ghidra/functions/ps2-pulse-eu/input.md`, "The divergence"), and the PS2
    /// front end's own footers draw the triangle beside `Back`. The screens that read this are
    /// listed in `docs/ui/campaign-screens.md`, "Back and Help on the PS2".
    pub back_button_ps2: Option<prompts::Prompt>,
    /// The same for this title's PSP pressing. `None`: circle, which Pulse's executable binds
    /// both names to. Pure's binds them to the triangle (`docs/ghidra/functions/psp-pure-usa/input.md`).
    pub back_button_psp: Option<prompts::Prompt>,
    /// How this title lays a touch-icon front end out, for a title that authors
    /// *that* vocabulary instead of [`menu::MenuSkin`]'s. See
    /// [`touch::TouchFrontEnd`].
    ///
    /// `None` for every title but 2048: Pulse and Pure are PSP titles with no
    /// touchscreen, HD's PS3 front end is [`menu::MenuSkin`]-shaped, and only 2048
    /// has been read for this axis. See [`touch`] for why this stays a
    /// single-corpus type.
    pub touch: Option<&'static touch::TouchFrontEnd>,
    /// How this title's own boot sequence goes. See [`boot::BootProfile`].
    ///
    /// Hung off [`Title`] so there is **one** selection point: opening one title's
    /// archives while driving another's chain cannot be constructed. `oag-assets`
    /// receives this and never reads it; inert data is cheaper than two things to
    /// keep in step.
    pub boot: &'static boot::BootProfile,
    /// The screen whose own widgets frame every menu: the rules, the corner marks
    /// and the colour the frame is cleared to.
    ///
    /// **A screen name, not a table of coordinates**, so the caller *plays the
    /// data* (CLAUDE.md). HD authors its whole front-end frame on one screen, `FE
    /// Screen`, as a `<ScreenClear>` and three `<Image>` widgets.
    ///
    /// `None` where the frame has not been read, which is Pure: **a gap, not a
    /// measurement.** Pulse's `Top FE Screen->FE Screen` carries its angled top
    /// bar and two bars framing the footer's ticker, one `<Image>` apiece, patches
    /// of `Data\FE\Images\pulse_assets.mip`, nested three anonymous `<Screen>`
    /// levels down (`oag_game::screen`'s `collect_widgets` walks through them).
    pub menu_frame: Option<&'static str>,
    /// File stems (lower case, no extension) of the `.gnf` images stored with
    /// their rows in HD's `.gtf` order, bottom-up, so the sprite sheet must
    /// reverse them as it does a `.gtf`'s. Empty for every title but Omega.
    ///
    /// A `.gnf` carries no row-order flag, so which ones is a census: Omega's
    /// packager re-authored the images its screens name top-down (135 of the 143
    /// asymmetric ones matching an HD file by name and size match rows reversed),
    /// but eight are HD's bytes exactly. Measured against HD's `.gtf` of the same
    /// stem (`docs/formats/omega-status.md`, "The menu blocks"); a listed image no
    /// HD file matches would be a guess.
    pub bottom_up_gnf: &'static [&'static str],
    /// Whether this title's front-end faces are drawn through a palette the game rewrites
    /// at run time (Pure's), so a PPSSPP texture pack's key, made against whatever palette
    /// PPSSPP held when it dumped the face, is not known to be the picture the original draws.
    /// A pack is not asked for such a face (`docs/formats/ppsspp-texture-pack.md`, "The front
    /// end's faces"). `false` where nothing says the palette moves, which is Pulse.
    pub runtime_palette_faces: bool,
    /// How this title's footer legend flows its prompts, or `None` where the legend is laid out
    /// as the skin authors it (Pulse and Pure). See [`legend::LegendFlow`].
    pub legend_flow: Option<legend::LegendFlow>,
    /// The definition file that authors the race box's own selection screens
    /// (`Track Creation`/`Track Selection`, `Team Selection`), or `None` for a
    /// title whose race box has not been read.
    ///
    /// Named rather than found through the skin's `LoadXML` list (23 files, too
    /// many to read at boot for two screens). Pulse and Pure both:
    /// `Selection_Definition.xml` beside their skin, the same relative path on both
    /// discs (`docs/formats/race-setup.md`), though Pure otherwise splits its race
    /// box into a chain of screens. HD authors its two in separate files: `None`
    /// here, see [`Self::team_select`].
    pub race_box: Option<&'static str>,
    /// A standalone `Team Selection` definition, for a title that authors its ship
    /// screen in a file of its own rather than in [`Self::race_box`]: HD/Fury's
    /// `Team_Selection_Definition.xml`, which its live skin includes and its `Cell
    /// Selection` redirects to. `None` elsewhere. **Only the ship screen**: a
    /// title naming this and no race box keeps its RACE page's own TRACK row.
    pub team_select: Option<&'static str>,
    /// A standalone `Track Creation` definition, the track screen's counterpart to
    /// [`Self::team_select`]: HD/Fury's `Track_Selection_Definition.xml`, which
    /// its live skin includes and only `DATA06` carries. `None` elsewhere, whose
    /// track screen is in [`Self::race_box`] or unread.
    pub track_select: Option<&'static str>,
    /// The footer ticker's message list: the `PI_News` rows (id, class, statistic slot,
    /// weight) Pulse's `Data\Plugins\news\Definition.xml` authors. `None` on a title with no
    /// ticker. Read by `oag_ui_screens::news`.
    pub news: Option<&'static str>,
    /// The definition holding the race box's `Single Player` screen, whose lists
    /// are the settings a custom race is set up with. Read today for the
    /// `Eliminations` list, the kill targets the `KILLS` row offers
    /// (`oag_game::boot::kill_targets`). `None` where unread: no kill target to pick.
    pub race_setup: Option<&'static str>,
    /// The list screens a compare still draws as pages of the menu model: Pulse's `Main Menu`,
    /// `Racebox`, `Single Player` and `Additional`. Empty where this build draws none of the
    /// disc's list screens that way. See [`ListScreen`].
    pub list_screens: &'static [ListScreen],
    /// Text widgets of the track screen whose string the screen's class sets in
    /// code in place of the one its XML authors: `(widget name, string id)` pairs,
    /// empty where the class takes the XML's own. Pulse's
    /// `TrackSelection_ApplySelection` (`0x088ee2f8`) labels the first stat row
    /// `MSC_DISTANCE` (`Distance(m)`) where `Track Creation` authors
    /// `IG_HUD_DISTANCE` (`Distance`).
    pub track_label_overrides: &'static [(&'static str, &'static str)],
    /// The key an event card titles a circuit by, per circuit and direction,
    /// where the executable keeps a table for it ([`TrackAlias`]). A circuit the
    /// table has no row for keeps its ordinary name; empty where the title has
    /// no such table or this build has not read it. 2048's
    /// `g_track_alias_table` (`0x814b1578`).
    pub card_track_aliases: &'static [TrackAlias],
    /// What the track screen's lap and race record rows read before a time is set.
    /// Pulse's `0x088edd60` draws `-.--.--` for a record of `0` or `0xffff`; the
    /// other titles keep the single dash (unmeasured).
    pub track_record_empty: &'static str,
    /// Whether the track screen's `Distance` row is the circuit's own authored
    /// `<Physical Length>` (`<location>\stats.xml`, `stats_reversed.xml` for a
    /// reversed entry) rather than a measured length. Pulse's
    /// `TrackStats_ParseElement` (`0x088c46f8`) reads it into the definition's
    /// `+0xdc`, printed through `%d`. `false` where unmeasured: Pure keeps the
    /// measured lap.
    pub track_distance_authored: bool,
    /// Whether this title's race-setup screens preview an entry with a rendered
    /// mesh.
    ///
    /// **An axis because Pure disagrees with Pulse** ([ADR-0022]), totally. Pulse
    /// draws `<location>\FE\forward.vex` for a circuit and `<location>\ship_FE.vex`
    /// for a craft; neither exists on Pure's disc, where every preview is a
    /// pre-rendered still authored in the entry's `screen.xml` and matched to the
    /// original at RMSE 0 (`docs/formats/race-setup.md`). Pulse draws *both*, so
    /// "does this entry author stills?" cannot stand in for this.
    ///
    /// `false` for HD, which previews with stills and widgets of its own and loads
    /// no mesh through this path: the craft is [`Self::ship_preview_hull`], the
    /// circuit [`Self::circuit_models`] (`<Model name="TrackModel">` /
    /// `<Model name="ShipModel">`, a third convention).
    pub preview_meshes: bool,
    /// The file under a craft's directory that its selection screen draws as the
    /// 3-D craft when [`Self::preview_meshes`] is `false`: HD's `Team Selection`
    /// draws the race hull (`ship.vex`, the pair `ship.rcsmodel`) in its
    /// `ShipModel` frame, and no `*FE*` mesh exists under any HD
    /// `Data\Ships\<team>` (`docs/ui/selection-screens.md`). `None` where a title
    /// has no such screen or draws stills.
    pub ship_preview_hull: Option<&'static str>,
    /// The rest pose the screen's update writes over the `ShipModel` widget's rotation,
    /// position and scale, for a title whose craft is not framed by a fit (Omega). `None`
    /// draws the widget as authored. See [`ShipPose`].
    pub ship_preview_pose: Option<ShipPose>,
    /// The camera the `Team Selection` screen's own code poses its craft on,
    /// over whatever the `ShipModel` widget authors: HD's screen update writes
    /// the widget's rotation, depth, lens and origin every frame and fits the
    /// depth to the selected model's authored length. `None` where the title's
    /// screen has no such code or it is unread. See [`ShipFit`].
    pub ship_preview_fit: Option<ShipFit>,
    /// How the player turns, zooms and pans the craft on `Team Selection`: the update's own
    /// law over the widget's rotation, depth, lens and position. `None` where the title's
    /// screen has no such control (the PSP titles) or it is unread. See [`ShipControl`].
    pub ship_control: Option<ShipControl>,
    /// The model each circuit's selection screen draws in its circuit-model frame,
    /// found by the circuit's environment folder: HD's track-select record names
    /// `<environment>\FE\track0N.vex` per circuit, and the number is not the
    /// folder's (`10_Sebenco_Climb` is `track06`). A circuit with no row has no
    /// model on the disc. Empty where the screen draws no such model.
    pub circuit_models: &'static [CircuitModel],
    /// How `Track Creation`'s own update turns the circuit model and which pose it gives each
    /// circuit: the pitch, depth and vertical origin come from the circuit's own entry
    /// ([`CircuitPose`]) and the yaw is a per-frame turntable. `None` where the screen's code is
    /// unread (Omega), which draws the widget as authored and turns it on stand-in numbers.
    pub circuit_turntable: Option<CircuitTurntable>,
    /// The clip the track selection screen's `FlyByMovie` plays for a circuit, as a path
    /// under the circuit's environment folder (`FE\preview.bik` on HD). `None` where the
    /// title ships none: Omega authors the same widget (`Loading_Icon.bik`) and no
    /// environment of its carries a `preview` file.
    pub circuit_fly_by: Option<&'static str>,
    /// The definition file that authors the three screens a race ends on
    /// (`EndRace Results`/`EndRace Rewards`/`EndRace Menu`), or `None` for a title
    /// whose copy has not been read.
    ///
    /// **An axis as [`Self::root`] is.** Both PSP titles keep Pulse's numbered
    /// path `Data\Plugins\PI001\GUI\EndRace_Definition.xml`, dictionary-shortened
    /// like every PSP screen file. HD names its plugin
    /// (`Data\Plugins\Frontend\Gui\EndRace_Definition.xml`), plain UTF-8, and ships
    /// it in five of its seven archives, no two copies alike by MD5. **Three of
    /// HD's screen names match Pulse's and almost none of the widgets do**: HD's
    /// `EndRace Results` is the whole field's finishing order (`Grid{col}.{row}`, 4
    /// columns by 10 rows, two captioned: `POS`, `TIME`), not Pulse's per-lap
    /// table, and its `EndRace Menu` is one `<Block>` per option shown by mode. So
    /// this states *where the file is*, not that the titles share a reader; see
    /// `docs/formats/hd-endrace-screens.md` and `oag_game::endrace`.
    ///
    /// `None` for Pure and Omega is a gap, not a measurement that none is shipped,
    /// as for [`Self::race_box`]. 2048 ships one (`NEWGUI/EndRace_Definition.xml`).
    pub endrace_entry: Option<&'static str>,
    /// The dialect [`Self::endrace_entry`]'s file is written in, which picks the
    /// reader, see [`endrace`]. `None` where the file is unread (Pure) or read and
    /// not wired (Omega), neither a measurement that no such screen ships.
    pub endrace_style: Option<EndRaceStyle>,
    /// Whether this title, on a pressing that authors no two-person screens of its
    /// own, **borrows the PS2 pressing's design** for `Team SelectionSplit`,
    /// `EndRace Results Split` and `EndRace Rewards Split`, built from the
    /// pressing's own assets with the PS2's numbers rescaled into its grid. `true`
    /// for Pulse alone, whose PSP and PS2 pressings share anything invented for
    /// Pulse and only the PS2 authors a split screen. **Chosen, not measured** on
    /// the pressings that borrow it. `false` elsewhere: no other title has a PS2
    /// pressing read, and those with a split screen author it.
    pub borrows_ps2_split_screens: bool,
    /// What lights the 3-D models a front-end screen draws (the craft on Ship Select), or
    /// `None` where the screen lights them with the viewer's rig. See [`ModelLighting`].
    pub model_lighting: Option<ModelLighting>,
    /// The colour (ARGB) the selected column of a `HexSelection` grid (`Team Selection`'s and
    /// `Track Creation`'s) is filled with, where the title's widget writes its own, or `None` for
    /// `HD_Blue` of the front end's style. Omega's `_SetCell` carries a grey inline
    /// (`docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`, "Omega").
    pub hex_selected_fill: Option<u32>,
    /// The colour (ARGB) the `Team Selection` honeycomb's cursor ring pulses through between white
    /// and black, where the title computes its own, or `None` for `HD_Blue` of the front end's
    /// style. Omega's update carries the red inline
    /// (`docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`, "Omega").
    pub team_hex_ring_peak: Option<u32>,
}

/// The light a front-end screen's models are drawn in: the engine's own light parameters,
/// filled from a `.envsettings` the front end loads for itself, and then the front end's
/// tone stage (`docs/ghidra/functions/ps3-hdfury-eu/front-end-model-lighting.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelLighting {
    /// The file in the Fury front-end style.
    pub fury_file: &'static str,
    /// The file in the HD front-end style. A title that has one style names it twice.
    pub hd_file: &'static str,
    /// Whether the file spells its keys the way the PS4 and Vita titles do
    /// (`oag_tables::envsettings`, "Wipeout 2048 authors the same shape under different key
    /// names") and carries Omega's sun scale.
    pub ps4: bool,
    /// The tone stage the model's frame ends in, or `None` where its adaptation is unread.
    pub tone: Option<FrontEndTone>,
}

/// The front end's tone stage as `oag_post::hd_bloom::exposure` reads it:
/// `maximum - min(adapted * boost, clamp)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrontEndTone {
    /// `Tone adaption boost`, written by the front end's loader.
    pub boost: f32,
    /// `Tone darkening clamp`.
    pub clamp: f32,
    /// `Tone maximum brightness`.
    pub maximum: f32,
    /// The adapted luma the Fury style's frame sits at on the screen this is read for. The HD
    /// style's is unread and is taken at the clamp (`clamp / boost`), which is where the exposure
    /// bottoms out on any frame brighter than that.
    pub adapted_fury: f32,
}

/// What a `Team Selection` screen's own update writes over its `ShipModel` widget, for a title
/// whose update poses the craft by rest values rather than by a fit, see
/// [`FrontEnd::ship_preview_pose`]. The widget keeps its origin, `nearZ` and `ObeySafeZone`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipPose {
    /// The rest rotation about X then Y, radians.
    pub rotation: [f32; 2],
    /// Where the model sits across, up and in front of the lens, model units.
    pub position: [f32; 3],
    /// How much the model is scaled before it is placed.
    pub scale: f32,
    /// The lens the update writes into the item, a vertical field of view in degrees.
    pub fov_degrees: f32,
    /// A zoom of the finished picture about the widget's origin that the frame needs and no
    /// field read explains: the lens and scale above draw the craft this many times too small.
    pub zoom: f32,
}

/// How a `Team Selection` screen frames its craft, see
/// [`FrontEnd::ship_preview_fit`]: a fixed rest rotation and lens, and a depth
/// stepped every frame until a box the size of the selected model just fits a
/// window on screen. The box is `half_box` across and up and the model's own
/// authored length (`<FE length>`) fore and aft, turned by `rotation` and seen
/// through `fit_half_fov`.
///
/// **The fit measures in its own unit**: a projected point is its normalised
/// device coordinate times `unit`, so with `unit` the whole screen size a
/// window of `window` is half that many pixels each way, and the hull, longer
/// than its authored length, overhangs the box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipFit {
    /// The rest rotation about X then Y, radians.
    pub rotation: [f32; 2],
    /// The vertical field of view the craft is drawn through, degrees.
    pub fov_degrees: f32,
    /// The far plane the craft is drawn through.
    pub far: f32,
    /// Where the model's origin lands in the screen's own grid, before the fit
    /// slides it.
    pub origin: [f32; 2],
    /// Half the fit box's width and height, model units.
    pub half_box: [f32; 2],
    /// Half the vertical field of view the fit projects its box through, radians.
    pub fit_half_fov: f32,
    /// What a normalised device coordinate is multiplied by, across and down.
    pub unit: [f32; 2],
    /// The window the projected box is held inside, each way from its own
    /// centre, in `unit`s.
    pub window: [f32; 2],
    /// How far the depth moves in one frame, model units.
    pub step: f32,
    /// The depth target the screen opens at.
    pub start: f32,
    /// The fraction of the way to the depth target the drawn depth moves each
    /// frame.
    pub depth_ease: f32,
    /// The same for the origin slide's target.
    pub slide_ease: f32,
    /// The nearest the fit brings the craft.
    pub floor: f32,
    /// The box's fore and aft extent in a Zone race, in place of the model's
    /// length (the Zone hull's model authors none).
    pub zone_box: [f32; 2],
    /// The same in a Detonator race.
    pub detonator_box: [f32; 2],
}

/// The player's own turn, zoom and pan of the craft on `Team Selection`, read off the screen's
/// update (`docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`, "The player's own
/// turn, pan and zoom"). Every number is a constant of that function; per frame, not per second,
/// so a caller turns a tick into frames at the console's 60 a second.
///
/// The turn is two accumulators (about X and Y) the right stick pushes at `turn_rate` per frame
/// outside `deadzone`; the drawn turn eases toward them at `turn_ease` and both wrap at
/// `+-wrap` (`wrap` is the original's `3.1459`, a typo of pi, kept). The zoom is the zoom
/// button's pull times `zoom_scale`, eased at `zoom_ease`; the pan is the left stick times the
/// zoom (`pan_scale` across and down), eased at `pan_ease`, so it is dead until the craft is
/// zoomed. Anything that moves one resets an idle counter, and `idle_frames` frames later the
/// accumulators walk back to rest `idle_step` a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipControl {
    /// The right stick's dead band; outside it the stick counts as the distance past it.
    pub deadzone: f32,
    /// Radians an accumulator moves each frame at a stick's full travel past the dead band.
    pub turn_rate: f32,
    /// The fraction of the way to its accumulator the drawn turn moves each frame.
    pub turn_ease: f32,
    /// The accumulators wrap from `wrap` to `-wrap`, and the drawn turn goes with them.
    pub wrap: f32,
    /// What the zoom button's pull is multiplied by to make the zoom term.
    pub zoom_scale: f32,
    /// The fraction of the way to its target the zoom term moves each frame.
    pub zoom_ease: f32,
    /// A zoom term past this counts as the player touching the controls.
    pub zoom_active: f32,
    /// Degrees of lens the whole zoom term takes off the resting lens.
    pub fov_per_zoom: f32,
    /// How far the craft moves across and down at a full left stick, per unit of the zoom
    /// button's pull.
    pub pan_scale: [f32; 2],
    /// The fraction of the way to its target the pan moves each frame.
    pub pan_ease: f32,
    /// The push (depth) term's decay per frame: nothing in the update drives it.
    pub push_decay: f32,
    /// How much a push term moves the craft in depth.
    pub push_scale: f32,
    /// Frames without input before the accumulators walk back to rest.
    pub idle_frames: u32,
    /// Radians an accumulator walks each idle frame.
    pub idle_step: f32,
}

/// One circuit's front-end model, see [`FrontEnd::circuit_models`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircuitModel {
    /// The circuit's environment folder name, matched ignoring case against
    /// the last component of its location.
    pub environment: &'static str,
    /// The scene file under that folder's `FE` directory.
    pub file: &'static str,
}

/// The turntable `TrackSelection_Screen`'s update drives the circuit model with, every frame.
///
/// The update writes the widget's `RotY` from one global angle that it steps by `step` radians
/// each frame it runs (forwards on a circuit, backwards on a reversed one) and wraps at
/// `+-limit`, and writes `RotX`, `z` and `OriginY` from the selected circuit's own entry
/// ([`CircuitPose`], with `origin_y` added to its `PosY`). Per frame, not per second, so a
/// caller turns `dt` into frames at `frames_per_second`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircuitTurntable {
    /// Radians the angle moves each frame the screen's update runs.
    pub step: f32,
    /// The angle wraps from `limit` to `-limit` (and back, reversed).
    pub limit: f32,
    /// The vertical origin a circuit's `PosY` is added to, pixels of the 1920 by 1080 grid.
    pub origin_y: f32,
    /// The frame rate `dt` is read at: the console's display rate, so what a tick of the
    /// fixed 60 Hz clock is in the original's frames.
    pub frames_per_second: f32,
}

/// A circuit's own pose for its model on `Track Creation`: the `RotX`, `PosY` and `PosZ` its
/// `PI_Track` entry authors (`Values`), which the screen's update copies over the widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircuitPose {
    /// The pitch, radians.
    pub rot_x: f32,
    /// Added to [`CircuitTurntable::origin_y`].
    pub pos_y: f32,
    /// The depth, the widget's `z`.
    pub pos_z: f32,
}

// A pose is read from three finite numbers, so it equals itself; the entries that carry one are
// compared as a whole.
impl Eq for CircuitPose {}

impl FrontEnd {
    /// The control `backward` and `cancel` are on `platform`'s pressing: [`Self::back_button_ps2`]
    /// on a PS2, [`Self::back_button_psp`] on a PSP, the circle everywhere else. What the player's CONTROLS choice is relative to
    /// ([`Self::menu_buttons`]).
    #[must_use]
    pub fn back_button(&self, platform: Platform) -> prompts::Prompt {
        match platform {
            Platform::Ps2 => self.back_button_ps2.unwrap_or(prompts::Prompt::Circle),
            Platform::Psp => self.back_button_psp.unwrap_or(prompts::Prompt::Circle),
            _ => prompts::Prompt::Circle,
        }
    }

    /// The touch layout when the touch front end *is* the title's front end
    /// (2048): it authors no menu of its own vocabulary. A title with both
    /// (Omega: HD's menus plus 2048's campaign map) returns `None` here, and
    /// its [`Self::touch`] is only the map.
    #[must_use]
    pub fn touch_shell(&self) -> Option<&'static touch::TouchFrontEnd> {
        self.touch.filter(|_| self.menu.is_none())
    }

    /// Whether this title's circuit screen draws a model per circuit - see
    /// [`Self::circuit_models`].
    #[must_use]
    pub fn draws_circuit_models(&self) -> bool {
        !self.circuit_models.is_empty()
    }

    /// The model entry of the circuit whose environment folder is `location`
    /// (`Data\Environments\01_Vineta_K`), or `None` when the title draws no
    /// model for it.
    #[must_use]
    pub fn circuit_model(&self, location: &str) -> Option<String> {
        let folder = location
            .trim_end_matches(['\\', '/'])
            .rsplit(['\\', '/'])
            .next()?;
        let row = self
            .circuit_models
            .iter()
            .find(|row| row.environment.eq_ignore_ascii_case(folder))?;
        let base = location.trim_end_matches(['\\', '/']);
        Some(format!(r"{base}\FE\{}", row.file))
    }

    /// The language plugins a source with this `serial` offers, in picker order.
    ///
    /// The release's own manifest when one is recorded, otherwise
    /// [`Self::language_plugins`].
    #[must_use]
    pub fn offered_languages(&self, serial: Option<&str>) -> &'static [&'static str] {
        language::offered(
            self.language_manifests,
            self.language_plugins,
            self.assumed_release,
            serial,
        )
    }

    /// The overlay plugins a source with this `serial` loads over its language's
    /// strings, in manifest order. Empty for a release that loads none.
    #[must_use]
    pub fn language_overlays(&self, serial: Option<&str>) -> &'static [&'static str] {
        language::overlays_of(self.language_manifests, self.assumed_release, serial)
    }
}

/// The archive names a title's releases carry, in the order they are tried.
///
/// **Names, not paths, and found rather than derived.** Candidates are matched
/// against a source's own file list by trailing path component, so a PS2 pressing
/// whose serial directory differs needs no change, and a source that identifies
/// as neither console still opens if it holds a recognised archive.
///
/// The [`Platform`] on each candidate is what the archive *implies* about its
/// source, used only when the source says nothing. Nothing branches on it: every
/// decode is chosen by the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveCandidates {
    /// Update archives the original mounts *over* the base package: every one
    /// present is mounted, and they are searched **before** [`Self::data`], in the
    /// order listed (first listed wins a collision).
    ///
    /// For a title whose patch replaces entries its base also holds;
    /// [`Self::extra`] is searched after `data`, so a patch there loses to the
    /// entries it replaces. Empty where the patch is not mounted or absent;
    /// Omega's mandatory patch sits in `data`/`extra`, see `oag_omega::archives`.
    pub patch: &'static [(&'static str, Platform)],
    /// The bulk archive: tracks, ships, handling.
    pub data: &'static [(&'static str, Platform)],
    /// The companion archive, for the releases that have one.
    pub fe: &'static [(&'static str, Platform)],
    /// Every other archive the release ships, **all** of which are mounted.
    ///
    /// Not a candidate list: [`Self::data`] and [`Self::fe`] are alternatives tried
    /// in order, this is a set, every member opened if present.
    ///
    /// **HD ships seven** and they do not split by kind: `data/environments` and
    /// `data/ships` each appear in four, so a circuit's archive is not predictable
    /// from its path, and four of the twelve teams' `handlingstats.xml` are in the
    /// archive holding neither the circuits nor the other eight.
    ///
    /// **They overlap, so this list's order decides which copy is served.** 360 of
    /// HD's 11,139 distinct paths are in more than one archive, worst
    /// `/data/plugins/frontend/gui/skin.xml` (six, differing by MD5 in each; which
    /// the original loads is unresolved). `oag_assets::Archives::holder_of` takes
    /// the first hit walking `data`, `fe`, then this: a documented choice, not a
    /// measurement of the original.
    ///
    /// That matters for [`Self::plugin_definition`]: the file is in **five** of the
    /// seven and the copies disagree on how many teams exist (`DATA00`'s declares
    /// twelve, `DATA02`'s eight, `docs/formats/hd-hud.md`). First-listed happens to
    /// serve the fullest copy. See `oag_hd::names::FRONT_END_PLUGIN_DEFINITION`;
    /// `crates/hd/tests/hd_title_ground_truth.rs` asserts the overlap, which copy
    /// is served, and that the race path is outside it.
    ///
    /// Empty for both PSP titles.
    pub extra: &'static [(&'static str, Platform)],
    /// The members of [`Self::extra`] that are optional downloadable packages,
    /// each with what it declares for itself. Empty where all extras are part of
    /// the release.
    pub add_ons: &'static [AddOn],
}

/// A downloadable package the release mounts when the player owns it, in the
/// original and here: 2048's `dlc1.psarc` and `dlc2.psarc`.
///
/// One of [`ArchiveCandidates::extra`], so its files resolve through the search
/// order; this adds what the package says about *itself*. That has to be read out
/// of the package, not by name through the search order, because the base archive
/// carries a skeleton of the same file (2048's `data.psarc` holds a
/// sixteen-circuit, forward-only copy of the circuit list the packages ship in
/// full) and the base wins a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddOn {
    /// The archive's file name, matched case-insensitively against the end of a
    /// mounted extra archive's specifier.
    pub archive: &'static str,
    /// The entry the package declares its circuits in: `PI_Track` nodes in the
    /// schema [`Title::track_plugin_definition`] names, read from this archive alone.
    /// alone.
    pub tracks: &'static str,
    /// The entry listing the `PI_Grid` files (`Data\Plugins\grids\Definition.xml`
    /// on 2048). The grids ship in the base package and its patch; this package
    /// adds the circuits they race.
    pub grids: &'static str,
    /// The `Campaign=` value of the `PI_Grid` files this package opens.
    pub campaign: &'static str,
    /// The `GameModeChoice` tile that enters that campaign (`FE_RC_HD`).
    pub tile: &'static str,
    /// The team the campaign flies when the player's pick is not on its roster:
    /// the id Select Team opens on in that campaign.
    pub default_team: &'static str,
    /// The HUD layouts the campaign's events read in place of [`Title::hud`]: the
    /// race managers load other files for a downloadable campaign's modes.
    pub hud: &'static HudLayouts,
    /// Where the campaign's cells sit on the campaign map.
    pub nodes: NodeMap,
}

/// Where a downloadable campaign's cells land on the campaign map: a node's
/// position in canvas units (the column and row of a lattice cell) is the bias,
/// plus the offset of its grid, plus the cell's own `(x, y)` scaled. The numbers are the title's, read off its
/// executable; this is the arithmetic alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeMap {
    /// Canvas units one cell step moves a node, across and down.
    pub scale: (u32, u32),
    /// Added to every node of the campaign.
    pub bias: (u32, u32),
    /// The offset of each grid, by the grid's index (`grid0`, `grid1`...).
    pub group_offsets: &'static [(u32, u32)],
}

impl NodeMap {
    /// A node's position in canvas units. `None` for a grid with no offset row.
    #[must_use]
    pub fn node(&self, group: usize, cell: (u32, u32)) -> Option<(u32, u32)> {
        let (dx, dy) = *self.group_offsets.get(group)?;
        Some((
            self.bias.0 + dx + self.scale.0 * cell.0,
            self.bias.1 + dy + self.scale.1 * cell.1,
        ))
    }

    /// [`Self::node`] for a cell a file authors as signed numbers, moved by a
    /// signed nudge in canvas units. `None` as [`Self::node`], and for a cell
    /// or a result below zero.
    #[must_use]
    pub fn nudged(&self, group: usize, cell: (i32, i32), nudge: (i32, i32)) -> Option<(u32, u32)> {
        let cell = (u32::try_from(cell.0).ok()?, u32::try_from(cell.1).ok()?);
        let (x, y) = self.node(group, cell)?;
        Some((
            x.checked_add_signed(nudge.0)?,
            y.checked_add_signed(nudge.1)?,
        ))
    }
}

/// Which walk of a title's campaign map painter reaches an event, which decides
/// the ring of canvas cells painted around its tile: the main chain, a branch off
/// it, one of the speed-lap events a second loop paints, or none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MapRole {
    /// On the main chain.
    Chain,
    /// On a branch off the chain.
    Branch,
    /// One of the speed-lap events.
    SpeedLap,
    /// None: no ring.
    #[default]
    Side,
}

/// A serial positively identified as belonging to some other title.
///
/// **This rules a source out, never in.** The serials a title has been verified
/// against are not the universe of its legitimate pressings, so an allow-list
/// would reject a real player's disc. A serial absent from every list gets no
/// verdict and still has to find its archives by name.
///
/// Needed because sibling titles ship archives under identical names: Pure's PSP
/// disc carries `Data.wad` and `FE.wad` as Pulse does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForeignSerial {
    /// The serial, normalised as `AAAA-NNNNN`.
    pub serial: &'static str,
    /// What it actually is, for the error message.
    pub title: &'static str,
}

impl Title {
    /// The entries a Detonator race loads of its own beside the circuit: its layouts, its weapon
    /// table and its HUD layout (with the PS2's, where it has one). Empty where the title has no
    /// Detonator. A title offers the mode only where every one resolves in its archives, which on
    /// Pulse is where an enabled mod carries them
    /// (`docs/architecture/adr/0100-a-mod-carries-fallback-entries.md`).
    #[must_use]
    pub fn detonator_data(&self) -> Vec<&'static str> {
        let Some(detonator) = self.race.detonator else {
            return Vec::new();
        };
        let mut entries: Vec<&'static str> = detonator
            .layouts
            .iter()
            .flat_map(|l| [l.pattern_file, l.gameplay_values, l.scoring])
            .collect();
        entries.extend(self.weapons.detonator);
        entries.push(self.hud.detonator);
        entries.extend(detonator.ps2_hud);
        entries
    }

    /// What `choice` makes the front end's Back and Help buttons on `platform`'s pressing, for a
    /// Help screen the disc opens on `authored_help`. The one place that decides; a title with no
    /// front end of its own backs on the circle.
    #[must_use]
    pub fn menu_buttons(
        &self,
        platform: Platform,
        choice: prompts::MenuBack,
        authored_help: prompts::Prompt,
    ) -> prompts::MenuButtons {
        let original = self.front_end.map_or(prompts::Prompt::Circle, |front_end| {
            front_end.back_button(platform)
        });
        prompts::MenuButtons::resolve(original, choice, authored_help)
    }

    /// A grid's team id as the string table keys the team: a variant directory (`Assegai_c1`,
    /// `Feisar2048\3`) is its base team, spelled as [`Title::teams`] lists it. An id neither
    /// knows is returned as it is, livery and all.
    ///
    /// The variant half is [`race::RaceDefaults::base_team`], the rule [`Self::team_identity`]
    /// credits by, so the name a screen prints and the row an award banks agree. It drops a
    /// livery suffix first: an event's authored grid row reaches a slot with one still on it.
    #[must_use]
    pub fn team_key(&self, id: &str) -> String {
        let base = self.race.base_team(id).unwrap_or(id);
        team::listed_id(self.teams, base).to_string()
    }

    /// The shared identity `team` (an id as this title spells it) is kept under.
    ///
    /// **A variant directory credits its team.** The id a race reaches the records
    /// with is the combined one (`Assegai_c1`, `Feisar2048\3`) and a variant is a
    /// directory under a team, not a team of its own, so the row is the team's - the
    /// row that directory's own `<Unlock>` asks for
    /// (`docs/formats/race-setup.md`, "HD-era model unlocks") and the row Team
    /// Selection prints. See [`race::RaceDefaults::base_team`].
    #[must_use]
    pub fn team_identity(&self, team: &str) -> String {
        team::identity_of(self.teams, self.race.base_team(team).unwrap_or(team))
    }
    /// The name as a mod's `titles` list spells it: lowercase ASCII, every run of
    /// anything else one `-` (`Wipeout: Omega Collection` is
    /// `wipeout-omega-collection`).
    #[must_use]
    pub fn slug(&self) -> String {
        let mut out = String::new();
        for c in self.name.chars() {
            if c.is_ascii_alphanumeric() {
                out.push(c.to_ascii_lowercase());
            } else if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
        }
        out.trim_end_matches('-').to_string()
    }

    /// What this title throws on `trigger`, or `None` when it draws nothing there.
    #[must_use]
    pub const fn effect_on(
        &self,
        trigger: effects::Trigger,
    ) -> Option<&'static effects::EffectSpec> {
        self.effects.on(trigger)
    }

    /// What `serial` really belongs to, if this title knows it belongs to
    /// something else.
    #[must_use]
    pub fn foreign_title(&self, serial: &str) -> Option<&'static str> {
        self.foreign_serials
            .iter()
            .find(|known| known.serial == serial)
            .map(|known| known.title)
    }

    /// Every archive name this title's releases might carry, bulk first.
    ///
    /// For the "looked for" list in a "no archive here" error.
    #[must_use]
    pub fn archive_names(&self) -> Vec<String> {
        self.archives
            .patch
            .iter()
            .chain(self.archives.data)
            .chain(self.archives.fe)
            .chain(self.archives.extra)
            .map(|(name, _)| (*name).to_string())
            .collect()
    }
}
