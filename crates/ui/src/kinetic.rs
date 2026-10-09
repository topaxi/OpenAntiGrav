//! Touch scrolling the way a phone does it: the content follows the finger,
//! a flick keeps it going under friction, and it comes to rest on an item.
//!
//! **Ours, with no counterpart on any disc.** Every title this project
//! reimplements was authored for a pad (see [`crate::pointer`]), so there is
//! nothing to measure; every constant below is **chosen, not measured**, and
//! says what it was chosen against. One model for every drag-scrolled view
//! rather than one per screen, so a list, HD's hex grids and 2048's campaign
//! map all feel the same under a finger.
//!
//! # The model
//!
//! One axis of content position, [`Kinetic::offset`], in whatever unit the
//! caller scrolls by - columns, rows, grid pixels - with [`Extent::pitch`]
//! saying how many of those units make one item, so the speeds below are in
//! items a second whatever the unit is.
//!
//! - **Held.** From touch-down ([`Pointer::pressed`]) until lift the content
//!   follows the finger 1:1 ([`Kinetic::drag`]). Past either end of content
//!   that does not wrap it follows with rubber-band resistance
//!   ([`RUBBER_BAND`]): the further it is pulled, the less it gives.
//! - **Released.** On lift ([`Pointer::released`]) the finger's velocity is
//!   read off the last [`VELOCITY_WINDOW`] of ticks, so a finger that
//!   stopped before lifting flings nothing. Faster than [`MIN_FLING`] it
//!   coasts, its speed decaying exponentially at [`FRICTION`], capped at
//!   [`MAX_FLING`]. With [`Extent::snap`] the coast is aimed: the item the
//!   unaimed coast would stop nearest is picked at release and the decay
//!   rate adjusted (within [`FRICTION_RANGE`]) so it arrives there.
//! - **Settling.** Once the coast is slow ([`HANDOFF`]), or at once for a
//!   slow release, a critically damped spring ([`SPRING`]) carries it the
//!   rest of the way onto the item - or back to the end it overran, which
//!   is what makes an overrun bounce.
//! - **Caught.** A touch-down while it is still moving stops it where it
//!   is, and the tap that touch makes selects nothing
//!   ([`Kinetic::gesture`] swallows it), as on every phone.
//!
//! Time comes only from the screen's own tick `dt` ([`Kinetic::tick`]);
//! there is no wall clock and no randomness, so the same ticks give the same
//! motion. This is the front end, not the simulation: nothing here reaches
//! a state hash.

use crate::pointer::Pointer;

/// How fast a free coast loses speed: its velocity is multiplied by
/// `exp(-FRICTION * t)`, so a flick travels `speed / FRICTION` before it
/// stops. **Chosen**: 2.0 a second is the decay of iOS's normal scroll
/// deceleration (0.998 per millisecond), the feel most players have in
/// their hands.
pub const FRICTION: f32 = 2.0;

/// The decay rates an aimed coast may be given to land on its item, per
/// second. **Chosen**: wide enough that any flick can be aimed at the item
/// it would have stopped nearest, narrow enough that none of them reads as
/// a different material.
pub const FRICTION_RANGE: (f32, f32) = (1.0, 8.0);

/// The spring that settles onto an item, in radians a second, critically
/// damped so it never overshoots a target it approaches. **Chosen**: at 18
/// a release half an item off its target is within 1% of it in about a
/// third of a second - quick enough to read as a snap, slow enough to read
/// as motion.
pub const SPRING: f32 = 18.0;

/// A coast slower than this, in items a second, hands over to the
/// [`SPRING`]. **Chosen**: well under the speed at which the spring would
/// overshoot from the distance a coast that slow has left (`FRICTION_RANGE`'s
/// fastest rate is under `SPRING`).
pub const HANDOFF: f32 = 2.0;

/// A release slower than this, in items a second, does not coast at all:
/// it settles straight onto the nearest item. **Chosen**: a finger
/// drifting as it lifts is not a flick.
pub const MIN_FLING: f32 = 1.0;

/// The fastest a flick may leave the finger, in items a second.
/// **Chosen**: a hard swipe on a phone travels about two screens; and on
/// HD's team grid each column passed reloads the preview, so this is kept
/// under one item per tick at 30 frames a second.
pub const MAX_FLING: f32 = 24.0;

/// How much of the recent past a release's velocity is read from, in
/// seconds. **Chosen**: a tenth of a second, the window Android's own
/// velocity tracker uses; a finger held still that long before lifting
/// flings nothing.
pub const VELOCITY_WINDOW: f32 = 0.1;

