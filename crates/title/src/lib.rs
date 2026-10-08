//! What a title ships, described in types a title package fills in.
//!
//! This crate is the *vocabulary* half of [ADR-0022]. It holds no constant that
//! belongs to any game: `oag-pulse` and `oag-pure` supply those, and
//! `oag-assets` reads them.
//!
//! # Two questions, and only one of them is here
//!
//! **"How does this byte stream decode?" is answered by the file**, inside
//! `oag-formats`, from the artifact's own version word. Class-ID tables, header
//! shapes and schema variants live there and are selected per blob, the same way
//! PSP and PS2 have always been told apart. No type in this crate describes a
//! file's contents, and adding one would be the first step toward a decoder that
//! has to be told which disc it is reading.
//!
//! **"What does this title ship?" is answered here.** Which archives exist and
//! what they are called is not in any file - it is a property of the release -
//! so it is the one thing a title package has to state.
//!
//! # Why this is so small
//!
//! ADR-0022 supersedes ADR-0009's refusal to abstract at n=1, but only for the
//! axes where a second corpus has actually been measured. `pure-status.md`
//! measured Pure's archive layout; it did not map Pure's HUD atlas or its mode
//! set. Types for those would be designed from one example, which is the failure
//! ADR-0009 named and ADR-0022 does not license. They stay as plain constants
//! inside the title crate that knows them until a second title forces their
//! shape.
//!
//! [`boot`] is the one axis that has since been forced, and by measurement:
//! both titles' boot sequences were cold-booted, they differ in length as well as
//! in content, and neither can be derived from the other or from its own XML. See
//! [ADR-0023], which supersedes ADR-0022 item 4 for that axis alone.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [ADR-0023]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0023-boot-sequence-as-title-data.md

pub mod adverts;
pub mod boot;
pub mod campaign;
pub mod effects;
pub mod endrace;
pub mod engine_effects;
pub mod exhaust;
pub mod flare;
pub mod hud;
pub mod language;
pub mod launch_hover;
pub mod loading;
pub mod menu;
pub mod pre_race;
pub mod pressing;
pub mod prompts;
pub mod race;
pub mod speed;
pub mod touch;
pub mod weapons;

pub use boot::{BootProfile, BootStep, Provenance};
pub use campaign::{Campaign, CampaignDialect};
pub use effects::{
    Burst, EffectSpec, Effects, Looks, Origin, Platforms, Rule, ShieldPalette, ShieldPalettes,
    Trigger,
};
pub use endrace::{EndRaceDialect, EndRaceStyle};
pub use hud::{HudArt, HudLayouts, ZoneSpeedClasses};
pub use language::LanguageManifest;
pub use loading::Loading;
pub use menu::{
    HelpText, ListBlocks, MenuBlocks, MenuList, MenuSettings, MenuSkin, MenuStrip, StripBlocks,
};
pub use oag_disc::Platform;
pub use pressing::{Pressing, Pressings};
pub use race::sound::{FrontEndSounds, MenuCues, StyledCue};
pub use race::{
    CircuitBanks, CountdownVoice, Crossfade, GuestRoster, RaceDefaults, SequenceTick, SoundBanks,
    SpeedClasses, TeamVariant, TeamVariants, TrackBanks, VariantJoin, ZoneAnnouncer, ZoneCircuit,
    ZoneClassAnnouncer, ZoneCraft, ZonePalette, ZoneStageTextures, ZoneStages, ZoneTransition,
};
pub use touch::{TouchButton, TouchFrontEnd};

