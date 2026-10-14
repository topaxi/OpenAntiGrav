# ADR-0070: The network transport is WebTransport

## Status

**Accepted (2026-10-14) for what the maintainer decided, and only that:** network
play runs over WebTransport, "if possible, otherwise UDP" (maintainer, 2026-10-14);
self-signed certificates on a LAN are acceptable (maintainer, 2026-10-13).
WebTransport worked, so the UDP fallback was not built.

**Proposed, not accepted**, the rest: the crate, the certificate scheme, the
channel split and the wire shape below are choices made while building the first
server and client, for the maintainer to accept or change.

Relates to [ADR-0069](0069-remote-craft-are-predicted-in-one-timeline-and-corrected-on-a-threshold.md)
(Proposed), whose prediction and correction this transport carries, and to
[ADR-0002](0002-determinism-model.md), which the wire shape leans on.
Evidence: `docs/networking/README.md`, "Transport" and "Running a local server".

## Context

Online play is server-authoritative and tick-based (decisions 1 to 6 of
`docs/networking/README.md`). It needs two kinds of channel: an unreliable one for
each tick's input and state, where a late packet is worthless, and a reliable,
ordered one for joining, starting and ending. The web build runs in a browser tab,
which cannot open a UDP socket; WebSocket is TCP and stalls every packet behind a
lost one. WebTransport (HTTP/3 over QUIC) has both channel kinds, in browsers and
natively.

## Decision

1. **WebTransport, native and (later) web, one protocol.** `oag-net`'s `Link` trait
   is the two channel kinds and nothing else; `wt::WtLink` implements it natively. A
   browser client would implement the same trait over the browser's own
   `WebTransport` object (not built).
2. **`web-transport-quinn`, on rustls' `ring` provider** (Proposed). A half-day
   spike tried it and `wtransport` on this workspace's MSRV (1.97.1): both opened a
   session on localhost, carried datagrams and a stream both ways and pinned a
   self-signed certificate by hash. `web-transport-quinn` was taken because its
   family has a browser client (`web-transport-wasm`, and `web-transport` over both)
   and it was updated a week before; `wtransport` mints a self-signed certificate
   itself, which is less code here. `ring` rather than the default `aws-lc-rs`, which
   builds C with cmake and would reach the Android and iOS builds.
3. **A certificate per run, pinned by SHA-256** (Proposed). The server mints an
   ECDSA P-256 certificate valid 13 days (a browser's `serverCertificateHashes`
   takes at most 14) and prints its hash; a client is given the hash and accepts
   that certificate only. A wrong hash fails the TLS handshake (`UnknownIssuer`)
   within milliseconds. A public server would use a CA-signed certificate (not
   built).
4. **Inputs on the wire, not state** (Proposed). A client sends its held buttons
   and four axes per tick, the last eight ticks in each datagram. The server sends,
   per tick, the input it applied to every person and the state hash it reached,
   the newest sixteen ticks in each datagram, and the last ones again on the stream
   when it ends. A client re-runs those inputs on its own copy of the server's state
   and checks the hash on every tick. Nothing else is needed because the simulation
   is deterministic and both ends start from the same content, which the join
   checks: the tick-0 state hash must match the server's.

## Why

- WebTransport was asked for, and it worked first time on both crates.
- A `RaceSim` snapshot is about 1.7 MiB and a datagram about 1,400 bytes. The
  server's choice of inputs (after buffering and repeat-on-empty) is its authority,
  and a hash per tick turns any disagreement into a named error instead of a drift.
- Measured on localhost (`docs/networking/README.md`): two clients through a whole
  race at a 0 and a 100 ms round trip, and at 100 ms with 5 percent loss, agreed with
  the server's hash on every confirmed tick.

## Consequences

- **The protocol is only as good as determinism.** Any un-hashed setting that
  differs between server and client (the control scheme, pilot assist, which AI
  flies) is a silent desync until it moves something hashed. Version 1 sends the
  scheme and fixes the rest (`oag_net::race`); a new per-race setting must be added
  there or to the hash.
- **A late joiner cannot be served.** It would need the whole input history from
  tick 0 replayed, or a state snapshot, which the wire does not carry.
- **A client keeps two timelines**, the confirmed one and the present, and steps the
  confirmed one as the server's ticks arrive (one extra simulation tick per tick).
- **The dependency weight**: about 140 crates in a localhost spike, quinn, rustls,
  HTTP/3 framing, rcgen and tokio among them (816 lines of `Cargo.lock`). All of it is
  native-only and outside the simulation crates.
- **Networks that block UDP cannot play**, and no TCP fallback exists.
- The UDP fallback named in the status was never needed and is not built; it would be
  this trait over a socket, without TLS.
