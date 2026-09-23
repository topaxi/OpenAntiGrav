//! Replays: what a run consumed, what it arrived at, and a ghost to race.
//!
//! The design is [ADR-0055]; this is its implementation. In one paragraph:
//! **the input stream is the truth, and a ghost is drawn from poses.** A
//! [`Replay`] holds every [`oag_gameplay::InputSnapshot`] the simulation was
//! handed, per human slot and per tick, exactly as [`Race::tick`] received it,
//! plus a [`StateHasher`] fingerprint of the whole race every
//! [`Header::hash_interval`] ticks. Feeding those inputs back into the same
//! build reproduces the run bit for bit (that is what `docs/architecture/
//! determinism.md` exists to guarantee), and [`Verifier`] compares the hashes as
//! it goes, so a replay that stopped reproducing reports the first tick it
//! diverged on rather than showing something that never happened.
//!
//! A ghost is the other half. Its [`GhostLap`] is a pose per tick of one lap,
//! sampled from the run as it was driven, and it is what gets drawn - never a
//! resimulation - because this project's simulation changes every week and a
//! personal best has to keep showing the lap that set it. The inputs travel
//! with it, so the lap can still be reproduced and checked on a build that
//! agrees with the one that recorded it.
//!
//! # Title-agnostic
//!
//! Nothing here knows a title, a track format or how a race is loaded. The
//! [`Header`] carries the key a composition root files a replay under (title,
//! track, mode, class, team), the seed and tick rate, and a free-form
//! [`Header::options`] table for whatever else that composition root needs to
//! rebuild the same race. Stepping a race is a closure handed to
//! [`Verifier::run`], so the same file serves Pulse, Pure, HD and 2048.
//!
//! # What reaches a hash
//!
//! Nothing this crate computes feeds simulation state: it copies inputs in and
//! out bit for bit, and reads hashes the caller computed. The floats it touches
//! are stored by their bit patterns and never operated on, so the determinism
//! rules are met by construction rather than by care.
//!
//! [ADR-0055]: ../../../docs/architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md
//! [`Race::tick`]: https://docs.rs/oag-game
//! [`StateHasher`]: oag_core::StateHasher

pub mod codec;
pub mod file;
pub mod header;
pub mod pose;
pub mod record;
pub mod verify;

pub use file::{FORMAT_VERSION, ReadError, Replay};
pub use header::Header;
pub use pose::{GhostLap, Pose};
pub use record::Recorder;
pub use verify::{Desync, Verifier};
