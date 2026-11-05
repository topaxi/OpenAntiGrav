//! The blur rung: the controller's second output, the motion blur's gather
//! resolution, moved down the ladder `full`, `half`, `quarter` at runtime.
//!
//! Measured (`docs/rendering/motion-blur-half-rung.md`), gathering at half
//! resolution saves 45-52 % of the blur chain's cost: about 2.4 ms on a
//! Raphael iGPU at 1280x800 and 8.5-9.1 ms on a Steam Deck at 3840x2160, at no
//! pixel loss on the hull, and judged by the maintainer on one frame per title
//! to look better than a 75 % render scale at full blur. So it is **shed first,
//! before the extent falls, and restored last**, once the extent is back at
//! its ceiling, one step at a time. See
//! [ADR-0075](../../../../docs/architecture/adr/0075-dynamic-resolution-sheds-the-blur-gather-before-pixels.md)
//! and [ADR-0087](../../../../docs/architecture/adr/0087-the-blur-rung-has-a-quarter-step.md).
//!
//! # Why it cannot fight the extent loop
//!
//! A flip moves [`super::Cost::scalable`] by the whole saving in one frame,
//! which the extent loop would chase. These keep the two apart, the same for
//! either step:
//!
//! - **A shed replaces a fall.** It answers exactly the reading the extent
//!   loop would have fallen on, and then the extent loop's own
//!   [`super::COOLDOWN`] runs, so the readings still in flight with the
//!   coarser saving missing never reach a decision.
//! - **A restore happens only at the extent's ceiling, and only when the
//!   restored cost is predicted to land at or under [`super::DEADBAND`]'s
//!   lower edge**, for [`RESTORE_SECONDS`] of frames running. The prediction is
//!   the cost now, minus the gather measured now, plus the gather above it
//!   stands for. Landing at or under 0.80 of the budget cannot shed again,
//!   which needs more than 0.95: the same margin
//!   [`super::Controller::record`]'s rise refusal takes.
//! - **The gather above is measured, twice.** The frame that sheds reads its
//!   cost; the first reading the controller acts on after it (the cooldown
//!   later, at the same scale) reads the coarser gather's. Their ratio is this
//!   machine's saving, and from then on the gather above is predicted as the
//!   gather now over that ratio, so the prediction follows the scene rather
//!   than the moment of the shed. The blur's cost moves a lot with motion: on
//!   a Raphael iGPU at 4K one run read a full gather of 2.3 ms at the start
//!   and 15.3 ms half a minute later. Each step holds its own ratio. A second
//!   shed reads the first step's on its way down, and a quarter gather's is
//!   read only once the strength cap has settled.
//! - **A restore waits out the scene's spikes.** The run a restore needs is a
//!   duration, not a handful of frames: every frame in it must predict a fit,
//!   so it is a maximum over the stretch, and a stretch shorter than the gap
//!   between the scene's spikes predicts a fit that the next spike undoes. A
//!   32-frame run on a Raphael iGPU at 1280x800 aiming at 90 restored between
//!   spikes 1 to 5 seconds apart and flipped 21 times in 90 seconds.
//! - **A restore that does not hold backs off.** A shed within the relapse
//!   window ([`relapse_frames`], the longest backed-off patience) of a restore
//!   doubles the patience the next restore needs, up to [`MAX_BACKOFF`]
//!   doublings, and every window that holds pays one doubling back. A scene
//!   that keeps fooling the prediction costs a bounded number of flips. The
//!   patience and the backoff are one, shared by the steps.
//! - **Quarter stays drawn as the extent falls.** Whether it exists is judged
//!   on the ceiling ([`oag_display::display::QUARTER_MIN_LINES`]), so a fall
//!   never draws a dearer gather than the frame it answers, and the extent
//!   loop reads one cost curve all the way down.
//!
//! **The shed side has no patience of its own, deliberately.** The brief this
//! was built to asked for patience in both directions; a shed that waited
//! would let the extent fall first, which is the order this exists to invert,
//! or would hold the extent over budget and drop frames. A single-frame shed
//! is harmless once a restore needs a spike-free stretch: the patience lives
//! on the restore side, where a wrong decision is a dropped frame.
//!
//! # The player's choice is the ceiling
//!
//! [`super::Limits`] carries the profile's `motion_blur_resolution` the way it
//! carries `render_scale`: as the most this may ask for. At `quarter` there is
//! nothing to shed and the rung is inert; with the blur off it is inert too.
//! At `half` only the second step exists. Like the extent, the answer is
//! re-derived from the limits every frame by
//! [`super::Controller::blur_resolution`] and never written anywhere: this
//! module does not import settings.
//!
//! **One rung, one set of thresholds, whatever the blur's strength.** The
//! maintainer reports half is more visible at high strength than at low in
//! some scenes; a per-strength threshold would be a second table here, keyed
//! on a strength the limits would also have to carry. Not built: it is the
//! seam a follow-up widens.
//!
//! # The strength cap
//!
//! A quarter gather draws at no more shutter than `low`
//! ([`BlurResolution::shutter_cap`]). Strength is a per-frame uniform, so a
//! one-frame step would be visible: the cap eases across
//! [`CAP_EASE_SECONDS`] of the target's frames, out as well as in. It is a cap
//! on the player's strength, applied where the chain is drawn, so it never
//! raises a lower one and never reaches a setting. It eases from `high`, the
//! most any player draws, so a lower strength stops moving as soon as the cap
//! meets it.

