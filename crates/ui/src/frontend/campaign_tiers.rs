//! The downloadable campaigns on the campaign map: HD's and Fury's, each a map
//! of its own that the `GameModeChoice` tile of the same name opens.
//!
//! **What the disc authors.** The v1.04 patch adds two tiles to `GameModeChoice`
//! (`offlinehd`, `offlinefury`, `FE_RC_HD` and `FE_RC_FURY`) and two widgets to
//! the campaign shell (`<TouchCampaignHD>`, `<TouchCampaignFury>`), whose
//! hotspots the executable adds one per `PI_Cell` when the matching package is
//! mounted (`docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md`).
//! The tile opens the same shell the single-player events sit on; a caller gives
//! the front end a map per mounted package ([`Tier`]) and the tile of the same
//! name swaps it in. A tile whose package is not mounted has no map and does
//! what it always did here: says it leads to another title's campaign and stays.
//!
//! **Chosen, not measured.** That the original shows one campaign's nodes at a
//! time (the HD and Fury projections share one bias and overlap, so they cannot
//! be on screen together) is read off those two tables rather than watched, and
//! so is the map each tile swaps in being a separate list rather than a view of
//! one canvas.
//!
//! A tier's events are keyed by their `PI_Cell` names, which no single-player
//! event shares, so the progress a save gives
//! ([`Frontend::refresh_campaign_progress`]) reaches every map by name alone.

use super::campaign_map::{CampaignMap, EarnedTier, MapEvent};
use super::*;

/// One downloadable campaign's map: the `GameModeChoice` tile that opens it
/// and its nodes, in the order the grids list their cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Tier {
    /// The tile's `idstring`: `FE_RC_HD`, `FE_RC_FURY`.
    pub tile: String,
    /// Its cells, one node each.
    pub events: Vec<MapEvent>,
}

/// A map that is not on screen, with what the map on screen would hold.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Parked {
    /// The tile that opens it, `None` for the single-player map.
    pub(super) tile: Option<String>,
    pub(super) events: Vec<MapEvent>,
    pub(super) earned: Vec<Option<EarnedTier>>,
    pub(super) closed: Vec<bool>,
    pub(super) selected: usize,
}

impl CampaignMap {
    /// Calls `each` for the map on screen and every parked one, with each
    /// map's events and the two vectors a caller's progress fills.
    pub(super) fn each_map(
        &mut self,
        mut each: impl FnMut(&[MapEvent], &mut Vec<Option<EarnedTier>>, &mut Vec<bool>),
    ) {
        each(&self.events, &mut self.earned, &mut self.closed);
        for map in &mut self.parked {
            each(&map.events, &mut map.earned, &mut map.closed);
        }
    }
}

impl Frontend {
    /// Gives the front end the maps of the downloadable campaigns, parked
    /// until their tiles are confirmed. An empty map is dropped.
    pub fn set_tiers(&mut self, tiers: Vec<Tier>) {
        for tier in tiers.into_iter().filter(|tier| !tier.events.is_empty()) {
            let earned = vec![None; tier.events.len()];
            self.campaign.parked.push(Parked {
                tile: Some(tier.tile),
                events: tier.events,
                earned,
                closed: Vec::new(),
                selected: 0,
            });
        }
    }

    /// Whether `tile` opens a map: the package behind it is mounted.
    #[must_use]
    pub fn has_campaign_tier(&self, tile: &str) -> bool {
        self.campaign
            .parked
            .iter()
            .any(|map| map.tile.as_deref() == Some(tile))
    }

    /// The tile of the map on screen, `None` for the single-player one.
    #[must_use]
    pub fn campaign_tile(&self) -> Option<&str> {
        self.campaign.tile.as_deref()
    }

    /// Puts the map `tile` opens on screen, or the single-player one for
    /// `None`. `false`, and nothing changes, when no such map was given.
    pub fn show_campaign_map(&mut self, tile: Option<&str>) -> bool {
        if self.campaign.tile.as_deref() == tile {
            return true;
        }
        let Some(at) = self
            .campaign
            .parked
            .iter()
            .position(|map| map.tile.as_deref() == tile)
        else {
            return false;
        };
        let incoming = self.campaign.parked.remove(at);
        let outgoing = Parked {
            tile: self.campaign.tile.take(),
            events: std::mem::take(&mut self.campaign.events),
            earned: std::mem::take(&mut self.campaign.earned),
            closed: std::mem::take(&mut self.campaign.closed),
            selected: self.campaign.selected,
        };
        self.campaign.parked.push(outgoing);
        self.campaign.tile = incoming.tile;
        self.campaign.events = incoming.events;
        self.campaign.earned = incoming.earned;
        self.campaign.closed = incoming.closed;
        self.campaign.selected = incoming.selected;
        self.campaign.card = None;
        self.campaign.scroll = Default::default();
        self.campaign.follow(self.space.size);
        true
    }

    /// Closes the events of every map `closed` answers `true` for, by name -
    /// the cells of a grid that has not earned the points the grid before it
    /// asks (`oag_tables::race_campaign::grid_points_met`). A single-player
    /// event is asked too and is never closed here: its names are not cells'.
    pub fn refresh_campaign_locks(&mut self, closed: impl Fn(&str) -> bool) {
        self.campaign.each_map(|events, _, shut| {
            *shut = events.iter().map(|event| closed(&event.name)).collect();
        });
        // A map not yet opened starts on its first open node, where a player
        // can act, instead of on a closed one the grids happen to list first.
        for map in &mut self.campaign.parked {
            if map.closed.get(map.selected).copied().unwrap_or(false)
                && let Some(open) = map.closed.iter().position(|&shut| !shut)
            {
                map.selected = open;
            }
        }
    }
}
