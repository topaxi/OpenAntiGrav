//! What Wipeout Pulse ships: which archives its releases carry, what the
//! entries inside them are called, and the hashes of the ones nobody has named.
//!
//! This is a title package in the sense of [ADR-0022]: tables, not decoding.
//! Every constant here was resolved by hashing a candidate name and matching it
//! against a real archive directory, not guessed. See
//! `docs/architecture/frontend-boot.md` for how each one was found and what it
//! contains.
//!
//! # What is deliberately *not* here
//!
//! Nothing about how a Pulse file decodes. `.vex` class IDs, the `WO Track`
//! header shape and the handling-stats schema all live in `oag-formats`,
//! selected from each file's own version word, because those are properties of a
//! format version rather than of a release - Pulse and Pure differ there, but a
//! decoder learns which it is holding by reading the file, never by being told
//! which disc it came off.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

pub mod campaign;
pub mod effects;
pub mod frontend;
pub mod hud;
pub mod loading;
pub mod movies;
pub mod prompts;
pub mod race;
pub mod shadow;
pub mod tag_input;
pub mod textures;

use oag_assets::{Archives, Result};
use oag_title::{ArchiveCandidates, ForeignSerial, Platform, Title};

/// Wipeout Pulse, as the asset layer needs to know it.
pub const TITLE: &Title = &Title {
    name: "Wipeout Pulse",
    archives: ArchiveCandidates {
        data: DATA_CANDIDATES,
        fe: FE_CANDIDATES,
        // Both PSP titles ship exactly two archives anything reads. See
        // `ArchiveCandidates::extra` for the release that does not.
        extra: &[],
    },
    foreign_serials: FOREIGN_SERIALS,
    front_end: Some(FRONT_END),
    hud: hud::LAYOUTS,
    hud_art: hud::ART,
    race: race::DEFAULTS,
    // `Data\Tex\engineFlare\Engine_noise.mip`, a literal string in both the
    // PSP and PS2 executables, so an exact archive hit rather than a mined
    // candidate. Everything else the ribbon needs is `Trail_InitPreset`'s
    // table in the code - see `docs/rendering/trail-ribbon.md`.
    exhaust: &oag_title::exhaust::Exhaust::Named(r"Data\Tex\engineFlare\Engine_noise.mip"),
    // The flare's own texture, a second literal in the same executable -
    // `Texture_LoadEngineFlare` at `0x08a84c80`. Note the directory case
    // differs from the noise map's; the WAD hash is case-insensitive.
    flare: &oag_title::flare::Flare::Sprite(r"Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip"),
    // Measured: the six stand-in codepoints and the glyph each draws, read off
    // `pulse_text.fnt`, `Pulse_14.fnt` and `Pulse_20.fnt` - see `prompts`.
    prompts: prompts::PROMPTS,
    plugin_definition: names::GAME_PLUGIN_DEFINITION,
    track_plugin_definition: None,
    loading: Some(&loading::LOADING),
    music: Some(MUSIC),
    cursor: include_str!("../../../assets/cursors/pulse.svg"),
    // The two tables Pulse picks between by race mode. `DAT_08b32428` is what
    // selects the file - measured `0` in a single race and `1` in Eliminator,
    // at Venom both times, so it is the *file* and not the speed class. See
    // `docs/ghidra/functions/psp-pulse-usa/missile.md`.
    weapons: &oag_title::weapons::Weapons {
        race: r"Data\XML\WeaponStats_Race.xml",
        elimination: Some(r"Data\XML\WeaponStats_Elimination.xml"),
        ai: Some(r"Data\XML\WeaponAIstats.xml"),
    },
    weapon_models: race::WEAPON_MODELS,
    effects: effects::EFFECTS,
    looks: effects::LOOKS,
    // Measured: `docs/ui/campaign-screens.md` (the grid and cell screens),
    // `docs/formats/race-campaign.md`'s `<Unlock Grid>` rows and
    // `docs/formats/race-setup.md`'s `Loyalty` unlock.
    campaign: &oag_title::Campaign {
        dialect: oag_title::CampaignDialect::Pulse,
        definition_entry: Some(campaign::DEFINITION_ENTRY),
        circuit_unlocks: true,
        loyalty_unlocks: true,
        unlocks_origin: oag_title::Origin::Measured,
        selection_strings: false,
        origin: oag_title::Origin::Measured,
    },
    pressings: None,
};