use oag_display::display::{BlurResolution, MotionBlur};

use super::{GRID, Limits};

/// How long, in the target's own frames, a restore must be predicted to fit
/// before it is taken, before any backoff.
///
/// **Chosen, not measured**, against one observation: the scene's cost spikes
/// 1 to 5 seconds apart on a Pulse race, and a run shorter than that gap
/// restores between spikes. Counted in frames of the target period (180 at
/// 60, 270 at 90) rather than read off a clock, so the policy stays a pure
/// function of what it is fed; a machine running faster than its target
/// waits a little less wall time, which is the cheap direction.
pub const RESTORE_SECONDS: f32 = 3.0;

/// The most doublings of the restore patience a run of relapses earns: three
/// seconds becomes at most twenty-four. **Chosen.**
pub const MAX_BACKOFF: u32 = 3;

/// How long the strength cap takes to ease in or out, full range. **Chosen**:
/// long enough that a step from `high` to `low` is not one visible frame,
/// short enough that the bleed the cap exists for is over quickly.
pub const CAP_EASE_SECONDS: f32 = 0.25;

/// The gathers in the order they are shed.
const LADDER: [BlurResolution; 3] = [
    BlurResolution::Full,
    BlurResolution::Half,
    BlurResolution::Quarter,
];

/// The most steps there are to shed, from a `full` ceiling.
const STEPS: usize = LADDER.len() - 1;

/// The restore patience at a target `period`, before any backoff.
#[must_use]
pub fn restore_frames(period: f32) -> u32 {
    let frames = (RESTORE_SECONDS / period).ceil();
    if frames.is_finite() && frames >= 1.0 {
        frames as u32
    } else {
        1
    }
}

/// How many frames after a restore a shed counts as the restore not holding:
/// the longest backed-off patience, so a relapse that the longest wait would
/// have seen is never forgiven early.
#[must_use]
pub fn relapse_frames(period: f32) -> u32 {
    restore_frames(period) << MAX_BACKOFF
}

/// The smallest saving ratio, a gather over the one above it, that is
/// believed. Each step reads a quarter of the pixels and keeps every other
/// pass of the chain, so it cannot cost less than a quarter; a lower reading
/// is a scene that changed under the cooldown, and clamping it keeps the
/// prediction from multiplying a noisy reading by more than four.
const MIN_SAVING_RATIO: f32 = 0.25;

/// What one shed step remembers.
#[derive(Debug, Clone, Copy, Default)]
struct Shed {
    /// The gather above's cost at the ceiling, read on the frame that shed it.
    above: f32,
    /// This gather's cost over the gather above's, read on the first frame
    /// after the shed that was not cooled down, or `None` until then.
    ratio: Option<f32>,
}

impl Shed {
    /// Fixes the ratio from `below`, this gather's cost at the ceiling's scale,
    /// if it is not known yet.
    fn learn(&mut self, below: f32) {
        if below > 0.0 && self.ratio.is_none() {
            self.ratio = Some((below / self.above).clamp(MIN_SAVING_RATIO, 1.0));
        }
    }

