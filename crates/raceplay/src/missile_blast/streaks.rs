//! The Missile explosion's sixteen streaks: each blast starts [`COUNT`] entries at
//! its centre in random directions, every one laying a `ThickLine_bighalo`
//! [`oag_fx::fat_line`] node per update as it flies off, from one pool of twenty
//! elements shared by every live blast (`RibbonEffects` pool `+0x5c`).
//!
//! The law is `MissileExplosion_Start` (`0x00155568`) and `MissileExplosion_Update`
//! (`0x00154a78`), `docs/ghidra/functions/ps3-hdfury-eu/missile-explosion.md`,
//! "The sixteen entries at `+0x190`". **A step is one update call, not scaled by
//! `dt`**: the tick is the original's 60 Hz frame.
//!
//! **Chosen, not measured**: the blast's basis is the struck craft's own, as the model's
//! is (see the parent module), and the three spawn turns take the right-handed sense of a
//! uniformly random angle, which no sense can bias.
//!
//! View state with a generator of its own, so no draw reaches the simulation's `Rng`
//! or the stage's.

use oag_core::Rng;
use oag_core::math::{Quat, Vec3};
use oag_fx::fat_line::{Ribbon, Spec};
use oag_fx::rocket_smoke::OpacityRamp;
use oag_mesh::mesh::GpuVertex;
use oag_title::weapons::BlastStreak;

use crate::missile_smoke::spec;

/// Entries one blast starts: `MissileExplosion`'s sixteen at `+0x190`.
pub(crate) const COUNT: usize = 16;

/// Seed for the streaks' draws, distinct from the others for the same reason as
/// [`crate::LEACH_BEAM_SEED`].
const SEED: u64 = 0x5_9a_2b_06;

/// The swirl's angular rate: `sin(15 age)`, `cos(15 age)` (`TOC[-0x224c]`).
const SWIRL_RATE: f32 = 15.0;

/// One entry: the position and the basis rows `MissileExplosion_Start` copies from its
/// matrix and turns, the speed and node-life scale it draws, and the pool element it took.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Streak {
    pub(super) position: Vec3,
    /// Rows 0, 1, 2 of the entry's matrix: the step runs along row 1, the swirl in the
    /// plane of rows 0 and 2.
    pub(super) rows: [Vec3; 3],
    /// Units per update along row 1, `U(7, 8)`.
    pub(super) speed: f32,
    /// Node-life scale, `U(0.5, 1)`.
    pub(super) scale: f32,
    /// The element taken from the pool, or `None` when none was free.
    pub(super) element: Option<usize>,
}

/// The shared pool, the entries each live blast started (on the heap, so the race's own
/// size does not grow by sixteen blasts' worth of entries) and the generator they draw
/// from.
#[derive(Debug, Clone)]
pub(crate) struct Streaks {
    spec: Spec,
    pool: Vec<Option<Ribbon>>,
    /// Indexed by the blast's slot in [`super::SLOTS`].
    entries: Vec<Option<[Streak; COUNT]>>,
    rng: Rng,
}

impl Streaks {
    pub(crate) fn new(streak: BlastStreak) -> Self {
        Self {
            spec: spec(streak.line),
            pool: vec![None; streak.pool],
            entries: vec![None; super::SLOTS],
            rng: Rng::new(SEED),
        }
    }

    /// `U(a, b)`: `Random_RangeFloat`, `a + (b - a) * next`.
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.rng.next_f32()
    }

    /// `MissileExplosion_Start`'s entry loop: each entry copies the blast's basis, draws
    /// its speed and scale, turns by three angles about its row 1, then row 0, then row 2,
    /// and takes the first free element, or none.
    pub(super) fn start(&mut self, slot: usize, position: Vec3, orientation: Quat) {
        let basis = [Vec3::X, Vec3::Y, Vec3::Z].map(|axis| orientation * axis);
        self.entries[slot] = Some(std::array::from_fn(|_| {
            let speed = self.range(7.0, 8.0);
            let scale = self.range(0.5, 1.0);
            let mut rows = basis;
            for about in [1, 0, 2] {
                let angle = self.range(-std::f32::consts::TAU, std::f32::consts::TAU);
                let turn =
                    Quat::from_axis_angle(rows[about].try_normalize().unwrap_or(Vec3::Y), angle);
                rows = rows.map(|row| turn * row);
            }
            let element = self.pool.iter().position(Option::is_none);
            if let Some(free) = element {
                self.pool[free] = Some(Ribbon::new(self.spec));
            }
            Streak {
                position,
                rows,
                speed,
                scale,
                element,
            }
        }));
    }

    /// The blast in `slot` is spent: its entries stop; their elements stay in the pool
    /// until their last node dies.
    pub(super) fn retire(&mut self, slot: usize) {
        self.entries[slot] = None;
    }

    /// The entries of the blast in `slot`.
    #[cfg(test)]
    pub(super) fn entries(&self, slot: usize) -> Option<&[Streak; COUNT]> {
        self.entries[slot].as_ref()
    }

    /// One `MissileExplosion_Update` of a live blast at `age` seconds: every entry lays a
    /// node at its place, then moves along its row 1 with a swirl in the other two axes and
    /// a jitter that shrinks to nothing over the blast's second.
    pub(super) fn step(&mut self, slot: usize, age: f32) {
        let Some(mut entries) = self.entries[slot].take() else {
            return;
        };
        let (sin, cos) = (SWIRL_RATE * age).sin_cos();
        let jitter = 2.0 * (1.0 - age);
        for entry in &mut entries {
            if let Some(ribbon) = entry.element.and_then(|i| self.pool[i].as_mut()) {
                ribbon.push(entry.position, entry.rows[2], entry.rows[1], entry.scale);
            }
            let across = sin + jitter * self.range(-1.0, 1.0);
            let beside = cos + jitter * self.range(-1.0, 1.0);
            entry.position +=
                entry.rows[1] * entry.speed + entry.rows[0] * across + entry.rows[2] * beside;
        }
        self.entries[slot] = Some(entries);
    }

    /// `RibbonEffects_UpdatePools`: ages every element and frees the empty ones.
    pub(super) fn advance(&mut self, dt: f32) {
        for element in &mut self.pool {
            if let Some(live) = element.as_mut() {
                live.advance(dt);
                if live.is_empty() {
                    *element = None;
                }
            }
        }
    }

    /// Elements taken, owned by a live blast or ageing out.
    pub(crate) fn live(&self) -> usize {
        self.pool.iter().flatten().count()
    }

    /// Every element's triangle list for a camera at `eye`.
    pub(crate) fn vertices(&self, eye: Vec3, ramp: &OpacityRamp) -> Vec<GpuVertex> {
        let mut out = Vec::new();
        for ribbon in self.pool.iter().flatten() {
            ribbon.extend_vertices(&mut out, eye, ramp);
        }
        out
    }
}
