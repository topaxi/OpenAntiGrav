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
#[cfg(not(target_arch = "wasm32"))]
pub mod extract;
pub mod host;
#[cfg(not(target_arch = "wasm32"))]
pub mod lan;
pub mod link;
pub mod lobby;
pub mod online;
pub mod race;
pub mod serve;
pub mod stats;
#[cfg(feature = "client")]
pub mod weapon_sync;
pub mod wire;
#[cfg(not(target_arch = "wasm32"))]
pub mod wt;

/// The short commit hash this build was made from (`OAG_GIT_HASH`, or `git`'s answer, or
/// `unknown`); see `build.rs`.
pub const GIT_HASH: &str = env!("OAG_GIT_HASH");

/// The version suffix the build was given (`OAG_VERSION_SUFFIX`), empty for a release.
pub const VERSION_SUFFIX: &str = env!("OAG_VERSION_SUFFIX");

/// The version a build reports: the crate version, `OAG_VERSION_SUFFIX` if the
/// build was given one (the nightly workflow passes `-nightly`), and the commit.
/// A tagged release leaves the suffix empty. What a LAN host and an online lobby compare
/// ([`discovery::differs`]) and what `oag-game --version` and `oag-server --version` print.
pub const BUILD_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    env!("OAG_VERSION_SUFFIX"),
    " (",
    env!("OAG_GIT_HASH"),
    ")"
);

#[cfg(test)]
mod build_version_tests {
    use super::{BUILD_VERSION, GIT_HASH, VERSION_SUFFIX};

    #[test]
    fn the_build_version_carries_suffix_and_commit() {
        let expected = format!("{}{VERSION_SUFFIX} ({GIT_HASH})", env!("CARGO_PKG_VERSION"));
        assert_eq!(BUILD_VERSION, expected);
        assert!(BUILD_VERSION.contains(VERSION_SUFFIX));
    }
}
