//! How many samples the PSP-preset ribbon keeps: the original's ten, or HD/Fury's 54.
//!
//! The longer ribbon is an enhancement of this project's, not the original's. It keeps what the
//! preset authors - the head width, the colour fade's curve, the taper's curve and the
//! stencil ramp - and stretches each across the whole ring, by reading it at an age scaled to
//! the original's ten ([`Exhaust::shape_age`]). The texture still steps once per sample, so
//! the noise keeps its size along the ribbon. `docs/rendering/trail-ribbon.md`, "A longer ribbon".

use super::{Exhaust, LAYERS, TRAIL_FINS, TRAIL_SAMPLES, hd};

/// The ring the longer ribbon keeps: HD/Fury's, 54 samples, one a frame
/// (`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`).
pub const LONG_TRAIL_SAMPLES: usize = hd::SAMPLES;

/// What an [`Exhaust`] has room for, whichever length it is set to.
pub const TRAIL_CAPACITY: usize = LONG_TRAIL_SAMPLES;

const _: () = assert!(TRAIL_CAPACITY >= TRAIL_SAMPLES);

/// One craft's ribbon at the longest length, with the one segment per layer a frame drawn between
/// ticks adds at the tail.
pub const TRAIL_MAX_VERTICES_PER_CRAFT: usize = LAYERS * TRAIL_CAPACITY * TRAIL_FINS * 6;

impl Exhaust {
    /// Sets how many samples the ring keeps: [`TRAIL_SAMPLES`] or [`LONG_TRAIL_SAMPLES`].
    ///
    /// Idempotent: a frame loop may call it every frame. A change clears the ring, because the
    /// samples already in it were laid down for another length, and the ribbon is drawn again
    /// once the new ring is full.
    pub fn set_trail_samples(&mut self, samples: usize) {
        let samples = samples.clamp(2, TRAIL_CAPACITY);
        if samples != self.trail_cap {
            self.trail_cap = samples;
            self.clear_trail();
        }
    }

    /// How many samples the ring keeps now.
    #[must_use]
    pub fn trail_samples_kept(&self) -> usize {
        self.trail_cap
    }

    /// A sample's age as the preset's laws read it: its real age, scaled so the ring's last
    /// sample lands where the original's does at ten. Exactly the age itself at ten.
    #[must_use]
    pub(super) fn shape_age(&self, age: f32) -> f32 {
        age * ((TRAIL_SAMPLES - 1) as f32 / (self.trail_cap - 1) as f32)
    }

    /// The vertices one ribbon of the length now set can draw, with the extra segment of a
    /// frame between ticks ([`TRAIL_MAX_VERTICES_PER_CRAFT`] at the longest).
    pub(super) fn max_vertices(&self) -> usize {
        LAYERS * self.trail_cap * TRAIL_FINS * 6
    }
}
