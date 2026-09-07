//! A team that carries more than one selectable directory, on either of the
//! two measured shapes - see [`TeamVariants`] and [`GuestRoster`].
//!
//! Split out of [`super`] under this project's 1,000-line ceiling
//! (`scripts/check-file-size.py`) - a move, with no behaviour change.

use super::{RaceDefaults, ShipPaths};

/// A second roster a title carries besides its own - team identities
/// reshipped from another title's package, verbatim, under one directory of
/// this title's own. See [`RaceDefaults::guest_roster`].
///
/// **Answers a different question from [`TeamVariants`].** That type says how
/// a suffix joins a team's own id; this one says *which directory* a team's
/// roster lives under at all - and Wipeout 2048's two rosters disagree on
/// both: its own five keep the ship model and the tuning in two different
/// trees ([`RaceDefaults::ship_dir`]/[`RaceDefaults::handling_dir`]) with a
/// `Subdirectory` join, while its twelve HD-derived teams keep both in one
/// tree with a `Suffix` join - the same scheme
/// [`oag_hd::race::TEAM_VARIANTS`](https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/race.rs)
/// carries, confirmed to resolve identically under 2048's own tree. Folding
/// the two questions into one struct would be the single-example
/// generalisation [ADR-0009] warns against - this crate does not depend on
/// `oag-hd` to reuse its table by reference, so 2048's own copy is a
/// duplicate, measured against 2048's own manifest like every other table in
/// its title package.
///
/// [ADR-0009]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0009-multi-game-fanout.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestRoster {
    /// Where the guest roster's ship model *and* tuning both live - one
    /// directory, unlike [`RaceDefaults::ship_dir`]/[`RaceDefaults::handling_dir`],
    /// which can differ for a title's *own* roster.
    pub dir: &'static str,
    /// The team ids, their variant suffixes and how they join.
    pub variants: &'static TeamVariants,
}

/// A team that carries more than one selectable directory - see
/// [`RaceDefaults::team_variants`].
///
/// **Two measured shapes, not one imagined in advance**: 2048's own roster
/// (a *numbered* level - fighter/agility/speed/prototype under
/// `<team>\<1..4>`) and HD/Fury's twelve (a *suffixed* one - classic HD,
/// Fury's concept reskin and Fury's nitro reskin as `<team>`, `<team>_c1`,
/// `<team>_n1`). [`VariantJoin`] is what keeps the second measurement from
/// forcing the first title's shape onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamVariants {
    /// The base team ids this applies to - every other team on the title
    /// resolves with no second directory at all.
    pub teams: &'static [&'static str],
    /// Every selectable directory, in document order - what to append to a
    /// team's own id, and what to call the result to a player.
    pub variants: &'static [TeamVariant],
    /// How a variant's suffix combines with a team's own id. See
    /// [`VariantJoin`].
    pub join: VariantJoin,
}

impl TeamVariants {
    /// Whether `combined` is one of this table's own combined ids - some team
    /// of [`Self::teams`] joined with one of [`Self::variants`]' own
    /// suffixes, [`Self::join`]'s rule.
    ///
    /// Reuses [`VariantJoin::combine`] rather than parsing a suffix back out
    /// of `combined`, so this can never disagree with what actually built the
    /// id in the first place - by construction, not by keeping two rules in
    /// step by hand.
    #[must_use]
    pub fn recognizes(&self, combined: &str) -> bool {
        self.teams.iter().any(|team| {
            self.variants
                .iter()
                .any(|v| self.join.combine(team, v.suffix) == combined)
        })
    }
}

/// One of a team's selectable directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamVariant {
    /// What [`VariantJoin`] appends to the team's own id. Empty for "the
    /// team's own directory, unsuffixed" - HD/Fury's classic skin is exactly
    /// that, not a fourth thing beside the two reskins.
    pub suffix: &'static str,
    /// What to call it to a player.
    pub label: &'static str,
}