/// Where Pulse keeps its music.
///
/// The front-end path is **named, not guessed**: the executable builds it at
/// run time from the template at `0x08a88e94`, `Data\Music\FEMusic\frontend%d.at3`
/// (see `docs/formats/vex.md`), and hashing the expansion finds an entry for
/// every `%d` from 1 to 8 and none for 0, so the numbering starts at one. All
/// eight are stereo ATRAC3+ at 44,100 Hz. Which of the eight belongs to which
/// menu is not established, so the first is what this names.
///
/// [`oag_title::Music::tracks`] is `None` here; its doc comment carries the
/// whole of why.
pub const MUSIC: &oag_title::Music = &oag_title::Music {
    front_end: Some(names::FRONT_END_MUSIC),
    tracks: None,
    state_tracks: None,
};

/// Pulse's front end: the layout its `Skin.xml` authors and the boot chain a
/// cold boot was measured to walk.
///
/// Both halves are recovered, so this is `Some` on [`TITLE`]. See
/// [`oag_title::FrontEnd`] for why a title that has neither says `None` rather
/// than borrowing this one.
pub const FRONT_END: &oag_title::FrontEnd = &oag_title::FrontEnd {
    root: names::FRONTEND_ROOT,
    language_plugins: LANGUAGE_PLUGINS,
    disc_strings: Some("pulse"),
    language_manifests: LANGUAGE_MANIFESTS,
    menu: Some(frontend::MENU_SKIN),
    // Read off the PS2 pressing's own `Skin.xml`/`MainMenu_Definition.xml`,
    // not scaled from the PSP's - see `frontend::PS2_MENU_SKIN`'s own doc.
    menu_ps2: Some(frontend::PS2_MENU_SKIN),
    // Pulse authors the `FEGlobals`/`<Menu>` vocabulary `MenuSkin` describes,
    // not the touch-icon idiom `TouchFrontEnd` does - see ADR-0054.
    touch: None,
    boot: frontend::BOOT_PROFILE,
    // Pulse authors a frame on `Top FE Screen->FE Screen`: a light angled top
    // bar and two bars framing the footer's news ticker, all three patches of
    // one shared `Data\FE\Images\pulse_assets.mip` sitting three anonymous
    // `<Screen>` levels down. Naming it puts that bar on screen and changes a
    // picture this build already draws.
    menu_frame: Some(frontend::states::FE_SCREEN),
    race_box: Some(names::RACE_BOX_DEFINITION),
    team_select: None,
    track_select: None,
    race_setup: Some(names::RACE_SETUP_DEFINITION),
    // `<location>\FE\forward.vex` and `<location>\ship_FE.vex`, both
    // resolving on both pressings - see `docs/formats/race-setup.md`.
    preview_meshes: true,
    // `Data\Plugins\PI001\GUI\EndRace_Definition.xml` - `oag_game::endrace`'s
    // own long-standing constant, restated here now that a second title
    // needs this axis. See `oag_title::FrontEnd::endrace_entry`.
    endrace_entry: Some(r"Data\Plugins\PI001\GUI\EndRace_Definition.xml"),
    endrace_style: Some(oag_title::EndRaceStyle::measured(
        oag_title::EndRaceDialect::Pulse,
    )),
};

/// The bulk archive's candidates, in the order they are tried.
const DATA_CANDIDATES: &[(&str, Platform)] = &[
    (archives::DATA, Platform::Psp),
    (archives::ps2::DATA, Platform::Ps2),
];

/// The companion archive's candidates, in the order they are tried.
const FE_CANDIDATES: &[(&str, Platform)] = &[
    (archives::FE, Platform::Psp),
    (archives::ps2::FE, Platform::Ps2),
];

/// Serials positively identified as a Studio Liverpool title other than
/// Wipeout Pulse, each backed by an owned disc recorded in
/// `docs/reverse-engineering/source-images.md`.
///
/// **Not the inverse of a Pulse allow-list.** The two Pulse serials this
/// project has actually verified (`UCUS-98712`, `SCES-54748`) are not the
/// full universe of legitimate Pulse pressings - an unconfirmed EU PSP
/// release (`UCES-00465`) is already implied by evidence recorded on
/// [`hashes::DEVPUB_REEL_SCEE`]'s doc comment. Allow-listing would hard-reject
/// a real player's own legitimately-owned disc, which is worse than not
/// checking at all. So this table only rules a source *out*: a serial absent
/// from it gets no verdict and still has to find its own archive by name,
/// same as before this table existed.
const FOREIGN_SERIALS: &[ForeignSerial] = &[
    ForeignSerial {
        serial: "UCUS-98612",
        title: "Wipeout Pure",
    },
    ForeignSerial {
        serial: "UCES-00001",
        title: "Wipeout Pure",
    },
];

