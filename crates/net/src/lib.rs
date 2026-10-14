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
//!
//! **No gameplay crate depends on this one** (rule 1 of `just check-deps`): the
//! simulation never knows the network exists. What reaches it is an
//! `InputSnapshot` per slot per tick, the same as from a pad.

pub mod link;
pub mod wire;
#[cfg(not(target_arch = "wasm32"))]
pub mod wt;
