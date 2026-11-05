//! The blur ramp: the blur's strength as a continuous function of frame rate.
//!
//! The drawn strength is the player's times [`Ramp::factor`], which is 1 at
//! [`TOP_FPS`] and above and falls linearly in frames per second to 0 at
//! [`BOTTOM_FPS`]. Both ends are **chosen, not measured**. The smear stands in
//! for the motion between frames, and a low rate shows a player the same smear
//! held for longer; at 30 this engine draws none.
//!
//! Pure presentation, and pure in the module sense: it is fed frame times and
//! chain costs and reads no clock, no GPU and no settings. Nothing here reaches
//! a simulation crate or a settings file. The caller feeds it only in a window,
//! during a running race, so a capture, a compare pair and a paused frame never
//! see it.
//!
//! It has no target: it runs whether or not dynamic resolution does, which is
//! why it is not part of [`super::Controller`], whose `record` returns early
//! with the target off and only ever hears about frames a GPU timer came back
//! for.
//!
//! # Strength is not free, so the ramp is a feedback loop
//!
//! At 4K on a Raphael iGPU going from `high` to `low` saved 5.3 ms of 8.5
//! (`docs/rendering/motion-blur-half-rung.md`, "The quarter step"). So every
//! point of the ramp feeds back: less strength, a cheaper frame, a higher rate,
//! more strength. That is negative feedback, so there is one fixed point, and
//! what could go wrong is reaching it by swinging around it.
//!
//! Write `T` for the frame time and `k` for the seconds a whole unit of factor
//! costs. The ramp's slope against `T` is `1 / (30 T²)`, so the loop gain is
//! `g = k / (30 T²)`. A chain cannot cost more than the frame it is in, so
//! `k <= T`, and `T >= 1/60` inside the ramp, which gives `g <= 1 / (30 T) <= 2`
//! on any machine, however bad. [`SMOOTHING`] follows from that bound.
//!
//! # A hitch is not a frame rate
//!
//! A plain average would carry a 100 ms frame: with [`SMOOTHING`] it adds 2.6 ms
//! to a 16.7 ms average, which takes a quarter of the factor at 60 fps away. So
//! the smoothing reads the median of the last [`WINDOW`] frames, which is the
//! same number with or without a hitch of up to three frames, and only then
//! averages.
//!
//! # Factor 0 draws nothing, and has to earn its way back
//!
//! The chain has a cost that does not scale with strength, so the factor has a
//! step in it the fade cannot hide: at 0 the chain is not drawn at all, which
//! gives its whole cost back, and the rate jumps. If that alone brought the
//! blur back, 30 fps would flip with about 40 for ever.
//!
//! So a return is the blur rung's restore, on one fixed period: it needs the
//! frame predicted to land inside the top of [`super::DEADBAND`] of the ramp's
//! bottom with the chain's last measured cost added, for
//! [`super::RESTORE_SECONDS`] of frames running, and a return that does not hold
//! doubles that patience. It comes back at the gather it
//! left at, because the rung holds still while nothing is drawn. The factor
//! comes back from about 0, not from what the cheap frames read, so the fade is
//! continuous in that direction too.
//!
//! # It does not starve the blur rung
//!
//! The rung refuses a reading the quarter step's strength cap is moving under,
//! because the cap crosses its range in a quarter of a second. It does not
//! refuse for the ramp: the ramp would then hold it off for ever. It does not
//! need to. Across the rung's whole read, the [`super::COOLDOWN`] of four frames
//! between a shed and the next reading, the ramp's smoothed frame time moves at
//! most 12 % of the way to where it is going (`4 * SMOOTHING`), so a ratio read
//! there is a ratio at one strength.

use super::DEADBAND;
use super::blur::Patience;

/// The frame rate at and above which the factor is 1. **Chosen, not measured.**
pub const TOP_FPS: f32 = 60.0;

/// The frame rate at and below which the factor is 0. **Chosen, not measured.**
pub const BOTTOM_FPS: f32 = 30.0;

const TOP_PERIOD: f32 = 1.0 / TOP_FPS;
const BOTTOM_PERIOD: f32 = 1.0 / BOTTOM_FPS;

/// How many frames the median runs over. Odd, so a median is a frame that
/// happened. Seven rejects a hitch of up to three frames whole, and delays the
/// signal by three.
const WINDOW: usize = 7;

/// How far the smoothed frame time moves toward the median, per frame.
///
/// **Derived from the loop, not tuned.** With [`WINDOW`]'s three frames of delay
/// and a GPU timer's one to two, the loop has `D = 5` frames of delay. A
/// delayed average settles without ringing while `SMOOTHING * g * D <= 1/e`,
/// and the module docs bound `g` by 2, so `SMOOTHING <= 0.037` on any machine.
/// One in 32 meets that, and takes about half a second at 60 fps to cross most
/// of its range and a second at 30, which is a fade rather than a step.
/// `a_fixed_cost_model_settles_at_every_rate` holds the bound over a sweep.
pub const SMOOTHING: f32 = 1.0 / 32.0;