/// The stiffness of the rubber band past an end, as the fraction of the
/// first bit of pull that still moves the content. **Chosen**: 0.55, the
/// coefficient of iOS's own rubber band.
pub const RUBBER_BAND: f32 = 0.55;

/// A coast or settle still moving faster than this, in items a second,
/// is caught by a touch-down and its tap swallowed; slower than this the
/// tap selects as usual. **Chosen**: the tail of a settle is too slow to
/// be what the player meant to stop.
pub const CATCH_SPEED: f32 = 0.5;

/// An unsnapped coast slower than this, in items a second, has stopped.
/// **Chosen**: a tenth of an item a second is under a pixel a tick at any
/// pitch these screens use, so stopping there is not seen.
pub const REST_SPEED: f32 = 0.1;

/// How many ticks of finger travel are kept for [`VELOCITY_WINDOW`]: enough
/// for a tenth of a second at 240 ticks a second.
const SAMPLES: usize = 24;

/// What the content scrolls over: its ends, and what an item is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    /// Units per item. Speeds are in items a second, so a caller scrolling
    /// in pixels gives its row pitch here.
    pub pitch: f32,
    /// Whether the content comes to rest on a whole multiple of `pitch`.
    pub snap: bool,
    /// The ends, if the content has any.
    pub bounds: Bounds,
}

/// Where scrolling stops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bounds {
    /// The content wraps around - HD's hex grids - or is otherwise endless:
    /// the caller folds the offset back itself (see [`Kinetic::shift`]).
    Wrap,
    /// The offset rests inside `min..=max`, and is pulled past either end
    /// only against a rubber band `band` units long: the most it can ever
    /// be pulled past is `band`.
    Clamp { min: f32, max: f32, band: f32 },
}

impl Extent {
    /// Item-unit content that wraps and snaps: a hex column, a page.
    #[must_use]
    pub fn wrapping() -> Self {
        Self {
            pitch: 1.0,
            snap: true,
            bounds: Bounds::Wrap,
        }
    }

    /// Item-unit content from `0` to `last`, snapping: a list whose offset
    /// is its first visible row. The band is a few rows.
    #[must_use]
    pub fn rows(last: f32) -> Self {
        Self {
            pitch: 1.0,
            snap: true,
            bounds: Bounds::Clamp {
                min: 0.0,
                max: last.max(0.0),
                band: 2.0,
            },
        }
    }

    fn clamp(&self, offset: f32) -> f32 {
        match self.bounds {
            Bounds::Wrap => offset,
            Bounds::Clamp { min, max, .. } => offset.clamp(min, max),
        }
    }

    /// Where content held at `raw` by the finger is drawn: `raw` inside the
    /// ends, banded past them.
    fn band(&self, raw: f32) -> f32 {
        let Bounds::Clamp { min, max, band } = self.bounds else {
            return raw;
        };
        let pull = |over: f32| {
            let stretched = over * RUBBER_BAND;
            stretched * band / (stretched + band)
        };
        if raw < min {
            min - pull(min - raw)
        } else if raw > max {
            max + pull(raw - max)
        } else {
            raw
        }
    }

    /// [`Self::band`]'s inverse, so a finger catching banded content holds
    /// it where it is drawn.
    fn unband(&self, offset: f32) -> f32 {
        let Bounds::Clamp { min, max, band } = self.bounds else {
            return offset;
        };
        let raw = |over: f32| {
            let over = over.min(band * 0.999);
            over * band / (RUBBER_BAND * (band - over))
        };
        if offset < min {
            min - raw(min - offset)
        } else if offset > max {
            max + raw(offset - max)
        } else {
            offset
        }
    }

    /// The item `offset` comes to rest on.
    fn rest(&self, offset: f32) -> f32 {
        let at = if self.snap && self.pitch > 0.0 {
            (offset / self.pitch).round() * self.pitch
        } else {
            offset
        };
        self.clamp(at)
    }
}

/// What the content is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// At rest.
    Still,
    /// Under a finger, which holds it at `raw` before the rubber band.
    Held { raw: f32 },
    /// The finger has lifted; the next tick reads its velocity.
    Lifted,
    /// Coasting toward `target` (or freely, without snap) at decay `rate`.
    Coast { target: Option<f32>, rate: f32 },
    /// Settling onto `target`.
    Settle { target: f32 },
}

