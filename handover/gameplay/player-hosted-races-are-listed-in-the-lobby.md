---
status: ready-to-wire
titles: [all]
updated: 2026-11-17
summary: "ADR-0092 slice landed; open: server's second race slot, HOST GAME menus test, UPnP and hole punching, the anti-cheat path"
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

## Landed (2026-11-17, lane `net-playerhost`)

Design: [ADR-0092](../../docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md) (Proposed). What is built and how to run
it: [online-lobby.md](../../docs/networking/online-lobby.md), part 12; the roadmap's
"Player-hosted races" section.

- Items 2, 3 (its cheapest option), 4, 5, 6 (the race ends), 7 (the key pair, the ban file, the
  host log) and 9 of the list below are built and tested: `oag-net` `player_host::`,
  `oag-game` `net_hosted_ground_truth` (real processes), `identity`, `online_screen`.
- Item 8 is designed (ADR-0092 decision 7), not built.

## Open

1. **Server capacity for version one: two races.** Untouched by this lane: the lobby still has
   one race slot of its own (`Server::race`). The memory for two is measured (roadmap, "The
   lobby and two races on the 1 GB VPS"); the wiring is the roadmap's V2 item 1.
2. **HOST GAME from the menus has no driven test.** The row, the offer, the "checking" line
   and the launch are wired (`session::online`) and the same `oag_game::online::Hosting` runs
   under `--online-host`, which the real-process test covers; nobody has walked the menus as
   host. A `--drive` script needs a saved `[online]` address, hash and name to pass the three
   prompts.
3. **Reachability beyond an opened port** (ADR-0092 decision 2): UPnP/NAT-PMP/PCP first, then
   hole punching (client and listener on one `quinn` endpoint each), then a capped relay.
   Today a host behind a router gets a refusal that names the port and nothing more.
4. **The host's lobby link dropping closes its room**, mid-race too. Re-registering with the
   session token, and the race moving to another host from the journal, are later.
5. **Anti-cheat** (ADR-0092 decision 7), cheapest first: a hash of the job's `RaceParams` in
   the hand-off that a client checks against the host's welcome; a seed the lobby draws at
   random (today `seed ^ room << 32 ^ launches`, predictable, on server races too); signed
   once-a-second checkpoints; the fraud proof and the lobby's re-simulation; signed inputs.
6. **Moderation beyond the file**: a REPORT row on a hosted room, operator subcommands, and a
   host log file of its own (today `host-log:` lines in the lobby's log).
7. **Records.** A player-hosted race must not feed a record or leaderboard once online races
   feed any; none do today, so nothing is excluded yet.
8. **Not checked:** IPv6 hosts (the code binds the lobby link's family; only IPv4 localhost
   was run), a real forwarded port over the internet, Windows, titles other than Pulse in the
   real-process test (the path is title-blind; the stills were read on Pulse, Pure, HD and
   Omega).

## Next Steps

1. The driven menus test of item 2 (an afternoon): `--drive` to ONLINE with a saved lobby,
   `select online_host`, the RACE pages, then a second `--online` client as the guest.
2. The params hash and the random seed of item 5 (half a day, one wire field and one check).
3. The server's second race slot (roadmap V2 item 1), which is what "version one" still lacks
   on the server side.
4. UPnP (item 3), behind the same probe.

## The thread as it stood (2026-11-17, before the lane)

### Open then

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
7. **Registration to host (maintainer, 2026-11-17: "maybe registration is needed for
   'serving' a game, so that moderating and banning will be possible").** Joining stays
   open; hosting a listed race needs a registered identity the lobby can moderate. Options,
   lightest first (chosen, not measured; none built):
   - **A key pair per player**, made on first use and kept in the profile; the host signs
     its registration and the lobby knows it by the public key. No email, no password. A
     ban is by key (plus address as a speed bump); a banned player can make a new key, so
     this alone stops only casual abuse.
   - **A key bound to an account** the operator approves or that comes from a platform
     login (Steam, for example): costlier to replace, so a ban holds. Needs an account
     store on the server, which the roadmap so far has none of.
   - Moderation the lobby needs either way: a report button on a player-hosted room, an
     operator command to delist a room and ban a host (and list bans), a log of who hosted
     what and when, and a reason shown to a banned host. The ban list lives with the lobby
     (a file on the server for version one).
   **Decided 2026-11-17 (maintainer): the key pair per player is enough for version one,**
   accepted with its known weakness (a banned player can make a new key). Accounts stay
   outside the roadmap.
8. **Anti-cheat for a player host (agreed with the maintainer 2026-11-17; none built).**
   Every client already re-simulates the race from the confirmed inputs and compares state
   hashes with the host every tick (server-roadmap decision 8), so:
   - **Simulation cheats are detected by one honest client.** A host that alters the
     simulation (speed, shield, a pickup) disagrees with every honest client on that tick.
     For a player-hosted race, a disagreement that survives decision 8's resync becomes a
     **host report** to the lobby, not only a drop.
   - **A report is a fraud proof, judged by the lobby.** The host signs every confirmed
     tick record it broadcasts; each player signs their own inputs with their key pair
     (item 7). A client that disagrees sends the lobby the signed records and inputs up to
     the tick and the host's claimed hash; the lobby, which has the content, re-simulates
     (about a second for a five-minute race) and decides. This works with two players,
     where a vote cannot, and a lying client cannot forge the host's signature. The
     verdict and the proof are the moderation evidence (bans, item 7).
   - **A quorum is only extra weight** when the lobby cannot re-simulate (content it does
     not carry): several clients agreeing with each other against the host.
   - **Input-level cheats pass every hash check**, because the simulation of a manipulated
     input stream is correct, so each needs its own measure:
     - forging another player's input: rejected by the input signatures;
     - delaying or dropping a rival's inputs ("packet loss"): clients measure how long
       their own inputs take to be confirmed against their round trip and attach the
       statistics to a report; evidence, not proof;
     - choosing the race seed to rig pickup rolls: a commit-reveal seed (each player sends
       a hashed random part, then reveals it; the seed combines all of them);
     - the host's zero latency and first look at everyone's inputs: inherent, disclosed by
       the PLAYER-HOSTED mark, not fixed.
   - **Not in reach of any comparison:** bots and clients reading their own copy of the
     race, as on server races.
   Terms, for reading elsewhere: this is state verification with fraud proofs (optimistic
   execution checked after the fact), not Byzantine consensus; full BFT ordering would
   replace the single host with peer-to-peer lockstep, which costs every player latency.
9. **Strings and screens.** The lobby's room list gets the mark and a short explanation;
   rows need `string_id` and English text (CLAUDE.md, `just check-strings`).

### Next steps then

1. After `net-mem` lands: confirm the two-race cap and whether starts must be staggered.
2. A design pass (sonnet `oag-docs`, 2 h) that picks the reachability option and the
   lobby-side host registration over the V2 host protocol, as a section of the server
   roadmap; then wire lanes.
