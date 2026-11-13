//! The pad's focus on the 2048 campaign map: which event the d-pad moves to,
//! how a held direction repeats, and how the focused event looks.
//!
//! Omega's `TouchCampaign` is the source (`docs/ghidra/functions/ps4-omega-eu/campaign-map-pad.md`,
//! confidence 72 to 84, static reading). The one shared map takes it on both
//! titles: Omega's law as measured, applied to Vita 2048 by choice (Vita's own
//! `TouchCampaign` answers touch only).
//!
//! - **Pick** ([`pick`]): on the raw canvas tile indices of the events (an
//!   event step is four columns across and two rows down, so the space is not
//!   isotropic), the angle from the row axis picks a 130 degree cone per
//!   direction and the nearest event inside it wins; none leaves the focus where
//!   it is. Cones overlap by 40 degrees and there is no sideways penalty.
//! - **Input**: the d-pad only (the left stick never reaches `Button`), polled
//!   left, right, up, down, first to fire wins. A held direction fires again
//!   after [`REPEAT_FIRST`] and then every [`REPEAT_EVERY`] (`Pad_UpdateState`'s
//!   `pad+0x10d0`/`+0x10d4`).
//! - **Look**: the focused event's tile brightens by [`pulse`] on a
//!   [`PULSE_RATE`] phase, and a ripple runs out over the lattice for
//!   [`RIPPLE_SECONDS`] ([`ripple_alpha`]).
//!
//! **Chosen, not measured**: the pulse's base grey is the open event's grey
//! (`canvas_prism::colours::OPEN`, whose low byte the original's `DAT_01aeb688`
//! may or may not be; unread), the ripple's reach in lattice units (the page
//! divides by a tile pitch it did not read) and the view's glide speed (the
//! `TouchScroll`'s easing is unread; [`crate::kinetic::SPRING`] stands in).

use crate::menu::nav::{Dir, Nav};

use super::campaign_map::CampaignMap;
use super::canvas_camera;
use super::*;

/// Seconds a direction is held before it fires again.
pub const REPEAT_FIRST: f32 = 0.4;
/// Seconds between the fires that follow.
pub const REPEAT_EVERY: f32 = 0.12;
/// Radians a second the pulse's phase advances (`TouchCampaign_Update`).
pub const PULSE_RATE: f32 = 5.0;
/// How long the ripple lasts, seconds (`canvas+0x5f8b4`).
pub const RIPPLE_SECONDS: f32 = 2.5;
/// The ripple's radius for a default event: half its button's size field, 7.0.
pub const RIPPLE_RADIUS: f32 = 3.5;

/// The directions in the order `TouchCampaign_Update` polls them.
const POLL: [Button; 4] = [Button::Left, Button::Right, Button::Up, Button::Down];

/// One direction's auto-repeat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Repeat {
    held: f32,
    due: f32,
}

impl Default for Repeat {
    fn default() -> Self {
        Self {
            held: 0.0,
            due: REPEAT_FIRST,
        }
    }
}

impl Repeat {
    /// One tick: whether the direction fires. A press fires at once; a hold
    /// fires again at [`REPEAT_FIRST`] and every [`REPEAT_EVERY`] after.
    pub(super) fn step(&mut self, dt: f32, held: bool, pressed: bool) -> bool {
        if !held {
            *self = Self::default();
            return false;
        }
        if pressed {
            *self = Self::default();
            return true;
        }
        self.held += dt;
        if self.held >= self.due {
            self.due += REPEAT_EVERY;
            return true;
        }
        false
    }
}

/// A ripple running out from a focus change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Ripple {
    /// The focus point, as `canvas_camera::tile_top` gives it.
    pub(super) at: (f32, f32),
    pub(super) radius: f32,
    pub(super) elapsed: f32,
}

/// The pad's side of the map.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct PadFocus {
    /// The pad moved the focus last, so the focus look replaces the pointer's
    /// marker.
    pub(super) active: bool,
    repeats: [Repeat; 4],
    /// Radians, wrapped at a full turn.
    pub(super) phase: f32,
    pub(super) ripple: Option<Ripple>,
}

impl PadFocus {
    /// Advances the pulse's phase.
    pub(super) fn tick(&mut self, dt: f32) {
        self.phase = (self.phase + dt * PULSE_RATE) % std::f32::consts::TAU;
    }
}

