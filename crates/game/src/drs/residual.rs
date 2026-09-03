//! The residual: what a frame spends on work no timer in this build reaches,
//! learned from the frames that can prove it rather than assumed forever.
//!
//! [`super::RESIDUAL_SHARE`] is a number somebody measured on one machine, and
//! the budget subtracted it on every other machine too. Reported from play on
//! a Steam Deck (AMD, RADV) at a 90 Hz target: the machine holds 90 at the
//! full render scale with no reconstruction running, and the controller
//! shrank it to 60-65 % and settled there. Nothing was broken - the loop was
//! live, the readings were real, and 0.20 of an 11.1 ms frame is 2.2 ms this
//! particular machine does not spend. A constant cannot be right on two
//! machines, so this makes it a starting point instead of an answer.
//!
//! # The reading, and why it is an upper bound rather than a measurement
//!
//! `frame - (scene + blur + bloom + fsr3)` is already computed every frame for
//! the `dev` overlay's `OTHER` row - see [`crate::perf::GpuCost::residual_ms`].
//! It is the real per-machine number, and it is **contaminated by sleep**: a
//! paced loop sleeps to its vsync or its frame limit whenever there is
//! headroom, so a comfortable frame's wall clock reads as the target period
//! however little work it did. That is the whole of
//! [ADR-0040](../../../../docs/architecture/adr/0040-the-dynamic-resolution-budget-is-a-share-of-a-frame.md),
//! and feeding this reading in raw would reproduce the failure it rejects.
//!
//! What survives the contamination is an inequality. Sleep is never negative,
//! so
//!
//! ```text
//! reading = frame - timed = residual + sleep >= residual
//! ```
//!
//! **every reading is an upper bound on the truth**, on every frame, paced or
//! not. That is what makes a learned estimate safe: an observation can never
//! claim the frame spends *less* than it does, only less than the last guess
//! did.
//!
//! # The gate, which is the no-slack condition solved for the reading
//!
//! Under pacing a frame's wall clock is the target period, so with `E` the
//! estimate in force and `B = period - fixed - E` the budget the controller
//! divides by:
//!
//! ```text
//! reading <= E  <=>  period - fixed - scalable <= E  <=>  scalable >= B
//! ```
//!
//! **A reading at or below the estimate in force is exactly a frame whose
//! scalable cost had already filled its budget** - a ratio of 1.0 or more, and
//! no slack left for the loop to have slept away. The gate is not a threshold
//! somebody picked; it is the condition ADR-0040 names, rearranged.
//!
//! The rearrangement also rules out the obvious wrong gate. Accepting readings
//! at [`super::DEADBAND`]'s lower edge instead - the ratio the controller
//! settles at - folds in `reading = (1 - r)(period - fixed) + rE` for
//! `r` in 0.80..=0.95, which is *above* `E` for every `E` below
//! `period - fixed`. Its fixed point is `E = period - fixed`: a budget of
//! zero, a scale at the floor, and a loop that sleeps more the harder it
//! shrinks. A learning rule that runs away in the direction the controller
//! already errs in is worse than the constant it replaces.
//!
//! # Two directions, and only one of them is free
//!
//! **Down** needs no further evidence: the gate above already proves the
//! reading is a tighter upper bound than the estimate in force, and every
//! reading is bounded below by the truth, so the estimate converges on the
//! machine's real residual from above and cannot pass it.
//!
//! **Up** does. A reading larger than the estimate is either real untimed work
//! (the AI/physics cost for a full grid that
//! [ADR-0043](../../../../docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md)
//! measured at ~2.4 ms of a ~8.7 ms frame and could not time) or it is sleep.
//! The one thing that tells them apart without a clock in this module is
//! whether the loop was keeping its own pace at all, which is what [`OVERRUN`]
//! asks: a frame that ran past the target period by a margin is a frame the
//! loop did not sleep through.
//!
//! # What it cannot do, stated where the code is
//!
//! **A loop sleeping to the very rate the controller is aiming at destroys the
//! evidence before this module sees it**, and no rearrangement of the gate
//! gets it back. Vsync on at 90 Hz against a 90 Hz target is that case: once
//! the controller has bought back enough slack to be comfortable, every
//! reading is inflated by the sleep it just created, the gate refuses all of
//! them, and the estimate stops where the last tight frame left it. The
//! modelled machine in `drs::tests::machines` gets two grid steps of scale
//! back that way and not the four the truth would allow.
//!
//! Under a frame limiter above the target - the shipped default is 240 - the
//! same machine converges on the truth exactly, because the loop's own pace is
//! shorter than the budget's period and the work is what is left on the clock.
//! Getting the vsync case the rest of the way needs the untimed work timed,
//! which is a fifth ring and a CPU-side one, not a better inference from this
//! one.
//!
//! That test has one hole, and [`super::RESIDUAL_SHARE`] is what plugs it. A
//! display refreshing *below* the target - vsync on at 60 Hz against a 90 Hz
//! target - makes every frame overrun the period while still sleeping, and
//! this build cannot ask a surface what its refresh is (see
//! [`super::Target::at_most`], which clamps to a frame limiter for the same
//! reason and cannot clamp to a panel). So the learned estimate is capped at
//! the constant: **`RESIDUAL_SHARE` stops being the reserve and becomes the
//! most that may ever be reserved.** In that configuration the budget is the
//! one this build already shipped, and everywhere else it can only be larger.

use super::RESIDUAL_SHARE;

