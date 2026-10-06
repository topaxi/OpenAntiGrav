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
    /// Where the guest roster's **tuning** lives when it is not beside the
    /// model: `None` for 2048's HD-derived twelve (one tree, [`Self::dir`]),
    /// `Some` for the 2048-era craft Omega carries, whose
    /// `handlingstats.xml` sit one tree away from their models, as 2048's own
    /// five do ([`RaceDefaults::handling_dir`]).
    pub handling_dir: Option<&'static str>,
    /// The team ids, their variant suffixes and how they join.
    pub variants: &'static TeamVariants,
    /// The title whose roster this reships, by [`crate::Title::name`]. A race
    /// remix that wants that title's craft and has no source for it falls back
    /// to a title carrying this roster.
    pub reships: &'static str,
    /// The CRAFT TITLE entry's own name when [`Self::reships`] is *also*
    /// mounted as a title of its own: `Some` for a roster that is a distinct
    /// copy (Omega's re-textured 2048 craft, with their own models and
    /// tuning) so both entries are offered and each loads its own source;
    /// `None` for a roster that is the very same data (2048's HD twelve),
    /// where a real source of the reshipped title simply wins.
    pub alongside_label: Option<&'static str>,
    /// Where the claim came from.
    pub origin: crate::effects::Origin,
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

/// A team's own alternate hull **file**, inside its own directory - see
/// [`RaceDefaults::hull_variants`].
///
/// **Not a [`TeamVariant`].** That type's `suffix` combines into a *new team
/// identity* - a second directory carrying its own tuning, its own string-table
/// entry, everything [`TeamVariants::recognizes`] and [`VariantJoin::combine`]
/// exist for. A [`HullVariant`]'s `stem` does none of that: the team stays
/// exactly who it was, at exactly the directory it always resolved to, and
/// only the hull *file* inside it changes - `Ship.vex` for `extra.vex`, both
/// siblings of the same `handlingstats.xml`. Reusing [`TeamVariant`] for this
/// would make `combine`'s return value a lie for every other lookup a race
/// makes off a team id (handling stats, the string-table label, the roster
/// membership check `teams_for_slots` runs).
///
/// **Measured against Pulse's own `Data\Plugins\PI001\Definition.xml`,
/// disc-shipped, not a DLC pack's**: every one of its eight base teams
/// declares `PI_TeamModel name="Normal"` (`Ship.vex`, plus two nested
/// `PI_ModelSkin`s that are a texture swap on this same hull - see
/// `oag_texture::ship_skin` - not a second one) and `PI_TeamModel
/// name="Concept"` (`Values location="extra"`), each `Unlock`-gated. Confirmed
/// against the archive: `extra.vex` resolves for all eight.
/// `crates/game/examples/pulse_variant_probe.rs` reproduces both counts and
/// the file list without printing the `Unlock` loyalty numbers themselves,
/// which are shipped tuning data.
///
/// **A third declared model, `PI_TeamModel name="Zone"` (`Values
/// location="zone01"`), is deliberately not one of [`RaceDefaults::hull_variants`]'s
/// entries.** `Data\Ships\<Team>\zone01.vex` really does exist for all eight
/// teams, but it is not what a Zone race loads: that is `Zone.vex`, reached
/// through a recovered selector
/// (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md`), and the two files
/// are not identical. What `zone01.vex` is for is not established, so
/// offering it here would be authoring a stand-in for something the disc
/// already specifies differently - the rule `CLAUDE.md`'s "never invent what
/// the assets already author" section states outright. Left as an open
/// question on the handover thread instead.
///
/// **There is no save or progression system in this project**, so `Unlock`'s
/// `loyalty` thresholds cannot be modelled - the same gap
/// [`TeamVariant`]/[`GuestRoster`] already leave for HD's and 2048's own
/// reskins. Every variant is offered unconditionally, chosen rather than
/// measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HullVariant {
    /// The file stem inside the team's own directory, always spelled out -
    /// `"Ship"` for the title's own baseline hull, or the `PI_TeamModel`'s
    /// own `location` otherwise (`"extra"` for Pulse's Concept). Unlike
    /// [`TeamVariant::suffix`], never empty: there is no identity to combine
    /// into, so there is nothing for an empty string to mean here.
    pub stem: &'static str,
    /// What to call it to a player. Not shipped text - `PI_TeamModel`'s own
    /// `name` attribute, the same convention [`TeamVariant::label`] follows
    /// for HD's "Fury Concept"/"Fury Nitro".
    pub label: &'static str,
}

impl RaceDefaults {
    /// The directory `circuit`'s `.pob` effects are read from: the first
    /// [`Self::effect_dir_by_circuit`] prefix `circuit` starts with (either
    /// slash, any case), else [`Self::effect_dir`].
    #[must_use]
    pub fn effect_dir_for(&self, circuit: &str) -> &'static str {
        let norm = |s: &str| s.replace('/', "\\").to_ascii_lowercase();
        let circuit = norm(circuit);
        self.effect_dir_by_circuit
            .iter()
            .find(|(prefix, _)| circuit.starts_with(&norm(prefix)))
            .map_or(self.effect_dir, |&(_, dir)| dir)
    }

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
    /// **Wipeout Pure authors none of the three sources below, on any team,
    /// ever - measured against its own `Definition.xml`, zero `PI_TeamModel`
    /// occurrences.** Wipeout Pulse was believed to be the same until
    /// [`Self::hull_variants`] was measured: it authors no
    /// [`Self::team_variants`] and no [`Self::guest_roster`], but does author
    /// a real, disc-shipped `PI_TeamModel`/`PI_ModelSkin` axis, on every base
    /// team - see [`HullVariant`]'s own doc comment for the evidence. So this
    /// is a three-way disjunction rather than the two-way one it was: not
    /// "exists but happens to be empty for the team currently held", true
    /// three sources, checked in one place so a caller adding a fourth cannot
    /// miss one of the earlier three the way this one was missed at first.
    /// This is what lets `oag_game` drop the RACE page's VARIANT row entirely
    /// on Pure - and only Pure - rather than draw it permanently unusable:
    /// see `oag_game`'s `boot::shell_definition`.
    #[must_use]
    pub fn has_team_variants(&self) -> bool {
        self.team_variants.is_some() || self.guest_roster.is_some() || self.hull_variants.is_some()
    }

    /// Which directory an **already-combined** team id's guest roster lives
    /// under, if it is one - the fact [`Self::ships_for`] and
    /// [`Self::handling_dir_for`] both key off.
    fn guest_for(&self, team: &str) -> Option<&'static GuestRoster> {
        self.guest_roster
            .filter(|guest| guest.variants.recognizes(team))
    }

    /// Where an **already-combined** team id's ship model resolves - this
    /// title's own [`Self::ship_dir`], or [`Self::guest_roster`]'s single
    /// directory when `team` is one of its own combined ids.
    ///
    /// Combined because this is all `oag_raceplay::load` has by the time it
    /// asks - the bare/combined split only exists transiently in the menu
    /// layer, see [`Self::team_variants_for`].
    #[must_use]
    pub fn ships_for(&self, team: &str) -> ShipPaths {
        ShipPaths {
            dir: self
                .guest_for(team)
                .map_or(self.ship_dir, |guest| guest.dir),
            zone: self.zone_craft,
            boost: self.boost,
        }
    }

    /// [`Self::ships_for`]'s sibling for [`Self::handling_dir`].
    #[must_use]
    pub fn handling_dir_for(&self, team: &str) -> &'static str {
        self.guest_for(team).map_or(self.handling_dir, |guest| {
            guest.handling_dir.unwrap_or(guest.dir)
        })
    }
}
