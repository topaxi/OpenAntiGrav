//! Which of a Wipeout HD/Fury-era team's `PI_TeamModel`s Ship Select offers: the model's own
//! `<Unlock>` rows ([`oag_raceplay::catalogue::ModelUnlock`]) against the profile's records and
//! the title's campaign grids.
//!
//! What each condition counts (`docs/formats/race-setup.md`, "HD-era model unlocks"):
//!
//! - `loyalty="N"`: the named team's own running total ([`Store::loyalty_total`], shared across
//!   titles by team identity) reaches `N`; `team="any"` is the best single team, Pulse's rule.
//! - `MedalCountHD="N"` / `MedalCountFURY="N"`: gold medals over the grids whose `Campaign` is
//!   `HD` / `Fury` reach `N`. The two thresholds the disc authors, 87 and 80, are exactly those
//!   campaigns' cell counts (every gold), the `GOLD MEDALS` denominators `Campaign Selection`
//!   prints; a plain `MedalCount` counts every grid. Gold rather than any medal is **chosen**:
//!   the evaluator is not traced, and the denominator the screen prints counts golds.
//! - `grid="gridN"`: that grid's own medal points reach its `RequiredPoints`, Pulse's
//!   `Unlock_GridPointsMet`, **inherited from Pulse, not traced on HD**.
//!
//! A model passes when every row passes; a row naming nothing this build reads never passes.
//! `--unlock-all` ([`super::unlock_all`]) opens every model.

use oag_raceplay::catalogue::{ModelUnlock, TeamModel};
use oag_tables::race_campaign::{self, Grid, MedalTiers};

use crate::records::Store;

/// The campaign data a title's model rows are evaluated against. [`ModelGate::none`] gates
/// nothing: a title whose models carry no rows this build reads.
#[derive(Debug, Clone, Default)]
pub struct ModelGate {
    /// `Some` on a title whose models gate ([`oag_title::Campaign::model_unlocks`]), holding its
    /// campaign grids (empty when they would not read: every medal and grid row then fails).
    grids: Option<Vec<Grid>>,
    /// The records' key for this title's campaign medals ([`crate::records::campaign_title`]).
    campaign_title: String,
}

impl ModelGate {
    /// Gates nothing: every model is offered.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Gates on `grids`, whose cell medals `records` keeps under `campaign_title`.
    #[must_use]
    pub fn on_grids(grids: Vec<Grid>, campaign_title: impl Into<String>) -> Self {
        Self {
            grids: Some(grids),
            campaign_title: campaign_title.into(),
        }
    }

    /// The gate `title`'s models answer to, its grids read off `archives`. A title with no
    /// [`oag_title::Campaign::model_unlocks`] gates nothing; one whose grids will not read is
    /// logged and keeps every medal- and grid-gated model locked, loyalty still evaluated.
    #[must_use]
    pub fn read(title: &oag_title::Title, archives: &mut oag_assets::Archives) -> Self {
        let Some(entry) = title.campaign.model_unlocks else {
            return Self::none();
        };
        let campaign_title = crate::records::campaign_title(title, archives.layout.platform);
        match crate::campaign::read_grids(archives, entry, title.campaign.grid_archive) {
            Ok(grids) => Self::on_grids(grids, campaign_title),
            Err(error) => {
                log::warn!("{error:#} - medal- and grid-gated craft models stay locked");
                Self::on_grids(Vec::new(), campaign_title)
            }
        }
    }

    /// Whether this gate evaluates anything: `false` offers every model.
    #[must_use]
    pub fn gates(&self) -> bool {
        self.grids.is_some()
    }

    /// Whether `model` of team `team` of `title` is offered, given `records`.
    #[must_use]
    pub fn offers(
        &self,
        model: &TeamModel,
        team: &str,
        records: &Store,
        title: &oag_title::Title,
    ) -> bool {
        let Some(grids) = self.grids.as_deref() else {
            return true;
        };
        super::unlock_all()
            || model
                .unlock
                .iter()
                .all(|row| self.row_holds(grids, row, team, records, title))
    }

    fn row_holds(
        &self,
        grids: &[Grid],
        row: &ModelUnlock,
        team: &str,
        records: &Store,
        title: &oag_title::Title,
    ) -> bool {
        if row.names_nothing() {
            return false;
        }
        let tiers = MedalTiers::standard();
        let medal_of = super::campaign_medal_of(records, &self.campaign_title, tiers);
        let loyalty = row.loyalty.is_none_or(|needed| {
            let owner = row.team.as_deref().unwrap_or(team);
            let total = if owner.eq_ignore_ascii_case("any") {
                records.loyalty_best(title)
            } else {
                records.loyalty_total(title, owner)
            };
            total >= needed
        });
        let medals = row.medals.as_ref().is_none_or(|count| {
            let golds: u32 = grids
                .iter()
                .filter(|grid| match (&count.campaign, &grid.campaign) {
                    (None, _) => true,
                    (Some(wanted), Some(own)) => own.eq_ignore_ascii_case(wanted),
                    (Some(_), None) => false,
                })
                .map(|grid| {
                    oag_ui_screens::campaign::GridSummary::from_grid_with_medals(
                        grid, tiers, &medal_of,
                    )
                    .gold_medals
                })
                .sum();
            golds >= count.at_least
        });
        let grid = row.grid.as_deref().is_none_or(|name| {
            race_campaign::grid_points_met(grids, tiers, name, &medal_of)
        });
        loyalty && medals && grid
    }
}
