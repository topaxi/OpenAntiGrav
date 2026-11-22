//! Multi-touch camera gestures: one-finger drag, two-finger pan, pinch and
//! twist, taps and double taps, and the coast after a flick.
//!
//! **Chosen, not measured.** No Studio Liverpool title has a touch scheme; the
//! conventions are the ones a 3D viewer or a map applies (the picture follows
//! the fingers), and every constant below is this project's own.
//!
//! Pure geometry and state with no window and no winit type, driven by the
//! caller's tick count rather than a clock, so every rule is a unit test and a
//! replayed script produces the same [`Frame`]s. The consumer maps a [`Frame`]
//! onto a camera; nothing here reaches `InputSnapshot` or the simulation.
//!
//! Positions come in as window pixels. Travel goes out as a fraction of the
//! window's height, so a gesture is the same physical size on every phone.
//!
//! Rules (`docs/ui/photo-mode.md`, "Touch"):
//!
//! - A touch that stays inside [`TAP_SLOP`] and lifts within [`TAP_TICKS`] is a
//!   tap. Past the slop it is a drag for good.
//! - Two fingers give three channels, each held back until its own cumulative
//!   travel passes a dead-band ([`TAP_SLOP`], [`PINCH_SLOP`], [`TWIST_SLOP`]),
//!   then live for the rest of the gesture. The held-back amount is released
//!   the tick the band is crossed, so nothing is lost.
//! - A second finger landing or lifting re-anchors the pair, so neither
//!   produces a jump. A third finger is ignored. A touch that ever had two
//!   fingers never taps.
//! - A flick coasts ([`FRICTION`]); pinch and twist never do. A finger landing
//!   catches the coast, and that touch does not tap.

use std::f32::consts::{PI, TAU};

/// Travel, as a fraction of the window's height, past which a touch is a drag
/// and no longer a tap.
pub const TAP_SLOP: f32 = 0.02;
/// Longest a touch may last and still be a tap, in ticks (0.3 s at 60 Hz).
pub const TAP_TICKS: u32 = 18;
/// Longest gap between two taps that make a double tap, in ticks.
pub const DOUBLE_TAP_TICKS: u32 = 18;
/// Furthest apart two taps may land, as a fraction of the window's height.
pub const DOUBLE_TAP_RADIUS: f32 = 0.06;
/// Cumulative `ln(distance ratio)` before a pinch registers (about 3%).
pub const PINCH_SLOP: f32 = 0.03;
/// Cumulative rotation, in radians, before a twist registers.
pub const TWIST_SLOP: f32 = 0.08;
/// How many ticks back a lift reads its release velocity from.
pub const VELOCITY_TICKS: u32 = 6;
/// Slowest release, in height fractions per tick, that still coasts.
pub const MIN_FLING: f32 = 0.002;
/// What a coast keeps of its speed each tick (about a 0.3 s half-life).
pub const FRICTION: f32 = 0.90;

/// What happened since the last [`Gestures::take`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Fingers down now.
    pub fingers: u8,
    /// One-finger travel, as a fraction of the window's height, y down. A
    /// coast after a one-finger flick arrives here too.
    pub drag: (f32, f32),
    /// Two-finger centroid travel, as a fraction of the height, y down. A
    /// coast after a two-finger flick arrives here too.
    pub pan: (f32, f32),
    /// Distance ratio of the two fingers this tick: above 1 they spread.
    pub pinch: f32,
    /// Rotation of the pair this tick in radians, positive clockwise on screen.
    pub twist: f32,
    /// A short one-finger touch that just lifted, in window pixels.
    pub tap: Option<(f32, f32)>,
    /// A tap that no second tap followed, in window pixels.
    pub single_tap: Option<(f32, f32)>,
    /// A second tap in time and radius, in window pixels.
    pub double_tap: Option<(f32, f32)>,
    /// A drag, pinch, pan or twist is under way, so taps are not wanted.
    pub gesturing: bool,
    /// A coast is moving `drag` or `pan` with no finger down.
    pub coasting: bool,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            fingers: 0,
            drag: (0.0, 0.0),
            pan: (0.0, 0.0),
            pinch: 1.0,
            twist: 0.0,
            tap: None,
            single_tap: None,
            double_tap: None,
            gesturing: false,
            coasting: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Finger {
    id: u64,
    start: (f32, f32),
    last: (f32, f32),
    down_tick: u32,
    /// Past the slop, or ever one of two: it can no longer be a tap.
    spent: bool,
    /// Past the one-finger slop: its travel is `drag`.
    dragging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Drag,
    Pan,
}

#[derive(Debug, Clone, Copy)]
struct Coast {
    kind: Kind,
    vel: (f32, f32),
}

/// One two-finger channel held back until it passes its dead-band.
#[derive(Debug, Clone, Copy, Default)]
struct Channel {
    held: f32,
    live: bool,
}

