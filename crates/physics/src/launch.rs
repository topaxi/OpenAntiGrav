//! The launch boost: a thrust multiplier for the first second after GO, graded by when thrust
//! first lands.
//!
//! Read off `Ship_UpdateStartBoost` (`0x0883fdec`), `FUN_0882773c` (the grade writer, once per
//! racing frame) and the head of `Ship_UpdateEngine` (`0x0884c5c8`, which raises the
//! first-thrust edge), and watched live on five launches of Pulse PSP (USA) on `16_Track`
//! (thrust held through the countdown, and first pressed 10, 24, 30 and 80 frames after
//! release). Law and captures: `docs/physics/launch-boost.md`.
//! ```text
//! each frame, in this order, from the frame the craft is released (state 1):
//!   engine:   T = T * craft+0x294 * 2.0          // reads LAST frame's multiplier
//!             edge = first frame control.thrust != 0   (craft+0x2d5, one-shot)
//!   grader:   (racing frames only - not the release frame itself)
//!             c = seconds since GO, 0 on the first racing frame
//!             if edge:  c <  windowStart  -> Stall
//!                       c <  windowEnd    -> Perfect
//!                       c <  stallEnd     -> Stall
//!                       c <  overall      -> Normal
//!             c += dt
//!   boost:    t = seconds since release (0 on the release frame)
//!             if t < overall + 0.5:
//!                 craft+0x294 = t < overall ? mul[grade] : 1.0
//!             t += dt
//! ```
//!
//! The grade starts at [`Grade::Normal`], so the frame thrust first lands (the edge) is still
//! multiplied by `normalMul`: the engine ran before the grader. That is the "1.4 for the frame
//! before GO and one more" of the first measurement, and why a held-through launch (an edge on
//! the first racing frame, a grade of Stall) reads `normalMul` once and `stallMul` for the rest
//! of the window, while a late first press reads `normalMul` throughout.
//!
//! **The original has a fourth grade, not ported.** Its grader writes grade 3 to every craft
//! whose player record is not the local human's, on every frame of the perfect window with no
//! thrust edge, and the multiplier is `boostMul` times a per-grid-slot figure from the AI stat
//! block: the original's AI cheating at the start line. The maintainer's rule is that the AI
//! obeys the player's physics, so **every craft here is graded by its own thrust edge**; an AI
//! that thrusts at GO lands in Stall, as a human who held the button does. Chosen, not measured.
//!
//! **A respawn does not replay the window**, though the decompile reads as if it would (the
//! timer is zeroed whenever `craft+0x2a4 != 1` and the grade persists): watched live, the timer
//! holds through state 3 and the multiplier stays `1.0`. This port runs the window once.

/// The launch boost's window and multipliers, from the disc's `<StartBoost>`.
///
/// The launch boost's window and multipliers, from the disc's `<StartBoost>`: the seven numbers
/// `oag_tables::handling::StartBoost` parses, copied across by the caller as for
/// [`crate::params::Handling`] (this crate depends on `oag-core` only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StartBoost {
    /// Seconds after GO at which the perfect window opens.
    pub window_start: f32,
    /// Seconds after GO at which the perfect window closes.
    pub window_end: f32,
    /// Seconds after GO at which the stall window closes.
    pub stall_end: f32,
    /// How long the multiplier is held, in seconds.
    pub overall_duration: f32,
    /// The multiplier for [`Grade::Stall`].
    pub stall_mul: f32,
    /// The multiplier for [`Grade::Normal`].
    pub normal_mul: f32,
    /// The multiplier for [`Grade::Perfect`].
    pub boost_mul: f32,
}

impl StartBoost {
    fn multiplier(&self, grade: Grade) -> f32 {
        match grade {
            Grade::Normal => self.normal_mul,
            Grade::Stall => self.stall_mul,
            Grade::Perfect => self.boost_mul,
        }
    }
}

/// How the first thrust landed, `player+0x36c` in the original (grades 0 to 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Grade {
    /// `0`: no grade earned yet, or the thrust first landed after the stall window. The resting
    /// grade, so the first thrust frame reads it.
    #[default]
    Normal,
    /// `1`: the thrust first landed before the perfect window (a held-through start included)
    /// or in the stall window after it.
    Stall,
    /// `2`: the thrust first landed inside the perfect window.
    Perfect,
}

