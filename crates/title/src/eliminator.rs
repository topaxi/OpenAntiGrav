//! What an Eliminator is won on. A title states it here and the race resolves it once at load, so
//! nothing reads a title during a race.

/// A title's Eliminator rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EliminatorRule {
    /// The first craft to a kill target ends it: Pulse
    /// (`docs/ghidra/functions/psp-pulse-usa/eliminator-kill-target.md`).
    Kills,
    /// The first craft to a score target ends it: HD and Fury
    /// (`docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`).
    Score(ScoreRule),
}

/// A title's Eliminator flip: the craft turns half about its own up axis on a button press.
/// `docs/ghidra/functions/ps3-hdfury-eu/eliminator-flip.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlipRule {
    /// How long the half turn takes, seconds.
    pub seconds: f32,
}

/// The awards of a score Eliminator and where a Racebox race's target is authored.
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
    /// The definition holding the Racebox slider whose minimum is the target a race starts on.
    pub race_setup: &'static str,
}
