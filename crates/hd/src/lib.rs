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
//!   boot chain its own XML declares. See [`frontend`] for what that does and
//!   does not license.
//!
//! # Deliberately absent
//!
//! Nothing here describes how an HD file decodes. Byte order is read from each
//! file's own magic inside `oag-formats` (`VEXX` against `XXEV`, `dtOW` against
//! `WOtd`), never from the console it came off, which is why this crate needs no
//! `ByteOrder` constant and why `oag_formats::track::parse` takes no argument.
//!
//! Nothing here names a `.rcsmodel`, a `.gtf` or a `.bik` beyond the two logo
//! reels the front-end XML names itself. All three formats are undecoded, and a
//! table of paths into formats nothing reads would be a list of names rather
//! than a measurement.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`hd-status.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/hd-status.md
//! [`hd-frontend.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/hd-frontend.md
//! [psarc]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/psarc.md

use oag_assets::{Archives, Result};
use oag_title::{ArchiveCandidates, ForeignSerial, Platform, Title};

pub mod frontend;
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
    front_end: None,
    race: race::DEFAULTS,
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
/// `oag_formats::handling::entry_name`'s `Data\Ships\<team>\handlingstats.xml`
/// resolves here unchanged. That is why the race path needed no HD-specific
/// spelling of a handling file, and it is worth knowing before adding one.
pub mod names {
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