/// One axis of touch scrolling. See the module doc.
#[derive(Debug, Clone, PartialEq)]
pub struct Kinetic {
    offset: f32,
    velocity: f32,
    phase: Phase,
    /// Finger travel per tick, newest last, as `(travel, dt)`.
    samples: [(f32, f32); SAMPLES],
    /// Travel since the last tick, not yet a sample.
    pending: f32,
    /// A touch-down caught the content moving: its tap selects nothing.
    swallow: bool,
}

impl Default for Kinetic {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl Kinetic {
    /// Content at rest at `offset`.
    #[must_use]
    pub fn new(offset: f32) -> Self {
        Self {
            offset,
            velocity: 0.0,
            phase: Phase::Still,
            samples: [(0.0, 0.0); SAMPLES],
            pending: 0.0,
            swallow: false,
        }
    }

    /// Where the content is now.
    #[must_use]
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// Its velocity, in units a second.
    #[must_use]
    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Whether a finger is on it.
    #[must_use]
    pub fn is_held(&self) -> bool {
        matches!(self.phase, Phase::Held { .. })
    }

    /// Whether it is at rest - neither held nor in motion.
    #[must_use]
    pub fn is_still(&self) -> bool {
        self.phase == Phase::Still
    }

    /// One tick of pointer input on this axis: a touch-down catches the
    /// content, `travel` (this tick's finger travel along the axis, in
    /// units, positive where the offset grows) moves it, and a lift lets it
    /// go. Returns the pointer as the screen should read it - without its
    /// tap when that tap is the one that caught the content moving.
    pub fn gesture(&mut self, pointer: &Pointer, travel: f32, extent: &Extent) -> Pointer {
        if pointer.pressed {
            self.press(extent);
        }
        if travel != 0.0 {
            self.drag(travel, extent);
        }
        let mut out = *pointer;
        if pointer.released {
            self.release(extent);
            if std::mem::take(&mut self.swallow) {
                out.clicked = false;
                out.moved = false;
                out.at = None;
            }
        }
        out
    }

    /// A finger lands: the content stops where it is drawn.
    pub fn press(&mut self, extent: &Extent) {
        // A finger lifted off a flick the tick before has not launched yet:
        // its own speed is what is moving.
        let velocity = match self.phase {
            Phase::Lifted => self.finger_velocity(),
            _ => self.velocity,
        };
        let speed = velocity.abs() / extent.pitch.max(f32::MIN_POSITIVE);
        let moving = matches!(
            self.phase,
            Phase::Coast { .. } | Phase::Settle { .. } | Phase::Lifted
        );
        self.swallow = moving && speed > CATCH_SPEED;
        self.velocity = 0.0;
        self.pending = 0.0;
        self.samples = [(0.0, 0.0); SAMPLES];
        self.phase = Phase::Held {
            raw: extent.unband(self.offset),
        };
    }

    /// The finger moves by `travel` units. A drag that arrives without a
    /// touch-down first (a pointer layer that missed it) catches the
    /// content as one would.
    pub fn drag(&mut self, travel: f32, extent: &Extent) {
        if !self.is_held() {
            self.press(extent);
        }
        if let Phase::Held { raw } = &mut self.phase {
            *raw += travel;
            self.offset = extent.band(*raw);
            self.pending += travel;
        }
    }

    /// The finger lifts. The velocity is read on the next tick, once the
    /// travel of the tick that ended is a sample.
    pub fn release(&mut self, _extent: &Extent) {
        if self.is_held() {
            self.phase = Phase::Lifted;
        }
    }

    /// Stops any motion and settles onto the nearest item, from where it
    /// is: what a pad step does to a scroll it interrupts.
    pub fn settle(&mut self, extent: &Extent) {
        self.velocity = 0.0;
        self.swallow = false;
        self.settle_from_here(extent);
    }

    /// Puts the content at `offset`, at rest: the view moved by a pad or a
    /// wheel, which this model follows rather than fights.
    pub fn set(&mut self, offset: f32) {
        *self = Self::new(offset);
    }

    /// Moves the content and everything it is aiming at by `by` units
    /// without changing its motion: how a wrapping caller folds a whole
    /// item it has consumed back out of the offset.
    pub fn shift(&mut self, by: f32) {
        self.offset += by;
        match &mut self.phase {
            Phase::Held { raw } => *raw += by,
            Phase::Coast {
                target: Some(target),
                ..
            }
            | Phase::Settle { target } => *target += by,
            Phase::Still | Phase::Lifted | Phase::Coast { target: None, .. } => {}
        }
    }

