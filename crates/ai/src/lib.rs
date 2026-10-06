//! Opponent behaviour: what the other seven craft do.
//!
//! One driver per craft, each following a line the caller supplies and emitting
//! the same [`ShipControls`] a human's buttons map to. Nothing here reaches the
//! world, the race rules or the renderer, so it is testable against a synthetic
//! circle with no track, disc image or GPU.
//!
//! [`ShipControls`]: oag_physics::ShipControls
//!
//! # What this is, and what the original does
//!
//! The original's opponents follow the same authored racing line with a
//! controller of the same shape (lookahead point, cross-track term, damping
//! constant, authored per speed class in `Data\XML\AIControlStats.xml`),
//! recovered in `docs/ghidra/functions/psp-pulse-usa/ai-stats.md`.
//!
//! **Their speed is not.** There it comes from a schedule keyed on the player's
//! race position and gap, so an opponent is fast because of where the player
//! is. This crate derives a speed target from the corner ahead and the craft's
//! own physics: an opponent's ceiling is the player's physics and it gets
//! quicker by driving better. Neither the gap to the player nor the player's
//! position is an input to anything here. Reasoning, schema and design:
//! `docs/gameplay/ai.md`.
//!
//! # Eight drivers, not one driver eight times
//!
//! A field sharing one [`Tuning`] and one line drives in single file: there is
//! one best place to be. So each craft carries a seed and
//! [`Personality::from_seed`] turns it into departures from the shared tuning:
//! corridor part held, drift about it, lookahead, grip assumed through a corner,
//! braking point. **The line bias** spreads the craft across the track and **the
//! commitment** gives eight corner speeds, which opens and closes gaps over a
//! lap; the rest is texture.
//!
//! The corridor is the disc's own (`ai_bound_left`, `ai_bound_right` per
//! control point), so the spread narrows where the artists narrowed it. See
//! [`Frame`] and `docs/formats/track.md`.
//!
//! ## What a personality does not cover
//!
//! A personality is a seed's draw of line, pace and braking. What a driver does
//! about *other craft* (yielding, blocking, shoving, dodging charges, firing,
//! rolling) is [`Field`], [`Pilot`] and the `Driver` decisions around it, and
//! the level is [`Difficulty`]. Not covered anywhere: adaptation between races
//! (`docs/gameplay/ai.md`).
//!
//! **Reaction latency is not a personality axis, on purpose**: it is
//! *perception*, so it lives on [`Driver`] as a [`Reflex`] and its length comes
//! from the [`Difficulty`], not a seed. Two craft of the same skill notice the
//! field equally quickly.
//!
//! # Determinism
//!
//! No clock and no allocation per tick, so a race replays identically. Both
//! sources of variation are pure functions of a craft's own seed: the
//! personality off `oag_core::Rng` at a fixed sequence of draws, the drift as
//! integer-hashed value noise over the driver's tick count. **Neither draws from
//! the world's generator**, which would move every later pickup roll, and
//! neither uses a transcendental: `sin` resolves to the platform's libm, which
//! `docs/architecture/determinism.md` forbids (reasoning atop `noise.rs`).

pub mod branch;
mod difficulty;
mod driver;
mod field;
mod line;
mod noise;
mod pilot;
pub mod plan;
pub mod probe;
pub mod weapon_ai;

pub use difficulty::Difficulty;
pub use driver::{
    AVOIDANCE_LOOKAHEAD, AWARENESS_RANGE, Context, Driver, PAD_LOOKAHEAD, Personality, Reflex,
    Tuning, hull_yaw_ceiling,
};
pub use field::{Field, Hazard, Pad, Rival};
pub use line::{Aim, Frame, Line, shift as line_shift};
pub use pilot::{Lean, Pilot, Span, pilot_for_slot};
pub use plan::SpeedPlan;
