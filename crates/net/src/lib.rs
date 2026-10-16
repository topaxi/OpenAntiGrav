//! Network play.
//!
//! Server-authoritative, tick-based, clients predicting every craft and
//! correcting on a threshold: the decisions in `docs/networking/README.md` and
//! [ADR-0069](../../../docs/architecture/adr/0069-remote-craft-are-predicted-in-one-timeline-and-corrected-on-a-threshold.md),
//! over the transport of
//! [ADR-0070](../../../docs/architecture/adr/0070-the-network-transport-is-webtransport.md).
//!
//! - [`wire`]: the messages and their bytes.
//! - [`link`]: the transport as the protocol sees it ([`link::Link`]), an
//!   in-process pair and a latency and loss shim.
//! - `wt`: [`link::Link`] over WebTransport, native only.
//! - [`race`]: the one way the server and a client build the same race.
//! - [`discovery`]: finding a LAN host (a UDP query and its answer).
//! - [`lobby`]: the connections waiting for a LAN host's START.
//! - [`serve`]: the race phase of a server, shared by `oag-server` and a hosting game.
//! - `lan`: a game hosting a race in-process, native only.
//! - [`host`]: the authoritative race and each person's input buffer.
//! - [`stats`]: what a race reports about itself, for the overlay and the log.
//! - [`client`]: prediction, the confirmed timeline and the threshold correction.
//!
//! **No gameplay crate depends on this one** (rule 1 of `just check-deps`): the
//! simulation never knows the network exists. What reaches it is an
//! `InputSnapshot` per slot per tick, the same as from a pad.

#[cfg(feature = "client")]
pub mod client;
pub mod discovery;
pub mod host;
#[cfg(not(target_arch = "wasm32"))]
pub mod lan;
pub mod link;
pub mod lobby;
pub mod online;
pub mod race;
pub mod serve;
pub mod stats;
pub mod wire;
#[cfg(not(target_arch = "wasm32"))]
pub mod wt;