/// One title's release-level facts.
///
/// A `&'static Title` is chosen once, by the composition root, from the title
/// crate the build is for. Nothing dispatches through it per call, and it is
/// deliberately a struct of tables rather than a trait: the differences between
/// two Wipeout releases are data, and a trait would put a virtual boundary
/// exactly where the PSP/PS2 split proved none is needed.
/// Not `Eq`: [`menu::MenuSkin`] carries the front end's offsets and scales as
/// `f32`, which is the unit the disc authors them in. Comparing two titles for
/// equality is not something this build does - selection is by `&'static`
/// identity - so the bound is not worth converting a layout table to fixed
/// point for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Title {
    /// The title as a person would name it, for reports and error messages.
    pub name: &'static str,
    /// Which bulk and companion archives its releases carry.
    pub archives: ArchiveCandidates,
    /// Serials known to belong to a *different* title. See
    /// [`ArchiveCandidates`] for why name matching alone cannot tell them apart.
    pub foreign_serials: &'static [ForeignSerial],
    /// This title's front end, or `None` when none of it has been recovered.
    ///
    /// See [`FrontEnd`] for why the layout and the boot chain are one field
    /// rather than two, and why `None` is a measurement rather than a hole
    /// waiting to be filled.
    pub front_end: Option<&'static FrontEnd>,
    /// Which layout this title's in-race HUD is read from, per mode. See
    /// [`hud::HudLayouts`].
    pub hud: &'static hud::HudLayouts,
    /// How this title's HUD sprites reach the screen: their textures, which of
    /// them are always up, and the one colour this build substitutes. See
    /// [`hud::HudArt`].
    pub hud_art: &'static hud::HudArt,
    /// What a race falls back to when the caller names no circuit or team. See
    /// [`race::RaceDefaults`].
    ///
    /// Hung off `Title` for the same reason [`Self::boot`] is: it is needed
    /// **before** the archives are open, because a command line has to name a
    /// circuit, and the alternative is a caller comparing titles to pick a
    /// constant.
    ///
    /// That comparison is not merely awkward, it does not work: a title package
    /// declares its `Title` as a `const`, so `std::ptr::eq` against it compares
    /// promoted temporaries whose addresses need not be equal. Carrying the
    /// answer removes the question.
    pub race: &'static race::RaceDefaults,
    /// Where this title keeps the exhaust ribbon's texture. See
    /// [`exhaust::Exhaust`], which is an axis because the two answers are not
    /// the same kind of thing: Pulse and Pure name a texture, Wipeout HD ships
    /// a ribbon template whose material names its own.
    pub exhaust: &'static exhaust::Exhaust,
    /// Where this title keeps the **engine flare** - the light at the nozzle,
    /// as distinct from the ribbon [`Self::exhaust`] answers for. See
    /// [`flare::Flare`], and see that module for why the two are separate axes
    /// rather than variants of one.
    pub flare: &'static flare::Flare,
    /// Which codepoints in this title's strings are controller-button glyphs,
    /// and which control each depicts. See [`prompts::Prompts`].
    pub prompts: &'static prompts::Prompts,
    /// The plugin definition declaring what this release carries to race with:
    /// one `PI_Team` node per team, one `PI_Track` per circuit, one `PI_Music`
    /// per soundtrack track.
    ///
    /// **An axis because a third title disagreed**, on exactly the terms
    /// [`FrontEnd::root`] became one: both PSP titles number the game plugin
    /// (`Data\Plugins\PI001\Definition.xml`) and Wipeout HD names it
    /// (`Data\Plugins\Frontend\Definition.xml`), so the `oag-pulse` constant
    /// every caller reached for is a constant that is wrong for one of three.
    /// Reaching for it anyway is what made an HD grid eight copies of the
    /// player's craft: the read missed, the roster came back empty, and
    /// `livery::teams_for_slots` did the only honest thing it could with no
    /// teams.
    ///
    /// The schema is shared, which is what makes one field enough - HD's file
    /// carries the same three node kinds under the same attribute names, and
    /// `oag_raceplay::catalogue` reads all three off it unchanged.
    ///
    /// **Overlaps [`DeclaredTracks::declared_in`] and is deliberately not
    /// merged with it.** The two agree on the two titles that carry both, and
    /// that agreement is unmeasured rather than structural: `Music` is `None`
    /// on a title whose music is unlocated and `tracks` is `None` on Pulse,
    /// so routing this through it could not answer for Pulse at all.
    pub plugin_definition: &'static str,
    /// Where `PI_Track` lives, when it is not [`Self::plugin_definition`].
    ///
    /// **`None` on every title but one.** Pulse, Pure and Wipeout HD each ship
    /// one file carrying all three node kinds, which is what makes
    /// [`Self::plugin_definition`] a single field for them; Wipeout 2048 splits
    /// into three, one file per kind, so its `plugin_definition` names the
    /// *team* one (a race needs a roster before it needs a circuit list) and a
    /// caller after `PI_Track` has nowhere else to read from `plugin_definition`
    /// alone. `None` means "the same file", never "no circuits".
    pub track_plugin_definition: Option<&'static str>,
    /// What this title puts on screen while it loads, or `None` when it puts up
    /// nothing of its own.
    ///
    /// **`None` is a measurement here**, the same way [`Self::front_end`]'s is:
    /// Pure ships neither the tips plugin nor the glow strip in any of its three
    /// archives, checked 2026-08-12, so there is nothing of its own to draw and
    /// the screen falls back to this build's own counts. See
    /// [`loading::Loading`], whose module doc carries the three-way table this
    /// axis exists for.
    pub loading: Option<&'static loading::Loading>,
    /// Where this title keeps its music, or `None` when none of it has been
    /// located. See [`Music`].
    pub music: Option<&'static Music>,
    /// Which file this title tunes its weapons from. See [`weapons::Weapons`].
    ///
    /// An axis because the answers differ in shape: Pulse and Wipeout HD ship
    /// two tables and choose between them by race mode, Pure ships one. Until
    /// 2026-08-26 every caller reached for `oag_tables::weapons::RACE_ENTRY`,
    /// which is Pulse's spelling - so a Pure race found no table, parsed no
    /// weapons and handed out no pickups, silently, with only a report line to
    /// say so.
    pub weapons: &'static weapons::Weapons,
    /// Where this title keeps each weapon's own body model. See
    /// [`weapons::WeaponModels`].
    ///
    /// A required field rather than `Option<&'static WeaponModels>`, on the
    /// same footing [`Self::weapons`] already takes: every title's `TITLE`
    /// const supplies one, `Default` where nothing is recovered yet, so a
    /// caller never has to ask "does this title even have a table" before
    /// asking a field of it.
    pub weapon_models: &'static weapons::WeaponModels,
    /// What this title throws on each race [`effects::Trigger`], with where each
    /// answer came from. `None` on a trigger means draw nothing. See
    /// [`effects`] and ADR-0058.
    pub effects: &'static effects::Effects,
    /// What this title draws or does in a race beyond its effect triggers, as
    /// per-platform rules with their origin. See [`effects::Looks`].
    pub looks: &'static effects::Looks,
    /// Which campaign reader and draw list this title's front end uses, and
    /// what of the campaign it authors. See [`campaign::Campaign`].
    pub campaign: &'static campaign::Campaign,
    /// What this title's executable resolves per pressing (the title screen's
    /// wordmark, a localised boot movie), or `None` when it resolves nothing
    /// per pressing. See [`pressing::Pressings`].
    pub pressings: Option<&'static pressing::Pressings>,
    /// The flyby this title plays before its countdown, or `None` where it plays none or none
    /// has been read. See [`pre_race::PreRace`].
    pub pre_race: Option<&'static pre_race::PreRace>,
    /// How this title draws its circuits' billboard adverts, or `None` where
    /// they are not drawn. See [`adverts::Adverts`].
    ///
    /// **`None` is "not measured or not shipped", never a default**: Pulse and
    /// Wipeout HD each carry their own target size and clip planes, and a title
    /// with none of its own draws no advert rather than borrowing another's.
    pub adverts: Option<&'static adverts::Adverts>,
    /// The mouse pointer drawn over this title's screens, as SVG source.
    ///
    /// **Ours, not the disc's, on every title.** Nothing in this lineage was
    /// authored for a mouse, so there is no cursor to recover; each title
    /// package draws one in its own menu palette and says so in the file -
    /// see `assets/cursors/README.md`. On `Title` rather than on
    /// [`MenuSkin`] because the one screen a title shows with no menu skin
    /// at all (a placeholder for a title whose front end is unread) still
    /// answers a pointer and still needs one to point with. Rasterised and
    /// drawn by `oag_game::cursor`.
    pub cursor: &'static str,
}