impl Channel {
    /// Adds `by` and returns what to emit: nothing while held back, the whole
    /// held amount on the tick the band is crossed, then each step.
    fn feed(&mut self, by: f32, band: f32) -> f32 {
        if self.live {
            return by;
        }
        self.held += by;
        if self.held.abs() > band {
            self.live = true;
            self.held
        } else {
            0.0
        }
    }
}

/// Tracks up to two fingers and turns them into [`Frame`]s.
#[derive(Debug, Clone)]
pub struct Gestures {
    window: (f32, f32),
    tick: u32,
    fingers: Vec<Finger>,
    /// The touch that landed during a coast, whose lift must not tap.
    swallow: Option<u64>,
    pan_held: (f32, f32),
    pan_live: bool,
    pinch_ch: Channel,
    twist_ch: Channel,
    velocity: Vec<(u32, (f32, f32))>,
    coast: Option<Coast>,
    pending_single: Option<((f32, f32), u32)>,
    queued_single: Vec<(f32, f32)>,
    acc_drag: (f32, f32),
    acc_pan: (f32, f32),
    acc_pinch: f32,
    acc_twist: f32,
    acc_tap: Option<(f32, f32)>,
    acc_double: Option<(f32, f32)>,
}

impl Gestures {
    /// A recogniser for a window of `window` pixels (width, height).
    pub fn new(window: (f32, f32)) -> Self {
        Self {
            window,
            tick: 0,
            fingers: Vec::new(),
            swallow: None,
            pan_held: (0.0, 0.0),
            pan_live: false,
            pinch_ch: Channel::default(),
            twist_ch: Channel::default(),
            velocity: Vec::new(),
            coast: None,
            pending_single: None,
            queued_single: Vec::new(),
            acc_drag: (0.0, 0.0),
            acc_pan: (0.0, 0.0),
            acc_pinch: 1.0,
            acc_twist: 0.0,
            acc_tap: None,
            acc_double: None,
        }
    }

    /// The window changed size.
    pub fn set_window(&mut self, window: (f32, f32)) {
        self.window = window;
    }

    fn h(&self) -> f32 {
        self.window.1.max(1.0)
    }

    fn index(&self, id: u64) -> Option<usize> {
        self.fingers.iter().position(|f| f.id == id)
    }

    fn reset_pair(&mut self) {
        self.pan_held = (0.0, 0.0);
        self.pan_live = false;
        self.pinch_ch = Channel::default();
        self.twist_ch = Channel::default();
        self.velocity.clear();
    }

    /// Stops a coast. Returns whether one was moving.
    pub fn catch(&mut self) -> bool {
        self.coast.take().is_some()
    }

    /// A finger landed at `at` (window pixels).
    pub fn down(&mut self, id: u64, at: (f32, f32)) {
        if self.index(id).is_some() || self.fingers.len() >= 2 {
            return;
        }
        let caught = self.catch();
        let mut finger = Finger {
            id,
            start: at,
            last: at,
            down_tick: self.tick,
            spent: false,
            dragging: false,
        };
        if !self.fingers.is_empty() {
            finger.spent = true;
            for f in &mut self.fingers {
                f.spent = true;
            }
            self.reset_pair();
        } else {
            self.velocity.clear();
            if caught {
                self.swallow = Some(id);
            }
        }
        self.fingers.push(finger);
    }

    /// A finger moved to `at`.
    pub fn moved(&mut self, id: u64, at: (f32, f32)) {
        let Some(i) = self.index(id) else { return };
        let h = self.h();
        if self.fingers.len() == 1 {
            let f = &mut self.fingers[i];
            let prev = f.last;
            f.last = at;
            let mut d = ((at.0 - prev.0) / h, (at.1 - prev.1) / h);
            if !f.dragging {
                let off = ((at.0 - f.start.0) / h, (at.1 - f.start.1) / h);
                if off.0.hypot(off.1) <= TAP_SLOP {
                    return;
                }
                f.dragging = true;
                f.spent = true;
                d = off;
            }
            self.acc_drag.0 += d.0;
            self.acc_drag.1 += d.1;
            self.velocity.push((self.tick, d));
            return;
        }
        let before = (self.fingers[0].last, self.fingers[1].last);
        self.fingers[i].last = at;
        let after = (self.fingers[0].last, self.fingers[1].last);
        let centre = |p: ((f32, f32), (f32, f32))| ((p.0.0 + p.1.0) / 2.0, (p.0.1 + p.1.1) / 2.0);
        let (c0, c1) = (centre(before), centre(after));
        let span = |p: ((f32, f32), (f32, f32))| (p.1.0 - p.0.0, p.1.1 - p.0.1);
        let (s0, s1) = (span(before), span(after));
        let (d0, d1) = (s0.0.hypot(s0.1), s1.0.hypot(s1.1));

        let by = ((c1.0 - c0.0) / h, (c1.1 - c0.1) / h);
        let (px, py) = if self.pan_live {
            by
        } else {
            self.pan_held = (self.pan_held.0 + by.0, self.pan_held.1 + by.1);
            if self.pan_held.0.hypot(self.pan_held.1) > TAP_SLOP {
                self.pan_live = true;
                self.pan_held
            } else {
                (0.0, 0.0)
            }
        };
        self.acc_pan.0 += px;
        self.acc_pan.1 += py;
        if px != 0.0 || py != 0.0 {
            self.velocity.push((self.tick, (px, py)));
        }
        if d0 > 1.0 && d1 > 1.0 {
            let ln = (d1 / d0).ln();
            let out = self.pinch_ch.feed(ln, PINCH_SLOP);
            self.acc_pinch *= out.exp();
            let mut da = s1.1.atan2(s1.0) - s0.1.atan2(s0.0);
            if da > PI {
                da -= TAU;
            } else if da <= -PI {
                da += TAU;
            }
            self.acc_twist += self.twist_ch.feed(da, TWIST_SLOP);
        }
    }

