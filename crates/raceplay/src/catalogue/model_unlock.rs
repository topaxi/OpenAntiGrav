//! A Wipeout HD/Fury-era `PI_TeamModel`'s `<Unlock>` row, read into what it asks for.
//!
//! The attribute names are the ones the `<Unlock>` parser compares
//! (`docs/ghidra/functions/ps3-hdfury-eu/model-unlock.md`): `Team`, `Grid`, `MedalCount`,
//! `MedalCountHD`, `MedalCountFURY`, `Loyalty`, case-insensitively. Whether a row holds is
//! `oag_game::unlock::ModelGate`'s, which needs the profile's records.

use oag_tables::fexml::Node;

/// One `<Unlock>` row of a `PI_TeamModel`: every condition it names must hold.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelUnlock {
    /// `team`: whose loyalty total [`Self::loyalty`] compares; `None` reads the model's own team.
    pub team: Option<String>,
    /// `loyalty`: the running total the team must reach.
    pub loyalty: Option<u32>,
    /// `MedalCount`, `MedalCountHD` or `MedalCountFURY`: gold medals the profile must hold.
    pub medals: Option<MedalCount>,
    /// `grid`: the campaign grid whose own points must be met, by name (`grid8`).
    pub grid: Option<String>,
}

/// A medal-count condition: how many, and which campaign's grids they count over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MedalCount {
    /// The grids' own `Campaign` value the attribute names (`HD` for `MedalCountHD`, `FURY` for
    /// `MedalCountFURY`, compared case-insensitively); `None` for a plain `MedalCount`, which
    /// counts every grid.
    pub campaign: Option<String>,
    /// The count to reach.
    pub at_least: u32,
}

impl ModelUnlock {
    /// Whether the row names no condition this build reads: such a row never holds, so a model
    /// gated on something unread stays locked rather than open.
    #[must_use]
    pub fn names_nothing(&self) -> bool {
        self.loyalty.is_none() && self.medals.is_none() && self.grid.is_none()
    }
}

/// Every `<Unlock>` child of `model`, in file order.
pub(super) fn read(model: &Node) -> Vec<ModelUnlock> {
    model
        .children_named("Unlock")
        .map(|row| {
            let attr = |name: &str| row.attr(name).map(str::trim);
            let number = |name: &str| attr(name).and_then(|value| value.parse::<u32>().ok());
            let medals = [
                ("MedalCountHD", Some("HD")),
                ("MedalCountFURY", Some("FURY")),
            ]
            .into_iter()
            .chain(std::iter::once(("MedalCount", None)))
            .find_map(|(name, campaign)| {
                Some(MedalCount {
                    campaign: campaign.map(str::to_string),
                    at_least: number(name)?,
                })
            });
            ModelUnlock {
                team: attr("team").map(str::to_string),
                loyalty: number("loyalty"),
                medals,
                grid: attr("grid").map(str::to_string),
            }
        })
        .collect()
}
