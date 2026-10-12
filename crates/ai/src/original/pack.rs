//! The formation an original opponent holds: `AiManager_ShufflePack`
//! (`0x088354c8`) once a race, and `Ai_UpdateSpreadWander` (`0x08852ef4`) at
//! the head of every thrust update. Both measured live (confidence 85 for the
//! functions, 90 for the shuffle reading the integer difficulty):
//! `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`.

use oag_core::Rng;
use oag_tables::ai_race_stats::PackSwapping;

/// One driver's formation slot and its wander (`driver+0x74..+0x98`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wander {
    /// The slot at the start (`+0x78`): `8 - grid position`.
    pub start: f32,
    /// The slot at the finish (`+0x7c`), after the shuffle.
    pub end: f32,
    /// The wander's phase (`+0x88`): the race fraction the current segment ends at.
    pub phase: f32,
    /// Segments a race (`+0x8c`), at least one once shuffled.
    pub segments: f32,
    /// The offset's amplitude (`+0x90`).
    pub amplitude: f32,
    /// The offset the current segment starts from (`+0x94`).
    pub prev: f32,
    /// The offset it ends at (`+0x98`).
    pub next: f32,
}

impl Wander {
    /// What `Ai_Construct` leaves: a flat slot, `8 - grid position` (`1` the
    /// front, read off the live racer table: racer 0 starts at `7`), and a phase
    /// (`1e6`) no race fraction reaches, so a driver the shuffle never touched
    /// holds its grid order. `segments` is not read off `Ai_Construct`; `1` is
    /// chosen so the unused term stays finite (it is multiplied by zero).
    #[must_use]
    pub fn constructed(grid_position: u8) -> Self {
        let slot = 8.0 - f32::from(grid_position);
        Self {
            start: slot,
            end: slot,
            phase: 1.0e6,
            segments: 1.0,
            amplitude: 0.0,
            prev: 0.0,
            next: 0.0,
        }
    }

    /// `Ai_UpdateSpreadWander`: advances the wander to race fraction `f` and
    /// returns the formation slot (`driver+0x74`).
    ///
    /// The draw is the original's `amplitude * (2 * rand() * 2^-31 - 1)` with
    /// `rand()` the seeded generator's top 31 bits.
    pub fn update(&mut self, f: f32, rng: &mut Rng) -> f32 {
        if self.phase <= f {
            self.phase = if self.phase < 0.0 {
                0.0
            } else {
                self.phase + 1.0 / self.segments
            };
            self.prev = self.next;
            self.next = if self.phase < 0.999 {
                let draw = (rng.next_u32() >> 1) as f32;
                self.amplitude * (2.0 * draw * (1.0 / 2_147_483_648.0) - 1.0)
            } else {
                0.0
            };
        }
        let w = (f - (self.phase - 1.0 / self.segments)) * self.segments;
        self.prev * (1.0 - w) + self.next * w + self.start * (1.0 - f) + self.end * f
    }
}

/// The race fraction the wander reads: `(progress - start) / (laps * lap_length)`
/// clamped to `0..=1`, and `0` outside `0 < laps < 20` (the original's guard).
/// `distance` is the progress past the start line.
#[must_use]
pub fn race_fraction(distance: f32, laps: u32, lap_length: f32) -> f32 {
    if laps == 0 || laps >= 20 {
        return 0.0;
    }
    (distance / (laps as f32 * lap_length)).clamp(0.0, 1.0)
}

/// `AiManager_ShufflePack`, once, on the opponents in racer-table order.
///
/// `numAIPositionSwaps` random adjacent transpositions of the end slots, with
/// the row picked by the **integer** difficulty (`g_skill_level`, measured by
/// the parity of the live end permutations), then every wander armed.
pub fn shuffle(opponents: &mut [Wander], row: &PackSwapping, rng: &mut Rng) {
    let n = opponents.len();
    if n >= 2 {
        let swaps = row.num_ai_position_swaps as i32;
        for _ in 0..swaps.max(0) {
            let i = rng.below((n - 1) as u32) as usize;
            let (a, b) = (opponents[i].end, opponents[i + 1].end);
            opponents[i].end = b;
            opponents[i + 1].end = a;
        }
    }
    let segments = (row.num_intermediate_positions as i32).max(1) as f32;
    for wander in opponents {
        wander.phase = 0.0;
        wander.segments = segments;
        wander.amplitude = row.intermediate_offset_size;
    }
}

#[cfg(test)]
mod tests;