/// Opens whichever archives `source` carries, as Wipeout Pulse.
///
/// A one-line convenience over [`Archives::open`] so a caller that already knows
/// which title it wants does not repeat [`TITLE`] at every call site.
///
/// # Errors
///
/// Propagates [`Archives::open`].
pub fn open(source: &str) -> Result<Archives> {
    Archives::open(source, TITLE)
}

/// [`open`], with downloadable content mounted behind the source.
///
/// The packs are Pulse's, so the title they mount behind is Pulse's too - see
/// [`Archives::open_with_packs`] for the search order and why the disc wins a
/// collision.
///
/// # Errors
///
/// Propagates [`Archives::open_with_packs`].
pub fn open_with_packs(source: &str, packs: Vec<oag_assets::dlc::Pack>) -> Result<Archives> {
    Archives::open_with_packs(source, TITLE, packs)
}

/// Reads a raster image by the name its own data declares, from wherever this
/// source keeps it.
///
/// The declared name is tried first, so the PSP path is exactly
/// [`Archives::read_name`] and needs no console test. Only when that misses is
/// the name rewritten the way the PS2 build rewrites it - see
/// [`ps2_texture_name`] - and an image that is under neither name still fails
/// with the original in the message: `Data\FE\Images\gameshare_backdrop.mip`
/// really is absent from the PS2 disc and should keep saying so.
///
/// No platform test. The rewritten name is simply a second name to try, and on
/// a PSP source the first one has already answered; a `.pct` entry does not
/// exist there for the rewrite to collide with.
///
/// This lives here rather than on [`Archives`] because the rewrite is a fact
/// about how Pulse's PS2 build resolves a texture name, not about archives in
/// general.
///
/// # Errors
///
/// [`oag_assets::Error::NoSuchEntry`] naming the entry the caller asked for.
pub fn read_image(archives: &mut Archives, name: &str) -> Result<Vec<u8>> {
    match archives.read_name(name) {
        Ok(blob) => Ok(blob),
        Err(original) => {
            let Some(ps2) = ps2_texture_name(name) else {
                return Err(original);
            };
            archives.read_name(&ps2).map_err(|_| original)
        }
    }
}

pub use oag_formats::wad::{ps2_strip_build_prefix, ps2_texture_name};

/// The four archives a PSP Pulse disc ships, relative to the image root.
pub mod archives {
    /// Front-end fonts and shared images.
    pub const FE: &str = "PSP_GAME/USRDIR/FE.wad";
    /// Per-ship and per-track front-end screens.
    pub const FEDATA: &str = "PSP_GAME/USRDIR/FEData.wad";
    /// Everything else: tracks, ships, movies, plugins, string tables.
    pub const DATA: &str = "PSP_GAME/USRDIR/Data.wad";
    /// Back-end data.
    pub const BEDATA: &str = "PSP_GAME/USRDIR/BEData.wad";

    /// The four archives a PS2 Pulse disc ships.
    ///
    /// **File names only, and deliberately so.** The directory holding them is
    /// derived from the disc's own serial - `54748/` on SCES-54748 - so a path
    /// constant would be right for one pressing and wrong for the next.
    /// [`oag_assets::Layout::resolve`] finds them by name instead, which needs no
    /// assumption about how a serial becomes a directory.
    ///
    /// See `docs/ps2/pulse-disc-layout.md`.
    pub mod ps2 {
        /// Tracks, ships and handling: the [`super::DATA`] analogue, 7,200
        /// entries in the same WAD container the PSP uses.
        pub const DATA: &str = "WADS2.WAD";
        /// The 193-entry companion archive. Also carries models, which is why
        /// it is searched rather than assumed redundant.
        pub const FE: &str = "WADSP.WAD";
        /// Music, in a container with a different header. Not parsed.
        pub const MUSIC: &str = "PS2MUSIC.WAD";
        /// 85 MiB at entropy 0.084, header unread. Not parsed.
        pub const PRERACE: &str = "PRERACE.WAD";
    }
}

