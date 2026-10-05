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
//! are on `docs/gameplay/ai.md`.
//!
//! # Eight drivers, not one driver eight times
//!
//! A field that all shares one [`Tuning`] and one line drives in single file,
//! because there is one best place to be and nothing to separate two craft that
//! both compute it. So each craft carries a seed, and [`Personality::from_seed`]
//! turns that into a small set of departures from the shared tuning: which part
//! of the corridor it holds, how much it drifts about that and how fast, how far
//! ahead it looks, how much grip it assumes through a corner, and how early it
//! brakes.
//!
//! Two of those do the visible work. **The line bias** puts the eight craft on
//! eight different parts of the track rather than on one line, and **the
//! commitment** gives them eight different corner speeds, which is what opens
//! gaps and closes them again over a lap. The rest is texture.
//!
//! The corridor it all gets spent inside is the disc's own: `ai_bound_left` and
//! `ai_bound_right` are authored per control point, so how far the field spreads
//! is a property of the track and narrows where the artists narrowed it. See
//! [`Frame`] and `docs/formats/track.md`.
//!
//! ## What a personality does not cover
//!
//! Named so nobody assumes otherwise, and each a separate piece of work:
//! **nothing here knows another craft exists**, so there is no avoidance, no
//! overtaking line and no defending - the spread is what keeps craft apart, not
//! a rule that keeps them apart. No mistake injection and no recovery
//! behaviour, no difficulty selection, no weapon competence, and no adaptation
//! between races. `docs/gameplay/ai.md` describes all of them.
//!
//! **Reaction latency is not a personality axis either, and that is on
//! purpose.** It is a property of a driver's *perception* rather than of its
//! character, so it lives on [`Driver`] as a [`Reflex`] and its length comes
//! from the [`Difficulty`] rather than from a seed: two craft of the same
//! skill notice the field equally quickly, and get quicker together.
//!
//! # Determinism
//!
//! No clock and no allocation per tick, so a race replays identically.
//!
//! The two sources of variation are both pure functions of a craft's own seed:
//! the personality comes off `oag_core::Rng` at a fixed sequence of draws, and
//! the drift is integer-hashed value noise over the driver's own tick count.
//! **Neither draws from the world's generator**, which would move every pickup
//! roll after it, and neither uses a transcendental - `sin` resolves to the
//! platform's libm and is exactly the thing
//! `docs/architecture/determinism.md` forbids. The reasoning is at the top of
//! `noise.rs`.

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
