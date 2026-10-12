//! Which campaign a title's front end draws, and what of the campaign it
//! authors: per-title behaviour as data, [ADR-0058]'s second axis after
//! [`crate::effects`] and [`crate::endrace`].
//!
//! `oag-game` answered these by comparing a title's name against Wipeout HD's,
//! Omega's or Pulse's, in six places that each wanted a *different* subset of
//! the five titles. One flag per question is what keeps them apart:
//!
//! | question | Pulse | Pure | HD | 2048 | Omega |
//! | --- | --- | --- | --- | --- | --- |
//! | reader/screen set ([`Campaign::dialect`]) | Pulse | Pure | HD | Pulse | Omega |
//! | grids file `--campaign-cell` reads ([`Campaign::definition_entry`]) | yes | none | yes | none | none |
//! | list campaign read from ([`Campaign::list_entry`]) | none | `Definition.xml` | none | none | none |
//! | circuits gated by `<Unlock Grid>` ([`Campaign::circuit_unlocks`]) | yes | no | no | no | no |
//! | variants gated by loyalty rows ([`Campaign::loyalty_unlocks`]) | yes | no | no | no | no |
//! | `Campaign Selection` ids overlaid ([`Campaign::selection_strings`]) | no | no | yes | no | no |
//! | archive the screen file is read from ([`Campaign::screen_archive`]) | precedence | precedence | `DATA06` | precedence | precedence |
//! | archive the grid files are read from ([`Campaign::grid_archive`]) | precedence | precedence | `DATA06` | precedence | precedence |
//!
//! 2048 reads through Pulse's reader today because nothing else exists to read
//! it with; its [`Campaign::dialect`] is [`Origin::InheritedFrom`] Pulse, which
//! says so instead of leaving the fall-through of an `else` to imply it. It draws
//! no campaign screen (its front end carries no campaign layout), so the dialect
//! is read only by a path that then refuses. Pure's is its own: a list, not a
//! grid (`oag_tables::pure_campaign`).
//!
//! [ADR-0058]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md

use crate::effects::Origin;

/// Which reader and draw list a title's campaign screens use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignDialect {
    /// Pulse's `CellMode_Definition.xml` grid and cell screens.
    Pulse,
    /// Wipeout HD/Fury's own copies of the same roles, with their own widget
    /// names (`Event`, `RC Laps`, `NextPoints`, `EPoints Title`).
    Hd,
    /// Wipeout Pure's: no grid at all, a list of events per speed class read off
    /// the plugin `Definition.xml` ([`Campaign::list_entry`]) and each circuit's
    /// `stats.xml`.
    Pure,
    /// Omega's: HD's `PI001` front-end plugin carried forward, read off its own
    /// nineteen grids.
    Omega,
}

impl CampaignDialect {
    /// Whether the screens are drawn with HD's draw list rather than Pulse's.
    ///
    /// Omega is HD's: it used to fall through to Pulse's, which has no arm for
    /// HD's widget names, so their raw ids and the `%d` template drew on
    /// screen.
    #[must_use]
    pub const fn draws_hd_screens(self) -> bool {
        matches!(self, Self::Hd | Self::Omega)
    }
}

/// The two front-end files a title's SINGLE PLAYER chain is read from
/// (`oag_tables::pure_flow`): the skin holding `Main Menu` and `Single Player`,
/// and the definition holding the selection screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowFiles {
    /// The skin, by entry name.
    pub skin: &'static str,
    /// The selection screens' definition, by entry name.
    pub selection: &'static str,
}

/// A string this project owns: its id in `assets/ui/strings/english.toml` and
/// the English text it falls back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wording {
    /// The id.
    pub id: &'static str,
    /// The English text.
    pub text: &'static str,
}

/// How the main menu's campaign row reads on a title whose original names it
/// differently from this project's `RACE CAMPAIGN`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainRow {
    /// The row's label.
    pub label: Wording,
    /// The line under it while selected.
    pub subtitle: Wording,
}

/// What a title's campaign is, with where each answer came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Campaign {
    /// The reader and draw list.
    pub dialect: CampaignDialect,
    /// The plugin definition naming the campaign grids, by entry name, for the
    /// one reader that takes a grids file on its own (`--campaign-cell`).
    /// `None` on a title whose grids this build reads another way or not at
    /// all.
    pub definition_entry: Option<&'static str>,
    /// The plugin definition a **list-shaped** campaign is read from, by entry
    /// name: Wipeout Pure's `Definition.xml`, whose tracks, leagues and
    /// tournaments with their `<Unlock>` rows are the whole of its campaign.
    /// `None` on a title whose campaign is a grid.
    pub list_entry: Option<&'static str>,
    /// Where a **chain-shaped** campaign's screens are read from: Wipeout Pure's
    /// `Single Player` -> `Class Selection` -> ... walk. `None` on a title whose
    /// campaign is a grid.
    pub flow: Option<FlowFiles>,
    /// The speed classes a fresh profile offers, by the name the class menu
    /// authors; `None` offers every class from the start. Pure's
    /// `Class Selection` lists Vector and Venom on a fresh profile (PPSSPP
    /// capture, 2026-09-10). Which later class opens when is not read.
    pub classes_at_start: Option<&'static [&'static str]>,
    /// The main menu's campaign row where the title's original names it
    /// differently (`SINGLE PLAYER` on Pure); `None` keeps `RACE CAMPAIGN`.
    pub main_row: Option<MainRow>,
    /// Whether circuits carry an `<Unlock Grid="...">` this build reads, and
    /// the Track Select list is cut to the ones the profile has opened.
    pub circuit_unlocks: bool,
    /// Whether craft variants carry `loyalty` unlock rows this build reads.
    pub loyalty_unlocks: bool,
    /// Where [`Self::circuit_unlocks`] and [`Self::loyalty_unlocks`] came from,
    /// apart from [`Self::origin`]: a `false` there is "measured absent" on one
    /// title (Pure authors none) and merely "not wired" on the others, which is
    /// [`Origin::Chosen`] and says so.
    pub unlocks_origin: Origin,
    /// Whether `Campaign Selection`'s own ids are missing from the language
    /// table the front end carries and have to be overlaid from the archive
    /// that has them (HD/Fury only).
    pub selection_strings: bool,
    /// The archive label whose copy of the campaign screen file
    /// (`CellMode_Definition.xml`) is the one to read, where a title ships
    /// several that disagree and the front end's own precedence lands on the
    /// wrong one (Wipeout HD/Fury: `DATA06`'s later build, not `DATA02`'s).
    /// `None` reads the copy the source's ordinary precedence serves.
    pub screen_archive: Option<&'static str>,
    /// The archive label whose copy of each `grid_NN.xml` wins over the
    /// source's ordinary precedence, for a grid that archive carries; a grid it
    /// does not carry is read by precedence. Wipeout HD/Fury: `DATA06`, whose
    /// `grid_00`..`grid_07` carry the three medal rungs per cell where
    /// `DATA02`'s older copy carries one flat set, equal to the hard rung.
    /// `None` reads every grid by precedence.
    pub grid_archive: Option<&'static str>,
    /// Where the dialect, grids file, selection flag, screen archive and grid
    /// archive came from.
    pub origin: Origin,
}