/// Entry names inside `Data.wad`.
pub mod names {
    /// The front-end root: every boot screen, the `FEGlobals` variables, and
    /// the `LoadXML` list that pulls in the rest of the menus.
    pub const FRONTEND_ROOT: &str = r"Data\Plugins\PI001\GUI\Skin.xml";
    /// The definition holding the race box's `Track Creation` and `Team
    /// Selection` screens, one of [`FRONTEND_ROOT`]'s 23 `LoadXML` includes.
    /// See `docs/formats/race-setup.md`.
    pub const RACE_BOX_DEFINITION: &str = r"Data\Plugins\PI001\GUI\Selection_Definition.xml";
    /// The race box's `Single Player` screen: its `Mode`, `Class`, `Weapons`,
    /// `Difficulty` and `Eliminations` lists. See `docs/formats/race-setup.md`.
    pub const RACE_SETUP_DEFINITION: &str = r"Data\Plugins\PI001\GUI\RaceBox_Definition.xml";
    /// The in-race screens: the pause menu, the photo mode's, and `Race End
    /// Photo`, the state a race sits in between the flag and `EndRace Results`.
    /// No recovered name list carried it; it is the hash `31b50f2e`'s name,
    /// found by hashing the obvious sibling of `EndRace_Definition.xml`. See
    /// `docs/gameplay/after-the-finish.md`.
    pub const INGAME_DEFINITION: &str = r"Data\Plugins\PI001\GUI\InGame_Definition.xml";

    /// The game plugin's own definition: which circuits, teams, ship models and
    /// music tracks this release carries.
    ///
    /// `PI001` is the game plugin, where `PI008`-`PI012` are the language ones.
    /// The circuits are the part this project reads today - see
    /// `oag_raceplay::catalogue` - and they are **entries rather than directories**:
    /// two of them can name one environment and differ only by `Reversed`.
    pub const GAME_PLUGIN_DEFINITION: &str = r"Data\Plugins\PI001\Definition.xml";

    /// The music the front end loops under its menus.
    ///
    /// The first expansion of the executable's own template - see
    /// [`crate::MUSIC`] for the evidence and for why it is the first. Its
    /// `fact` chunk declares 1,302,720 samples, which decodes to **29.5
    /// seconds**, and that is what a `--dump-audio` capture of a PSP boot
    /// reports.
    pub const FRONT_END_MUSIC: &str = r"Data\Music\FEMusic\frontend1.at3";

    /// The intro movie played by the `LogoFMV` and `Play Intro` screens.
    ///
    /// The name is not in the executable: the `Movie` widget builds it from the
    /// `src` attribute in [`FRONTEND_ROOT`] plus `.PMF`.
    pub const INTRO_MOVIE: &str = r"Data\Movies\Intro.PMF";

    /// The looping backdrop behind the main menu.
    pub const BACKDROP_MOVIE: &str = r"Data\Movies\Backdrop.PMF";

    /// The dev/pub reel: the European cut, by name.
    ///
    /// **The name was recovered, and this used to be `hash:b1ba72c3`.** The reel
    /// is `localised="true"` and ships in four regional cuts, so the naming rule
    /// is a `_<REGION>` suffix - `_EU` here, hashing to
    /// [`crate::hashes::DEVPUB_REEL_SCEE`], verified by resolving it on both
    /// titles' discs. The other three are
    /// `oag_pure::names::INTRO_MOVIE_CUTS`; the reel is a shared asset, and Pure
    /// is the title that actually plays it.
    ///
    /// Its contents fit `Intro Screen->IntroMovie1`'s own constants - 260 frames,
    /// holds at 144 and 231 - which is what those constants describe: **Pure's
    /// developer/publisher screen**, inherited into Pulse's executable and never
    /// entered at boot there.
    ///
    /// A title fact rather than an engine one, which is why it lives here: the
    /// reel is a thing this title ships, and `--reel` is a flag that asks for it.
    pub const DEVPUB_REEL: &str = r"Data\Movies\IntroMovieP1_EU.PMF";