/// How much the focused event brightens, in 0..=255 levels: `sin(phase) * 32.5 + 32.5`.
#[must_use]
pub fn pulse(phase: f32) -> f32 {
    phase.sin() * 32.5 + 32.5
}

/// The alpha a lattice tile at squared distance `d2` from the focus gets `t`
/// seconds into a ripple of radius `r`, 0..=255 (`Canvas3D_Update`).
#[must_use]
pub fn ripple_alpha(t: f32, d2: f32, r: f32) -> f32 {
    let r2 = r * r;
    let shape = ((-5.5 * t - std::f32::consts::PI + 0.15 * d2).sin() + 1.0) * 128.0;
    let rise = (4.0 * t).min(1.0);
    let fade = ((RIPPLE_SECONDS - t) / RIPPLE_SECONDS).clamp(0.0, 1.0);
    let reach = ((r2 - d2) / r2).clamp(0.0, 1.0);
    (shape * rise * fade * reach).clamp(0.0, 255.0)
}

/// The event `direction` (`Left`, `Right`, `Up` or `Down`) moves to from
/// `from`, among `others` (index and corner, raw tile indices: column, row):
/// the nearest inside the direction's 130 degree cone, `None` when none is.
///
/// Corners stand in for centres: the centre is the corner plus a per-type
/// offset (`campaign-map-pad.md`), and every `MapEvent` carries the default
/// footprint, so the offset cancels out of the difference.
#[must_use]
pub fn pick(
    from: (f32, f32),
    others: impl IntoIterator<Item = (usize, (f32, f32))>,
    direction: Button,
) -> Option<usize> {
    others
        .into_iter()
        .filter_map(|(at, to)| {
            let (dcol, drow) = (to.0 - from.0, to.1 - from.1);
            let angle = (dcol.atan2(drow).to_degrees() + 360.0) % 360.0;
            let inside = match direction {
                Button::Right => (25.0..155.0).contains(&angle),
                Button::Up => (115.0..245.0).contains(&angle),
                Button::Left => (205.0..335.0).contains(&angle),
                Button::Down => angle >= 295.0 || angle < 65.0,
                _ => false,
            };
            inside.then(|| (at, (dcol * dcol + drow * drow).sqrt()))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(at, _)| at)
}

impl Frontend {
    /// The d-pad on the map: each direction in turn, first to fire moves the
    /// focus. Returns whether one did.
    pub(super) fn step_pad_focus(&mut self, dt: f32, input: &mut Input) -> bool {
        let mut moved = false;
        for (slot, button) in POLL.into_iter().enumerate() {
            let pressed = input.is_pressed(button);
            let fires = self.campaign.focus.repeats[slot].step(dt, input.is_held(button), pressed);
            if moved || !fires {
                continue;
            }
            input.consume_press(button);
            self.campaign.cursor.hide();
            self.move_focus(button);
            moved = true;
        }
        moved
    }

    /// Moves the focus to the event `direction` points at, if there is one.
    fn move_focus(&mut self, direction: Button) {
        let events = &self.campaign.events;
        let Some(from) = events.get(self.campaign.selected) else {
            return;
        };
        let at = pick(
            (from.x, from.y),
            events
                .iter()
                .enumerate()
                .filter(|&(at, _)| at != self.campaign.selected && !self.campaign.hidden(at))
                .map(|(at, event)| (at, (event.x, event.y))),
            direction,
        );
        let Some(next) = at else {
            return;
        };
        self.campaign.selected = next;
        self.focus_changed(Some(direction));
    }

    /// What a focus change does: the view glides to the event, the pad look
    /// takes over, a ripple starts and the cursor sound plays.
    pub(super) fn focus_changed(&mut self, direction: Option<Button>) {
        self.campaign.follow();
        let focus = &mut self.campaign.focus;
        focus.active = true;
        if let Some(event) = self.campaign.events.get(self.campaign.selected) {
            let top = canvas_camera::tile_top(CampaignMap::cell(event));
            self.campaign.focus.ripple = Some(Ripple {
                at: (top[0], top[1]),
                radius: RIPPLE_RADIUS,
                elapsed: 0.0,
            });
        }
        let dir = direction.and_then(|button| match button {
            Button::Up => Some(Dir::Up),
            Button::Down => Some(Dir::Down),
            Button::Left => Some(Dir::Left),
            Button::Right => Some(Dir::Right),
            _ => None,
        });
        self.navs.push(Nav::Moved(dir));
    }
}
