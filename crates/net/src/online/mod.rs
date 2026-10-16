//! The online lobby: rooms, the one game slot, and the server that moves them.
//!
//! - [`rooms`]: every rule, with no I/O.
//! - `race_proc`: the race behind a seam, as a child process or a thread (native only).
//! - `server`: the WebTransport service over the two (native only).
//! - `client`: a lobby client over any [`crate::link::Link`] (native only).
//!
//! `docs/networking/online-lobby.md` has the design and the wire messages.
pub mod client;
#[cfg(not(target_arch = "wasm32"))]
pub mod race_proc;
pub mod rooms;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
