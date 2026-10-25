//! The blur rung: the controller's second output, the motion blur's gather
//! resolution, moved between `full` and `half` at runtime.
//!
//! Measured (`docs/rendering/motion-blur-half-rung.md`), gathering at half
//! resolution saves 45-52 % of the blur chain's cost: about 2.4 ms on a
//! Raphael iGPU at 1280x800 and 8.5-9.1 ms on a Steam Deck at 3840x2160, at no
//! pixel loss on the hull, and judged by the maintainer on one frame per title
//! to look better than a 75 % render scale at full blur. So it is **shed first,
//! before the extent falls, and restored last**, once the extent is back at
//! its ceiling. See
//! [ADR-0075](../../../../docs/architecture/adr/0075-dynamic-resolution-sheds-the-blur-gather-before-pixels.md).
//!
//! # Why it cannot fight the extent loop
//!
//! A flip moves [`super::Cost::scalable`] by the whole saving in one frame,
//! which the extent loop would chase. Three things keep the two apart:
//!
//! - **A shed replaces a fall.** It answers exactly the reading the extent
//!   loop would have fallen on, and then the extent loop's own
//!   [`super::COOLDOWN`] runs, so the readings still in flight with the full
//!   gather in them never reach a decision.
//! - **A restore happens only at the extent's ceiling, and only when the
//!   restored cost is predicted to land at or under [`super::DEADBAND`]'s
//!   lower edge**, for [`RESTORE_PATIENCE`] frames running. The prediction is
//!   the cost now, minus the half gather measured now, plus the full gather
//!   measured on the frame that shed it. Landing at or under 0.80 of the
//!   budget cannot shed again, which needs more than 0.95: the same margin
//!   [`super::Controller::record`]'s rise refusal takes.
//! - **A restore that does not hold backs off.** The full gather's cost is
//!   remembered from the shed, so it goes stale as the scene changes. A shed
//!   within [`RELAPSE_WINDOW`] frames of a restore doubles the patience the
//!   next restore needs, up to [`MAX_BACKOFF`] doublings, so a stale estimate
//!   costs a bounded number of flips rather than a flip every second.
//!
//! # The player's choice is the ceiling
//!
//! [`super::Limits`] carries the profile's `motion_blur_resolution` the way it
//! carries `render_scale`: as the most this may ask for. At `half` there is
//! nothing to shed and the rung is inert; with the blur off it is inert too.
//! Like the extent, the answer is re-derived from the limits every frame by
//! [`super::Controller::blur_resolution`] and never written anywhere: this
//! module does not import settings.
//!
//! **One rung, one set of thresholds, whatever the blur's strength.** The
//! maintainer reports half is more visible at high strength than at low in
//! some scenes; a per-strength threshold would be a second table here, keyed
//! on a strength the limits would also have to carry. Not built: it is the
//! seam a follow-up widens.

use oag_display::display::BlurResolution;

use super::{GRID, Limits, RISE_PATIENCE};

/// How many frames running a restore must be predicted to fit before it is
/// taken, before any backoff.
///
/// **Four times [`RISE_PATIENCE`], chosen, not measured.** Slower than the
/// extent loop's rise on purpose: a restore is the larger of the two jumps
/// (half the blur chain at once against one grid step), and its prediction
/// rests on a cost remembered from the shed rather than read this frame.
pub const RESTORE_PATIENCE: u32 = 4 * RISE_PATIENCE;

/// How many frames after a restore a shed counts as the restore not holding.
/// Five seconds at 60. **Chosen, not measured.**
pub const RELAPSE_WINDOW: u32 = 300;

/// The most doublings of [`RESTORE_PATIENCE`] a run of relapses earns: 32
/// frames becomes at most 1,024, about seventeen seconds at 60. **Chosen.**
pub const MAX_BACKOFF: u32 = 5;

