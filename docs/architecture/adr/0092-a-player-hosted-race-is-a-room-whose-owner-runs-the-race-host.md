# ADR-0092: A player-hosted race is a lobby room whose owner runs the race host

## Status

Proposed (2026-11-17). It follows the maintainer's decisions of 2026-11-17: version one is the
lobby plus two server races, further races are hosted by players and marked as such; a key pair
per player is enough identity for version one; a player-hosted race's results are not comparable
with server races. It builds on [ADR-0070](0070-the-network-transport-is-webtransport.md) (the
transport) and on decisions 1, 3 and 8 of
[the server roadmap](../../networking/server-roadmap.md). Every choice here is **chosen, not
measured**.

## Context

The server runs two races. A third room has to wait or run somewhere else, and the cheapest
"somewhere else" is the machine of the player who made the room: the race host is already a
separate role (`oag_net::host::Host`, the roadmap's "host protocol"), and a game already runs one
in-process for a LAN race (`oag_net::lan`). What a player's machine lacks is a public address, an
identity the lobby can moderate, and any reason for the other players to trust it.

## Decision

### 1. Shape: the room stays in the lobby, the race host moves

A player-hosted room is an ordinary lobby room (`oag_net::online::rooms`): members, picks, kick,
START and the board are the lobby's. Only the race host differs. At START the lobby sends the
room's owner the same `RaceJob` it would write to a race child (the race and a seat token per
person), the owner's game runs `Host` and `run_race` on a thread of its own, and the lobby hands
every member the owner's address instead of its own. This is the roadmap's host protocol
(`Register`, `Launch`, `Up`, `Over`) with one slot, spoken on the owner's lobby link. The host
plays as a client of its own host, dialling its own listener with its seat token, so there is one
path to a race.

The lobby issues the race (settings, seat tokens and the seed). A hosted room does not use a
server slot and never queues. Its reference edition is the owner's (title, pressing, mods, build),
so a room may race content the server does not carry.

### 2. Reachability: version one is a port the host opens, probed by the lobby

Options, cheapest first, with what WebTransport over QUIC does to each:

| Option | Works when | QUIC and WebTransport | Cost | Version |
| --- | --- | --- | --- | --- |
| **The host opens or forwards one UDP port; the lobby probes it before listing** | The host has a public address, a forwarded port or open IPv6 | Nothing special: one UDP port, the certificate hash pinned through the hand-off (ECDSA P-256, minted per session, well inside the 14 days a browser allows for a pinned hash) | A router setting for the host | **one** |
| UPnP, NAT-PMP or PCP asks the router for that port | The router offers it (often off, never behind carrier NAT) | None; it only automates the row above, and the same probe judges it | A dependency and a per-router failure surface | later, first |
| UDP hole punching coordinated by the lobby | Both NATs keep one mapping per socket (not symmetric NAT, not most carrier NAT pairs) | The lobby must see each side's mapping *of the socket that will carry the race*: the host registers from its listening endpoint, and a joiner dials the host from the endpoint its lobby link uses, so client and listener share one `quinn` endpoint each. QUIC retransmits the lost first packets itself. A browser cannot do it: `WebTransport` gives no control over the source socket | Transport work on both ends; a silent failure class per NAT pair | later, second |
| Relay through the server | Always | Opaque UDP forwarding keeps QUIC end to end; a WebTransport-level relay also serves a browser | The race's whole traffic, 0.8 to 5.3 Mbit/s a room (roadmap, "What a room costs"): the bandwidth player hosting exists to save | later, capped, last |

**The probe.** The registration names a port and the SHA-256 of the host's certificate. The lobby
dials that port **on the address the registration arrived from**, never one the host claims,
pinning that hash: one WebTransport handshake, 5 s at most. Success proves three things at once:
the port is reachable from the lobby's side of the internet, the listener is up, and it holds the
key of the certificate the signed registration names. The room exists only after the probe
passes; a failure is a refusal that names the port. Because the address is the observed one, a
registration cannot aim the lobby at a third party.

Limits of the probe, accepted: it is the lobby's vantage point, so a host on the lobby's own LAN
passes and is not reachable from outside; and it is one address family, the one the host's lobby
link uses, so a joiner without that family fails its dial and is told why.

### 3. Identity: an Ed25519 key pair per player, and a signed registration

- The game makes an Ed25519 key pair on first use and keeps it in `identity.toml` in the
  configuration directory (schema kind `identity`). No email, no password, no account.
- `HostRoom { settings, offer }`, client to lobby, where `offer` is `{ port, cert_hash, key,
  signature }`. The signature is over `"OAG-HOST-1" | the lobby certificate's hash | the session
  token the lobby's welcome issued | port | cert_hash`. The lobby's hash stops a registration
  being relayed to another lobby, the token (128 random bits per session) stops a replay, and the
  certificate hash ties the listener to the key. When the lobby gets a CA certificate (roadmap
  decision 6) the host name takes the place of its hash.
- The lobby verifies the signature, then the ban list, then probes. It knows a host by its public
  key and its address. Joining a room needs no key.