    /// A finger lifted at `at`.
    pub fn up(&mut self, id: u64, at: (f32, f32)) {
        let Some(i) = self.index(id) else { return };
        self.moved(id, at);
        let i = self.index(id).unwrap_or(i);
        let finger = self.fingers.remove(i);
        let h = self.h();
        if !self.fingers.is_empty() {
            // The remaining finger carries on from where it is, as a spent touch.
            let kind = Kind::Pan;
            self.start_coast(kind);
            self.velocity.clear();
            return;
        }
        let swallowed = self.swallow == Some(id);
        if swallowed {
            self.swallow = None;
        }
        if finger.dragging {
            self.start_coast(Kind::Drag);
            return;
        }
        if finger.spent || swallowed || self.tick.saturating_sub(finger.down_tick) > TAP_TICKS {
            if finger.spent && self.pan_live {
                self.start_coast(Kind::Pan);
            }
            return;
        }
        let at = finger.last;
        self.acc_tap = Some(at);
        if let Some((p, t)) = self.pending_single {
            let near = (at.0 - p.0).hypot(at.1 - p.1) <= DOUBLE_TAP_RADIUS * h;
            if near && self.tick.saturating_sub(t) <= DOUBLE_TAP_TICKS {
                self.pending_single = None;
                self.acc_double = Some(at);
                return;
            }
            self.queued_single.push(p);
        }
        self.pending_single = Some((at, self.tick));
    }

    fn start_coast(&mut self, kind: Kind) {
        let from = self.tick.saturating_sub(VELOCITY_TICKS);
        let (mut x, mut y) = (0.0, 0.0);
        for (t, d) in &self.velocity {
            if *t > from {
                x += d.0;
                y += d.1;
            }
        }
        let vel = (x / VELOCITY_TICKS as f32, y / VELOCITY_TICKS as f32);
        self.velocity.clear();
        if vel.0.hypot(vel.1) >= MIN_FLING {
            self.coast = Some(Coast { kind, vel });
        }
    }

    /// The system took a finger away (a cancelled touch). Emits nothing for it.
    pub fn cancel(&mut self, id: u64) {
        if let Some(i) = self.index(id) {
            self.fingers.remove(i);
            self.velocity.clear();
            if self.swallow == Some(id) {
                self.swallow = None;
            }
        }
    }

    /// Forgets every finger and coast: focus lost, or the mode changed.
    pub fn release_all(&mut self) {
        let window = self.window;
        let tick = self.tick;
        *self = Self::new(window);
        self.tick = tick;
    }

    /// Advances one tick and returns what happened since the last call.
    pub fn take(&mut self, window: (f32, f32)) -> Frame {
        self.window = window;
        self.tick += 1;
        let mut frame = Frame {
            fingers: self.fingers.len() as u8,
            drag: std::mem::take(&mut self.acc_drag),
            pan: std::mem::take(&mut self.acc_pan),
            pinch: std::mem::replace(&mut self.acc_pinch, 1.0),
            twist: std::mem::take(&mut self.acc_twist),
            tap: self.acc_tap.take(),
            double_tap: self.acc_double.take(),
            ..Frame::default()
        };
        if self.fingers.is_empty()
            && let Some(c) = &mut self.coast
        {
            let slot = match c.kind {
                Kind::Drag => &mut frame.drag,
                Kind::Pan => &mut frame.pan,
            };
            slot.0 += c.vel.0;
            slot.1 += c.vel.1;
            frame.coasting = true;
            c.vel = (c.vel.0 * FRICTION, c.vel.1 * FRICTION);
            if c.vel.0.hypot(c.vel.1) < MIN_FLING {
                self.coast = None;
            }
        }
        if !self.queued_single.is_empty() {
            frame.single_tap = Some(self.queued_single.remove(0));
        } else if let Some((p, t)) = self.pending_single
            && self.tick.saturating_sub(t) > DOUBLE_TAP_TICKS
        {
            self.pending_single = None;
            frame.single_tap = Some(p);
        }
        frame.gesturing = self.fingers.iter().any(|f| f.dragging)
            || (self.fingers.len() == 2
                && (self.pan_live || self.pinch_ch.live || self.twist_ch.live));
        frame
    }
}

#[cfg(test)]
mod tests;
