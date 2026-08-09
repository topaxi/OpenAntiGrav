//! Which movie a name resolves to on each of Pulse's two releases.
//!
//! The PSP release keeps its movies inside `Data.wad` as `.PMF`; the PS2 one
//! keeps them **loose on the ISO filesystem** in two different containers, and
//! ships two cuts of each sized for PAL and NTSC. That is a fact about what this
//! title pressed, not about either format, so the mapping is a table here and the
//! loader that consumes it is `oag_game::boot`.
//!
//! Both `.PMF` names are the PSP entry names from [`crate::names`], so a caller
//! asks for one name on both platforms and this table answers it.
//!
//! See `docs/ps2/pulse-disc-layout.md` and `docs/formats/ipf.md`.

/// Names that are answered by a loose file, and the two cuts that answer them.
///
/// 512 is listed first on each row because the traced disc is EU/PAL; the
/// ultimate trigger that decides which value `0x0027a85c` takes is not traced.
///
/// Four rows for two movies: the front-end XML and the ISO 9660 directory
/// disagree on both spelling and extension, so the intro is asked for as both
/// `Intro.PMF` and `Intro.pss` and the backdrop as both `Backdrop.PMF` and
/// `Backdrop.ipf`. Matching is case-insensitive at the use site for the same
/// reason.
pub const LOOSE_MOVIES: [(&str, [&str; 2]); 4] = [
    (
        crate::names::INTRO_MOVIE,
        ["DATA/MOVIES/INTRO512.PSS", "DATA/MOVIES/INTRO640.PSS"],
    ),
    (
        r"Data\Movies\Intro.pss",
        ["DATA/MOVIES/INTRO512.PSS", "DATA/MOVIES/INTRO640.PSS"],
    ),
    (
        r"Data\Movies\Backdrop.ipf",
        ["DATA/MOVIES/BG512.IPF", "DATA/MOVIES/BG640.IPF"],
    ),
    (
        crate::names::BACKDROP_MOVIE,
        ["DATA/MOVIES/BG512.IPF", "DATA/MOVIES/BG640.IPF"],
    ),
];
