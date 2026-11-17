---
status: re-first
titles: [all]
updated: 2026-11-17
summary: "Version one: the server runs the lobby and two races; further races are hosted by players, listed in the lobby and marked as player-hosted"
categories: [tooling]
---

# Player-hosted races are listed in the lobby

2026-11-17, the maintainer: "best case I'd like to have when I cut version one is the lobby
and two concurrent races being run on the server. Further races may be advertised from local
clients, and they will be the authoritative race server; they shall be annotated as such in
the lobby, so it is clear that this is a player's game (potential of cheating, definitely
unfair latency, etc.)."

## What exists

- The lobby places a room on a race process it starts itself (one slot today); a race process
  is one race (server-roadmap decision 3), and the hand-off gives clients a port, a
  certificate hash and a seat token
  ([server-roadmap.md](../../docs/networking/server-roadmap.md), "Lobby and race server: the
  boundary").
- A race host is already a separate role (`oag-server` race child; the roadmap's V2 makes a
  host an agent the lobby talks to). A game client that also runs the host for its own race
  is that agent, on a player's machine.
- Memory: the race process peaks at 542 MB while loading (measured on the remote host,
  [online-lobby.md](../../docs/networking/online-lobby.md), "Measured over the internet, with
  the limits"); lane `net-mem` is cutting it with "lobby plus two races on the 1 GB VPS" as its
  acceptance bar.

## Open

1. **Server capacity for version one: two races.** The lobby runs at most two races itself;
   a third room waits or goes to a player host. Depends on `net-mem`'s numbers.
2. **A player host registers with the lobby** (the roadmap's host protocol, V2: `Up { room,
   port, cert_hash }` and health lines), from the game client instead of a server box, and
   the lobby lists its room with a **PLAYER-HOSTED** mark: the host's name, that the host's
   machine is the authority (it could cheat), and that latency is the host's own (unfair to
   the others, best for the host).
3. **Reachability.** A player's machine is usually behind NAT. Options, cheapest first: the
   host opens or forwards a UDP port itself (UPnP or manual; the lobby probes it before
   listing the room); UDP hole punching coordinated by the lobby; relaying through the
   server (costs the server's bandwidth, which is what player hosting is meant to save). The
   lobby lists only a room it could reach.
4. **Trust.** The host is authoritative: its own inputs arrive with no latency and it could
   alter the simulation. The mark says so; nothing more is promised. A player-hosted race's
   results are not comparable with server races (no records, no shared leaderboards).
5. **Identity.** A player host has no CA certificate (roadmap decision 6 covers the server
   only): it uses the certificate-hash method through the lobby's hand-off, as today.
6. **When the host leaves.** The race ends for everyone (or, later, the journal moves to
   another player or to the server if a slot is free; decision 1 makes that possible).
7. **Strings and screens.** The lobby's room list gets the mark and a short explanation;
   rows need `string_id` and English text (CLAUDE.md, `just check-strings`).

## Next Steps

1. After `net-mem` lands: confirm the two-race cap and whether starts must be staggered.
2. A design pass (sonnet `oag-docs`, 2 h) that picks the reachability option and the
   lobby-side host registration over the V2 host protocol, as a section of the server
   roadmap; then wire lanes.