/// Where a title keeps the music its front end and its races play.
///
/// # A measured axis, not a designed one
///
/// Both PSP titles build their front end's music path in the executable rather
/// than reading it from any file, and the two paths differ in shape as well as
/// in spelling: Pulse expands a `Data\Music\FEMusic\frontend%d.at3` template
/// and Pure names one literal file. Neither is derivable from the other, which
/// is what ADR-0022 asks for before an axis becomes a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Music {
    /// The front end's own looping track, by entry name in the bulk archive,
    /// or `None` when this build has not recovered one (Omega: no front-end
    /// music event or state is as clear in its bank as the race tracks').
    ///
    /// Not one of the soundtrack tracks on either title: it is short, it loops,
    /// and it is addressed by a name the executable spells out where a
    /// soundtrack track is addressed by [`Self::track_file`] or not at all.
    pub front_end: Option<&'static str>,
    /// How the soundtrack tracks are addressed, for a title whose entries this
    /// build has recovered **by name**. See [`DeclaredTracks`].
    ///
    /// `None` for Wipeout Pulse, and that is a statement about **this build**
    /// rather than about the disc. Pulse's `Definition.xml` declares its sixteen
    /// tracks exactly the way Pure's declares nineteen and every one of them
    /// resolves to a real entry; switching Pulse over would reorder its race
    /// playlist and change which track a PS2 boot's menu plays, which is a
    /// separate change with its own evidence to record. `oag_sound` finds
    /// Pulse's sixteen by what the entries *are* until then - see `HANDOVER.md`.
    pub tracks: Option<DeclaredTracks>,
    /// A soundtrack addressed by a Wwise bank state rather than by file. See
    /// [`StateTracks`]. Exclusive with [`Self::tracks`] in practice.
    pub state_tracks: Option<StateTracks>,
}

