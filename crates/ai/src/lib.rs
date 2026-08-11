//! Opponent behaviour: what the other seven craft do.
//!
//! One driver per craft, each following a line the caller supplies and emitting
//! the same [`ShipControls`] a human's buttons map to. Nothing here reaches the
//! world, the race rules or the renderer - it takes a craft's state and a line,
//! and returns what to hold this tick - which is what lets it be tested against a
//! synthetic circle with no track, no disc image and no GPU.
//!
//! [`ShipControls`]: oag_physics::ShipControls
//!
//! # What this is, and what the original does
//!
//! The original's opponents follow the same authored racing line, with a
//! controller of the same shape: a lookahead point, a cross-track term, and a
//! damping constant, authored per speed class in `Data\XML\AIControlStats.xml`.
//! That much is recovered - see
//! `docs/ghidra/functions/psp-pulse-usa/ai-stats.md`.
//!
//! **Their speed is not.** In the original it comes from a schedule keyed on the
//! player's race position and the gap to the player, so an opponent is fast
//! because of where the player is rather than because of how it drove. This
//! crate derives a speed target from the corner ahead and the craft's own
//! physics instead: an opponent's ceiling is the same physics the player has,
//! and it gets quicker by driving better. Neither the gap to the player nor the
//! player's race position is an input to anything here, and there is nowhere to
//! pass them in.
//!
//! The reasoning, the recovered schema and the design this is the first half of
//! are on `docs/gameplay/ai.md`. **Not built yet, and deliberately**: the skill
//! vector, mistake injection, difficulty selection and the adaptive rules that
//! page describes. This is the basic driver they hang off.
//!
//! # Determinism
//!
//! No randomness, no clock and no allocation per tick, so a race replays
//! identically. A future skill vector that wants noise takes it from
//! `oag_core::Rng` through the world, never from a generator of its own. See
//! `docs/architecture/determinism.md`.

mod driver;
mod line;

pub use driver::{Driver, Tuning};
pub use line::Line;