- Bounds on what a registrant controls: one registration under way per connection, four probes
  under way in all, 32 hosted rooms in all, two per address, one registration every 5 s per
  connection. A malformed offer is a refusal by name.

### 4. The PLAYER-HOSTED mark

The room list and the room show the mark and the host's name before anybody joins. It promises
nothing; it discloses:

1. the host's machine is the authority and could cheat;
2. the host's own latency is zero and everybody else's is their path to the host, not to the
   server;
3. the result is not comparable with a server race: no record, no leaderboard;
4. the host's address is given to the people who join, and theirs to the host (documented, not on
   the one line).

### 5. When the host leaves

Version one: the race ends for everyone, with a reason. A host that leaves in order tells its
seats (`End`, "the host left") and the lobby (`HostOver`). A host that vanishes is noticed by the
lobby when its link closes; the lobby closes the room and tells each member why, and a client
whose race link is lost stops redialling as soon as the lobby says so. Hosting needs the lobby
link: a host whose lobby link drops loses its room (accepted; re-registering is later). A hosted
race the lobby hears nothing of ends 35 minutes after its launch.

Later: decision 1 of the roadmap (the journal is the session) lets the race move, because every
client already holds every confirmed tick. The lobby would pick the next member with a probed
port, or a server slot that is free.

### 6. Moderation

Version one:

- **A ban list as a file** (`oag-server lobby --ban-file`): lines `key <64 hex> <date> [reason]` and
  `addr <ip> <date> [reason]`, the date a UTC stamp of when the ban was made (so a list can be
  pruned or bans given a lifetime later), re-read when it changes. A banned host is refused with the reason; a room
  whose host is banned while it is listed is closed. Bans cover hosting only.
- **A host log**: one `host-log` line in the lobby's log per registration, refusal, launch and
  end, with the key, address, name, room and settings.
- The operator's "command" is editing the file.

Later: a REPORT row on a player-hosted room (a count without a proof is weak evidence, so it
waits for decision 7 below), `oag-server` subcommands over a control socket, an account-bound key
if a new key per ban becomes a problem (outside the roadmap).

### 7. The anti-cheat path (design only)

State verification with fraud proofs, as agreed 2026-11-17, with the wire shapes it needs:

1. **The lobby is the reference for the race.** It issues the job, so the hand-off carries a hash
   of the job's `RaceParams` and a client refuses a host whose welcome differs. This also settles
   the seed: the lobby draws it (at random, which today it is not), so the host cannot choose it
   and no commit-reveal is needed while a lobby is in the path. Commit-reveal stays the answer for
   a race with no lobby.
2. **Signed checkpoints, not signed ticks.** A signature per tick is 60 a second and 64 bytes on
   every repeated record. Instead both ends keep a running hash over the confirmed tick records
   (inputs and state hash), and once a second the host sends that hash for the tick, signed with
   its player key. A client that cannot reproduce a checkpointed state hash after decision 8's
   resync sends the lobby the tick records since tick 0 and the signed checkpoint.
3. **The lobby re-simulates** from the job it issued (about a second for a five-minute race) and
   bans on a proven mismatch. Where it lacks the content, agreeing reports from several members
   are evidence, not proof.
4. **Player-signed inputs.** Each client signs each input datagram with its own key (one
   signature per datagram, about 4 KB/s up). A client that sees an input confirmed for its seat
   that it never sent and that is not a repeat reports the tick; the host must show the signed
   datagram or the report stands.
5. **Withheld or delayed inputs** leave no proof: clients report how long their inputs took to be
   confirmed against their round trip, as evidence.
6. Out of reach, as on a server race: bots and a client reading its own copy of the race.

## Version one and later

| | Version one (built with this ADR) | Later |
| --- | --- | --- |
| Reachability | opened or forwarded port, probed | UPnP family, hole punching, a capped relay |
| Identity | key pair per player, signed registration | account-bound key |
| Mark | list and room, one line | region and round trip to the host |
| Host leaves | race ends with a reason | the journal moves to another host |
| Moderation | ban file, host log | report, operator commands |
| Anti-cheat | hash comparison every tick (already there) | params hash, signed checkpoints, fraud proof, signed inputs |

## Consequences

- A player with a forwarded port can host a listed race today; most players behind a home router
  cannot until the UPnP or hole-punching steps land, and nothing in version one helps them beyond
  a refusal that names the port.
- The other players' addresses are visible to the host and the host's to them. A server race
  shows a player's address to the server only.
- A host that cheats in the simulation is noticed by every honest client and nothing more happens
  in version one than a logged bad hash: the mark is the whole defence.
- A banned player makes a new key; the address ban is a speed bump. Accepted by the maintainer.
- The lobby now dials out (the probe). It is bounded and aims only at the registrant, but it is a
  new kind of traffic for the server's firewall.
- A hosted room can race content the lobby cannot check or re-simulate.
- The wire grows messages under `PROTOCOL` 1, edited in place (roadmap decision 4).