/// A soundtrack whose playlist is plugin XML and whose audio is picked by
/// setting Wwise states, the way Wipeout: Omega Collection plays one.
///
/// Each `PI_Music` entry's `location` is the `N` of an event
/// ([`Self::set_track_event`]) that sets [`Self::track_group`] to that song's
/// state; with the flow states of [`Self::race_flow`] also set, playing
/// [`Self::play_event`] walks the bank's music switches down to the song.
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
    /// `(state group, state)` pairs set for the menus, or empty when this
    /// build has not read which plays the front end's music.
    pub front_end_flow: &'static [(&'static str, &'static str)],
}

/// Where a title declares its soundtrack, and what a declaration points at.
///
/// One `PI_Music` node per track, each naming the directory the track lives in;
/// the audio itself is [`Self::file`] inside that directory. Reading the
/// declaration is what makes the soundtrack order the **disc's own** rather
/// than whatever order the archive directory happens to be in - which matters
/// because "the first track" is then something the release decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclaredTracks {
    /// The plugin definition whose `PI_Music` nodes declare them, by entry name
    /// in the bulk archive.
    pub declared_in: &'static str,
    /// The file inside each declared location.
    pub file: &'static str,
}

/// One title's front end: how it lays menus out, and how it boots into them.
///
/// # Why the two are one field
///
/// They are recovered together and they are useless apart. A boot chain walks
/// screens whose layout comes from the skin; a skin with no chain has nothing
/// to open it. Carrying them as two independent `Option`s would admit three
/// states, two of which no title can be in - and `oag-game`'s `load_shell`
/// would then need two refusals where one is the honest answer.
///
/// **[ADR-0054](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md)
/// narrows this for one axis, not both.** [`Self::menu`] became
/// `Option<&'static menu::MenuSkin>` because a fourth title's chain *is*
/// real and walkable while its menu vocabulary genuinely is not
/// `MenuSkin`-shaped - "a chain with a skin that cannot be honestly filled"
/// is a state a real title is in, unlike "a skin with no chain" above, which
/// stays impossible: [`Self::boot`] is still mandatory on `FrontEnd`, so
/// `oag-game` still needs exactly one refusal for "no front end at all"
/// ([`crate::Title::front_end`] being `None`) and a second, narrower one for
/// "no [`menu::MenuSkin`]-shaped menu to draw" - see
/// [`Self::menu`]/[`Self::touch`].
///
/// # Why `None` is a result
///
/// Wipeout HD ships `/data/plugins/frontend/gui/skin.xml` and a directory of
/// screen definitions beside it, and **not one number out of either has been
/// read**. A [`MenuSkin`](menu::MenuSkin) filled in from Pulse's would be a
/// table of measurements attributed to a disc nobody measured, which is exactly
/// what `CLAUDE.md`'s "never invent what the assets already author" forbids.
/// `None` says the front end is unrecovered; a caller that needs one refuses by
/// name and says so, which is a visible absence rather than a wrong picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrontEnd {
    /// The archive entry holding this title's front-end root: every boot
    /// screen, the `FEGlobals` block, and the `LoadXML` list pulling in the
    /// rest.
    ///
    /// **An axis because a third title disagreed**, which is the bar
    /// [ADR-0022] sets. Both PSP titles keep it at
    /// `Data\Plugins\PI001\GUI\Skin.xml` - a *numbered* plugin - and sharing
    /// one constant between them was correct while the corpus was two. Wipeout
    /// HD names its plugins instead (`Data\Plugins\Frontend\Gui\Skin.xml`), the
    /// same way its soundtrack plugin is named rather than numbered, so a
    /// constant in `oag-pulse` reached for by every title is now a constant
    /// that is wrong for one of three.
    ///
    /// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
    pub root: &'static str,
    /// The plugins that carry a language, in the order the disc lists them.
    ///
    /// A *token* rather than a path, joined into
    /// `Data\Plugins\{token}\Definition.xml` by the caller - which is why the
    /// PSP titles' `PI008` and Wipeout HD's `Languages\English` can share one
    /// mechanism despite one being a numbered plugin and the other a named
    /// directory two deep. Same divergence as [`Self::root`], same fix.
    ///
    /// The plugin token is the only stable handle: a language's own name is
    /// inside its definition, not in its path, so this list is what is read back
    /// from the archive rather than trusted.
    pub language_plugins: &'static [&'static str],
    /// What each release of this title offers, keyed by disc serial, read off
    /// the executable's own plugin manifest. Empty where no manifest has been
    /// read; [`Self::language_plugins`] is then the whole answer.
    ///
    /// [`Self::language_plugins`] stays the default for a caller with no serial
    /// in hand and for a release not listed here, so it is the *superset* a
    /// disc may carry, not an offered list. See [`Self::offered_languages`].
    pub language_manifests: &'static [language::LanguageManifest],
    /// The release a source that reports **no serial** is taken to be, for
    /// [`Self::offered_languages`] alone. `None` where every source this title
    /// opens names its own release, which is every title but Omega: its PS4
    /// extract keeps no `param.sfo`, and the EU base-plus-patch pair is the only
    /// one this project holds. **Chosen, not measured.**
    pub assumed_release: Option<&'static str>,
    /// The namespace under `assets/ui/strings/disc/` holding this project's own
    /// translations of this title's disc text, keyed by this title's idstrings.
    /// `None` for a title nobody has translated yet: Pure, HD and Omega reuse
    /// id spellings, so a namespace is never shared between titles and a
    /// project language there reads the disc's English until one is written.
    /// A disc's own language only has its *gaps* filled from it (an id the
    /// disc's table lacks or leaves empty); a project language is overlaid.
    pub disc_strings: Option<&'static str>,
    /// How this title lays menus out, for a title that authors the
    /// `FEGlobals`/`<Menu>`/`<HorizMenu>` vocabulary [`menu::MenuSkin`]
    /// describes. See that type.
    ///
    /// A measured axis: both PSP titles' `Skin.xml` files were read and they
    /// agree on nothing they share. Presentation only - the menu tree itself is
    /// this project's, not the disc's, and lives in `assets/ui/menu.toml`.
    ///
    /// **`Option` since [ADR-0054], not because a fourth title's menu is
    /// merely unread.** Wipeout 2048 authors no `<FEGlobals>` block and no
    /// `<Menu>`/`<HorizMenu>` widget anywhere - a touch-icon grid over a
    /// persistent scene instead, a vocabulary this type was never built to
    /// hold. Filling it with placeholder numbers for a menu shape 2048 does
    /// not draw would be exactly the invented-stand-in failure `CLAUDE.md`'s
    /// "never invent what the assets already author" section forbids, so
    /// `None` here is a measurement about *this front end's idiom*, on the
    /// same footing [`Self::menu_frame`]'s own `None` already carries for a
    /// title whose frame has not been read - except here no reading would
    /// ever fill it. See [`Self::touch`] for what 2048 fills instead, and
    /// `docs/formats/2048-frontend.md`'s "Why `front_end` stays `None`"
    /// section (written before this field existed, when the whole struct had
    /// to refuse for want of this one axis).
    ///
    /// [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md
    pub menu: Option<&'static menu::MenuSkin>,
    /// [`Self::menu`] again, for a title's PS2 pressing specifically, when one
    /// has been read off that pressing's own `Skin.xml` rather than approximated
    /// from the other.
    ///
    /// **Why this exists instead of `menu` alone scaling by [`menu::MenuSkin::space`]:**
    /// `oag_game::menu::Skin::new`'s own doc already says the PS2 conversion
    /// is "an approximation and says so" - `oag_display::space::Space` records
    /// 30 of 43 shared `Skin.xml` coordinates landing within a pixel of a
    /// uniform 640/480, 448/272 stretch and 13 not. Wipeout Pulse's own PS2
    /// `Data\Plugins\PI001\GUI\MainMenu_Definition.xml` is one of the 13: its
    /// `<Menu>` authors `y="53"` (not `32 * 448/272 = 52.7` - that one is
    /// close by coincidence) and its `helptext0` a `y="75"`, 22 below the row
    /// rather than PSP's 18 scaled to 29.6 - the gap the scaled number leaves
    /// before the next row (`row_pitch` 41, authored in the same file's own
    /// `<!-- +41 for each line -->` comment, against the scaled approximation's
    /// 33.9) is what let that subtitle overrun `RACEBOX`/`COURSE` below it.
    /// See `oag_pulse::frontend::PS2_MENU_SKIN`'s own doc for the full table.
    ///
    /// `None` for every title including Pulse's *other* pressings - a PSP-only
    /// title has nothing to read here, and Pulse's own PSP pressings keep
    /// using [`Self::menu`] unchanged. Falls back to [`Self::menu`] when
    /// `None`, so filling this in only ever narrows an approximation, never
    /// widens a gap: a title read for one pressing and not the other still
    /// boots and draws on both.
    pub menu_ps2: Option<&'static menu::MenuSkin>,
    /// How this title lays a touch-icon front end out, for a title that
    /// authors *that* vocabulary instead of [`menu::MenuSkin`]'s. See
    /// [`touch::TouchFrontEnd`].
    ///
    /// `None` for every title but Wipeout 2048 today - not because the other
    /// three were checked and found to lack one (Pulse and Pure are PSP
    /// titles with no touchscreen at all; Wipeout HD's PS3 controller front
    /// end was already measured as [`menu::MenuSkin`]-shaped), but because
    /// only 2048 has ever been read for this axis. See [`Self::menu`] for the
    /// vocabulary this is not a replacement for, and the module docs on
    /// [`touch`] for why this stays a single-corpus type rather than an
    /// [ADR-0022]-licensed one.
    pub touch: Option<&'static touch::TouchFrontEnd>,
    /// How this title's own boot sequence goes. See [`boot::BootProfile`].
    ///
    /// Hung off [`Title`] rather than selected separately so that there is
    /// **one** selection point: opening one title's archives while driving
    /// another's chain is then not an inconsistency to be avoided but a state
    /// that cannot be constructed. The build reached this design after three
    /// independent screen-name probes had each been answering "which title is
    /// this?" in their own words, with three chances to disagree.
    ///
    /// `oag-assets` receives this and never reads it, which is the price: the
    /// asset layer holds a field of screen names it has no business in. Inert
    /// data, and cheaper than two things to keep in step.
    pub boot: &'static boot::BootProfile,
    /// The screen whose own widgets frame every menu: the rules, the corner
    /// marks and the colour the frame is cleared to.
    ///
    /// **A screen name, and deliberately not a table of coordinates.** Wipeout
    /// HD authors its whole front-end frame on one screen - `FE Screen`, the
    /// parent every menu screen is nested inside - as a `<ScreenClear>` and
    /// three `<Image>` widgets. Naming the screen lets the caller *play the
    /// data*, which is what `CLAUDE.md` asks for; transcribing the four numbers
    /// per image into this crate is the hand-transcribed-table failure that rule
    /// exists to prevent, and it would not survive the disc being re-read.
    ///
    /// `None` for a title whose frame has not been read, which is Pure today.
    /// **That is a gap and not a measurement.** Pulse's own `Top FE
    /// Screen->FE Screen` carries its light angled top bar and the two bars
    /// framing the footer's news ticker - one `<Image>` apiece, all three
    /// patches of one shared `Data\FE\Images\pulse_assets.mip`, nested three
    /// anonymous `<Screen>` levels down (an XML grouping idiom this build's
    /// widget collection now walks through - see `oag_game::screen`'s own
    /// `collect_widgets`). Not `topbarleft`/`topbarcenter`/`topbarright`: that
    /// was a guess made before the XML was actually read: on the disc it is
    /// one bar image, not three.
    pub menu_frame: Option<&'static str>,
    /// File stems (lower case, no extension) of the `.gnf` images stored with
    /// their rows in HD's `.gtf` order, bottom-up, so the sprite sheet must
    /// reverse them as it does a `.gtf`'s. Empty for every title but Omega.
    ///
    /// A `.gnf` carries no row-order flag, so which ones is a census: Omega's
    /// packager re-authored the images its screens name top-down (135 of the 143 asymmetric ones
    /// that match an HD file by name and size match its rows reversed), but
    /// eight are HD's bytes exactly, rows in HD's file order. The names are
    /// measured against HD's own `.gtf` of the same stem
    /// (`docs/formats/omega-status.md`, "The menu blocks"); an image on this
    /// list that no HD file matches would be a guess, which is why it is a
    /// census and not a rule.
    pub bottom_up_gnf: &'static [&'static str],
    /// The definition file that authors the race box's own selection
    /// screens - `Track Creation`/`Track Selection` and `Team Selection` -
    /// or `None` for a title whose race box has not been read.
    ///
    /// Named here rather than found through the skin's `LoadXML` list
    /// because the list is 23 files and reading every one at boot for two
    /// screens is the wrong trade; and because which file holds them is a
    /// fact about the title. Pulse and Pure both:
    /// `Selection_Definition.xml` beside their own skin, the same relative
    /// path on both discs (`docs/formats/race-setup.md`) - Pure's own
    /// confirmed 2026-09-10, not assumed from Pulse's, despite Pure
    /// otherwise splitting its race box into a chain of screens the rest of
    /// which are not read. HD authors its two in separate files, in its own
    /// dialect - `None` here, see [`Self::team_select`].
    pub race_box: Option<&'static str>,
    /// A standalone `Team Selection` definition, for a title that authors
    /// its ship screen in a file of its own rather than in [`Self::race_box`]:
    /// Wipeout HD/Fury's `Team_Selection_Definition.xml`, which its live
    /// skin includes and its `Cell Selection` redirects to. `None` on every
    /// other title. **Only the ship screen**: a title naming this and no
    /// race box keeps its RACE page's own TRACK row, since nothing reads a
    /// track screen for it.
    pub team_select: Option<&'static str>,
    /// A standalone `Track Creation` definition, the track screen's
    /// counterpart to [`Self::team_select`]: Wipeout HD/Fury's
    /// `Track_Selection_Definition.xml`, which its live skin includes and
    /// which only `DATA06` carries. `None` on every other title, whose
    /// track screen is in [`Self::race_box`] or unread.
    pub track_select: Option<&'static str>,
    /// The definition holding the race box's `Single Player` screen, the page
    /// whose lists are the settings a custom race is set up with. Read for
    /// one thing today: the `Eliminations` list, the kill targets the
    /// `KILLS` row offers (see `oag_game::boot::kill_targets`). `None` on a
    /// title whose setup page is unread, which shows no kill target to pick.
    pub race_setup: Option<&'static str>,
    /// Whether this title's race-setup screens preview an entry with a
    /// rendered mesh.
    ///
    /// **An axis because Pure disagrees with Pulse**, which is the bar
    /// [ADR-0022] sets, and the disagreement is total rather than a change of
    /// file name. Pulse draws `<location>\FE\forward.vex` for a circuit and
    /// `<location>\ship_FE.vex` for a craft, and neither file exists on
    /// Pure's disc at all: every preview on both of Pure's screens is a
    /// pre-rendered still, authored in the entry's own `screen.xml` the same
    /// way Pulse's hexagon-window stills are, and matched to the pixels the
    /// original draws at RMSE 0 (`docs/formats/race-setup.md`). Pulse draws
    /// *both* - a mesh and a still chain - so "does this entry author
    /// stills?" cannot stand in for this, which is why it is stated here
    /// rather than inferred from the data.
    ///
    /// `false` for HD, where the screens preview with stills and widgets of
    /// their own (its track screen's emblem, see [`Self::track_select`]) and
    /// no mesh is loaded. HD authors `<Model
    /// name="TrackModel">` / `<Model name="ShipModel">` widgets with a camera
    /// stated, which is a third convention again, and reading it is what
    /// would set this to `true`.
    ///
    /// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
    pub preview_meshes: bool,
    /// The file under a craft's own directory that its selection screen draws
    /// as the 3-D craft when [`Self::preview_meshes`] is `false`: HD's
    /// `Team Selection` draws the race hull (`ship.vex`, the pair
    /// `ship.rcsmodel`) in its `ShipModel` frame, and ships no `ship_FE.vex`
    /// (checked 2026-10-08: no `*FE*` mesh under any HD `Data\Ships\<team>`).
    /// `None` everywhere a title has no such screen or draws stills.
    pub ship_preview_hull: Option<&'static str>,
    /// The definition file that authors the three screens a race ends on -
    /// `EndRace Results`/`EndRace Rewards`/`EndRace Menu` - or `None` for a
    /// title whose copy has not been read.
    ///
    /// **An axis for the same reason [`Self::root`] is: a second title
    /// disagreed.** Both PSP titles keep Pulse's own numbered-plugin path,
    /// `Data\Plugins\PI001\GUI\EndRace_Definition.xml`, dictionary-shortened
    /// the way every PSP screen file is. Wipeout HD names its own plugin
    /// instead, `Data\Plugins\Frontend\Gui\EndRace_Definition.xml` - plain
    /// UTF-8, not shortened - and ships it in five of its seven archives, no
    /// two copies alike by MD5. **Three of HD's own screen names match
    /// Pulse's and almost none of the widgets underneath do**: HD's `EndRace
    /// Results` is the whole field's finishing order (`Grid{col}.{row}`, a 4
    /// column by 10 row template - two columns actually captioned, `POS` and
    /// `TIME` - not Pulse's own per-lap table), and its `EndRace Menu` is one
    /// `<Block>` per option shown by mode rather than a populated list. So
    /// this field states *where the file is*, not that the two titles' own
    /// screens share a reader - see `docs/formats/hd-endrace-screens.md` for
    /// the widget-by-widget read and `oag_game::endrace`'s own title dispatch
    /// for the two separate loaders this axis feeds.
    ///
    /// `None` for Pure and Omega: neither this project's own build nor a
    /// documented pass has wired whether either ships such a screen, so `None`
    /// here is a gap, not a measurement that one is absent - the same
    /// distinction [`Self::race_box`]'s own doc draws for the titles it is
    /// `None` on. Wipeout 2048 ships one (`NEWGUI/EndRace_Definition.xml`).
    pub endrace_entry: Option<&'static str>,
    /// The dialect [`Self::endrace_entry`]'s file is written in, which picks
    /// the reader - see [`endrace`]. `None` for a title whose file is unread
    /// (Pure) or read and not wired (Omega), neither of which is a
    /// measurement that the title ships no such screen.
    pub endrace_style: Option<EndRaceStyle>,
}