    /// The five `.fnt` bitmap fonts, in `FE.wad`.
    ///
    /// Named from the `<Font Src="...">` attributes in the language plugins and
    /// confirmed by hashing: each of these hashes to an entry that really is in
    /// the archive. See `docs/formats/fnt.md`.
    pub mod fonts {
        /// `Default`, 13-pixel line height.
        pub const TEXT: &str = r"Data\FE\Fonts\pulse_text.fnt";
        /// `Small`, `Title`, `InGame` and `Stats`, 17-pixel line height.
        pub const PULSE_14: &str = r"Data\FE\Fonts\Pulse_14.fnt";
        /// `Menu`, 22-pixel line height.
        pub const PULSE_20: &str = r"Data\FE\Fonts\Pulse_20.fnt";
        /// `HUD`, 25-pixel line height, the only 512-wide atlas.
        pub const HUD: &str = r"Data\FE\Fonts\PulseHud.fnt";
        /// `HUDSmall`, 10-pixel line height.
        pub const SMALL: &str = r"Data\FE\Fonts\small.fnt";
    }

    /// The font the front end draws body text with.
    pub const DEFAULT_FONT: &str = fonts::TEXT;

    /// A language plugin's font table, native language name and string table
    /// pointer.
    #[must_use]
    pub fn language_definition(plugin: &str) -> String {
        format!(r"Data\Plugins\{plugin}\Definition.xml")
    }

    /// A language plugin's string table, the file its `Definition.xml` calls
    /// the "Dynamic Entry File Source".
    #[must_use]
    pub fn language_entries(plugin: &str) -> String {
        format!(r"Data\Plugins\{plugin}\entries.xml")
    }
}

/// Name hashes for `Data.wad` entries whose names are not recovered.
///
/// A WAD directory stores only the hash of each name, so an entry nobody has
/// named is still perfectly addressable. Recording the hash is what keeps such
/// an entry usable without inventing a name for it - which the naming rules in
/// `CLAUDE.md` forbid below 50 confidence, and a movie filename that no string
/// search, no XML and no runtime trace has produced is well below that.
pub mod hashes {
    /// `Data.wad`'s front-end screen holding the `Name`/`Tag` `<TagInput>`
    /// profile-entry widgets - see [`crate::tag_input`] and
    /// `docs/formats/fexml.md`'s `TagInput` section.
    ///
    /// **Name unresolved**: 16 prefix variants were tried against this hash
    /// and none hit, so it is reached by hash rather than by path. **The
    /// index is not stable across pressings and the hash is**: entry 1083 on
    /// `pulse-psp-usa.chd`'s `Data.wad`, entry 1082 on `pulse-psp-eu.chd`'s -
    /// both this same hash, 40,083 bytes.
    pub const TAG_INPUT_SCREENS: u32 = 0xb94f_e6f9;

    /// The dev/pub reel `Intro Screen->IntroMovie1` plays, in its three
    /// regional cuts.
    ///
    /// **Not a boot movie.** The disc's own boot opens `Data\Movies\Intro.PMF`
    /// and `Data\Movies\Backdrop.PMF` and nothing else, and never enters the
    /// state whose counters these fit; where they *are* played is unestablished.
    ///
    /// Each is 480x272, 260 frames, 8.68 s, `PSMF0012` - a different container
    /// version from the `PSMF0014` of [`names::INTRO_MOVIE`], which is a second
    /// sign the two came off different pipelines - with ATRAC3+ audio. All three
    /// are static across frames 144 and 231, the two the intro state holds for
    /// two seconds, and moving on either side. Decoded, all three show the same
    /// pair of cards, and only the publisher line differs:
    ///
    /// | Constant | Frame 144 | Frame 231 |
    /// | --- | --- | --- |
    /// | [`DEVPUB_REEL_SCEE`] | Sony Computer Entertainment *Europe* presents | A Studio Liverpool game |
    /// | [`DEVPUB_REEL_SCEI`] | Sony Computer Entertainment *Inc.* presents | A Studio Liverpool game |
    /// | [`DEVPUB_REEL_SCEA`] | Sony Computer Entertainment *America* presents | A Studio Liverpool game |
    ///
    /// See `docs/architecture/frontend-boot.md`.
    /// All three ship on every disc regardless of that disc's own region -
    /// *Pure*'s USA disc carries these same three hashes at the same three
    /// sizes - so the set is region-invariant content and the cut must be picked
    /// at runtime. Which mechanism picks it has not been read out of the binary.
    ///
    /// This one is what `oag-game --reel` defaults to, because the executable on the image
    /// this project reads is the EU build throughout despite its `UCUS-98712`
    /// serial: 18 `UCES00465` strings in `BOOT.BIN` and no `UCUS` string at all,
    /// an ISO volume id and publisher of `SCEE`, and a whole
    /// `PSP_GAME/USRDIR/UCES00465/` tree on the disc.
    pub const DEVPUB_REEL_SCEE: u32 = 0xb1ba_72c3;
    /// The Japanese cut. See [`DEVPUB_REEL_SCEE`].
    pub const DEVPUB_REEL_SCEI: u32 = 0x41fb_d22f;
    /// The American cut, which the disc's `UCUS-98712` serial argues for and
    /// nothing else does. See [`DEVPUB_REEL_SCEE`].
    pub const DEVPUB_REEL_SCEA: u32 = 0x3d2c_85f8;

