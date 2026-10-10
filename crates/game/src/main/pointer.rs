//! The mouse and the touchscreen, between winit's events and one tick's
//! [`oag_ui::pointer::Pointer`].
//!
//! The same shape `oag_input::Keyboard` is to a key: events arrive on the
//! window's own schedule and are latched here, and the tick loop takes one
//! [`Pointer`] per tick through [`Window::take`]. A click that lands between
//! two ticks is therefore delivered on the next one rather than lost, which
//! is finding U5's argument for the keyboard's latch applied to a button
//! that has no repeat rate at all.
//!
//! Positions stay in **physical window pixels** here. The tick loop maps
//! them into whichever grid the stage on screen is drawn in
//! (`oag_game::render::to_grid`), because the grid is the stage's - a PSP
//! title's 480x272, HD's 1920x1080 - and this module does not know which
//! stage is up.
//!
//! # A finger is a pointer that is only there while it is down
//!
//! A finger has no hover, so a tap on a row has to select and activate it
//! together, which is exactly what the models do with a `moved` and a
//! `clicked` in the same tick. Since a finger can also *drag* a list, the
//! tap is decided on **lift**, not on touch-down: `Started` only remembers
//! where the finger landed, and `Ended` reports the position, a move and a
//! click in one tick if the finger never left the landing spot's
//! [`DRAG_THRESHOLD`]. Past it the finger is a drag for good: its travel
//! goes out as [`Pointer::drag`], and it reports no position and no click,
//! so a scroll never selects a row. A lifted finger is nowhere, and leaving
//! it "hovering" over the last row would highlight a row nobody is
//! touching. Only one finger is followed; a second is ignored until the
//! first lifts.
//!
//! Touch-down and lift are reported as edges of their own,
//! [`Pointer::pressed`] and [`Pointer::released`], with no `at` (the landing
//! point rides [`Pointer::press_at`] on the press tick alone): they
//! are what `oag_ui::kinetic` needs to catch a moving scroll and to let one
//! coast, and a scroll is the only thing that reads them. A cancelled touch
//! and a focus loss with a finger down lift it too, so no scroll is left
//! held by a finger that is gone.
//!
//! **Only a mouse draws a cursor.** The drawn pointer (`oag_game::cursor`)
//! exists so a mouse can be seen. Under a finger it would sit beneath the
//! fingertip, showing nothing the finger does not already show and
//! covering the row it is on; beside a pad or a keyboard it would be an
//! arrow parked over a row the player is not using, saying that row is
//! pointed at when nothing is. So [`Window::cursor_at`] answers `None` for
//! as long as the last device to speak was anything but the mouse -
//! [`Window::other_device`] is how the keyboard and the pad say so - and
//! the arrow comes back the moment the mouse moves again, the way every
//! desktop game hides and shows one.

use oag_ui::pointer::Pointer;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase};

/// How many pixels of a smooth scroll count as one detent.
///
/// `MouseScrollDelta::PixelDelta` is what a trackpad and some mice report,
/// with no notion of a line; this is the same figure the platforms
/// themselves round to when they have to invent one.
const PIXELS_PER_DETENT: f64 = 40.0;

/// How far a finger may wander from where it landed, in physical pixels,
/// and still be a tap. **Chosen, not measured**: about a fingertip's
/// width on a phone screen; a mouse is never subject to it.
const DRAG_THRESHOLD: f32 = 12.0;

/// The window's pointer state, accumulated between ticks.
#[derive(Debug, Default)]
pub(crate) struct Window {
    /// Where the pointer is, in physical pixels, or `None` when it is not
    /// over the window.
    at: Option<(f32, f32)>,
    moved: bool,
    clicked: bool,
    back: bool,
    /// Wheel detents since the last take, with the fractional remainder of
    /// a smooth scroll carried forward so it is not thrown away.
    scroll: f64,
    /// The finger being followed, if one is down.
    finger: Option<u64>,
    /// Where the followed finger landed.
    touch_origin: (f32, f32),
    /// Where it was when its travel was last turned into a drag.
    touch_last: (f32, f32),
    /// Whether it has travelled past [`DRAG_THRESHOLD`] and so is no tap.
    dragging: bool,
    /// Drag travel since the last take, in physical pixels.
    drag: (f32, f32),
    /// A tap was just reported: the position is dropped after this tick.
    lifted: bool,
    /// The followed finger touched down since the last take.
    pressed: bool,
    /// The followed finger lifted, or was cancelled, since the last take.
    released: bool,
    /// Touch events that arrived after a lift not yet taken, replayed on
    /// the next take: a lift and a new touch-down in one tick would lose
    /// their order, and a tap on a list still coasting from the lift would
    /// then read as a tap on a list at rest. A frame stall (a preview
    /// loading) is all it takes to put both in one tick.
    queued: Vec<Touch>,
    /// Whether the last device to speak was the mouse - see
    /// [`Self::cursor_at`]. `false` until it has said anything, so a fresh
    /// window shows no arrow until the mouse moves.
    mouse_spoke_last: bool,
    /// Whether the last device to speak was a finger - the front end draws
    /// its taller touch rows while this holds. Cleared by the mouse, a key or
    /// the pad, the same events that set [`Self::mouse_spoke_last`].
    touch_spoke_last: bool,
}

impl Window {
    pub(crate) fn cursor_moved(&mut self, x: f64, y: f64) {
        self.touch_spoke_last = false;
        self.mouse_spoke_last = true;
        self.place((x as f32, y as f32));
    }

