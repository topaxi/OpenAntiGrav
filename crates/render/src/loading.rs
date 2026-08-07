//! The loading screen's procedural wave.
//!
//! Reproduces `Loading_DrawWave` (`0x0890a8e4`), documented at
//! `docs/ghidra/functions/psp-pulse-usa/loading-screen.md`. There is no loading
//! *movie* in either build - the band is generated every frame, and the only
//! asset it touches is one 32x32 cyan glow strip
//! (`data/defaults/loading/LoadingPulseOverlay.mip`). That is why this can be
//! drawn while the thing being loaded does not exist yet, which is the whole
//! reason it is worth having.
//!
//! # The randomness is spatial, not temporal
//!
//! The original zeroes its six-float state array at the top of every call and
//! walks it along **screen X**, not along time. One frame's wave is therefore a
//! single random walk read left to right: pinned flat at the left edge, fully
//! developed at the right. Re-running it with fresh draws each frame is what
//! makes it wriggle. Getting this backwards - integrating over time instead -
//! produces a wave that looks superficially similar and behaves nothing like
//! the original, so it is the first thing the tests below pin.
//!
//! Only three things survive a frame: the envelope phase, the finished flag,
//! and the caller's [`Rng`].
//!
//! # Resolution independence is a deliberate divergence
//!
//! The original is written in PSP pixels: 240 columns of 2x32 quads, baseline
//! at y=220 on a 272-line display, and two X ramps with bounds `(10, 350)` and
//! `(30, 286)`. **The PS2 port kept every one of those numbers on a 640-wide
//! screen**, so its wave spans 480 of 640 pixels and its ramps sit in the wrong
//! place - a real bug in the original, tabulated at
//! `docs/ghidra/functions/ps2-pulse-eu/loading-screen.md`. We do not reproduce
//! it. Everything here is computed in the PSP's 480x272 reference space and
//! mapped to the target framebuffer at the end, in the same category of
//! deliberate divergence as [ADR-0007](../../../docs/architecture/adr/0007-fixed-timestep-vs-original.md)'s
//! fixed timestep.
//!
//! **The column count is not a resolution parameter.** [`COLUMNS`] is the random
//! walk's step count, so it is part of the waveform's identity: adding columns
//! on a wider screen makes the walk finer and visibly changes the wave's
//! character. The quad *width* scales; the count does not.

use oag_core::rng::Rng;

/// Reference display width, in the pixels every constant here is written in.
pub const REF_WIDTH: f32 = 480.0;

/// Reference display height.
pub const REF_HEIGHT: f32 = 272.0;

/// Columns across the strip.
///
/// The original steps screen X by 2 from 0 to 480. This is the random walk's
/// step count as much as it is a geometry figure - see the module doc comment
/// on why it does not scale with resolution.
pub const COLUMNS: usize = 240;

/// Bands drawn per column: the two oscillator layers, and the slew-limited
/// blend of them.
pub const BANDS: usize = 3;

/// Where the strip sits, in reference pixels from the top.
pub const BASELINE_Y: f32 = 220.0;

/// Quad height, and the glow strip's own size.
pub const STRIP_SIZE: f32 = 32.0;