/// How much of the gap to a new reading one observation closes.
///
/// **A quarter, and the number is empirical.** The window a sample can arrive
/// in is short and the controller is what closes it: a machine over budget
/// falls in a handful of steps, and every step it takes buys back slack that
/// the next frame's reading then hides in. Against the model in
/// `the_reported_machines_shape_keeps_more_pixels_than_the_constant_leaves_it`
/// a twentieth reaches 2.10 ms of a 2.22 ms reserve before the evidence dries
/// up, and one grid step of scale; a quarter reaches 1.61 ms and two steps. Under a frame limiter, where the evidence does not dry up,
/// both converge on the truth and the rate only decides how fast.
///
/// The risk a faster rate carries is the reverse one, and it is bounded by the
/// same two gates: a reading paired with a GPU timestamp a frame older than
/// itself can read low while the cost is moving, and an estimate that
/// tightened too far leaves the budget claiming room the machine does not
/// have. That shows up as frames past the period, which is exactly what
/// [`OVERRUN`] admits - so the pair converges back rather than sitting wrong,
/// at a cost of a few frames rather than a session.
pub const LEARN_RATE: f32 = 0.25;

/// How far past the target period a frame has to run before its reading may
/// *raise* the estimate, as a multiple of the period.
///
/// 5 %, which is jitter's width rather than a preference: a paced loop lands a
/// hair either side of its deadline every frame - `Session::schedule_next_frame`
/// exists because a timer wakes at or after its deadline and never before -
/// and a frame one wake-up late is not evidence of untimed work. A frame 5 %
/// late is the loop failing to keep the pace, and a loop that is behind its
/// own target is not asleep.
pub const OVERRUN: f32 = 1.05;

/// What a frame spends outside every timer this build has, in seconds.
///
/// **In memory, for this session, and structurally unable to be anything
/// else.** No `Serialize`, no `Deserialize`, and nothing in [`super`] imports
/// [`crate::settings`] - the same argument [`super::Limits`] is passed by
/// value for. A player's settings file records what they chose; this records
/// what their machine is doing this run, which is not a preference and must
/// not be allowed to become one. It starts over on every boot, and a boot is
/// exactly the granularity at which the answer can change: a different
/// adapter, a different driver, a different display.
///
/// Absent until something has been learned, rather than seeded with the
/// constant, so [`Self::learned`] can say which of the two a log line is
/// looking at.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Residual {
    /// The estimate in seconds, or `None` while [`RESIDUAL_SHARE`] still
    /// stands unamended.
    learned: Option<f32>,
}

impl Residual {
    /// An estimate that has seen nothing, and so is the constant.
    #[must_use]
    pub fn new() -> Self {
        Self { learned: None }
    }

    /// What to reserve out of a frame of `period` seconds.
    ///
    /// `RESIDUAL_SHARE * period` until a reading has moved it, and never more
    /// than that afterwards. **The cap is applied here rather than when the
    /// estimate is written**, because the period is not a property of the
    /// machine: a player who changes `target_fps` mid-session changes what
    /// share of a frame the same number of milliseconds is, and the reserve
    /// they get should be the smaller of what was learned and what the
    /// constant allows *at the target in force now*.
    #[must_use]
    pub fn seconds(self, period: f32) -> f32 {
        let prior = RESIDUAL_SHARE * period.max(0.0);
        match self.learned {
            Some(learned) if learned.is_finite() => learned.clamp(0.0, prior),
            _ => prior,
        }
    }

    /// The estimate in seconds, or `None` while nothing has been learned. For
    /// a log line, never for a decision - [`Self::seconds`] is what a budget
    /// asks, because it is the one that knows the target.
    #[must_use]
    pub fn learned(self) -> Option<f32> {
        self.learned
    }

    /// Folds in one frame: how long it took on the wall clock, and how much of
    /// that every timer in this build accounts for.
    ///
    /// **Fed, never read.** The wall clock is the caller's - the composition
    /// root hands over the same `elapsed` it feeds [`crate::perf::Meter`] with,
    /// which is what keeps [`super::Controller`] a pure function of a
    /// sequence somebody chose, exactly as ADR-0040 requires.
    ///
    /// Dropped rather than folded in:
    ///
    /// - a reading that is not a finite positive number. A frame whose timed
    ///   passes add up to more than its own wall clock is two signals out of
    ///   phase - `crate::perf::GpuCost::residual_ms` documents the same
    ///   negative and prints it, because there it is a diagnostic; here it
    ///   would be a claim that the frame spends nothing outside the timers,
    ///   which is the one direction an upper bound may not be wrong in.
    /// - a reading above the estimate on a frame that kept its pace, which is
    ///   the sleeping frame this module exists to refuse. See the module docs
    ///   for why that gate is the no-slack condition and not a threshold.
    pub fn observe(&mut self, frame_seconds: f32, timed_seconds: f32, period: f32) {
        if !frame_seconds.is_finite() || !timed_seconds.is_finite() || !period.is_finite() {
            return;
        }
        if period <= 0.0 {
            return;
        }
        let reading = frame_seconds - timed_seconds;
        if reading <= 0.0 {
            return;
        }
        let held = self.seconds(period);
        // Either the reading is a tighter upper bound than what is held - and
        // every reading is bounded below by the truth - or the frame ran past
        // its own deadline and had nowhere to hide a sleep.
        if reading > held && frame_seconds <= period * OVERRUN {
            return;
        }
        let next = held + LEARN_RATE * (reading - held);
        self.learned = Some(next.clamp(0.0, RESIDUAL_SHARE * period));
    }
}
