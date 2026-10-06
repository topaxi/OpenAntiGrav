//! What Wipeout HD / Fury ships.
//!
//! A title package in the sense of [ADR-0022], and the third one. Pure proved
//! the title axis was real on a disc shaped exactly like Pulse's; **HD is the
//! one that is shaped differently** - a PS3 BD-ROM, seven [PSARC](psarc)
//! archives instead of two WADs, big-endian payloads, and geometry that has
//! left the `.vex` entirely. What survives that is the interesting part, and it
//! is most of the table: HD's circuits are `.vex` trees carrying `WO Track`
//! splines, its collision soup and pads read with the same parsers, and its
//! `handlingstats.xml` parses with Pulse's schema unchanged.
//!
//! # What is measured, and where
//!
//! - [`hd-status.md`] - the format layer: archives, `.vex`, spline, collision,
//!   pads, visibility, handling. This is the page every path constant below
//!   comes off.
//! - [`hd-frontend.md`] - the front end: `skin.xml`'s layout globals and the
//!   boot chain it was watched taking. See [`frontend`] for what that does and
//!   does not license.
//!
//! # Deliberately absent
//!
//! Nothing here describes how an HD file decodes. Byte order is read from each
//! file's own magic inside `oag-formats` (`VEXX` against `XXEV`, `dtOW` against
//! `WOtd`), never from the console it came off, which is why this crate needs no
//! `ByteOrder` constant and why `oag_vex::track::parse` takes no argument.
//!
//! Nothing here names a `.rcsmodel` beyond what the disc's own files name, and
//! a table of paths into a format nothing reads would be a list of names rather
//! than a measurement. **Two of the three formats this paragraph used to cover
//! have since earned their exceptions, and each earned it the same way** - by
//! being named in the disc's own data rather than composed here:
//!
//! - **`.bik`** is read ([`bik.md`]) and played.
//!   `frontend::names::STUDIO_LOGO_MOVIE` is here because HD's own `Studio Logo`
//!   screen names it and the boot draws it. The other 35 stay out: the front-end
//!   XML does not name them, so listing them would be this crate inventing an
//!   index the disc does not author.
//! - **`.gtf`** is read ([`gtf.md`]), and [`hud::TEXTURES`] names the twelve
//!   textures the HUD layouts sample - ten of them `.gtf`. Those are not a list
//!   this crate composed either: they are read off the eighteen layouts, and all
//!   twelve resolve to a shipped entry under the extension rule
//!   [`hud::texture_entry`] states. No texture path outside the HUD's own set is
//!   named.
//!
//! [`bik.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/bik.md
//! [`gtf.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/gtf.md
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`hd-status.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/hd-status.md
//! [`hd-frontend.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/hd-frontend.md
//! [psarc]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/psarc.md

use oag_assets::{Archives, Result};
use oag_title::{ArchiveCandidates, ForeignSerial, Platform, Title};

pub mod campaign;
pub mod effects;
pub mod endrace;
pub mod frontend;
pub mod hud;
pub mod loading;
pub mod loyalty;
pub mod race;