/// The heartbeat.
///
/// 24 floats straight out of `.rodata` at `0x08a88034`, byte-identical in the
/// EU and PS2 builds. Two peaks of 99 with a shallow trough between them, a
/// decay tail, then six frames of silence. At the loading thread's 30 Hz that
/// is one beat every 0.8 s, and it is why the game is called Pulse.
pub const ENVELOPE: [f32; 24] = [
    0.0, 0.0, 10.0, 40.0, 70.0, 99.0, 70.0, 40.0, 10.0, 40.0, 70.0, 99.0, 70.0, 40.0, 30.0, 20.0,
    10.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

/// The envelope's own maximum, which is what it is divided by.
pub const ENVELOPE_PEAK: f32 = 99.0;

/// Amplitude floor. The wave idles at a tenth rather than going fully flat.
pub const ENVELOPE_FLOOR: f32 = 0.1;

/// Tracking rates for the two oscillator layers, from `g_loading_wave_rates`.
pub const LAYER_RATES: [f32; 2] = [0.1, 0.05];

/// Per-column damping applied to each layer's energy.
pub const ENERGY_DAMPING: f32 = 0.985;

/// Full width of the per-column random impulse: uniform in `+/-7.5`.
pub const ENERGY_IMPULSE: f32 = 15.0;

/// Most the slew-limited band moves in one column.
pub const SLEW_STEP: f32 = 0.5;

/// How far the slew-limited band must lag before it moves at all.
pub const SLEW_DEADBAND: f32 = 4.0;

/// Weights blending the two layers into the third band.
pub const BLEND_WEIGHTS: [f32; 2] = [0.7, 0.3];

/// Low bound of the amplitude ramp across X, in reference pixels.
pub const AMPLITUDE_RAMP: (i32, i32) = (30, 286);

/// Low and high bounds of the alpha ramp across X, in reference pixels.
///
/// Tints `0xff000000` to `0xff808080` in the original.
pub const ALPHA_RAMP: (i32, i32) = (10, 350);

/// A clamped integer ramp, `((t - lo) * 255) / (hi - lo)`.
///
/// `Loading_Ramp255` (`0x0890a280`). Integer throughout, including the divide,
/// because the original is - a float version drifts by up to one step and there
/// is no reason to introduce that.
#[must_use]
pub fn ramp255(lo: i32, hi: i32, t: i32) -> i32 {
    if hi <= lo {
        return 255;
    }
    (((t - lo) * 255) / (hi - lo)).clamp(0, 255)
}

/// Moves `current` toward `target`, but only once the gap is worth moving for.
///
/// `Loading_SlewToward` (`0x0890a3e8`). The deadband is what makes the third
/// band lag visibly rather than tracking: it does not move at all until it is
/// more than `deadband` behind, and then by at most `step`.
#[must_use]
pub fn slew_toward(current: f32, target: f32, step: f32, deadband: f32) -> f32 {
    let gap = target - current;
    if gap.abs() <= deadband {
        return current;
    }
    current + if gap > 0.0 { step } else { -step }
}

/// The texture column this screen column samples.
///
/// `u = x & 0x3f; if u > 0x1f { u = 0x3f - u }` - a triangle wave, so the strip
/// is mirror-tiled with period 64 and its scanlines never show a seam.
///
/// **How wide a slice each column takes is not pinned.** The original sets up a
/// 32x32 source against a 2x32 destination and then passes `u` per column,
/// which is over-determined as written; the recovered geometry carries
/// confidence 88 for this reason. One texel per column is taken here because it
/// is the only reading that stays inside the strip - `u` reaches 31 at `x = 32`,
/// so a two-texel slice would sample past the right edge. Worth settling with a
/// runtime capture, which that page already lists as its missing step.
#[must_use]
pub fn texture_column(x: i32) -> i32 {
    let u = x & 0x3f;
    if u > 0x1f { 0x3f - u } else { u }
}

/// One column's three band offsets, in reference pixels from [`BASELINE_Y`].
pub type Column = [f32; BANDS];

/// One quad, in normalised screen space: origin top left, `0..1` on both axes.
///
/// `u0`/`u1` are normalised texture coordinates into the 32x32 glow strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
    /// Left texture coordinate.
    pub u0: f32,
    /// Right texture coordinate.
    pub u1: f32,
    /// Alpha from the across-screen ramp, `0..1`.
    pub alpha: f32,
}

/// Everything about the wave that outlives a frame.
///
/// Which is very little, deliberately - see the module doc comment. The state
/// the wave *looks* like it should carry is rebuilt from scratch every frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Wave {
    phase: u32,
    finished: bool,
}

impl Wave {
    /// A wave at the start of its first beat.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the heartbeat by one frame.
    ///
    /// Does nothing once [`Wave::finish`] has been called: the original gates
    /// both the phase and the motion on `g_loading_finished`, so the wave
    /// freezes in place while the screen fades out rather than continuing to
    /// animate under the fade.
    pub fn advance(&mut self) {
        if !self.finished {
            self.phase = (self.phase + 1) % ENVELOPE.len() as u32;
        }
    }

    /// Freezes the wave, for when whatever was loading has finished.
    pub fn finish(&mut self) {
        self.finished = true;
    }

    /// Whether the wave is frozen.
    #[must_use]
    pub fn is_finished(self) -> bool {
        self.finished
    }

    /// Index into [`ENVELOPE`].
    #[must_use]
    pub fn phase(self) -> u32 {
        self.phase
    }

    /// This frame's envelope value, before the floor is applied.
    #[must_use]
    pub fn pulse(self) -> f32 {
        ENVELOPE[self.phase as usize % ENVELOPE.len()]
    }