    /// The PS2 `WADS2.WAD` entry holding the in-race HUD atlas, the one the
    /// five layouts call `Data\HUD\Textures\PulseHUD.mip`.
    ///
    /// **That name hashes to nothing on the PS2 disc**, and neither does any
    /// spelling of it: 60 path, case and extension variants were tried against
    /// all 7,393 hashes in both archives and none hit. The entry was found by
    /// its picture instead - see [`PS2_IMAGES`] for the method and the
    /// evidence.
    pub const PS2_HUD_ATLAS: u32 = 0xbeaf_613c;
    /// A second, byte-identical copy of [`PS2_HUD_ATLAS`], at entry 3583 where
    /// the first is at 3518.
    ///
    /// Not an ambiguity to resolve: the PSP ships this atlas twice too, in
    /// `FE.wad` and `Data.wad`, byte-identical at 66,576 bytes. Both PS2 copies
    /// are 13,446 stored / 66,829 unpacked and both score 0.9999 against the
    /// PSP silhouette. Recorded so a reader who finds the second one knows it
    /// is the same picture.
    pub const PS2_HUD_ATLAS_DUPLICATE: u32 = 0xf012_b5af;
    /// The PS2 entry holding `Data\FE\Images\pulse_logo.mip`, entry 3421.
    ///
    /// This is the missing half of the `Show Logo` screen recorded in
    /// `HANDOVER.md`.
    pub const PS2_PULSE_LOGO: u32 = 0x1e6c_873e;
    /// The PS2 entry holding `Data\FE\Images\pulse_assets.mip`, entry 3420 -
    /// immediately before [`PS2_PULSE_LOGO`].
    pub const PS2_PULSE_ASSETS: u32 = 0x0d31_af1b;
}

/// Three PS2 entries that were found by their picture, kept as the check on the
/// rule that now finds every one of them by name.
///
/// # This was a table, and it is now evidence
///
/// The PS2 release keeps its raster images as ordinary PS2 textures (see
/// `oag_texture::ps2_texture`) in `WADS2.WAD`, and for a long time this crate
/// held no way to reach them: `Data\HUD\Textures\PulseHUD.mip` hashes to
/// `57d37d8c`, which is in neither archive, and 60 candidates across path shape,
/// case and extension were hashed against all 7,393 entry hashes and every one
/// missed. So these three were found by their picture instead, and the note here
/// said the game's own lookup was not known.
///
/// **It is known now**, and the earlier sweep missed it by one extension:
/// [`ps2_texture_name`] rewrites `.mip` to `.pct`, and all three of these
/// hashes fall straight out of it - `Data\HUD\Textures\PulseHUD.pct` is
/// `beaf613c`, `pulse_logo.pct` is `1e6c873e`, `pulse_assets.pct` is
/// `0d31af1b`. Three hashes recovered by picture correlation, reproduced exactly
/// by a one-line name rule derived independently from the executable, is as
/// close to a proof as this project gets, which is why the table stays: it is
/// checked against the rule offline, with no disc, by
/// `the_pct_rule_reproduces_every_picture_matched_hash`.
///
/// Nothing calls [`ps2_image_hash`] on the loading path any more -
/// [`read_image`] applies the rule instead, which reaches every PS2 texture on
/// the disc rather than these three.
///
/// # How the entries were found, and how sure it is
///
/// By the picture. Each PSP `.mip` was decoded, reduced to a silhouette - one
/// bit per pixel, "is this texel's palette entry transparent" - and compared
/// against every same-shaped PS2 texture on the disc. The measure is
/// independent of palette order, which differs between the two builds.
///
/// | PSP name | PS2 entry | Silhouette agreement |
/// | --- | ---: | ---: |
/// | `Data\HUD\Textures\PulseHUD.mip` | 3518, 3583 | 0.9999, over 614 same-shape candidates |
/// | `Data\FE\Images\pulse_logo.mip` | 3421 | 0.9946 |
/// | `Data\FE\Images\pulse_assets.mip` | 3420 | 0.9999 |
///
/// Each is separated from the runner-up by a wide margin (0.71, 0.51, 0.51),
/// and all three were then decoded and looked at: 3518 is the speed and shield
/// bars with the weapon icons, 3421 is the Wipeout Pulse wordmark. For the HUD
/// atlas there is a fourth, independent agreement: the `U`/`V`/`TxtrWidth`/
/// `TxtrHeight` boxes in the PS2 layouts are **identical** to the PSP's, so the
/// two discs index the same 256x256 arrangement.
///
/// # One image really is absent
///
/// `Data\FE\Images\gameshare_backdrop.mip` is **not on the PS2 disc**, which is
/// a finding rather than a gap - Game Sharing is a PSP ad-hoc feature. Note the
/// silhouette test cannot say so: that image is 93.75 % opaque, so every
/// candidate scores 0.9375 trivially. Correlating the picture instead settles
/// it, at 0.02 across all 28 same-shaped candidates. Do not re-run the weaker
/// test on it.
pub const PS2_IMAGES: &[(&str, u32)] = &[
    (r"Data\HUD\Textures\PulseHUD.mip", hashes::PS2_HUD_ATLAS),
    (r"Data\FE\Images\pulse_logo.mip", hashes::PS2_PULSE_LOGO),
    (r"Data\FE\Images\pulse_assets.mip", hashes::PS2_PULSE_ASSETS),
];

