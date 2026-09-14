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

pub mod boot;
pub mod exhaust;
pub mod flare;
pub mod hud;
pub mod loading;
pub mod menu;
pub mod race;
pub mod speed;
pub mod weapons;

pub use boot::{BootProfile, BootStep, Provenance};
pub use hud::{HudArt, HudLayouts, ZoneSpeedClasses};
pub use loading::Loading;
pub use menu::{ListBlocks, MenuBlocks, MenuList, MenuSkin, MenuStrip, StripBlocks};
pub use oag_disc::Platform;
pub use race::{
    GuestRoster, RaceDefaults, SoundBanks, SpeedClasses, TeamVariant, TeamVariants, VariantJoin,
    ZoneAnnouncer, ZoneCircuit, ZoneClassAnnouncer, ZoneCraft, ZonePalette, ZoneStageTextures,
    ZoneStages,
};

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
    /// `oag_game::catalogue` reads all three off it unchanged.
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
    /// The front end's own looping track, by entry name in the bulk archive.
    ///
    /// Not one of the soundtrack tracks on either title: it is short, it loops,
    /// and it is addressed by a name the executable spells out where a
    /// soundtrack track is addressed by [`Self::track_file`] or not at all.
    pub front_end: &'static str,
    /// How the soundtrack tracks are addressed, for a title whose entries this
    /// build has recovered **by name**. See [`DeclaredTracks`].
    ///
    /// `None` for Wipeout Pulse, and that is a statement about **this build**
    /// rather than about the disc. Pulse's `Definition.xml` declares its sixteen
    /// tracks exactly the way Pure's declares nineteen and every one of them
    /// resolves to a real entry; switching Pulse over would reorder its race
    /// playlist and change which track a PS2 boot's menu plays, which is a
    /// separate change with its own evidence to record. `oag_game::audio` finds
    /// Pulse's sixteen by what the entries *are* until then - see `HANDOVER.md`.
    pub tracks: Option<DeclaredTracks>,
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
    /// How this title lays menus out. See [`menu::MenuSkin`].
    ///
    /// A measured axis: both PSP titles' `Skin.xml` files were read and they
    /// agree on nothing they share. Presentation only - the menu tree itself is
    /// this project's, not the disc's, and lives in `assets/ui/menu.toml`.
    pub menu: &'static menu::MenuSkin,
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
    /// which are not read. HD authors
    /// `track_selection_definition.xml`/`team_selection_definition.xml` in a
    /// dialect this build's picker does not yet read - `None`, a gap rather
    /// than a measurement.
    pub race_box: Option<&'static str>,
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
    /// `false` for HD, where it is inert: HD's [`Self::race_box`] is `None`,
    /// so no picker opens and nothing reads this. HD authors `<Model
    /// name="TrackModel">` / `<Model name="ShipModel">` widgets with a camera
    /// stated, which is a third convention again, and reading it is what
    /// would set this to `true`.
    ///
    /// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
    pub preview_meshes: bool,
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
            .data
            .iter()
            .chain(self.archives.fe)
            .chain(self.archives.extra)
            .map(|(name, _)| (*name).to_string())
            .collect()
    }
}