/// Wipeout HD / Fury, as the asset layer needs to know it.
pub const TITLE: &Title = &Title {
    name: "Wipeout HD",
    archives: ArchiveCandidates {
        data: DATA_CANDIDATES,
        fe: FE_CANDIDATES,
        extra: EXTRA_CANDIDATES,
    },
    foreign_serials: FOREIGN_SERIALS,
    // **A watched order since 2026-09-05.** See `frontend::BOOT`'s provenance
    // and ADR-0025. This field held `None` until the type could carry the
    // difference between a declared order and a measured one, then held a
    // declared one; three cold boots on RPCS3 closed that out.
    front_end: Some(frontend::FRONT_END),
    hud: hud::LAYOUTS,
    hud_art: hud::ART,
    race: race::DEFAULTS,
    // **HD authors its ribbon rather than naming a texture.** The template is
    // one triangle; its material names the colour texture, the noise texture
    // and the blend factors, so this one path supplies all three. See
    // `docs/rendering/trail-ribbon.md`.
    // **The `bluered` one, because that is the one the shipped executable
    // names.** `Data/RibbonEffects/enginetrail_bluered_triangle.vex` is a
    // literal at `0x007a2d70` in `EBOOT.elf`, in `TrailEffectManager`'s own
    // TOC block; the plain `enginetrail_triangle` beside it in `DATA02` is
    // named nowhere in the executable and is the pre-Fury build's. See
    // `docs/rendering/trail-ribbon.md`.
    //
    // **This path is on the Fury disc's `DATA06` only.** `Exhaust::Authored`
    // carries one path, so a base-HD pressing without that archive would report
    // the entry as missing and fall back to a procedural glow rather than
    // silently drawing the dead `enginetrail_triangle` beside it - which is the
    // right way round, and is a loud failure this project has never seen
    // because Fury is the only HD image it has.
    exhaust: &oag_title::exhaust::Exhaust::Authored(
        "/data/ribboneffects/enginetrail_bluered_triangle.rcsmodel",
    ),
    // **HD authors its flare too, and per team.** It carries none of Pulse's
    // `grabbedEngineFlare` sprite; every one of its fourteen craft ships an
    // `engineflare.vex`/`.rcsmodel` pair of 961-2141 triangles instead, split
    // into an always-on `EF_Main` group and an `EF_Boost` one. See
    // `docs/rendering/trail-ribbon.md`, "HD's flare is a model, not a sprite".
    flare: &oag_title::flare::Flare::PerTeam(oag_title::flare::Authored {
        stem: "engineflare",
        always: "EF_Main",
        boost: "EF_Boost",
        // The literal at `0x0079bdd8` in `EBOOT.elf`.
        sprite: "/data/tex/engineflare/engine_flare_rich.gtf",
        engine_light: true,
    }),
    // **The same file the soundtrack is declared in**, because on this title the
    // front-end plugin *is* the game plugin - it carries the `PI_Team` and
    // `PI_Track` nodes beside the `PI_Music` ones. See
    // [`names::FRONT_END_PLUGIN_DEFINITION`].
    plugin_definition: names::FRONT_END_PLUGIN_DEFINITION,
    track_plugin_definition: None,
    loading: Some(&loading::LOADING),
    music: Some(MUSIC),
    cursor: include_str!("../../../assets/cursors/hd.svg"),
    // **Pulse's two names, and HD answers them.** `Data\XML\WeaponStats_Race.xml`
    // resolves in `DATA00.PSARC` through the same path normalisation every other
    // HD entry goes through, and parses with thirteen weapons and four pickup
    // tables - so this is measured rather than inherited. The Eliminator variant
    // is declared on the same reading and nothing has opened it.
    weapons: &oag_title::weapons::Weapons {
        race: r"Data\XML\WeaponStats_Race.xml",
        elimination: Some(r"Data\XML\WeaponStats_Elimination.xml"),
        // `Data\XML\WeaponAIstats.xml` is in `DATA00.PSARC` (searched
        // 2026-10-03) and carries Pulse's thirteen element names (with its
        // own values) plus an `AllWeapons` row and an `EliminatorAIStats` row,
        // both parsed and neither consumed; `DATA02`'s base-game copy has
        // neither. The
        // opponents run Pulse's fire law on these odds: the law is **inherited
        // from Pulse, unmeasured on HD** - HD's own decision code is unread.
        ai: Some(r"Data\XML\WeaponAIstats.xml"),
    },
    weapon_models: race::WEAPON_MODELS,
    effects: effects::EFFECTS,
    looks: effects::LOOKS,
};