    /// The gather above's cost at the ceiling, predicted from this gather's
    /// now (`below`, also at the ceiling's scale).
    fn above_now(&mut self, below: f32) -> f32 {
        self.learn(below);
        match self.ratio.filter(|_| below > 0.0) {
            Some(ratio) => below / ratio,
            None => self.above,
        }
    }
}

/// The rung's state inside a [`super::Controller`].
#[derive(Debug, Clone, Default)]
pub(super) struct Rung {
    /// The ceiling `depth` counts down from, so a player's change of it starts
    /// over rather than reinterpreting a depth.
    ceiling: Option<BlurResolution>,
    /// How many steps below the ceiling the gather is held.
    depth: usize,
    /// What each step measured; `sheds[depth - 1]` is the one in force.
    sheds: [Shed; STEPS],
    /// Consecutive frames at the ceiling a restore was predicted to fit.
    run: u32,
    /// Frames since the last restore, or since the last relapse window that
    /// held, while a backoff is still being paid back.
    since_restore: Option<u32>,
    /// Doublings of the restore patience earned by restores that did not hold.
    backoff: u32,
    /// The restore patience at this frame's target, before backoff.
    frames: u32,
    /// How far the strength cap has eased in: 0 none, 1 all the way to `low`.
    capped: f32,
}

/// `resolution`'s place on [`LADDER`].
fn rank(resolution: BlurResolution) -> usize {
    LADDER
        .iter()
        .position(|&step| step == resolution)
        .unwrap_or(0)
}

impl Rung {
    /// Whether this frame's limits leave anything to shed: a target running, a
    /// blur drawn, and the player's own choice above the last gather.
    pub(super) fn eligible(limits: Limits) -> bool {
        limits.target().is_on() && limits.blur().is_some_and(|ceiling| rank(ceiling) < STEPS)
    }

    /// Whether the depth in force reaches the last step of the ladder.
    fn holds_quarter(&self) -> bool {
        self.ceiling
            .is_some_and(|ceiling| rank(ceiling) + self.depth == STEPS)
    }

    /// The step of the ladder the rung holds under `limits`.
    fn held(&self, limits: Limits) -> BlurResolution {
        let ceiling = limits.blur().unwrap_or_default();
        if Self::eligible(limits) {
            LADDER[(rank(ceiling) + self.depth).min(STEPS)]
        } else {
            ceiling
        }
    }

    /// The gather resolution to draw at under `limits`: judged on the
    /// ceiling's lines, so it holds whatever the extent does.
    pub(super) fn resolution(&self, limits: Limits) -> BlurResolution {
        self.held(limits).for_lines(limits.ceiling().1)
    }

    /// The most shutter the blur may draw with this frame, or `None` when the
    /// rung is not running and the drawn gather's own cap applies at once.
    pub(super) fn shutter_cap(&self, limits: Limits) -> Option<f32> {
        Self::eligible(limits).then(|| {
            let (from, to) = (MotionBlur::High.shutter(), MotionBlur::Low.shutter());
            from + (to - from) * self.capped
        })
    }

    /// Whether any step is shed, whatever the limits say.
    pub(super) fn is_shed(&self) -> bool {
        self.depth > 0
    }

    /// The restore patience in force, after backoff.
    fn patience(&self) -> u32 {
        self.frames << self.backoff.min(MAX_BACKOFF)
    }

    /// Called once per recorded frame, before any decision.
    ///
    /// Ineligible limits put the rung back at the player's choice and forget
    /// everything, so a blur switched back on, or a target turned back on,
    /// starts from a clean state rather than from a remembered cost that
    /// belongs to another scene. So does a change of that choice.
    pub(super) fn begin_frame(&mut self, limits: Limits) {
        let Some(period) = limits.target().period().filter(|_| Self::eligible(limits)) else {
            *self = Self::default();
            return;
        };
        if self.ceiling != limits.blur() {
            *self = Self {
                ceiling: limits.blur(),
                ..Self::default()
            };
        }
        // A step the ceiling itself can no longer draw is gone for good, not
        // waiting for the extent to come back.
        while self.depth > 0 && self.held(limits) != self.resolution(limits) {
            self.depth -= 1;
        }
        self.frames = restore_frames(period);
        if let Some(since) = self.since_restore.as_mut() {
            *since += 1;
            if *since >= relapse_frames(period) {
                // A window held: one doubling paid back, and the next window
                // starts counting.
                self.backoff = self.backoff.saturating_sub(1);
                self.since_restore = (self.backoff > 0).then_some(0);
            }
        }
        let wanted = f32::from(self.resolution(limits) == BlurResolution::Quarter);
        let step = period / CAP_EASE_SECONDS;
        self.capped = if self.capped < wanted {
            (self.capped + step).min(wanted)
        } else {
            (self.capped - step).max(wanted)
        };
    }

