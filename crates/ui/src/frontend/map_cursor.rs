//! The pad's pointer on the 2048 campaign map: the right stick always moves
//! a cursor, and pushing it past the view's edge pans the map.
//!
//! **Ours, with no counterpart on the disc.** The original's map answers a
//! finger only (`docs/formats/2048-frontend.md`, "Controller focus"), and the
//! d-pad's step navigation (`CampaignMap`'s `nearest_event`) cannot reach
//! every event from every other, so a player with only a pad could not open
//! one the steps skip or one that is off screen. The cursor is
//! the same box hit test a mouse click uses ([`CampaignMap::hit_at`]), so a
//! rank-gated event the map hides is as unreachable here as under a mouse.
//!
//! **Chosen, not measured**: the speed ([`MAX_SPEED`], the stick's deflection
//! squared so a small push is precise), the edge pan taking the cursor's
//! overshoot one for one, the cursor appearing on the selected event and
//! hiding after [`IDLE_HIDE`] seconds without stick input, on a d-pad press
//! and as soon as a mouse or finger speaks. The right stick's press is left
//! unbound for later.

use super::campaign_map::CampaignMap;
use super::canvas_camera;
use super::*;
use oag_2048::frontend::states as w2048;

/// Cursor speed at full deflection, in canvas viewport pixels a second: one
/// view width a second.
pub(super) const MAX_SPEED: f32 = canvas_camera::VIEWPORT.0;

/// Seconds without a stick push after which the cursor goes.
pub(super) const IDLE_HIDE: f32 = 3.0;

/// The right stick's state on the map: whether the cursor is up, where, and
/// the stick it is moved by.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapCursor {
    /// The cursor, in the canvas viewport's pixels. `Some` while shown.
    pub(super) at: Option<(f32, f32)>,
    /// Seconds since the stick last moved it.
    idle: f32,
    /// The right stick as `oag_input` reads it, right and up positive,
    /// deadzone already applied.
    stick: (f32, f32),
}

impl MapCursor {
    /// The cursor goes, e.g. because another device took over.
    pub(super) fn hide(&mut self) {
        self.at = None;
    }
}

impl CampaignMap {
    /// Moves the cursor by `dt` seconds of the stick, panning the view by the
    /// part of the move that would leave it, and puts the selection on the
    /// event under it. Nothing while the stick rests and the cursor is down
    /// but for the idle timer.
    pub(super) fn step_cursor(&mut self, dt: f32) {
        let (sx, sy) = self.cursor.stick;
        if sx == 0.0 && sy == 0.0 {
            if self.cursor.at.is_some() {
                self.cursor.idle += dt;
                if self.cursor.idle >= IDLE_HIDE {
                    self.cursor.hide();
                }
            }
            return;
        }
        self.cursor.idle = 0.0;
        let (view_w, view_h) = canvas_camera::VIEWPORT;
        let from = self.cursor.at.unwrap_or_else(|| {
            self.face_of(self.selected)
                .map(|f| (f[0] + f[2] * 0.5, f[1] + f[3] * 0.5))
                .unwrap_or((view_w * 0.5, view_h * 0.5))
        });
        let speed = MAX_SPEED * (sx * sx + sy * sy);
        let norm = sx.hypot(sy).max(f32::MIN_POSITIVE);
        let to = (
            from.0 + sx / norm * speed * dt,
            from.1 - sy / norm * speed * dt,
        );
        let at = (to.0.clamp(0.0, view_w), to.1.clamp(0.0, view_h));
        let over = (to.0 - at.0, to.1 - at.1);
        for (axis, over) in [(0, over.0), (1, over.1)] {
            if over != 0.0 {
                let (min, max) = self.limits(axis);
                let by = (self.scroll[axis].offset() + over).clamp(min, max);
                self.scroll[axis].set(by);
                let extent = self.extent(axis);
                self.scroll[axis].settle(&extent);
            }
        }
        self.cursor.at = Some(at);
        if let Some(hit) = self.hit_at(at) {
            self.selected = hit;
        }
    }
}

impl Frontend {
    /// The right stick this tick (right and up positive, deadzoned): what moves
    /// the campaign map's cursor. A caller that has no stick passes nothing.
    pub fn set_view_stick(&mut self, right: (f32, f32)) {
        self.campaign.cursor.stick = right;
    }

    /// Where the pad's cursor is, in this front end's grid, while it is up on
    /// the campaign map and no card covers it.
    #[must_use]
    pub fn map_cursor(&self) -> Option<(f32, f32)> {
        if !self.machine.is(w2048::NEW_FE_SHELL) || self.campaign.card.is_some() {
            return None;
        }
        let at = self.campaign.cursor.at?;
        Some((
            at.0 * self.space.size.0 / canvas_camera::VIEWPORT.0,
            at.1 * self.space.size.1 / canvas_camera::VIEWPORT.1,
        ))
    }

    /// Whether a held mouse button should pan this screen rather than click at
    /// once: the campaign map with no card over it.
    #[must_use]
    pub fn wants_mouse_pan(&self) -> bool {
        self.machine.is(w2048::NEW_FE_SHELL) && self.campaign.card.is_none()
    }

    /// One tick of the cursor, on the campaign map only.
    pub(super) fn step_map_cursor(&mut self, dt: f64) {
        if self.machine.is(w2048::NEW_FE_SHELL) && self.campaign.card.is_none() {
            #[expect(clippy::cast_possible_truncation, reason = "a tick is a fraction of a second")]
            self.campaign.step_cursor(dt as f32);
        }
    }
}