/// Where Wipeout HD keeps its music.
///
/// **Both halves are read off the disc**, and neither is a PSP title's shape.
/// The paths are literals and templates in the decrypted `EBOOT.elf`, sitting
/// together a few bytes before `MusicManager.cpp`:
///
/// ```text
/// 793858  %s\%s_stereo%s
/// 793868  music
/// 793870  .mp3
/// 793878  %s\%s_surround%s
/// 793890  Data\Music\FEMusic\frontend%d_stereo_fury.mp3
/// 7938c0  Data\Music\FEMusic\frontend%d_surr_fury.mp3
/// 7938f0  Data\Music\FEMusic\frontend%d_stereo.mp3
/// 793920  Data\Music\FEMusic\frontend%d_surround.mp3
/// ```
///
/// A soundtrack track is therefore the `PI_Music` location joined with `music`,
/// `_stereo` and `.mp3` - and all **fifteen** the front-end plugin declares
/// resolve to real entries. The front end has four candidates rather than one;
/// see [`names::FRONT_END_MUSIC_VARIANTS`] for which axis is unread.
///
/// Confidence **88**: the templates and the declarations are each read off the
/// disc and every expansion hits, but nothing has been watched running under an
/// emulator - so which of the four the original picks has not been observed.
/// The front end is wired now (see [`frontend::BOOT`] and ADR-0025), which
/// changes what is *reachable* and not what has been *watched*: the boot walks
/// an order read out of the disc's XML, so a run of this build picking a cut is
/// this build's behaviour rather than evidence about a PS3's.
pub const MUSIC: &oag_title::Music = &oag_title::Music {
    front_end: Some(names::FRONT_END_MUSIC),
    tracks: Some(oag_title::DeclaredTracks {
        declared_in: names::FRONT_END_PLUGIN_DEFINITION,
        file: names::MUSIC_TRACK_FILE,
    }),
    state_tracks: None,
};

/// The seven archives a Wipeout HD / Fury disc ships, under `PS3_GAME/USRDIR/`.
///
/// **They do not split by kind, and that is the fact the [`extra`
/// role](oag_title::ArchiveCandidates::extra) exists for.** `data/environments`
/// and `data/ships` each appear in four of the seven; measured on
/// `hdfury-ps3-eu-dec.iso`:
///
/// | Archive | Entries | What is in it |
/// | --- | ---: | --- |
/// | `DATA00.PSARC` | 3,157 | 8 circuits including Talon's Junction, the Zone tracks, Detonator |
/// | `DATA01.PSARC` | 53 | sound and music, and nothing else |
/// | `DATA02.PSARC` | 5,486 | the 8 numbered circuits, `data/xml/handlingstats.xml`, 10 teams |
/// | `DATA03.PSARC` | 1,163 | Auricom, Harimau, Icaras and Mirage - and no circuit at all |
/// | `DATA04.PSARC` | 54 | the Asian fonts and the trophy models |
/// | `DATA05.PSARC` | 151 | front end and plugins |
/// | `DATA06.PSARC` | 1,600 | 24 `_c1`/`_n1` team variants |
///
/// The row that settles the design is `DATA03`: four of the twelve base teams
/// are in an archive that carries neither the circuits nor the other eight, so
/// mounting a bulk archive and a companion - which is all Pulse and Pure ever
/// needed - would silently lose a third of the roster.
pub mod archives {
    /// Talon's Junction's archive, and the one this project has read end to end.
    pub const DATA00: &str = "PS3_GAME/USRDIR/DATA00.PSARC";
    /// Sound and music only.
    pub const DATA01: &str = "PS3_GAME/USRDIR/DATA01.PSARC";
    /// The largest, and where the global handling file lives.
    pub const DATA02: &str = "PS3_GAME/USRDIR/DATA02.PSARC";
    /// Four teams, no circuits.
    pub const DATA03: &str = "PS3_GAME/USRDIR/DATA03.PSARC";
    /// Asian fonts and trophy models.
    pub const DATA04: &str = "PS3_GAME/USRDIR/DATA04.PSARC";
    /// Front end and plugins.
    pub const DATA05: &str = "PS3_GAME/USRDIR/DATA05.PSARC";
    /// The Fury and campaign team variants.
    pub const DATA06: &str = "PS3_GAME/USRDIR/DATA06.PSARC";

