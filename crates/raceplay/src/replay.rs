//! Recording a race as it is driven, racing a ghost of a lap, and rebuilding a recorded race.
//!
//! The format and the reasoning are [ADR-0055] and `oag_replay`.
//!
//! **Rebuilding and re-driving build without the `view` feature** ([`rebuild`], [`Launch`],
//! [`Race::apply_event`]), so a host with no renderer can verify a replay (dependency rule 6).
//! The other half watches a live [`Race`] and draws from it (`recording`, `view` only). Its two
//! independent pieces:
//!
//! - **The recording**: every tick's [`PlayerInputs`] and a state hash a
//!   second, through [`oag_replay::Recorder`], plus the player's pose every
//!   tick of the lap being driven. When a lap completes quicker than the best
//!   so far, its poses become a [`GhostLap`].
//! - **The ghost**: a [`GhostLap`] read at the player's own lap clock. It is
//!   poses and nothing else - no craft, no grid slot, no collision.
//!
//! # Neither is simulation state
//!
//! Both sit on [`Race`] beside [`RaceView`], never inside [`RaceSim`] or
//! [`oag_gameplay::World`], so nothing here can reach
//! [`RaceSim::state_hash`]: a race with a ghost hashes exactly as it does
//! without one, tick for tick, which
//! `crates/raceplay/tests/replay_ground_truth.rs` asserts on a real circuit. The
//! recorder *reads* the hash; it never writes anything the tick reads.
//!
//! **Off by default.** A race records nothing until
//! [`Race::start_recording`] is called, so every trace, capture and
//! ground-truth test that does not ask for it runs exactly the code it ran
//! before.
//!
//! [`Race`]: crate::Race
//! [`Race::apply_event`]: crate::Race::apply_event
//! [`Race::start_recording`]: crate::Race::start_recording
//! [`RaceView`]: crate::RaceView
//! [`RaceSim`]: crate::RaceSim
//! [`RaceSim::state_hash`]: crate::RaceSim::state_hash
//! [ADR-0055]: ../../../../docs/architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md

mod events;
mod launch;
mod rebuild;
#[cfg(feature = "view")]
mod recording;

pub use events::ApplyError;
pub use launch::{Launch, UnknownOption};
pub use rebuild::{apply_recorded, load_recorded, rebuild, start_recorded};
#[cfg(feature = "view")]
pub(crate) use recording::ReplayState;
#[cfg(feature = "view")]
pub use recording::{Ghost, GhostCapture};