/// The rung's state inside a [`super::Controller`].
#[derive(Debug, Clone, Default)]
pub(super) struct Rung {
    /// The full gather's cost at the ceiling, measured on the frame that shed
    /// it, or `None` while the full gather is in force.
    shed: Option<f32>,
    /// Consecutive frames at the ceiling a restore was predicted to fit.
    run: u32,
    /// Frames since the last restore, while it is still inside
    /// [`RELAPSE_WINDOW`].
    since_restore: Option<u32>,
    /// Doublings of [`RESTORE_PATIENCE`] earned by restores that did not hold.
    backoff: u32,
}

impl Rung {
    /// Whether this frame's limits leave anything to shed: a target running, a
    /// blur drawn, and the player's own choice the full gather.
    pub(super) fn eligible(limits: Limits) -> bool {
        limits.target().is_on() && limits.blur() == Some(BlurResolution::Full)
    }

    /// The gather resolution to draw at under `limits`.
    pub(super) fn resolution(&self, limits: Limits) -> BlurResolution {
        if Self::eligible(limits) && self.shed.is_some() {
            BlurResolution::Half
        } else {
            limits.blur().unwrap_or_default()
        }
    }

    /// Whether the half gather is in force, whatever the limits say.
    pub(super) fn is_shed(&self) -> bool {
        self.shed.is_some()
    }

    /// The restore patience in force, after backoff.
    pub(super) fn patience(&self) -> u32 {
        RESTORE_PATIENCE << self.backoff.min(MAX_BACKOFF)
    }

    /// Called once per recorded frame, before any decision.
    ///
    /// Ineligible limits put the rung back to the full gather and forget
    /// everything, so a blur switched back on, or a target turned back on,
    /// starts from a clean state rather than from a remembered cost that
    /// belongs to another scene.
    pub(super) fn begin_frame(&mut self, limits: Limits) {
        if !Self::eligible(limits) {
            *self = Self::default();
            return;
        }
        if let Some(since) = self.since_restore.as_mut() {
            *since += 1;
            if *since >= RELAPSE_WINDOW {
                // The restore held: it has paid back what the backoff was for.
                self.since_restore = None;
                self.backoff = 0;
            }
        }
    }

    /// Whether an over-budget reading should shed the full gather instead of
    /// stepping the extent, and sheds it if so.
    ///
    /// **Only when the blur was measured to cost something.** A reading with
    /// no blur in it (`blur` folded to zero by a timer that did not come back)
    /// would shed for no saving, and the no-oscillation argument above rests
    /// on every shed buying the cost it remembers.
    pub(super) fn try_shed(&mut self, blur: f32, steps: u32, limits: Limits) -> bool {
        if !Self::eligible(limits) || self.shed.is_some() || !(blur.is_finite() && blur > 0.0) {
            return false;
        }
        // Cost goes as the pixel count, so a gather measured below the
        // ceiling is scaled up to what it costs there, where a restore is
        // decided. A gather has a floor that does not scale, so this
        // over-estimates, which only makes a restore harder to earn.
        let scale = f64::from(GRID) / f64::from(steps.max(1));
        self.shed = Some((f64::from(blur) * scale * scale) as f32);
        self.run = 0;
        if self.since_restore.take().is_some() {
            self.backoff = (self.backoff + 1).min(MAX_BACKOFF);
        }
        true
    }

    /// Counts one frame toward a restore and takes it once the run is long
    /// enough. `predicted_ratio` is `None` when the frame cannot be judged
    /// (the extent is below its ceiling, or the budget is gone), which breaks
    /// the run.
    pub(super) fn try_restore(
        &mut self,
        blur_now: f32,
        scalable: f32,
        budget: f32,
        at_ceiling: bool,
    ) -> bool {
        let Some(full) = self.shed else {
            return false;
        };
        let blur_now = if blur_now.is_finite() && blur_now > 0.0 {
            blur_now
        } else {
            0.0
        };
        let fits = at_ceiling
            && budget > 0.0
            && (scalable - blur_now + full) / budget <= *super::DEADBAND.start();
        if !fits {
            self.run = 0;
            return false;
        }
        self.run += 1;
        if self.run < self.patience() {
            return false;
        }
        self.shed = None;
        self.run = 0;
        self.since_restore = Some(0);
        true
    }

    /// A stall throws the run away, as it does the extent's.
    pub(super) fn reset(&mut self) {
        self.run = 0;
    }
}