    /// All seven, in disc order.
    ///
    /// For a survey that has to look everywhere rather than through
    /// `oag_assets::Archives`' precedence - which archive an entry comes out of
    /// is a question of its own here, since the same path ships in more than one
    /// of these. See `docs/formats/hd-hud.md`.
    pub const ALL: &[&str] = &[DATA00, DATA01, DATA02, DATA03, DATA04, DATA05, DATA06];
}

/// Paths inside the archives.
///
/// **A PSARC stores its paths in full and in the clear**, which is the one
/// structural way this container is easier than the [WAD](oag_formats::wad) it
/// replaces: nothing here was recovered by hashing a candidate and hoping, the
/// way every name in `oag_pulse::names` had to be. They were read off the
/// manifest.
///
/// The paths are stored lowercase with a leading `/`, and
/// `oag_assets::psarc::Archive` folds separators and case - so
/// `oag_tables::handling::entry_name`'s `Data\Ships\<team>\handlingstats.xml`
/// resolves here unchanged. That is why the race path needed no HD-specific
/// spelling of a handling file, and it is worth knowing before adding one.
pub mod names {
    /// The front-end plugin's own definition, which declares the roster, the
    /// circuits and the soundtrack.
    ///
    /// HD's counterpart of the PSP titles' `Data\Plugins\PI001\Definition.xml`,
    /// under a name rather than a number, and carrying the same schema: one
    /// `PI_Team` node per team, one `PI_Track` per circuit and one `PI_Music`
    /// per soundtrack track, under the same attribute names. See
    /// [`crate::MUSIC`] and [`oag_title::Title::plugin_definition`].
    ///
    /// **This file ships five times and the copies disagree**, which is the
    /// [`oag_title::ArchiveCandidates::extra`] overlap biting something that is
    /// actually read. Measured on `hdfury-ps3-eu-dec.iso`:
    ///
    /// | Archive | Bytes | `PI_Team` | `PI_Track` | `PI_Music` |
    /// | --- | ---: | ---: | ---: | ---: |
    /// | `DATA00` | 36,996 | 12 | 28 | 15 |
    /// | `DATA02` | 11,133 | 8 | 8 | 9 |
    /// | `DATA03` | 17,667 | 12 | 16 | 9 |
    /// | `DATA05` | 19,355 | 12 | 16 | 9 |
    /// | `DATA06` | 32,764 | 12 | 16 | 9 |
    ///
    /// `oag_assets::Archives` serves `DATA00`'s, which is the fullest of the
    /// five - twelve teams, and the fifteen `PI_Music` nodes [`crate::MUSIC`]
    /// already reports every expansion of as resolving. That the precedence
    /// lands on the fullest copy is a **fact about this ordering**, not a
    /// measurement of which one a PS3 loads; `DATA02`'s eight-team copy is
    /// presumably the base game's, from before Fury added four. Which one the
    /// original reads is unresolved, on the same terms as `skin.xml`'s six.
    ///
    /// Confidence **85**: every copy is read off the disc and every team it
    /// declares resolves to a `ship.vex`, a `ship.rcsmodel`, a `locators.vex`
    /// and a `handlingstats.xml`, but nothing has been watched running.
    ///
    /// # The names of what it declares are in a *different* archive
    ///
    /// The asymmetry is the recovered fact and it is easy to walk into: the
    /// circuit **list** comes from `DATA00`'s copy of this file, and the only
    /// string table that names all 28 of them is `DATA06`'s `entries.xml` -
    /// `DATA00` ships none at all, and the other four copies stop at 24 keys,
    /// lacking exactly the four Zone circuits. Taking the list and the names
    /// from whichever archive precedence happens to serve puts a confident
    /// wrong name on eight circuits. See `oag_game::language::CircuitNames` and
    /// `docs/formats/hd-frontend.md`.
    pub const FRONT_END_PLUGIN_DEFINITION: &str = r"Data\Plugins\frontend\definition.xml";

    /// The file a `PI_Music` location holds, joined onto that location to
    /// address one soundtrack track.
    ///
    /// The executable's `%s\%s_stereo%s` with its last two `%s` filled in from
    /// the `music` and `.mp3` strings beside it. **Stereo rather than
    /// surround**: the mixer is stereo, and `_surround` is the same recording
    /// in more channels rather than a different one - unlike the front end's
    /// four, which are not all the same music. See [`crate::MUSIC`].
    pub const MUSIC_TRACK_FILE: &str = "music_stereo.mp3";