/// One craft's launch-boost state: the multiplier the engine reads, and what decides it.
///
/// Per craft, on [`crate::ship::ShipState::launch`]. **Idle** (every field default) until
/// [`advance`] has run with the disc's parameters and the craft released, and hashed only once
/// it is not, so a world without `<StartBoost>` hashes as it did before this existed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaunchState {
    /// `craft+0x294`: what the engine multiplies its output by, [`Self::NEUTRAL`] outside the
    /// window. Set at the end of a tick, read by the next.
    pub multiplier: f32,
    /// `player+0x36c`.
    pub grade: Grade,
    /// `craft+0x2d4`: the first thrust has been seen. Never cleared.
    pub latched: bool,
    /// Ticks since the craft was released, `0` on the release frame and on the grid.
    /// `player+0x888` is this times the step; the race manager's clock (`race_manager+0x2bc`,
    /// seconds since GO) is this minus one, as it starts a frame later.
    ///
    /// **A count, not a running sum of `dt`.** The windows sit on tick boundaries at 60 Hz (a
    /// tenth, 0.35, 0.45 and one second are 6, 21, 27 and 60 ticks), so sixty `f32` sums of
    /// `1/60` land a hair short of `1.0` and boost a sixty-first tick; a count times the step is
    /// `>= 1.0` at 60. The original's `1/59.94` step overshoots, the same ticks.
    pub ticks: u32,
}

impl LaunchState {
    /// `1.0`, the multiplier outside the launch window and the value `Ship_InitCraft` and
    /// `Race_ResetCraftBoosts_q` write.
    pub const NEUTRAL: f32 = 1.0;

    /// Whether nothing has happened yet, so the state hashes as nothing.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for LaunchState {
    fn default() -> Self {
        Self {
            multiplier: Self::NEUTRAL,
            grade: Grade::Normal,
            latched: false,
            ticks: 0,
        }
    }
}

/// The grade a first thrust at `seconds` after GO earns, or `None` once the whole window is
/// over and the grader no longer writes.
fn grade_at(seconds: f32, p: &StartBoost) -> Option<Grade> {
    if seconds < p.window_start {
        Some(Grade::Stall)
    } else if seconds < p.window_end {
        Some(Grade::Perfect)
    } else if seconds < p.stall_end {
        Some(Grade::Stall)
    } else if seconds < p.overall_duration {
        Some(Grade::Normal)
    } else {
        None
    }
}

/// One tick of the grader and the boost writer, **after** the engine has read
/// [`LaunchState::multiplier`] for this tick. `released` is the craft being out of the grid
/// state (`ShipState::released`); `thrust` is whether the raw thrust input is non-zero. With no
/// `params` nothing runs and the multiplier stays neutral (every title and test without the
/// disc's `<StartBoost>`).
pub fn advance(
    launch: &mut LaunchState,
    params: Option<&StartBoost>,
    released: bool,
    thrust: bool,
    dt: f32,
) {
    let Some(p) = params else {
        return;
    };
    if !released {
        // `player+0x888 = 0` and the manager's clock held at zero by the countdown handler;
        // the grade and edge latch are the craft's own and left alone.
        launch.ticks = 0;
        return;
    }

    let edge = thrust && !launch.latched;
    if edge {
        launch.latched = true;
    }

    // The grader runs on racing frames only: the release frame is the countdown handler's, which
    // zeroes the clock and does not read the edge.
    if edge && launch.ticks > 0 {
        let clock = (launch.ticks - 1) as f32 * dt;
        if let Some(grade) = grade_at(clock, p) {
            launch.grade = grade;
        }
    }

    let t = launch.ticks as f32 * dt;
    if t < p.overall_duration + 0.5 {
        launch.multiplier = if t < p.overall_duration {
            p.multiplier(launch.grade)
        } else {
            LaunchState::NEUTRAL
        };
    }
    launch.ticks = launch.ticks.saturating_add(1);
}

#[cfg(test)]
mod tests;
