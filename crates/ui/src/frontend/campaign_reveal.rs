//! Finding a hidden prototype event on 2048's campaign map.
//!
//! An event that authors the "locked" option (`m_options & 0x10`) is plain
//! ground until the player's rank reaches its `m_RankRequired`. From then on
//! its button is state 3: it draws nothing, but the canvas averages the alpha
//! of its cells every frame, so it rises and falls with the music wave as one
//! flat spot while the ground round it ripples cell by cell. A tap on one of
//! its cells (`Canvas3D_TapCell` `0x81071de4`) sets its progress to 2: it is
//! then the ordinary grey prism, in pad navigation and openable
//! (`docs/ghidra/functions/vita-2048-eu-v104/canvas-music-wave.md`, sections 3
//! to 5).
//!
//! The game says which events are [`Reveal::Revealable`] or [`Reveal::Found`]
//! ([`Frontend::refresh_campaign_reveals`]: this crate keeps no XP and no
//! save) and hears of a find through [`Frontend::take_found_events`].
//!
//! **Chosen, not measured**: that a met gate makes an event revealable as soon
//! as the rank is met (in the original `Campaign_UnlockEvents` did not promote
//! on map entry at rank 50, so when progress reaches 1 is open), and that the
//! tap is a mouse click or Confirm on the right-stick cursor (the original's
//! is a Vita touch, which panel is open).

use super::campaign_map::CampaignMap;
use super::canvas_camera::Camera;
use super::canvas_cells::{self, HIDDEN_SHAPE};
use super::canvas_lattice::{CELL_DX, CELL_DY, CORNERS_DOWN, CORNERS_UP};
use super::*;

/// How far a rank-gated event has come.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Reveal {
    /// The rank is not met, or the event is not hidden: ordinary ground.
    #[default]
    Hidden,
    /// The rank is met: the flat moving spot a tap finds.
    Revealable,
    /// Found: the grey prism.
    Found,
}

/// The four cells of the hidden button whose first cell is `first`.
fn shape(first: (i32, i32)) -> [(i32, i32); 4] {
    HIDDEN_SHAPE.map(|(dc, dr)| (first.0 + dc, first.1 + dr))
}

impl CampaignMap {
    pub(super) fn is_found(&self, index: usize) -> bool {
        self.reveal.get(index) == Some(&Reveal::Found)
    }

    /// The cells of every event that is revealable and not yet found.
    pub(super) fn spots(&self) -> Vec<(usize, [(i32, i32); 4])> {
        self.reveal
            .iter()
            .enumerate()
            .filter(|(_, reveal)| **reveal == Reveal::Revealable)
            .filter_map(|(at, _)| {
                let (col, row) = Self::cell(self.events.get(at)?);
                Some((at, shape((col.round() as i32, row.round() as i32))))
            })
            .collect()
    }

    /// Finds the revealable event whose cells hold `at` (viewport pixels):
    /// marks it found, queues its name for the game and selects it. A point
    /// off every spot, or on a spot whose rank is not met, does nothing.
    pub(super) fn reveal_at(&mut self, at: (f32, f32)) -> Option<usize> {
        let camera = Camera::scrolled(self.scroll());
        let (index, _) = self
            .spots()
            .into_iter()
            .find(|(_, cells)| cells.iter().any(|&cell| cell_holds(&camera, cell, at)))?;
        self.reveal[index] = Reveal::Found;
        self.found.push(self.events[index].name.clone());
        self.selected = index;
        self.follow();
        Some(index)
    }
}

/// Whether the ground triangle of `cell`, projected by `camera`, holds `at`.
fn cell_holds(camera: &Camera, cell: (i32, i32), at: (f32, f32)) -> bool {
    let corners = if canvas_cells::is_up(cell) {
        &CORNERS_UP
    } else {
        &CORNERS_DOWN
    };
    let centre = (cell.0 as f32 * CELL_DX, cell.1 as f32 * CELL_DY);
    let p = corners.map(|corner| camera.project([centre.0 + corner[0], centre.1 + corner[1], 0.0]));
    let side = |a: (f32, f32), b: (f32, f32)| (b.0 - a.0) * (at.1 - a.1) - (b.1 - a.1) * (at.0 - a.0);
    let signs = [side(p[0], p[1]), side(p[1], p[2]), side(p[2], p[0])];
    signs.iter().all(|&s| s >= 0.0) || signs.iter().all(|&s| s <= 0.0)
}

impl Frontend {
    /// Folds the game's view of the profile into the map: `reveal(event)` is
    /// asked once per event. Only a rank-gated event can be anything but
    /// [`Reveal::Hidden`].
    pub fn refresh_campaign_reveals(&mut self, reveal: impl Fn(&MapEvent) -> Reveal) {
        self.campaign.reveal = self
            .campaign
            .events
            .iter()
            .map(|event| {
                if event.rank_gated {
                    reveal(event)
                } else {
                    Reveal::Hidden
                }
            })
            .collect();
    }

    /// The events found since the last call, by name: the game files them in
    /// the profile.
    pub fn take_found_events(&mut self) -> Vec<String> {
        std::mem::take(&mut self.campaign.found)
    }
}
