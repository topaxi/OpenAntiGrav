//! The `HexSelection` widget's step: the lattice slide, the deferred step and the ring's ease.
//!
//! `docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`, "The lattice slide" and "The ring's
//! position". A step sets a 0.1 s timer; while it runs every column moves one pitch toward the
//! step's side, linearly, and the cells keep the previous window's content. When it ends the
//! columns snap back to their slots with the new window. A selection that moves while a slide runs
//! is not lost: the screen compares its focused entry with the one last stepped to every frame, so
//! the step fires on the first frame after the timer reaches 0. The ring eases half way to its
//! target on every update, so a step that flips the centre column's half-row leaves it
//! 42 units off, halving each tick.

/// How long one step's slide takes.
pub const SLIDE_SECONDS: f32 = 0.1;

/// Slack for the six fixed ticks of 1/60 s summing a hair under [`SLIDE_SECONDS`] in `f32`.
const SLIDE_SLACK: f32 = 1e-4;

/// The per-tick factor the ring's offset from its target shrinks by.
const RING_EASE: f32 = 0.5;

/// One grid's step state, ticked with the screen. Frame 0 is the tick on which the step lands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slide {
    /// The entry the centre column shows once the slide is over: what the last step went to.
    shown: usize,
    /// The entry that was the centre column before the step.
    prev: usize,
    /// `-1` for a forward step (the lattice moves left), `+1` for a backward one.
    dir: i8,
    /// Seconds since the step landed; at or past [`SLIDE_SECONDS`] the slide is over.
    age: f32,
    /// The ring's offset below its target, in half rows: positive is low.
    ring: f32,
}

impl Slide {
    /// A grid at rest on `shown`.
    #[must_use]
    pub fn at_rest(shown: usize) -> Self {
        Self {
            shown,
            prev: shown,
            dir: 0,
            age: SLIDE_SECONDS,
            ring: 0.0,
        }
    }

    /// A grid `age` seconds into a forward step onto `shown` of `count` entries, for a still.
    #[must_use]
    pub fn stepped(shown: usize, count: usize, age: f32) -> Self {
        let mut out = Self::at_rest(shown);
        if count > 1 {
            out.begin((shown + count - 1) % count, shown, count);
            out.age = age;
            let ticks = (age * 60.0).floor().max(0.0);
            out.ring *= RING_EASE.powi(ticks as i32);
        }
        out
    }

    fn begin(&mut self, from: usize, to: usize, count: usize) {
        let forward = if from + 1 == count && to == 0 {
            true
        } else if from == 0 && to + 1 == count {
            false
        } else {
            to > from
        };
        self.prev = from;
        self.shown = to;
        self.dir = if forward { -1 } else { 1 };
        self.age = 0.0;
        // The centre column's half-row follows its team's parity: the ring heads to the new one.
        self.ring += low(from) - low(to);
    }

    /// Advances one fixed tick toward `target` of `count` entries.
    pub fn tick(&mut self, dt: f32, target: usize, count: usize) {
        if self.moving() {
            self.age += dt;
            self.ring *= RING_EASE;
        } else if target != self.shown && count > 1 {
            self.begin(self.shown, target, count);
        } else {
            self.ring *= RING_EASE;
        }
    }

    /// Whether the slide is running.
    #[must_use]
    pub fn moving(&self) -> bool {
        self.dir != 0 && self.age + SLIDE_SLACK < SLIDE_SECONDS
    }

    /// The entry the centre column held before the step; the window the cells show while it runs.
    #[must_use]
    pub fn previous(&self) -> usize {
        self.prev
    }

    /// The entry the last step went to.
    #[must_use]
    pub fn shown(&self) -> usize {
        self.shown
    }

    /// Which way the columns move: `-1` toward the left (a forward step), `+1` toward the right.
    #[must_use]
    pub fn direction(&self) -> i8 {
        self.dir
    }

    /// How far through the slide, `0..=1`: the columns have moved this fraction of a pitch.
    #[must_use]
    pub fn progress(&self) -> f32 {
        (self.age / SLIDE_SECONDS).clamp(0.0, 1.0)
    }

    /// The ring's offset below its target, in half rows.
    #[must_use]
    pub fn ring_offset(&self) -> f32 {
        self.ring
    }
}

/// `1.0` for an odd entry, whose column sits half a row low.
fn low(entry: usize) -> f32 {
    if entry % 2 == 1 { 1.0 } else { 0.0 }
}

#[cfg(test)]
mod tests;