    pub(crate) fn cursor_left(&mut self) {
        self.touch_spoke_last = false;
        self.mouse_spoke_last = true;
        self.at = None;
        self.moved = true;
    }

    pub(crate) fn button(&mut self, button: MouseButton, state: ElementState) {
        self.touch_spoke_last = false;
        self.mouse_spoke_last = true;
        if state != ElementState::Pressed {
            return;
        }
        match button {
            MouseButton::Left => self.clicked = true,
            MouseButton::Right => self.back = true,
            _ => {}
        }
    }

    /// A wheel or trackpad scroll. Positive winit `y` is *away* from the
    /// player, which every desktop reads as scrolling *up*; the models want
    /// "rows the cursor moves down by", so the sign flips here.
    pub(crate) fn wheel(&mut self, delta: MouseScrollDelta) {
        self.touch_spoke_last = false;
        self.mouse_spoke_last = true;
        let detents = match delta {
            MouseScrollDelta::LineDelta(_, lines) => f64::from(lines),
            MouseScrollDelta::PixelDelta(position) => position.y / PIXELS_PER_DETENT,
        };
        self.scroll -= detents;
    }

    pub(crate) fn touch(&mut self, touch: Touch) {
        if self.released || !self.queued.is_empty() {
            self.queued.push(touch);
            return;
        }
        self.mouse_spoke_last = false;
        self.touch_spoke_last = true;
        let at = (touch.location.x as f32, touch.location.y as f32);
        match touch.phase {
            TouchPhase::Started => {
                if self.finger.is_some() {
                    return;
                }
                self.finger = Some(touch.id);
                self.touch_origin = at;
                self.touch_last = at;
                self.dragging = false;
                self.pressed = true;
            }
            TouchPhase::Moved => {
                if self.finger != Some(touch.id) {
                    return;
                }
                let (dx, dy) = (at.0 - self.touch_origin.0, at.1 - self.touch_origin.1);
                if self.dragging || dx.hypot(dy) > DRAG_THRESHOLD {
                    self.dragging = true;
                    self.drag.0 += at.0 - self.touch_last.0;
                    self.drag.1 += at.1 - self.touch_last.1;
                    self.touch_last = at;
                }
            }
            TouchPhase::Ended => {
                if self.finger == Some(touch.id) {
                    self.finger = None;
                    self.released = true;
                    if self.dragging {
                        // The travel between the last move and the lift.
                        self.drag.0 += at.0 - self.touch_last.0;
                        self.drag.1 += at.1 - self.touch_last.1;
                    } else {
                        self.moved = true;
                        self.at = Some(at);
                        self.clicked = true;
                        self.lifted = true;
                    }
                    self.dragging = false;
                }
            }
            TouchPhase::Cancelled => {
                if self.finger == Some(touch.id) {
                    self.finger = None;
                    self.dragging = false;
                    self.released = true;
                }
            }
        }
    }

    /// A key or a pad button or stick spoke: the mouse is no longer the
    /// device in use, so the arrow goes until it moves again. See the
    /// module doc. Called by `app.rs` off a raw key event and by the tick
    /// loop off the pad's own reading - never off a press this module
    /// synthesised for a click, which is the mouse speaking.
    pub(crate) fn other_device(&mut self) {
        self.mouse_spoke_last = false;
        self.touch_spoke_last = false;
    }

    /// Whether a finger was the last device to speak. See the field.
    pub(crate) fn touch_spoke_last(&self) -> bool {
        self.touch_spoke_last
    }

    /// Drops every pending press. Call it on focus loss, for the reason
    /// `oag_input::Keyboard::release_all` gives: a click that happened just
    /// before the window lost focus was for whatever took it.
    pub(crate) fn release(&mut self) {
        self.clicked = false;
        self.back = false;
        self.scroll = 0.0;
        // A finger down when focus went is lifted, so nothing stays held.
        if self.finger.take().is_some() {
            self.released = true;
        }
        self.dragging = false;
        self.drag = (0.0, 0.0);
        self.queued.clear();
    }

    /// Where to draw the cursor right now, in window pixels: the latest
    /// position rather than the last tick's, so the arrow keeps up with the
    /// mouse between ticks - and `None` whenever the mouse was not the last
    /// device to speak. See the module doc.
    pub(crate) fn cursor_at(&self) -> Option<(f32, f32)> {
        if self.mouse_spoke_last { self.at } else { None }
    }

    fn place(&mut self, at: (f32, f32)) {
        if self.at != Some(at) {
            self.moved = true;
        }
        self.at = Some(at);
    }

    /// One tick's pointer, in window pixels, and clears the edges. The
    /// position persists; a still pointer is still somewhere.
    pub(crate) fn take(&mut self) -> Pointer {
        let whole = self.scroll.trunc();
        self.scroll -= whole;
        let pointer = Pointer {
            at: self.at,
            moved: self.moved,
            clicked: self.clicked,
            back: self.back,
            scroll: whole as i32,
            drag: std::mem::take(&mut self.drag),
            press_at: self.pressed.then_some(self.touch_origin),
            pressed: std::mem::take(&mut self.pressed),
            released: std::mem::take(&mut self.released),
        };
        self.moved = false;
        self.clicked = false;
        self.back = false;
        if std::mem::take(&mut self.lifted) {
            self.at = None;
            self.moved = true;
        }
        for touch in std::mem::take(&mut self.queued) {
            self.touch(touch);
        }
        pointer
    }
}

#[cfg(test)]
#[path = "pointer/tests.rs"]
mod tests;