    /// The music the front end loops under its menus.
    ///
    /// **One of four, and picking it unconditionally is a known limitation**
    /// rather than a measurement - the same shape as
    /// `oag_pure::names::INTRO_MOVIE_CUTS`. See
    /// [`FRONT_END_MUSIC_VARIANTS`].
    pub const FRONT_END_MUSIC: &str = r"Data\Music\FEMusic\frontend1_stereo.mp3";

    /// Every front-end music the disc carries, by the axis that selects it.
    ///
    /// **Two axes, and only one of them is understood.** Stereo against
    /// surround is a channel count, and the mixer settles it. Base against Fury
    /// is *not* a re-encode: `frontend1_stereo.mp3` is 4,097,664 bytes and
    /// `frontend1_stereo_fury.mp3` is 1,057,536, so they are **different pieces
    /// of music** and choosing between them is choosing what the player hears.
    /// What the original selects on has not been read.
    ///
    /// `%d` is 1 in all four: the executable's template takes a number and only
    /// `frontend1` exists on the disc, so the numbering starts at one and stops
    /// there - unlike Pulse, which ships eight.
    pub const FRONT_END_MUSIC_VARIANTS: &[(&str, &str)] = &[
        ("base stereo", r"Data\Music\FEMusic\frontend1_stereo.mp3"),
        (
            "base surround",
            r"Data\Music\FEMusic\frontend1_surround.mp3",
        ),
        (
            "fury stereo",
            r"Data\Music\FEMusic\frontend1_stereo_fury.mp3",
        ),
        (
            "fury surround",
            r"Data\Music\FEMusic\frontend1_surr_fury.mp3",
        ),
    ];

    /// The second front-end track, which nothing here plays.
    ///
    /// A literal in the executable rather than a template, and named for the
    /// ship-selection screen it presumably belongs to - but
    /// [`oag_title::Music`] carries **one** front-end track, so this is
    /// recorded and left alone. Its trigger is unread, which is the part that
    /// would have to be recovered first and which wiring the front end did not
    /// supply: no screen in any of the six skins names this file.
    pub const SHIP_SELECT_MUSIC: &str = r"Data\Music\FEMusic\FEship.mp3";

    /// One circuit's `.vex`, by its environment directory.
    ///
    /// The directory names are the disc's own and are **not** Pulse's: HD ships
    /// `talons_junction` where Pulse ships `16_Track` for the same circuit in
    /// the same world coordinates, alongside eight numbered ones
    /// (`01_vineta_k`, `02_track`, ...) that keep the older scheme.
    #[must_use]
    pub fn track(environment: &str) -> String {
        format!("/data/environments/{environment}/track.vex")
    }

    /// Every environment directory holding a `track.vex`, in archive order.
    ///
    /// **Sixteen, read off the manifest**, not a menu order and not a claim
    /// about what a player can select: four are Zone circuits and the disc's own
    /// track-selection XML has not been read. See
    /// `crates/hd/tests/hd_title_ground_truth.rs`, which re-derives this
    /// list from the disc rather than trusting it.
    pub const ENVIRONMENTS: &[&str] = &[
        "amphiseum",
        "modesto_heights",
        "talons_junction",
        "tech_de_ra",
        "zone_1",
        "zone_2",
        "zone_3",
        "zone_4",
        "01_vineta_k",
        "02_track",
        "03_track",
        "04_chenghou_project",
        "05_ubermall",
        "10_sebenco_climb",
        "12_sol_2",
        "15_anulpha_pass",
    ];