/// The PS2 entry hash for an image the XML names the PSP way, if there is one.
///
/// See [`PS2_IMAGES`] for how the mapping was established.
#[must_use]
pub fn ps2_image_hash(name: &str) -> Option<u32> {
    PS2_IMAGES
        .iter()
        .find(|(psp, _)| psp.eq_ignore_ascii_case(name))
        .map(|(_, hash)| *hash)
}

/// Every plugin that may carry a language on any Pulse release, English first.
///
/// A **superset**, for a caller with no serial in hand (the race's HUD strings)
/// and a release [`LANGUAGE_MANIFESTS`] does not list: a plugin a disc lacks is
/// skipped, so the EU's `PI000` and the USA's `PI012` can share one list. What a
/// release actually *offers* is [`LANGUAGE_MANIFESTS`]. The plugin id is the
/// only stable handle: the language's own name is inside the plugin, not in its
/// path.
pub const LANGUAGE_PLUGINS: &[&str] = &["PI000", "PI012", "PI010", "PI008", "PI009", "PI011"];

/// Each Pulse release's offered languages, read out of its executable's
/// `Plugin_LoadManifest` table (`docs/architecture/frontend-boot.md`): USA PSP
/// at pseudo-address `0x2ad10c`, EU PSP `0x2ac88c`, PS2 EU `SCES_547.48` (string run at file offset `0x1ab4b0`)
/// English is `PI012` on USA and the PS2 pressing and `PI000` on
/// the PSP EU disc, which has no `PI012`; the order is the manifest's own.
///
/// **That the picker follows manifest order is measured on Pure only** (its
/// USA and EU pickers were observed in manifest order); Pulse's picker exits
/// in under a frame on a cold boot and was never captured, so this carries
/// Pure's rule across. Confidence 70 for the order, 85 for the membership.
pub const LANGUAGE_MANIFESTS: &[oag_title::LanguageManifest] = &[
    oag_title::LanguageManifest {
        serial: "UCES-00465",
        plugins: &["PI000", "PI008", "PI009", "PI010", "PI011"],
        evidence: "pulse-psp-eu BOOT.BIN manifest; English is PI000, no PI012 on the disc",
    },
    oag_title::LanguageManifest {
        serial: "UCUS-98712",
        plugins: &["PI012", "PI010", "PI008", "PI009", "PI011"],
        evidence: "pulse-psp-usa BOOT.BIN manifest (0x08ab110c)",
    },
    oag_title::LanguageManifest {
        serial: "SCES-54748",
        plugins: &["PI012", "PI010", "PI008", "PI009", "PI011"],
        evidence: "pulse-ps2-eu SCES_547.48 manifest string run",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_language_plugins_are_distinct() {
        let mut sorted = LANGUAGE_PLUGINS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), LANGUAGE_PLUGINS.len());
    }

    #[test]
    fn plugin_entry_names_are_built_the_way_the_game_builds_them() {
        assert_eq!(
            names::language_definition("PI008"),
            r"Data\Plugins\PI008\Definition.xml"
        );
        assert_eq!(
            names::language_entries("PI012"),
            r"Data\Plugins\PI012\entries.xml"
        );
    }

    /// The deny-list rules a source *out* and never in: an unknown serial gets
    /// no verdict. See [`FOREIGN_SERIALS`].
    #[test]
    fn only_a_known_foreign_serial_gets_a_verdict() {
        assert_eq!(TITLE.foreign_title("UCUS-98612"), Some("Wipeout Pure"));
        assert_eq!(TITLE.foreign_title("UCES-00001"), Some("Wipeout Pure"));
        assert_eq!(TITLE.foreign_title("UCUS-98712"), None);
        assert_eq!(TITLE.foreign_title("SCES-54748"), None);
        assert_eq!(TITLE.foreign_title("UCES-00465"), None);
    }

    /// The three entries in [`PS2_IMAGES`] were recovered by correlating
    /// pictures, months before the executable's own rewrite was read. Every one
    /// of them falls out of [`ps2_texture_name`] plus the ordinary WAD name
    /// hash, which is the whole reason the table is still here.
    ///
    /// Offline: no disc, no archive, just the two functions agreeing.
    #[test]
    fn the_pct_rule_reproduces_every_picture_matched_hash() {
        for &(psp, found_by_picture) in PS2_IMAGES {
            let ps2 = ps2_texture_name(psp).expect("every entry in the table is a .mip name");
            assert_eq!(
                oag_formats::wad::hash_name(&ps2),
                found_by_picture,
                "{psp} -> {ps2}"
            );
        }
    }

    /// The rewrite is the PS2 texture resolver's, so it fires on the two
    /// extensions that resolver's own table lists and on nothing else. A `.vex`
    /// or a `.pob` is found under its declared name on both discs, and rewriting
    /// one would send a miss looking for a file that cannot exist.
    #[test]
    fn only_a_source_art_extension_is_rewritten() {
        assert_eq!(
            ps2_texture_name(r"Data\Tex\engineFlare\Engine_noise.mip").as_deref(),
            Some(r"Data\Tex\engineFlare\Engine_noise.pct")
        );
        assert_eq!(
            ps2_texture_name(r"DATA\SHIPS\COMMON\TEXTURES\ENGINE_GLOW.TGA").as_deref(),
            Some(r"DATA\SHIPS\COMMON\TEXTURES\ENGINE_GLOW.pct")
        );
        assert_eq!(ps2_texture_name(r"Data\Psys\WO_SHIP_ENGINEFLARE.POB"), None);
        assert_eq!(ps2_texture_name(r"Data\Defaults\Skycube.vex"), None);
        assert_eq!(ps2_texture_name("Data/Tex/Missing.pct"), None);
        assert_eq!(ps2_texture_name("nodot"), None);
    }

    /// `Texture_FindOrLoad` strips the build prefix wherever it appears, so a
    /// name with no prefix at all comes back unchanged and one that carries it
    /// under any case comes back with just the archive-relative part.
    #[test]
    fn build_prefix_is_stripped_case_insensitively() {
        assert_eq!(
            ps2_strip_build_prefix(r"Wipeout PSP\PS2\Data\Environments\12_Track\sky12_4.tga"),
            r"Data\Environments\12_Track\sky12_4.tga"
        );
        assert_eq!(
            ps2_strip_build_prefix(r"WIPEOUT PSP\PS2\Data\Ships\Feisar\Textures\hull.tga"),
            r"Data\Ships\Feisar\Textures\hull.tga"
        );
        assert_eq!(
            ps2_strip_build_prefix(r"Data\Environments\01_Track\Textures\sky1_1.tga"),
            r"Data\Environments\01_Track\Textures\sky1_1.tga"
        );
    }

    /// Both consoles' archives are offered, bulk before companion, so a "nothing
    /// here" error names everything that was looked for.
    #[test]
    fn every_archive_name_is_reported_bulk_first() {
        assert_eq!(
            TITLE.archive_names(),
            vec![
                archives::DATA,
                archives::ps2::DATA,
                archives::FE,
                archives::ps2::FE,
            ]
        );
    }
}