    /// Advances the motion by one tick of `dt` seconds.
    pub fn tick(&mut self, dt: f32, extent: &Extent) {
        if dt <= 0.0 {
            return;
        }
        match self.phase {
            Phase::Still => {}
            Phase::Held { .. } => self.sample(dt),
            Phase::Lifted => {
                // A lift that brought no travel of its own adds no sample:
                // the tick it happened in is not time the finger was still.
                if self.pending != 0.0 {
                    self.sample(dt);
                }
                self.launch(extent);
            }
            Phase::Coast { target, rate } => self.coast(dt, target, rate, extent),
            Phase::Settle { target } => self.spring(dt, target),
        }
    }

    fn sample(&mut self, dt: f32) {
        self.samples.rotate_left(1);
        self.samples[SAMPLES - 1] = (std::mem::take(&mut self.pending), dt);
    }

    /// The finger's velocity over the last [`VELOCITY_WINDOW`], in units a
    /// second.
    fn finger_velocity(&self) -> f32 {
        let (mut travel, mut time) = (0.0, 0.0);
        for &(moved, dt) in self.samples.iter().rev() {
            if time >= VELOCITY_WINDOW || dt <= 0.0 {
                break;
            }
            travel += moved;
            time += dt;
        }
        if time > 0.0 { travel / time } else { 0.0 }
    }

    /// Turns a lifted finger into a coast or a settle.
    fn launch(&mut self, extent: &Extent) {
        let pitch = extent.pitch.max(f32::MIN_POSITIVE);
        let limit = MAX_FLING * pitch;
        let velocity = self.finger_velocity().clamp(-limit, limit);
        let outside = extent.clamp(self.offset) != self.offset;
        if velocity.abs() < MIN_FLING * pitch || outside {
            self.velocity = if outside { 0.0 } else { velocity };
            self.settle_from_here(extent);
            return;
        }
        self.velocity = velocity;
        let natural = self.offset + velocity / FRICTION;
        if !extent.snap {
            self.phase = Phase::Coast {
                target: None,
                rate: FRICTION,
            };
            return;
        }
        let target = extent.rest(natural);
        let distance = target - self.offset;
        // Aim the decay so the coast's own asymptote is the target; a
        // target behind the flick (a tiny flick rounding back) is a settle.
        if distance * velocity <= 0.0 {
            self.settle_from_here(extent);
            return;
        }
        let rate = (velocity / distance).clamp(FRICTION_RANGE.0, FRICTION_RANGE.1);
        self.phase = Phase::Coast {
            target: Some(target),
            rate,
        };
    }

    fn settle_from_here(&mut self, extent: &Extent) {
        let target = if extent.snap || extent.clamp(self.offset) != self.offset {
            extent.rest(self.offset)
        } else {
            self.offset
        };
        if target == self.offset && self.velocity == 0.0 {
            self.phase = Phase::Still;
        } else {
            self.phase = Phase::Settle { target };
        }
    }

    fn coast(&mut self, dt: f32, target: Option<f32>, rate: f32, extent: &Extent) {
        let decay = (-rate * dt).exp();
        self.offset += self.velocity * (1.0 - decay) / rate;
        self.velocity *= decay;
        let pitch = extent.pitch.max(f32::MIN_POSITIVE);
        // Overran an end: the spring pulls it back, which is the bounce.
        if extent.clamp(self.offset) != self.offset {
            self.phase = Phase::Settle {
                target: extent.clamp(self.offset),
            };
            return;
        }
        if self.velocity.abs() < HANDOFF * pitch {
            match target {
                Some(target) => self.phase = Phase::Settle { target },
                None if self.velocity.abs() < REST_SPEED * pitch => {
                    self.velocity = 0.0;
                    self.phase = Phase::Still;
                }
                None => {}
            }
        }
    }

    /// One step of the critically damped spring onto `target`, solved
    /// exactly rather than integrated, so a long tick cannot make it ring.
    fn spring(&mut self, dt: f32, target: f32) {
        let x = self.offset - target;
        let c2 = self.velocity + SPRING * x;
        let decay = (-SPRING * dt).exp();
        let inner = x + c2 * dt;
        let x = inner * decay;
        self.velocity = (c2 - SPRING * inner) * decay;
        self.offset = target + x;
        if x.abs() < 1e-3 && self.velocity.abs() < 1e-2 {
            self.offset = target;
            self.velocity = 0.0;
            self.phase = Phase::Still;
        }
    }
}

#[cfg(test)]
mod tests;
