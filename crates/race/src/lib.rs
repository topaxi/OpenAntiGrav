//! Race rules: what mode is running, which lap it is, and how far round the
//! ship has got.
//!
//! This crate is the answer to a question the project had left open since the
//! track format was first read: **where does lap counting come from?** The
//! original does not tell us. `gate` (`.vex` class `0x3ca`) has no runtime class
//! registration at all - all 46 callers of the class registrar were enumerated
//! and none passes it - and no lap or split logic was found anywhere in the
//! executable. See `docs/formats/track.md#where-is-lap-counting`.
//!
//! So lap counting here is **our convention**, and it is written down as one:
//! [`Course`] turns the track's junction graph into a travel-ordered ring with
//! cumulative arc length, and a lap is a wrap of that arc length. The reasoning,
//! the confidence score and what would retire it are in
//! `docs/gameplay/lap-counting.md`.
//!
//! # What this crate does not know about
//!
//! No renderer, no audio, no input, no window - it is a gameplay crate and
//! `just check-deps` enforces that. It also does not depend on `oag-physics`:
//! the only thing the rules need from the force law is how hard the ship hit a
//! wall, and that arrives as a plain `f32`. Keeping the integrator's reporting
//! struct out of the rules means a rule change cannot ripple into the physics
//! crate's API.
//!
//! # Ordering
//!
//! The rules run **after** the physics step, on the position the step produced.
//! Running them before would count a lap against last tick's position, which is
//! one tick of error on a quantity whose whole job is to be exact at one instant.

pub mod course;
pub mod mode;
pub mod state;
pub mod zone;

#[cfg(test)]
pub(crate) mod testing;

pub use course::{Course, Located};
pub use mode::Mode;
pub use state::{LapGate, Outcome, RaceState};
