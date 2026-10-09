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
//! [`Pointer::pressed`] and [`Pointer::released`], with no position: they
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
    /// Whether the last device to speak was the mouse - see
    /// [`Self::cursor_at`]. `false` until it has said anything, so a fresh
    /// window shows no arrow until the mouse moves.
    mouse_spoke_last: bool,
}

impl Window {
    pub(crate) fn cursor_moved(&mut self, x: f64, y: f64) {
        self.mouse_spoke_last = true;
        self.place((x as f32, y as f32));
    }

    pub(crate) fn cursor_left(&mut self) {
        self.mouse_spoke_last = true;
        self.at = None;
        self.moved = true;
    }

    pub(crate) fn button(&mut self, button: MouseButton, state: ElementState) {
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
        self.mouse_spoke_last = true;
        let detents = match delta {
            MouseScrollDelta::LineDelta(_, lines) => f64::from(lines),
            MouseScrollDelta::PixelDelta(position) => position.y / PIXELS_PER_DETENT,
        };
        self.scroll -= detents;
    }

    pub(crate) fn touch(&mut self, touch: Touch) {
        self.mouse_spoke_last = false;
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
        pointer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalPosition;

    fn touch(id: u64, phase: TouchPhase, x: f64, y: f64) -> Touch {
        Touch {
            device_id: winit::event::DeviceId::dummy(),
            phase,
            location: PhysicalPosition::new(x, y),
            force: None,
            id,
        }
    }

    #[test]
    fn a_click_between_ticks_is_delivered_once_on_the_next() {
        let mut window = Window::default();
        window.cursor_moved(10.0, 20.0);
        window.button(MouseButton::Left, ElementState::Pressed);
        window.button(MouseButton::Left, ElementState::Released);
        let tick = window.take();
        assert_eq!(tick.at, Some((10.0, 20.0)));
        assert!(tick.moved && tick.clicked && !tick.back);
        let next = window.take();
        assert_eq!(next.at, Some((10.0, 20.0)), "the position persists");
        assert!(next.is_idle(), "the click does not: {next:?}");
    }

    #[test]
    fn the_right_button_is_back_and_a_release_alone_is_nothing() {
        let mut window = Window::default();
        window.button(MouseButton::Right, ElementState::Released);
        assert!(window.take().is_idle());
        window.button(MouseButton::Right, ElementState::Pressed);
        assert!(window.take().back);
    }

    #[test]
    fn a_wheel_up_moves_the_cursor_up_and_pixels_accumulate_to_detents() {
        let mut window = Window::default();
        window.wheel(MouseScrollDelta::LineDelta(0.0, 1.0));
        assert_eq!(window.take().scroll, -1);
        window.wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
            0.0, -30.0,
        )));
        assert_eq!(window.take().scroll, 0, "not a whole detent yet");
        window.wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
            0.0, -30.0,
        )));
        assert_eq!(window.take().scroll, 1, "the remainder was carried");
    }

    #[test]
    fn a_tap_reports_on_lift_as_a_move_and_a_click_and_then_is_nowhere() {
        let mut window = Window::default();
        window.touch(touch(1, TouchPhase::Started, 100.0, 50.0));
        let tick = window.take();
        assert_eq!(tick.at, None, "nothing is decided at touch-down");
        assert!(tick.pressed && !tick.moved && !tick.clicked, "{tick:?}");
        window.touch(touch(1, TouchPhase::Moved, 104.0, 52.0));
        assert!(window.take().is_idle(), "within the threshold");
        window.touch(touch(1, TouchPhase::Ended, 104.0, 52.0));
        let tick = window.take();
        assert_eq!(tick.at, Some((104.0, 52.0)));
        assert!(tick.moved && tick.clicked && tick.released);
        let after = window.take();
        assert_eq!(after.at, None);
        assert!(after.moved && !after.clicked, "{after:?}");
    }

    #[test]
    fn a_drag_past_the_threshold_scrolls_and_never_clicks() {
        let mut window = Window::default();
        window.touch(touch(1, TouchPhase::Started, 100.0, 100.0));
        window.touch(touch(1, TouchPhase::Moved, 100.0, 80.0));
        let tick = window.take();
        assert!(tick.pressed);
        assert_eq!(tick.drag, (0.0, -20.0), "the whole travel, once past it");
        assert_eq!(tick.at, None);
        assert!(!tick.clicked && !tick.moved);
        // Coming back inside the threshold does not turn it into a tap.
        window.touch(touch(1, TouchPhase::Moved, 100.0, 95.0));
        assert_eq!(window.take().drag, (0.0, 15.0));
        window.touch(touch(1, TouchPhase::Ended, 100.0, 98.0));
        let tick = window.take();
        assert!(
            tick.released && !tick.clicked && !tick.moved && tick.at.is_none(),
            "a lifted drag is not a tap: {tick:?}"
        );
        assert_eq!(tick.drag, (0.0, 3.0), "the travel up to the lift counts");
    }

    #[test]
    fn a_cancelled_touch_is_nothing() {
        let mut window = Window::default();
        window.touch(touch(1, TouchPhase::Started, 100.0, 100.0));
        window.take();
        window.touch(touch(1, TouchPhase::Cancelled, 100.0, 100.0));
        let tick = window.take();
        assert!(tick.released && !tick.clicked && tick.at.is_none(), "{tick:?}");
        assert!(window.take().is_idle());
    }

    #[test]
    fn only_the_mouse_draws_a_cursor_and_it_returns_when_the_mouse_moves() {
        let mut window = Window::default();
        assert_eq!(window.cursor_at(), None, "nothing has spoken yet");
        window.cursor_moved(10.0, 20.0);
        assert_eq!(window.cursor_at(), Some((10.0, 20.0)));
        // A finger is its own pointer: the models see it, the arrow does not.
        window.touch(touch(1, TouchPhase::Started, 100.0, 50.0));
        assert_eq!(window.cursor_at(), None);
        window.touch(touch(1, TouchPhase::Ended, 100.0, 50.0));
        assert_eq!(window.take().at, Some((100.0, 50.0)));
        assert_eq!(window.cursor_at(), None);
        window.cursor_moved(12.0, 22.0);
        assert_eq!(window.cursor_at(), Some((12.0, 22.0)));
        // A key or a pad hides it where it is; the position is kept, so the
        // models still know where the mouse is, and a wheel turn or a click
        // brings the arrow back as much as a move does.
        window.other_device();
        assert_eq!(window.cursor_at(), None);
        assert_eq!(window.take().at, Some((12.0, 22.0)));
        window.wheel(MouseScrollDelta::LineDelta(0.0, 1.0));
        assert_eq!(window.cursor_at(), Some((12.0, 22.0)));
        window.other_device();
        window.button(MouseButton::Left, ElementState::Pressed);
        assert_eq!(window.cursor_at(), Some((12.0, 22.0)));
    }

    #[test]
    fn a_second_finger_is_ignored_while_the_first_is_down() {
        let mut window = Window::default();
        window.touch(touch(1, TouchPhase::Started, 100.0, 50.0));
        window.take();
        window.touch(touch(2, TouchPhase::Started, 300.0, 200.0));
        window.touch(touch(2, TouchPhase::Moved, 310.0, 200.0));
        let tick = window.take();
        assert!(tick.is_idle(), "{tick:?}");
        window.touch(touch(2, TouchPhase::Ended, 310.0, 200.0));
        assert!(window.take().is_idle(), "the second finger never counted");
        window.touch(touch(1, TouchPhase::Ended, 100.0, 50.0));
        assert_eq!(window.take().at, Some((100.0, 50.0)), "still the first");
    }

    #[test]
    fn losing_focus_drops_the_pending_presses_but_not_the_position() {
        let mut window = Window::default();
        window.cursor_moved(10.0, 20.0);
        window.button(MouseButton::Left, ElementState::Pressed);
        window.wheel(MouseScrollDelta::LineDelta(0.0, 3.0));
        window.release();
        let tick = window.take();
        assert_eq!(tick.at, Some((10.0, 20.0)));
        assert!(!tick.clicked && tick.scroll == 0);
    }

    #[test]
    fn losing_focus_with_a_finger_down_lifts_it() {
        let mut window = Window::default();
        window.touch(touch(1, TouchPhase::Started, 100.0, 100.0));
        window.touch(touch(1, TouchPhase::Moved, 100.0, 60.0));
        window.take();
        window.release();
        let tick = window.take();
        assert!(tick.released && !tick.clicked, "{tick:?}");
        assert_eq!(tick.drag, (0.0, 0.0));
    }
}
