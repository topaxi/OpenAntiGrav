//! Mixing and playback.
//!
//! Nothing in the simulation may depend on this crate. The arrow points the
//! same way `oag-input`'s does - the simulation owns the types read here, and
//! `scripts/check-dependency-rules.py` fails the build if that is ever
//! reversed. See `docs/architecture/workspace-layout.md`.
//!
//! The crate owns no window, no event loop and no clock. It is split in two,
//! the way [`oag_render::sparks`] splits its particle state from its GPU
//! pipeline:
//!
//! - [`mixer`] is plain data: a voice pool, two buses, and the sample loop.
//!   Testable with no hardware, and identical under a real stream, the offline
//!   dump and CI.
//! - [`output`] is the device: a `cpal` stream, or none at all.
//! - [`wav`] writes rendered samples out, which is how a headless run is
//!   checked at all.
//!
//! # Audio is not the simulation
//!
//! Cues are a per-tick *output* of the simulation, like `oag_race::Outcome` and
//! `oag_physics::Evaluated` - never `World` state. Putting one in `World` would
//! move the committed hashes in `oag_core::hash`, and those are never edited to
//! make a test pass. The mixer may therefore allocate, lock and thread freely:
//! `docs/architecture/determinism.md` puts audio outside the simulation
//! explicitly.
//!
//! What the mixer *does* guarantee is that the same control sequence renders
//! the same samples, because it never reads a clock - elapsed time is whatever
//! the caller renders. That is what lets `--dump-audio` reproduce a real-time
//! run rather than approximate it.

pub mod mixer;
pub mod output;
pub mod wav;

pub use mixer::{Bus, Mixer, Play, Sound, VoiceId};
pub use output::Output;