    /// Runs the walk across X and returns every column's band offsets, in
    /// reference pixels.
    ///
    /// Draws from `rng` twice per column, which is the only reason two calls
    /// with the same [`Wave`] differ. A frozen wave still walks - the original
    /// keeps drawing the band it froze - but the caller stops advancing it.
    #[must_use]
    pub fn columns(&self, rng: &mut Rng) -> Vec<Column> {
        // Zeroed here, once per call, exactly as the original zeroes its stack
        // array. These are carried along X within this frame and thrown away
        // at the end of it.
        let mut raw = [0.0f32; 2];
        let mut display = [0.0f32; 2];
        let mut energy = [0.0f32; 2];

        let pulse = self.pulse() / ENVELOPE_PEAK + ENVELOPE_FLOOR;
        let mut out = Vec::with_capacity(COLUMNS);

        for column in 0..COLUMNS {
            let x = column as i32 * 2;
            let envelope = ramp255(AMPLITUDE_RAMP.0, AMPLITUDE_RAMP.1, x) as f32 / 255.0;

            for layer in 0..2 {
                // The target is read from *this* column's energy, before the
                // impulse below updates it. That ordering is what leaves the
                // first column flat, and it is visible in the original as the
                // wave being pinned at the left edge.
                let target = energy[layer] * envelope * pulse;
                raw[layer] += (target - raw[layer]) * LAYER_RATES[layer];
                display[layer] = slew_toward(display[layer], raw[layer], SLEW_STEP, SLEW_DEADBAND);
                let impulse = (rng.next_f32() - 0.5) * ENERGY_IMPULSE;
                energy[layer] = (energy[layer] + impulse) * ENERGY_DAMPING;
            }

            // The third band is a separate, calmer one built from the
            // slew-limited pair, not a filter applied to the other two.
            let blended = display[0] * BLEND_WEIGHTS[0] + display[1] * BLEND_WEIGHTS[1];
            out.push([raw[0], raw[1], blended]);
        }

        out
    }