    /// Whether an over-budget reading should shed one step instead of moving
    /// the extent, and sheds it if so.
    ///
    /// **Only when the blur was measured to cost something.** A reading with
    /// no blur in it would shed for no saving, and the no-oscillation argument
    /// above rests on every shed buying the cost it remembers. The caller
    /// already skips a frame whose blur ran and was not timed, so a zero here
    /// is a blur that did not run; this keeps the rung safe without that.
    ///
    /// **Only to a gather the ceiling allows**, or the shed would buy nothing.
    pub(super) fn try_shed(&mut self, blur: f32, steps: u32, limits: Limits) -> bool {
        if !Self::eligible(limits) || !(blur.is_finite() && blur > 0.0) {
            return false;
        }
        let next = rank(self.held(limits)) + 1;
        if next > STEPS || LADDER[next].for_lines(limits.ceiling().1) != LADDER[next] {
            return false;
        }
        // Cost goes as the pixel count, so a gather measured below the
        // ceiling is scaled up to what it costs there, where a restore is
        // decided. A gather has a floor that does not scale, so this
        // over-estimates, which only makes a restore harder to earn.
        let above = at_ceiling(blur, steps);
        // Shed again before a restore was ever tried, this reading is the
        // first of the step in force, and the only one that step will get.
        if self.depth > 0 {
            self.sheds[self.depth - 1].learn(above);
        }
        self.sheds[self.depth] = Shed { above, ratio: None };
        self.depth += 1;
        self.run = 0;
        // Inside a window: either a restore that did not hold, or one paying
        // back an earlier backoff, which did not hold either.
        if self.since_restore.take().is_some() {
            self.backoff = (self.backoff + 1).min(MAX_BACKOFF);
        }
        true
    }

    /// Counts one frame toward restoring the step in force and takes it once
    /// the run is long enough. Called on every frame past a cooldown; a frame
    /// below the extent's ceiling, or with no budget left, breaks the run.
    ///
    /// **Never on a reading the strength cap is still moving under.** A
    /// quarter gather drawn at `high` is not the gather a settled one is, and
    /// a ratio learned from it would call the restore cheaper than it is.
    pub(super) fn try_restore(
        &mut self,
        blur_now: f32,
        scalable: f32,
        budget: f32,
        steps: u32,
    ) -> bool {
        if self.depth == 0 {
            return false;
        }
        if self.holds_quarter() && self.capped < 1.0 {
            self.run = 0;
            return false;
        }
        let blur_now = if blur_now.is_finite() && blur_now > 0.0 {
            blur_now
        } else {
            0.0
        };
        // Learned on the first frame here whatever the scale, so the ratio is
        // read at the scale the shed was.
        let above = self.sheds[self.depth - 1].above_now(at_ceiling(blur_now, steps));
        let fits = steps == GRID
            && budget > 0.0
            && (scalable - blur_now + above) / budget <= *super::DEADBAND.start();
        if !fits {
            self.run = 0;
            return false;
        }
        self.run += 1;
        if self.run < self.patience() {
            return false;
        }
        self.depth -= 1;
        self.run = 0;
        self.since_restore = Some(0);
        true
    }

    /// A stall throws the run away, as it does the extent's.
    ///
    /// The ratio stays: like the controller's residual it is a property of the
    /// machine and the scene, and a load is the one moment it cannot be read.
    pub(super) fn reset(&mut self) {
        self.run = 0;
    }
}

/// A blur cost measured at `steps` grid points, scaled to what it costs at the
/// ceiling. Cost goes as the pixel count; a gather has a floor that does not
/// scale, so this over-estimates, which only makes a restore harder to earn.
fn at_ceiling(blur: f32, steps: u32) -> f32 {
    let scale = f64::from(GRID) / f64::from(steps.max(1));
    (f64::from(blur) * scale * scale) as f32
}