/// The frame time the factor takes up from when the blur returns: just under
/// the bottom, so the chain draws and can be measured, and the strength starts
/// from about nothing.
const RETURN_PERIOD: f32 = BOTTOM_PERIOD * (1.0 - SMOOTHING);

/// How close the smoothed frame time counts as there, in seconds: a
/// microsecond, a thousandth of a percent of the factor at the top.
const ARRIVED: f32 = 1e-6;

/// The ramp's state. One per session; see the module docs.
#[derive(Debug, Clone, Default)]
pub struct Ramp {
    /// The last [`WINDOW`] frame times.
    recent: [f32; WINDOW],
    /// How many frames [`Self::recent`] has taken since it was last cleared.
    seen: usize,
    /// The smoothed frame time, or `None` until a whole window has been read.
    smoothed: Option<f32>,
    /// Whether the chain is not being drawn.
    off: bool,
    /// The chain's smoothed cost, scaled to the extent's ceiling.
    chain: Option<f32>,
    /// What a return from zero has to earn.
    patience: Patience,
}

impl Ramp {
    /// What the player's strength is multiplied by this frame.
    #[must_use]
    pub fn factor(&self) -> f32 {
        if self.off {
            return 0.0;
        }
        let Some(period) = self.smoothed else {
            return 1.0;
        };
        if period <= TOP_PERIOD {
            return 1.0;
        }
        ((period.recip() - BOTTOM_FPS) / (TOP_FPS - BOTTOM_FPS)).clamp(0.0, 1.0)
    }

    /// Whether the chain is held off. For a log line.
    #[must_use]
    pub fn is_off(&self) -> bool {
        self.off
    }

    /// Records the chain's measured cost at `scale` of the extent's ceiling.
    /// Called with what the chain cost whenever it ran and was timed; a return
    /// from zero predicts from this.
    pub fn chain(&mut self, seconds: f32, scale: f32) {
        if !(seconds.is_finite() && seconds > 0.0 && scale > 0.0) {
            return;
        }
        let ceiling = seconds / (scale * scale);
        self.chain = Some(
            self.chain
                .map_or(ceiling, |chain| chain + (ceiling - chain) * SMOOTHING),
        );
    }

    /// Records one running frame, `scale` being the extent's now, and says
    /// whether the chain went off or came back on it.
    ///
    /// `None` is a frame with no reading, a stall, which leaves everything
    /// where it was.
    pub fn frame(&mut self, frame_seconds: Option<f32>, scale: f32) -> bool {
        let Some(seconds) = frame_seconds.filter(|s| s.is_finite() && *s > 0.0) else {
            return false;
        };
        self.recent[self.seen % WINDOW] = seconds;
        self.seen += 1;
        if self.seen < WINDOW {
            return false;
        }
        // Held at the top of the ramp, so a fast machine stays exactly at 1
        // and starts falling the moment it drops under 60, rather than having
        // to average its way up from 5 ms first.
        let held = median(self.recent).max(TOP_PERIOD);
        let smoothed = self.smoothed.map_or(held, |smoothed| {
            let next = smoothed + (held - smoothed) * SMOOTHING;
            // An average only ever nears its input: arrive, so a machine back
            // over 60 reads exactly 1.
            if (next - held).abs() < ARRIVED {
                held
            } else {
                next
            }
        });
        self.smoothed = Some(smoothed);
        self.patience.tick(smoothed);
        if !self.off {
            if smoothed < BOTTOM_PERIOD {
                return false;
            }
            self.off = true;
            self.patience.shed();
            return true;
        }
        let predicted = smoothed + self.chain.unwrap_or(0.0) * scale * scale;
        if !self
            .patience
            .earned(predicted <= BOTTOM_PERIOD * DEADBAND.end())
        {
            return false;
        }
        self.off = false;
        self.smoothed = Some(RETURN_PERIOD);
        // The readings in the window are of frames with no chain in them.
        self.seen = 0;
        true
    }

    /// A stall throws the readings in flight away, as the controller's does.
    /// The smoothed rate and the chain's cost stay: a load says nothing about
    /// either.
    pub fn reset(&mut self) {
        self.seen = 0;
        self.patience.interrupt();
    }
}

/// The median of an odd number of frame times.
fn median(mut frames: [f32; WINDOW]) -> f32 {
    frames.sort_unstable_by(f32::total_cmp);
    frames[WINDOW / 2]
}