    /// The twelve base teams, by the directory under `/data/ships/`.
    ///
    /// **Lowercase, where Pulse's are capitalised** - the only thing that
    /// changed about a team id. Twenty-four further directories carry a `_c1` or
    /// `_n1` suffix, two per team, and four more are mode ships rather than
    /// teams; see [`MODE_SHIPS`].
    ///
    /// Read off the manifest. Which of them a player can fly is the disc's
    /// front-end XML's business and is not read here.
    pub const TEAMS: &[&str] = &[
        "ag_systems",
        "assegai",
        "auricom",
        "egx",
        "feisar",
        "goteki",
        "harimau",
        "icaras",
        "mirage",
        "piranha",
        "qirex",
        "triakis",
    ];

    /// The four ship directories that are not teams.
    ///
    /// `detonator` and `zone` are the mode ships, and **they author no
    /// `<Class>` block at all** where every team file authors four - measured in
    /// `crates/assets/tests/hd_psarc_ground_truth.rs`. `test` is a development
    /// leftover that parses like a team.
    ///
    /// **`zone battle` has a space in it**, which is the disc's and not a
    /// transcription slip - it is the only ship directory on any of the three
    /// titles that does, and it is why a roster mined with a `[a-z0-9_]`
    /// pattern comes back one short. Left exactly as stored, because a
    /// normaliser here would stop it matching the manifest.
    pub const MODE_SHIPS: &[&str] = &["detonator", "zone", "zone battle", "test"];
}

/// The bulk archive's candidates.
///
/// One entry: HD is PS3-only, and `DATA00` is the archive carrying the default
/// circuit. Nothing branches on the [`Platform`] - see
/// [`ArchiveCandidates`]'s own docs - but a PS3 disc identifies itself out of
/// `PS3_DISC.SFB`, so this is only consulted for an extracted directory.
const DATA_CANDIDATES: &[(&str, Platform)] = &[(archives::DATA00, Platform::Ps3)];

/// The companion archive's candidates.
///
/// `DATA02`, the largest, chosen for this role because it carries the global
/// `handlingstats.xml` and ten of the twelve teams - so a source that somehow
/// mounted only two archives would still race. The other five are
/// [`EXTRA_CANDIDATES`] and all of them mount.
const FE_CANDIDATES: &[(&str, Platform)] = &[(archives::DATA02, Platform::Ps3)];

/// The remaining five, all mounted.
const EXTRA_CANDIDATES: &[(&str, Platform)] = &[
    (archives::DATA01, Platform::Ps3),
    (archives::DATA03, Platform::Ps3),
    (archives::DATA04, Platform::Ps3),
    (archives::DATA05, Platform::Ps3),
    (archives::DATA06, Platform::Ps3),
];

/// Serials positively identified as a Studio Liverpool title other than HD.
///
/// The same one-directional rule the other two title packages carry: this rules
/// a source *out*, never in, so an uncatalogued HD pressing still finds its
/// archives by name. See [`ForeignSerial`].
///
/// **HD needs this less than the PSP pair do and it is here anyway.** Pure and
/// Pulse ship `Data.wad` and `FE.wad` under identical names, which is what
/// makes name matching alone unsafe between them; no PSP or PS2 disc carries a
/// `DATA00.PSARC`, so nothing could currently be confused for HD. That will stop
/// being true the moment a fourth title package opens a PS3 or PSN release, and
/// a deny-list that is written when it is needed is a deny-list written under
/// pressure.
const FOREIGN_SERIALS: &[ForeignSerial] = &[
    ForeignSerial {
        serial: "UCUS-98712",
        title: "Wipeout Pulse",
    },
    ForeignSerial {
        serial: "UCES-00465",
        title: "Wipeout Pulse",
    },
    ForeignSerial {
        serial: "SCES-54748",
        title: "Wipeout Pulse",
    },
    ForeignSerial {
        serial: "UCUS-98612",
        title: "Wipeout Pure",
    },
    ForeignSerial {
        serial: "UCES-00001",
        title: "Wipeout Pure",
    },
];

/// Opens whichever archives `source` carries, as Wipeout HD.
///
/// # Errors
///
/// Propagates [`Archives::open`].
pub fn open(source: &str) -> Result<Archives> {
    Archives::open(source, TITLE)
}

#[cfg(test)]
mod tests;