impl FrontEnd {
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
}

/// The archive names a title's releases carry, in the order they are tried.
///
/// **Names, not paths, and found rather than derived.** The candidates are
/// matched against a source's own file list by trailing path component, so a
/// PS2 pressing whose serial directory differs needs no change here, and a
/// source that identifies as neither console still opens if it holds an archive
/// one of them would recognise. A path constant would be right for exactly one
/// pressing.
///
/// The [`Platform`] on each candidate is what the archive *implies* about its
/// source, used only when the source itself says nothing. Nothing branches on
/// it: every decode this project has is chosen by the data rather than by the
/// disc it came off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveCandidates {
    /// Update archives the original mounts *over* the base package: every one
    /// present is mounted, and they are searched **before** [`Self::data`], in
    /// the order listed (first listed wins a collision).
    ///
    /// The role a title needs when its patch replaces entries its base also
    /// holds. [`Self::extra`] cannot do it, because it is searched after
    /// `data`, and a patch behind the entries it exists to replace is worse
    /// than none. Empty for every title whose patch is not mounted or that has
    /// none; Omega's mandatory patch sits in `data`/`extra` for its own
    /// reasons, see `oag_omega::archives`.
    pub patch: &'static [(&'static str, Platform)],
    /// The bulk archive: tracks, ships, handling.
    pub data: &'static [(&'static str, Platform)],
    /// The companion archive, for the releases that have one.
    pub fe: &'static [(&'static str, Platform)],
    /// Every other archive the release ships, **all** of which are mounted.
    ///
    /// The one field here that is not a candidate list: [`Self::data`] and
    /// [`Self::fe`] are alternatives, tried in order until one is found, and
    /// this is a set, every member of which is opened if present.
    ///
    /// Two archives were enough while the corpus was Pulse and Pure, which ship
    /// two apiece that anything reads. **Wipeout HD ships seven**, and they do
    /// not split by kind: `data/environments` and `data/ships` each appear in
    /// four of them, so a circuit's archive is not predictable from its path and
    /// four of the twelve teams' `handlingstats.xml` are in the archive that
    /// holds neither the circuits nor the other eight. Picking two would have
    /// silently lost a third of the roster.
    ///
    /// **They overlap, and this list's order therefore decides which copy is
    /// served.** 360 of HD's 11,139 distinct paths are in more than one of the
    /// seven, worst of all `/data/plugins/frontend/gui/skin.xml`, which is in
    /// six and differs by MD5 in every one - and which of those six the original
    /// loads is unresolved. `oag_assets::Archives::holder_of` takes the first
    /// hit walking `data` then `fe` then this, so first-listed wins, and that is
    /// a documented choice rather than a measurement of what the original does.
    ///
    /// **It stopped being harmless on 2026-08-18.** A circuit and a team's
    /// handling do each sit in exactly one archive, and that was the whole of
    /// what this project read - until [`Self::plugin_definition`] wired the
    /// roster up. That file is in **five** of the seven and the copies disagree
    /// about how many teams exist: `DATA00`'s declares twelve and `DATA02`'s
    /// eight. So the ordering now decides part of what a player sees, and
    /// first-listed happens to serve the fullest copy rather than being chosen
    /// to. See `oag_hd::names::FRONT_END_PLUGIN_DEFINITION` for the table.
    ///
    /// `crates/hd/tests/hd_title_ground_truth.rs` asserts the overlap, which of
    /// the five is served, and that the race path is still outside it - so a
    /// reordering fails there rather than quietly changing a roster.
    ///
    /// Empty for both PSP titles, so nothing about their load changes.
    pub extra: &'static [(&'static str, Platform)],
}

/// A serial positively identified as belonging to some other title.
///
/// **This rules a source out, never in, and that asymmetry is the point.** The
/// serials any one title has actually been verified against are not the full
/// universe of its legitimate pressings, so an allow-list would hard-reject a
/// real player's own disc - worse than not checking at all. A serial absent from
/// every list gets no verdict and still has to find its archives by name.
///
/// The check exists because sibling titles ship archives under identical names:
/// Pure's PSP disc carries `Data.wad` and `FE.wad` exactly as Pulse does, so
/// name matching alone would open it as if it were Pulse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForeignSerial {
    /// The serial, normalised as `AAAA-NNNNN`.
    pub serial: &'static str,
    /// What it actually is, for the error message.
    pub title: &'static str,
}

impl Title {
    /// What this title throws on `trigger`, or `None` when it draws nothing
    /// there (it does not do it, or it is unread).
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
