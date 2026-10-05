//! Primitives shared by every OpenAntiGrav crate.
//!
//! The contents of this crate are load-bearing for determinism. The simulation
//! must produce bit-identical results on every platform we ship, because
//! replays, golden tests and the behavioural verification suite all compare
//! exact state hashes across machines.
//!
//! See `docs/architecture/determinism.md` for the full rules and
//! `docs/architecture/adr/0002-determinism-model.md` for why they were chosen.

pub mod buttons;
pub mod hash;
pub mod math;
pub mod probe;
pub mod rng;
pub mod tick;

pub use hash::StateHasher;
pub use rng::Rng;
pub use tick::{TickClock, TickRate};