/// How [`TeamVariant::suffix`] combines with a team's own id.
///
/// A join rule rather than a single format string because the two titles
/// measured disagree about more than the labels: 2048's numbered level is a
/// **new path segment** (`feisar2048\3`, the id two levels deep, recovered
/// and named in `oag_2048::race`'s own docs); HD/Fury's reskin is a
/// **suffix on the same segment** (`auricom_c1`, one directory, a longer
/// name). Folding both into one join would make one of the two titles the
/// example the type was designed from - the same failure [ADR-0009] names.
///
/// [ADR-0009]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0009-multi-game-fanout.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariantJoin {
    /// `<team>\<suffix>` - 2048's native roster.
    Subdirectory,
    /// `<team><suffix>` - HD/Fury's roster, `suffix` already carrying
    /// whatever separator it needs (`_c1`, not `c1`).
    Suffix,
}

impl VariantJoin {
    /// Combines a team's own id with one of its variants.
    #[must_use]
    pub fn combine(self, team: &str, suffix: &str) -> String {
        match self {
            Self::Subdirectory if suffix.is_empty() => team.to_string(),
            Self::Subdirectory => format!(r"{team}\{suffix}"),
            Self::Suffix => format!("{team}{suffix}"),
        }
    }
}

impl RaceDefaults {
    /// Which [`TeamVariants`] table a **bare** team id is offered variants
    /// from - this title's own [`Self::team_variants`], or
    /// [`Self::guest_roster`]'s, in that order. `None` when neither
    /// recognises it.
    ///
    /// Bare because this is what the menu layer has *before* combining a
    /// pick with a variant - see `oag_game`'s `session::menus::variant_choices`.
    #[must_use]
    pub fn team_variants_for(&self, team: &str) -> Option<&'static TeamVariants> {
        if self
            .team_variants
            .is_some_and(|variants| variants.teams.contains(&team))
        {
            return self.team_variants;
        }
        self.guest_roster
            .filter(|guest| guest.variants.teams.contains(&team))
            .map(|guest| guest.variants)
    }

    /// Whether this title has a variant axis at all, for *any* team.
    ///
    /// Wipeout Pure and Wipeout Pulse author neither [`Self::team_variants`]
    /// nor [`Self::guest_roster`] - the feature does not exist on either
    /// title, on any team, ever, not "exists but happens to be empty for the
    /// team currently held". That distinction is what lets `oag_game` drop
    /// the RACE page's VARIANT row entirely for those two rather than draw it
    /// permanently unusable: see `oag_game`'s `boot::shell_definition`. The
    /// same disjunction [`Self::team_variants_for`] already encodes ("own
    /// table, then guest roster"), kept in one place so a third source added
    /// later cannot be missed at one call site and not the other.
    #[must_use]
    pub fn has_team_variants(&self) -> bool {
        self.team_variants.is_some() || self.guest_roster.is_some()
    }

    /// Which directory an **already-combined** team id's guest roster lives
    /// under, if it is one - the fact [`Self::ships_for`] and
    /// [`Self::handling_dir_for`] both key off.
    fn guest_dir_for(&self, team: &str) -> Option<&'static str> {
        let guest = self.guest_roster?;
        guest.variants.recognizes(team).then_some(guest.dir)
    }

    /// Where an **already-combined** team id's ship model resolves - this
    /// title's own [`Self::ship_dir`], or [`Self::guest_roster`]'s single
    /// directory when `team` is one of its own combined ids.
    ///
    /// Combined because this is all `oag_game::race::load` has by the time it
    /// asks - the bare/combined split only exists transiently in the menu
    /// layer, see [`Self::team_variants_for`].
    #[must_use]
    pub fn ships_for(&self, team: &str) -> ShipPaths {
        ShipPaths {
            dir: self.guest_dir_for(team).unwrap_or(self.ship_dir),
            zone: self.zone_craft,
        }
    }

    /// [`Self::ships_for`]'s sibling for [`Self::handling_dir`].
    #[must_use]
    pub fn handling_dir_for(&self, team: &str) -> &'static str {
        self.guest_dir_for(team).unwrap_or(self.handling_dir)
    }
}
