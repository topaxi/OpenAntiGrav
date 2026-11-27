//! What an Eliminator is won on. A title states it here and the race resolves it once at load, so
//! nothing reads a title during a race.

/// A title's Eliminator rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EliminatorRule {
    /// The first craft to a kill target ends it: Pulse
    /// (`docs/ghidra/functions/psp-pulse-usa/eliminator-kill-target.md`).
    Kills,
    /// The first craft to a score target ends it: HD and Fury, and 2048 and Omega on the same
    /// awards (`docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`, "Other titles").
    Score(ScoreRule),
}

/// A title's Eliminator flip: the craft turns half about its own up axis on a button press.
/// `docs/ghidra/functions/ps3-hdfury-eu/eliminator-flip.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlipRule {
    /// How long the half turn takes, seconds, for a craft whose handling file authors no
    /// `eliminator_flip_time` of its own.
    pub seconds: f32,
}

/// The awards of a score Eliminator and where a Racebox race's target comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreRule {
    /// Points per point of damage dealt.
    pub damage: f32,
    /// Points for a kill, on top of the fatal blow's damage points.
    pub kill: f32,
    /// Points for a lap.
    pub lap: f32,
    /// Points a destroyed craft loses, never below zero. Not scaled.
    pub death: f32,
    /// The target a race starts on when no event authors one.
    pub target: ScoreTarget,
}

/// Where a score Eliminator's target comes from outside an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreTarget {
    /// The definition holding the Racebox slider whose minimum is the target, and whose steps
    /// the `TARGET` row offers.
    Slider(&'static str),
    /// The executable's own default, for a title with no such slider.
    Fixed(u32),
}

/// How a score Eliminator's opponents spend the weapons they hold, so a race reaches its target at
/// about the original's length (`docs/gameplay/race-modes.md`, "The pace of a score Eliminator").
/// Every term is ours: the original's drop and Cannon decisions are unread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScorePaceTerms {
    /// The share of a pilot's own trigger span every tier lays Mines and Bombs at.
    pub trigger_share: f32,
    /// Seconds an opponent holds a Mine, Bomb or Cannon before it spends it on the Shield instead,
    /// or `None` to hold it until it is used. A held weapon blocks the next pad.
    pub stale_seconds: Option<u32>,
}

impl ScorePaceTerms {
    /// The terms tuned on a draw that still holds the Repulser and the Shuriken: both are spent
    /// within about a second and a half, so the three slow weapons are fewer of the grants.
    pub const WITH_REPULSER_AND_SHURIKEN: Self = Self {
        trigger_share: 0.2,
        stale_seconds: None,
    };
}
