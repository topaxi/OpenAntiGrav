//! Which dialect of screen a title's race-ending definition file is written in.
//!
//! [`crate::FrontEnd::endrace_entry`] says *where* the file is; this says *what
//! reader it needs*, which is the question `oag_game::endrace::load` used to
//! answer by comparing a title's name against Wipeout HD's. The three dialects
//! share no widget vocabulary, so a reader for one cannot be pointed at another
//! file by changing the path alone:
//!
//! - [`EndRaceDialect::Pulse`]: the PSP's 480x272 `EndRace Results`/`EndRace
//!   Rewards`/`EndRace Menu`, a per-lap table and a populated option list.
//! - [`EndRaceDialect::Field`]: Wipeout HD/Fury's screens of the same names at
//!   1920x1080, a whole-field standings grid and one `<Block>` per option.
//! - [`EndRaceDialect::Touch`]: Wipeout 2048's `EndRace` screen tree
//!   (`RaceSummary`, `ObjectiveSummary`, `Results`, `Podium`, `Badges`) on a 960x544
//!   touch grid, with an icon-button strip along the bottom.
//!
//! `None` on [`crate::FrontEnd::endrace_style`] means the title's file has not
//! been read, not that it ships none.

/// How a title's end-of-race definition file is written. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndRaceDialect {
    /// The PSP titles' own `EndRace Results`/`Rewards`/`Menu`.
    Pulse,
    /// Wipeout HD/Fury's whole-field `EndRace Results`/`Menu`.
    Field,
    /// Wipeout 2048's touch-grid `EndRace` tree.
    Touch,
}

/// Where a title's [`EndRaceDialect`] came from, per ADR-0058.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleProvenance {
    /// Read off this title's own definition file and executable.
    Measured,
    /// The file is the same dialect as this title's and was checked to be.
    InheritedFrom(&'static str),
}

/// A title's end-of-race dialect with its provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndRaceStyle {
    /// The reader this title's file needs.
    pub dialect: EndRaceDialect,
    /// Where that came from.
    pub provenance: StyleProvenance,
}

impl EndRaceStyle {
    /// A dialect read off the title's own file.
    #[must_use]
    pub const fn measured(dialect: EndRaceDialect) -> Self {
        Self {
            dialect,
            provenance: StyleProvenance::Measured,
        }
    }
}
