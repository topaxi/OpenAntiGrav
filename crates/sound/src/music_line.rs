//! The race-only music control: one line the pad's music-up and music-down actions walk.
//!
//! The line is a count of ticks, `0` being the player's settings. Going up, the music rises
//! to the music slider's maximum ([`SWEEP_TICKS`] ticks), then every other bus is lowered
//! together, balance kept, down to [`OTHERS_FLOOR`] of its level (another [`SWEEP_TICKS`]).
//! Going down walks the same line back, and below `0` lowers the music towards
//! [`MUSIC_FLOOR`]. One integer position, so forward then backward returns exactly where it
//! started.
//!
//! The trims live on the mixer ([`oag_audio::Mixer::set_bus_trim`]), apart from the bus
//! gains the settings and HD's mix write, and nothing here is saved: the next race starts at
//! the player's settings ([`Audio::reset_music_line`]), and so does any settings change
//! ([`Audio::apply`]), whose slider the line is relative to.
//!
//! Output only: nothing here reaches `World`, the state hash or a replay.

use std::sync::atomic::{AtomicI32, Ordering};

use oag_audio::{Bus, GROUPS};

use crate::{Audio, Volume};

/// Ticks for a full sweep of either stage: two seconds at the 60 Hz tick.
pub const SWEEP_TICKS: i32 = 120;

/// The music's lowest multiplier on the player's level, about -20 dB: quiet, never silence.
pub const MUSIC_FLOOR: f32 = 0.1;

/// How far every non-music bus goes down, as a fraction of its level, so engine and weapon
/// cues stay audible.
pub const OTHERS_FLOOR: f32 = 0.25;

/// Where the line stands. Interior mutability because [`Audio::apply`] resets it from `&self`.
#[derive(Debug, Default)]
pub struct Line {
    ticks: AtomicI32,
}

impl Line {
    /// The position, in ticks: `-SWEEP_TICKS` the music floor, `0` the settings,
    /// `SWEEP_TICKS` music at maximum, `2 * SWEEP_TICKS` the others at their floor.
    #[must_use]
    pub fn ticks(&self) -> i32 {
        self.ticks.load(Ordering::Relaxed)
    }

    /// Back to the player's settings.
    pub fn reset(&self) {
        self.ticks.store(0, Ordering::Relaxed);
    }

    /// Moves `delta` ticks (positive is up), stopping at either end. `raisable` is whether
    /// the music slider leaves room above the settings: a muted slider has none, so the line
    /// does not go up.
    pub fn step(&self, delta: i32, raisable: bool) {
        let top = if raisable { 2 * SWEEP_TICKS } else { 0 };
        let next = (self.ticks() + delta).clamp(-SWEEP_TICKS, top);
        self.ticks.store(next, Ordering::Relaxed);
    }

    /// The music's multiplier on the player's level. `ceiling` is the factor that takes the
    /// slider to its maximum (`1 / slider gain`).
    #[must_use]
    pub fn music_trim(&self, ceiling: f32) -> f32 {
        let t = self.ticks();
        if t <= 0 {
            let along = (t + SWEEP_TICKS) as f32 / SWEEP_TICKS as f32;
            MUSIC_FLOOR + (1.0 - MUSIC_FLOOR) * along
        } else {
            let along = t.min(SWEEP_TICKS) as f32 / SWEEP_TICKS as f32;
            1.0 + (ceiling - 1.0) * along
        }
    }

    /// Every other bus's multiplier, one factor so their balance stays.
    #[must_use]
    pub fn others_trim(&self) -> f32 {
        let past = (self.ticks() - SWEEP_TICKS).clamp(0, SWEEP_TICKS);
        1.0 - (1.0 - OTHERS_FLOOR) * past as f32 / SWEEP_TICKS as f32
    }
}

/// The factor that takes `slider` to its maximum; `1.0` for a muted slider.
fn ceiling(slider: Volume) -> f32 {
    let gain = slider.gain();
    if gain > 0.0 { 1.0 / gain } else { 1.0 }
}

impl Audio {
    /// Moves the line `delta` ticks (see [`Line`]) and puts it on the mixer. `music` is the
    /// player's music slider, which the top of the first stage is measured from.
    pub fn step_music_line(&self, delta: i32, music: Volume) {
        self.music_line.step(delta, music.gain() > 0.0);
        self.push_music_line(music);
    }

    /// The line's position in ticks (see [`Line::ticks`]).
    #[must_use]
    pub fn music_line_ticks(&self) -> i32 {
        self.music_line.ticks()
    }

    /// Puts the player's settings back: the line at `0` and every trim at `1`. Called when a
    /// race starts or ends and by [`Audio::apply`].
    pub fn reset_music_line(&self) {
        self.music_line.reset();
        self.push_music_line(Volume::FULL);
    }

    fn push_music_line(&self, music: Volume) {
        let music_trim = self.music_line.music_trim(ceiling(music));
        let others = self.music_line.others_trim();
        self.output.with_mixer(|mixer| {
            mixer.set_bus_trim(Bus::Music, music_trim);
            mixer.set_bus_trim(Bus::Sfx, others);
            mixer.set_bus_trim(Bus::Speech, others);
            for group in 1..=GROUPS {
                mixer.set_bus_trim(Bus::Group(group as u8), others);
            }
        });
    }
}

#[cfg(test)]
mod tests;