    /// Lays the wave out as quads in normalised screen space.
    ///
    /// Takes no framebuffer size, which *is* the divergence: every figure is a
    /// fraction of the reference display, so the band spans the full width and
    /// sits at the same fraction of the height at any resolution, and there is
    /// no size for a caller to get wrong. The original works in PSP pixels, and
    /// its PS2 port shipping those same pixels on a 640-wide screen is the bug
    /// this avoids by construction.
    #[must_use]
    pub fn quads(&self, columns: &[Column]) -> Vec<Quad> {
        let column_w = 1.0 / COLUMNS as f32;
        let strip_h = STRIP_SIZE / REF_HEIGHT;
        let mut out = Vec::with_capacity(columns.len() * BANDS);

        for (index, bands) in columns.iter().enumerate() {
            let x = index as i32 * 2;
            let u = texture_column(x) as f32;
            let alpha = ramp255(ALPHA_RAMP.0, ALPHA_RAMP.1, x) as f32 / 255.0;

            for offset in bands {
                // The quad is anchored by its centre line, the way a 32-tall
                // strip drawn at a baseline is.
                let y = (BASELINE_Y + offset) / REF_HEIGHT - strip_h * 0.5;
                out.push(Quad {
                    x: index as f32 * column_w,
                    y,
                    w: column_w,
                    h: strip_h,
                    u0: u / STRIP_SIZE,
                    u1: (u + 1.0) / STRIP_SIZE,
                    alpha,
                });
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> Rng {
        Rng::new(0x10ad)
    }

    #[test]
    fn the_envelope_is_a_heartbeat() {
        assert_eq!(ENVELOPE.len(), 24);
        let peaks = ENVELOPE.iter().filter(|v| **v == ENVELOPE_PEAK).count();
        assert_eq!(peaks, 2, "two beats, with a trough between them");
        assert_eq!(
            ENVELOPE.iter().cloned().fold(0.0f32, f32::max),
            ENVELOPE_PEAK,
            "the divisor must be the table's own maximum"
        );
        assert!(
            ENVELOPE[18..].iter().all(|v| *v == 0.0),
            "the tail is six frames of silence"
        );
    }

    #[test]
    fn the_ramp_clamps_at_both_ends() {
        assert_eq!(ramp255(30, 286, 0), 0);
        assert_eq!(ramp255(30, 286, 30), 0);
        assert_eq!(ramp255(30, 286, 286), 255);
        assert_eq!(ramp255(30, 286, 480), 255);
        assert_eq!(ramp255(30, 286, 158), 127, "half way is half way");
    }

    #[test]
    fn the_slew_does_not_move_inside_its_deadband() {
        assert_eq!(slew_toward(0.0, 3.9, SLEW_STEP, SLEW_DEADBAND), 0.0);
        assert_eq!(slew_toward(0.0, 4.0, SLEW_STEP, SLEW_DEADBAND), 0.0);
        assert_eq!(slew_toward(0.0, 4.1, SLEW_STEP, SLEW_DEADBAND), 0.5);
        assert_eq!(slew_toward(0.0, -4.1, SLEW_STEP, SLEW_DEADBAND), -0.5);
        assert_eq!(
            slew_toward(0.0, 400.0, SLEW_STEP, SLEW_DEADBAND),
            0.5,
            "a huge gap still moves by at most one step"
        );
    }

    #[test]
    fn the_texture_mirrors_with_period_64() {
        assert_eq!(texture_column(0), 0);
        assert_eq!(texture_column(30), 30);
        assert_eq!(texture_column(32), 31);
        assert_eq!(texture_column(62), 1);
        assert_eq!(texture_column(64), 0, "back to the start after 64");
        for x in 0..512 {
            let u = texture_column(x);
            assert!((0..32).contains(&u), "u={u} is outside the 32-wide strip");
        }
    }

    #[test]
    fn the_wave_is_pinned_flat_at_the_left_edge() {
        let wave = Wave::new();
        let columns = wave.columns(&mut rng());
        assert_eq!(
            columns[0], [0.0; BANDS],
            "the first column reads energy before it is ever kicked"
        );
    }

    #[test]
    fn the_wave_develops_towards_the_right() {
        // The amplitude ramp is zero until x=30 and the walk starts from
        // nothing, so the left third must be calmer than the right third.
        let wave = Wave {
            phase: 5,
            finished: false,
        };
        let columns = wave.columns(&mut rng());
        let spread = |slice: &[Column]| slice.iter().map(|c| c[0].abs()).fold(0.0f32, f32::max);
        let left = spread(&columns[..COLUMNS / 3]);
        let right = spread(&columns[COLUMNS * 2 / 3..]);
        assert!(
            right > left,
            "left {left} should be calmer than right {right}"
        );
    }

    #[test]
    fn the_walk_is_spatial_so_a_frame_is_not_an_integration_of_the_last() {
        // The tell: with the same draws, the wave is identical. If any state
        // were carried across frames this would differ.
        let wave = Wave::new();
        let a = wave.columns(&mut rng());
        let b = wave.columns(&mut rng());
        assert_eq!(
            a, b,
            "nothing but the rng and the phase may survive a frame"
        );
    }

    #[test]
    fn fresh_draws_make_it_wriggle() {
        let wave = Wave::new();
        let mut shared = rng();
        let a = wave.columns(&mut shared);
        let b = wave.columns(&mut shared);
        assert_ne!(a, b, "consecutive frames must not be identical");
    }

    #[test]
    fn the_heartbeat_wraps_at_twenty_four() {
        let mut wave = Wave::new();
        for _ in 0..ENVELOPE.len() {
            wave.advance();
        }
        assert_eq!(wave.phase(), 0, "one beat is exactly the table's length");
    }

    #[test]
    fn a_finished_wave_freezes_rather_than_fading_on() {
        let mut wave = Wave::new();
        wave.advance();
        let frozen = wave.phase();
        wave.finish();
        for _ in 0..50 {
            wave.advance();
        }
        assert_eq!(wave.phase(), frozen);
        assert!(wave.is_finished());
    }

    #[test]
    fn the_amplitude_never_falls_to_nothing_between_beats() {
        // ENVELOPE[0] is zero, but the floor keeps the band alive.
        let wave = Wave::new();
        assert_eq!(wave.pulse(), 0.0);
        let idle = wave.pulse() / ENVELOPE_PEAK + ENVELOPE_FLOOR;
        assert_eq!(idle, ENVELOPE_FLOOR, "it idles at a tenth, not at zero");
    }

    #[test]
    fn the_band_spans_the_full_width_whatever_it_is_drawn_into() {
        // The PS2 port's bug, stated as a test: 240 columns of 2 px is 480 px,
        // which covers 640 only three quarters of the way across. Normalised
        // output cannot express that failure.
        let wave = Wave::new();
        let columns = wave.columns(&mut rng());
        let quads = wave.quads(&columns);
        assert_eq!(quads.len(), COLUMNS * BANDS);
        assert_eq!(
            quads.first().expect("a quad").x,
            0.0,
            "the band must start at the left edge"
        );
        let right = quads.last().expect("a quad");
        assert!(
            (right.x + right.w - 1.0).abs() < 1e-5,
            "the band must reach the right edge, got {}",
            right.x + right.w
        );
        for quad in &quads {
            assert!(
                (0.0..=1.0).contains(&quad.u0) && (0.0..=1.0).contains(&quad.u1),
                "texture coordinates must stay inside the strip"
            );
        }
    }

    #[test]
    fn the_baseline_sits_at_the_same_fraction_of_the_height() {
        let wave = Wave::new();
        let columns = wave.columns(&mut rng());
        let quads = wave.quads(&columns);
        // Column 0 is pinned flat, so its band is exactly on the baseline.
        let expected = BASELINE_Y / REF_HEIGHT - (STRIP_SIZE / REF_HEIGHT) * 0.5;
        assert!((quads[0].y - expected).abs() < 1e-6);
    }
}
