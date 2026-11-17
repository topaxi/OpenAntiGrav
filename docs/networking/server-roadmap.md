# Online server roadmap: today, three steps, and the target

**A proposal (2026-11-17), not an ADR and not built.** It reviews the online server as
it is in `crates/net`, describes the version worth aiming at, and cuts the way there into
three versions that each ship alone. It changes no code. Every design choice here is
**chosen, not measured** unless a number sits beside it; every number says whether it was
measured (and where) or estimated. The netcode it builds on is still Proposed
([ADR-0069](../architecture/adr/0069-remote-craft-are-predicted-in-one-timeline-and-corrected-on-a-threshold.md),
[ADR-0071](../architecture/adr/0071-weapons-are-predicted-simulation-with-a-discrete-state-correction-trigger.md));
the transport is [ADR-0070](../architecture/adr/0070-the-network-transport-is-webtransport.md).
Background: [networking](README.md), [the online lobby](online-lobby.md).

## Decisions asked of the maintainer, ranked

Each line says what it would become once accepted.

**Answered 2026-11-17 (maintainer):**

- **1 accepted**: the input journal is the session.
- **2 accepted, with the hand-back shown as a pickup**: once a returning player is
  reconnected, the HUD shows a fake **Autopilot** pickup with a countdown of 2 to 3
  seconds while the craft is handed back (the exact length is to be chosen). A pickup
  the player held before the disconnect is held again, usable, once that autopilot ends.
- **3 accepted**: one race, one process.
- **4 accepted**: the wire is frozen at the first public deployment; builds race together on an equal simulation fingerprint.
- **5 accepted**: one small flat-traffic VPS through V3.
- **6 accepted, a CA certificate for a host name**, probably from Let's Encrypt.
- **7 accepted as immediate relay** (target version, after V3): the server forwards each input when it arrives, marked unconfirmed, and **the first record for a seat and tick wins** (an identical resend changes nothing; a revised record for a tick already received is ignored, so seeing others early lets nobody rewrite their own past; today the last one wins, `host.rs:250`). Latency-scaled relay was weighed and declined: it is never better than immediate for anyone and only removes the low-ping lead. Measure corrections a minute before and after.
- **8 accepted, resync from the newest matching snapshot**: the client keeps a snapshot of its confirmed state about once a second and, on a hash mismatch, replays from the newest snapshot whose hash matched the server; if the same tick disagrees again it retries once from tick 0 (a divergence can sit in an unhashed field before any hash shows it); a third disagreement tells the player by name, hands the seat to the AI and logs tick, build and platform on the server.

1. **The input journal is the session.** A race is its `RaceParams`, its seat table and
   the append-only list of confirmed ticks. Whoever holds those can resume it: a player
   whose link dropped, a restarted race process, a spectator. No state ever crosses the
   wire. This is the mechanism behind V1 and V2. *New ADR.*
2. **A returning player takes the craft back from the AI.** Today a seat that leaves is
   retired for good. The alternative is to keep that and only let the player watch.
   *Part of the ADR of decision 1.*
3. **A race stays one process, and a host of race processes is an agent the lobby talks
   to.** Not rooms as threads of one process. Measured below: a race process costs 32 to
   36 MB and loads in under a second on every title, so memory does not argue for
   sharing. *New ADR; it settles the "Hosting model" section of the networking page.*
4. **The wire stops being edited in place** at the first public deployment. From then a
   change bumps `PROTOCOL`, and the "same build" rule becomes "same simulation
   fingerprint". *New ADR.*
5. **Hosting: stay on one small flat-traffic VPS through V3**, and step up on the triggers
   in "Infrastructure". *No ADR; an operations page when it is built.*
6. **A stable server identity**: a key kept on disk and rotated inside 14 days, or a CA
   certificate for a host name. The second needs a domain. *Supersedes ADR-0070's
   Proposed decision 3.*
7. **Early relay of unconfirmed inputs** (target version only). It shortens what a
   client has to guess by the server's buffer, and it gives a low-ping cheat a look at
   the others' next two to five ticks. *A follow-up ADR to 0069, if wanted at all.*
8. **A desync is resynced once by replay, then the player is told and dropped.** *Part
   of the ADR of decision 1.*

## Today (review, 2026-11-17)

### Topology and authority

- **Server-authoritative, inputs only.** The server runs the canonical race and sends,
  per tick, the input it applied to each person and the state hash it reached
  (`crates/net/src/host.rs:386-422`, `crates/net/src/wire.rs:10-17`). A client re-runs
  them and checks the hash (`crates/net/src/client.rs:602-649`).
- **One lobby process, one race at a time.** The lobby holds a single
  `race: Option<Running>` (`crates/net/src/online/server.rs:63`); a second room is queued.
- **A race is a child process** of the same binary (`crates/net/src/online/race_proc.rs:383-396`).
  It loads the race when the room starts (`crates/net/src/bin/oag_server/race_cmd.rs:54`),
  mints its own certificate and listens on the one race port
  (`race_cmd.rs:60-63`, `crates/net/src/bin/oag_server/lobby_cmd.rs:26-28`).
- **Players reconnect to it.** The lobby sends a port, a certificate hash and a seat token
  (`server.rs:327-339`); the client dials the host it already knows on that port
  (`crates/game/src/online.rs:83-91`).

### Netcode

- **Server input buffer.** Per seat, inputs by tick (`host.rs:46`); an empty buffer repeats
  the last input (`host.rs:394-399`); an input for a tick already run is dropped
  (`host.rs:242-249`).
- **Redundancy.** Each client datagram repeats 8 ticks (`client.rs:73`); each server
  datagram repeats up to 16 (`host.rs:35`), fewer when they do not fit (`host.rs:441-442`).
- **Prediction.** Every craft in one present timeline, other people on their last
  confirmed input (`client.rs:462-469`); a correction when a craft is more than 1 unit off
  (`client.rs:53`) or the weapon digest differs (`client.rs:676-694`).
- **Lead control.** The client steers its lead toward 2 ticks by running an extra tick or
  skipping one, at most every 12 ticks (`client.rs:61-68`, `client.rs:501-513`).
- **A stalled link.** Past 30 ticks ahead of the confirmed state the client stops and
  waits (`client.rs:71`, `client.rs:498-500`).
- **A hash that disagrees is logged and nothing else** (`client.rs:635-644`). The server
  is never told, and the client races on in a world that is no longer the server's.

### Robustness: what fails, and how

| Event | What happens today | Where |
| --- | --- | --- |
| A player's link drops mid-race | The seat is retired to the AI for good; the client's race is over | `host.rs:189-192`, `host.rs:425-432`, `client.rs:443-447` |
| A player is silent for 8 s | Same, with a message | `crates/net/src/serve.rs:21`, `host.rs:461-479` |
| The player comes back | Refused: "the race has started" | `host.rs:345`, `host.rs:366` |
| The race process crashes | The room returns with the exit status; the race is lost | `race_proc.rs:311-313` |
| The race process hangs | Nothing notices: after `Up` the lobby only waits for the exit, so the one slot is held | `race_proc.rs:330-354` |
| The lobby dies | Its race child stops within seconds; rooms, names and tokens were in memory | `race_cmd.rs:87-97`, `server.rs:59-61` |
| The server restarts | A new certificate, so a new hash to hand out | `server.rs:85`, `crates/net/src/wt.rs:61-79` |
| A client drops while its race loads | The takeover does not re-send the hand-off; it times out at the child | `server.rs:212-232` |
| A client of another build | Refused by name at the lobby; a bare race server checks only `PROTOCOL` and the tick-0 hash | `host.rs:276`, `host.rs:196-214` |
| A message that does not parse | Refused by name, one warning a message, no limit on warnings | `wire.rs:500-540`, `host.rs:228-229`, `host.rs:254-255` |
| Inputs for far-future ticks | Stored: any tick at or past the next is inserted, with no ceiling | `host.rs:250` |
| A second input for one tick | Overwrites the first until the tick runs | `host.rs:250` |
| `Resend { from: 0 }` in a loop | The whole history is cloned onto the stream each time, unthrottled | `host.rs:215-223` |
| A slow or hostile reader | Outgoing and incoming queues are unbounded | `wt.rs:167-169` |
| Many connections or rooms | No cap on clients or rooms; a hello has 10 s | `server.rs:24`, `server.rs:233-247`, `crates/net/src/online/rooms.rs:468-494` |
| A stalled server process | It catches up in a burst of back-to-back ticks on repeated inputs | `serve.rs:69-75` |
| NaN or out-of-range axes | Clamped on both ends | `wire.rs:67-77`, `crates/gameplay/src/input.rs:86-102` |

**Cheating surface.** An input-only protocol cannot be made to accept a position, a
speed, a shield value, a weapon or a result: none of them is on the wire, and the board
is the server's (`host.rs:521-525`). It cannot prevent a program that plays well, or a
client that reads its own full copy of the race (every craft, the seeded `Rng`, so the
next pickup roll). Late input gains nothing: the server has already repeated the last
one. The table in [networking](README.md#cheating-what-is-and-is-not-worth-defending)
stands.

### Performance (measured 2026-11-17, localhost)

**Method.** The lean release `oag-server` (built alone, so `view` is off) and release
`oag-game --connect --net-bot --no-audio --size 640x360 --screenshot F --ticks N` clients
on one machine (16 hardware threads, shared; 1-minute load average 11 to 23 during the
runs, so read CPU as inflated). 3,600 ticks, the AI on the rest of the grid, weapons on.
Server CPU is user plus system time from `/proc/PID/stat` over the racing seconds; memory
is `VmRSS` from `/proc/PID/status`, sampled each second; bandwidth is the client's own
once-a-second line. One run per row. 0 bad hashes in every run.

| Title (source) | People | Server CPU, share of one core | Server RSS, waiting / after 60 s | Load to listening | Client down / up |
| --- | --- | --- | --- | --- | --- |
| Pulse PSP (`.chd`) | 2 | 3.7 % | 18 MB / 34 MB | 0.5 s | 50 KB/s / 7.7 KB/s |
| Pulse PSP (`.chd`) | 4 | 4.3 % | 18 MB / 32 MB | 0.5 s | 83 KB/s / 7.7 KB/s |
| HD (PS3 `.iso`) | 2 | 3.1 % | 17 MB / 32 MB | 0.2 s | 50 KB/s / 8.9 KB/s |
| Omega (PS4 folder) | 2 | 3.1 % | 29 MB / 36 MB | 0.2 s | 51 KB/s / 8.9 KB/s |
| 2048 (Vita folder) | not raced | not measured | 26 MB / not measured | 0.2 s | not measured |
| Pulse PS2, Pure PSP | not raced | not measured | 18 MB, 16 MB / not measured | 0.4 s, 0.5 s | not measured |

About 9.5 MB of each figure is the binary's own file-backed pages, which race processes
of one binary share. The server has 4 threads: the tick loop, the log and two for the
transport (`wt.rs:35-45`).

What it says:

- **Memory per title is not a constraint.** HD and Omega cost what Pulse does. This
  answers "HD, Omega and 2048 memory on a 1 GB host unknown" for HD and Omega on
  localhost; 2048 was loaded and not raced. The remote figure of 2026-10-16 (9.5 MB, a
  static musl build from an extraction, no AI, no weapons) is lower still.
- **The simulation is about a tenth of the server's CPU** (estimated: 60 us a tick from
  the networking page is 0.36 % of a core, against 3.1 to 4.3 % measured). In the
  4-person run the tick loop's thread and the two transport threads used about equal
  time (1.43 s and 1.50 s over the process's life). The tick loop polls every link up to
  500 times a second (`serve.rs:72`), and every tick allocates: a `Vec` per record
  (`host.rs:389`), a clone of up to 16 records per seat (`host.rs:452`) and a fresh buffer
  per datagram (`wire.rs:546-547`). That is about 9,000 small allocations a second in a
  full room (estimated by reading).
- **A client's download is bound by the datagram, not by the room.** A record is
  `9 + 20 * people` bytes (`wire.rs:360-362`). At 2 people 16 records fit (50 KB/s); from
  4 people on the datagram is full (83 KB/s) and the redundancy shrinks instead: 8 records
  at 8 people, 133 ms of loss cover where 2 people have 267 ms (estimated from the fit
  rule, `host.rs:441-442`). A full room sends about 660 KB/s, 5.3 Mbit/s, 2.4 GB an hour
  (estimated: 8 times the measured 83 KB/s).
- **The server's buffer is most of a good link's latency.** On localhost the round trip
  is 0.1 to 0.3 ms and a tick comes back confirmed after 66 to 83 ms (measured, the
  `confirm` figure): 4 to 5 ticks of buffer and pacing.
- **The history grows for the whole race** (`host.rs:91`): about 23 MB for a 30-minute
  race of 8 people (estimated: 108,000 records of about 216 bytes).
- **A correction** costs 0.4 to 0.65 ms and about 0.1 percent of a client core
  (measured, [networking](README.md#replay-cost-on-this-machine-2026-10-19)). It is not
  where the cost is.

### Operations

One static binary and one extraction per title; started by hand; a line a second per race
on standard output (`serve.rs:77-80`, `serve.rs:111-126`); no metric, no status command,
no restart on failure. The wire is `PROTOCOL = 1`, edited in place (`wire.rs:24`). The
client's own quality figures (corrections, stalls, round trip) stay in the client's log.

### What a 150 ms player sees today

Estimated from the tables of the networking page, taking 150 ms of round trip plus the
server's 3 ticks as a 12-tick window (the "200 ms" rows). Their own craft answers at
once. Other people's craft are drawn in the present from their last input, about 1 to
1.9 units off at the 95th percentile (a craft is 13 long), and are corrected about 2.5
times a second with a visible step of up to about 5 units, glided over 100 ms. Another
person's shot first appears about 13 to 15 ticks into its flight. A hit on their own
craft shows that late too. The client spends about 1.5 times the simulation.

## Lobby and race server: the boundary

### Today's boundary

The responsibilities are already split; the deployment is not.

- **Rules with no I/O.** `rooms.rs` is a state machine (`rooms.rs:1-7`). A race to bring
  up comes out as a `Launch` (`rooms.rs:68-75`, `rooms.rs:244`) and its end goes back in
  through `race_over` and `launch_failed` (`rooms.rs:850`, `rooms.rs:874`).
- **A seam.** `Runner::launch` takes a `RaceJob` and returns a `Handle` whose `poll` says
  `Starting`, `Up { port, cert_hash }` or `Over` (`race_proc.rs:27-39`,
  `race_proc.rs:62-75`). The lobby service knows nothing else about a race
  (`server.rs:288-349`).
- **The job down.** The race, and a name and a fresh seat token per seat, as one hex line
  on the child's standard input (`crates/net/src/wire/online.rs:295-300`,
  `race_proc.rs:397-401`, `server.rs:292-303`).
- **The status up.** Text lines on the child's standard output: `listening on`,
  `cert-hash`, `race-board`, `race-over` (`race_cmd.rs:62-63`, `race_cmd.rs:79-80`,
  `race_proc.rs:323-329`, `race_proc.rs:304-310`).
- **Crash.** The exit status and the last `error:` line (`race_proc.rs:311-313`); a child
  that never listens is killed after the load timeout (`race_proc.rs:355-362`).
- **Liveness, one way.** The child stops when the lobby's pipe closes (`race_cmd.rs:87-97`).
  The lobby has no heartbeat from the child.
- **The race side owns** the content load, its listener and certificate, the seating by
  token (`host.rs:278-286`), the tick loop and the board.

What still ties the two to one machine:

1. The hand-off carries a port and no host (`wire/online.rs:272`), and the client reuses
   the host it dialled (`crates/game/src/online.rs:88-91`).
2. The protocol is a parent's pipes to its child.
3. The lobby opens every image itself to learn the titles (`lobby_cmd.rs:71-85`) and
   passes an image path to the child (`race_proc.rs:377-382`), so the lobby needs the
   game content.
4. One slot (`server.rs:63`).
5. A lobby that stops takes its race with it.

### The boundary recommended

| | The lobby owns | A race host owns |
| --- | --- | --- |
| People | Names, session tokens, rooms, settings, kick | Nothing but seat tokens it was given |
| Placement | Which host runs a room; the queue when none is free | How many rooms it can take, and which titles |
| Content | None. It learns the titles from the hosts | The extractions |
| The race | The roster and the result | The process, the simulation, the journal, the board |
| The address players dial | Its own, first | Its own, in the hand-off |
| Lifetime | May restart without ending a race | May finish its races with no lobby |

**The protocol between them** is one small set of typed messages (a "host protocol"),
the same whether it runs over a child's pipes or a network link:

- host to lobby: `Register { host, address, region, editions, slots, build }`,
  `Up { room, port, cert_hash }`, `Failed { room, why }`,
  `Health { room, tick, seats connected, repeated, late }` once a second,
  `Over { room, reason, board }`;
- lobby to host: `Launch { room, job }`, `Stop { room }`, `Drain`.

A room whose `Health` is missing for 5 s (chosen) is hung or dead: the lobby asks the
host to stop it and resume it from its journal (V2), and tells the room. A host that
loses the lobby keeps racing, keeps its `Over` messages and registers again.

**One binary throughout.** `oag-server lobby` stays the one-box form: the lobby with a
built-in host that runs race children. The split form is `oag-server lobby` on one
machine and `oag-server host --lobby ADDRESS` on others, the same host code reached over
a reliable `Link` instead of pipes, with a shared secret in its `Register`.

How each version moves there is in the version sections; in short V1 adds `Health` and
the host field, V2 makes the host a component with several slots and a journal, and the
target puts it on other machines.

## The target version

One line: **an authoritative input sequencer per race, a process each, whose journal is
the session, placed by a lobby that owns people and no content, on hosts that own
content and no people.**

### Topology and authority

- Server-authoritative, inputs only, as today. Lockstep waits on the slowest of eight.
  Peer-to-peer rollback needs every pair to reach each other and has no authority for
  the result. Neither is considered further.
- **A race is a process.** Measured above: 32 to 36 MB and under a second to load, on
  every title raced. A crash, a leak or a hang ends one race, and the kernel cleans up.
- **Rooms as threads of one process are not chosen.** They would save the per-process
  memory, which is not the constraint, and would let one abort end every race. The one
  thing they offer is a single UDP port; a host with one open port can run one race.
- **A relay with no simulation is not chosen** either. It would need no game content on
  a rented machine and no CPU for the race, and it would lose the board's authority, the
  server-side desync check and the finish rule. It stays a fallback worth knowing: the
  server's simulation is a tenth of its cost.
- **Several hosts, by region.** The lobby places a room on a host near its people
  ("Infrastructure").

### Netcode

- **Prediction and correction as ADR-0069 and ADR-0071 have them.** The measured cost is
  small and the threshold policy holds for every input stream tried.
- **The journal is the session** (decision 1). The server appends each confirmed tick to
  a file. Three things read it:
  1. *A player who dropped.* The client reconnects with its seat token. If it still has
     its confirmed state it asks for the ticks since; if it restarted it loads the race
     and asks from tick 0. Replaying 5 minutes is 18,000 ticks, about 1.1 s on a desktop
     (estimated from the 60 us tick) and about 3 MB on the stream for 8 people before
     compression (estimated). The seat's craft is the AI's while they are away and
     theirs again from a tick the server names (decision 2).
  2. *A race process that died.* A new one loads the race, replays the journal checking
     every hash, listens, and the lobby hands the players the new address. They stall
     for a few seconds and race on.
  3. *A spectator or a late joiner*, the same way as a restarted client, with no seat.
- **Desync.** A client reports the first tick whose hash disagrees. It then rebuilds its
  confirmed state by replaying from tick 0 once. If the same tick disagrees again the
  cause is content or code: the player is told so by name and leaves the race, and the
  server's log has the tick, the build and the platform. That log is the project's
  determinism check on real machines.
- **An adaptive buffer.** The server tells each client the lead it needs from the
  arrival jitter it sees, 1 tick on a clean link. The client reaches it by running its
  clock a percent or two fast or slow, not by doubling or skipping a tick. On a good
  link this takes about 30 ms off the 66 to 83 ms measured above (estimated).
- **Early relay** (decision 7, optional). The server forwards a person's inputs to the
  others when they arrive, 2 to 5 ticks before it applies them, so a client guesses
  fewer ticks. Estimated a third fewer replayed ticks at 100 ms. With it, the first
  input to arrive for a tick must win (today the last one does, `host.rs:250`).
- **Weapons and discrete events**: unchanged, ADR-0071. The first-sight cap follows the
  window, so a shorter window shows a shot sooner.
- **Loss, jitter, reordering.** Redundancy by acknowledgement, not by a fixed count: each
  side repeats what the other has not confirmed receiving, up to a ceiling of 30 ticks.
  A full room keeps its loss cover (today it halves at 8 people).
- **A 150 ms player** sees what they see today with a window about 2 ticks shorter
  (estimated: 10 ticks in place of 12, the "100 ms" to "200 ms" rows), and no longer
  loses the race to a ten-second dropout.

### Robustness

- Reconnect, crash resume and hand-over to another process: the journal, above.
- **Bounds on everything a peer controls**: inputs at most 120 ticks ahead, one input
  per tick, a datagram budget of twice the tick rate, one resend request a second and a
  bounded range, bounded queues that close a link that does not drain, a cap on
  connections per address and on rooms per person, one log line a second per peer. All
  chosen.
- **A slipped tick is counted, not replayed in a burst**: a server more than 3 ticks late
  re-anchors its clock and reports it.
- **Versions.** `PROTOCOL` plus a simulation fingerprint: a hash of the simulation
  crates' source, made at build time. Two builds that differ only in the renderer or the
  menus race together; today any other build is refused.
- **Identity**: decision 6.
- **Cheating**: unchanged in kind. Early relay, if taken, adds the one look-ahead named
  in decision 7.

### Performance

- No allocation in the tick: fixed arrays per record, a ring of the recent ticks in
  memory and the rest in the journal, one encoding per tick shared by every seat.
- The tick loop sleeps until a datagram arrives or the next tick is due.
- A compact wire: an input as a change mask and 8-bit axes (a pad's own resolution),
  ticks sent by acknowledgement. Estimated 15 to 20 KB/s down per client in a full room,
  against 83 measured today.

### Operations

- One static binary, one systemd unit, a deploy script, a `status` subcommand.
- A line of metrics per race every 10 s and one summary at its end, in a form a script
  can read: ticks, slipped ticks, each seat's fresh, repeated and late inputs, reconnects,
  desync reports, bytes. Each client sends its own summary at the end (corrections,
  stalls, round trip), so the operator sees what the players saw.
- The journal of a finished race is a replay
  ([ADR-0055](../architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md)); kept
  for a day, it is also the evidence for a desync report.

### What it costs and what it gives up

- About 150 hours for V1 to V3 and about 110 more for the rest of the target (hosts on
  other machines, spectators, the simulation fingerprint, early relay). A web client is
  not counted; it is its own project.
- A player cannot rejoin across a server update: the journal replays only on the build
  that wrote it.
- A resumed race stalls every player for a few seconds.
- A second machine and a second address to keep reachable once hosts are split.
- 8-bit axes online, where a local race reads the device's full value. Both ends apply
  the wire value, so nothing desyncs; a network race quantises the stick.

## The versions between

### V1: survive the network (about 45 hours)

**What changes.**

0. The lobby keeps a seated person's session 30 s after their link closes, so a reconnect gets the hand-off (`SEAT_GRACE`).
1. A seat can be taken back. The host seats a hello whose token names a seat that left,
   during the race; `TickRecord.retired` becomes the seats away *now*, not ever; the
   client reconnects by itself and asks for the ticks it lacks. No journal file yet: the
   history in memory serves it.
2. Bounds and limits: the list under "Robustness" above, on the race host and the lobby.
3. The client reports a hash mismatch and its end-of-race summary; the server logs both.
4. `Health` lines from the race child, and the lobby stops a child that goes quiet.
5. A session takeover re-sends a pending hand-off. The hand-off gains an optional host.
6. The refusal-before-close ordering is made certain (the lobby waits for the refusal to
   be acknowledged or for 1 s before closing).

**Why first.** A dropped link ending the race is the failure a real network shows most,
and nothing here changes the topology or the encoding of a tick. Everything later needs
the limits.

**A player notices.** A dropout of a few seconds no longer ends their race: the AI holds
the craft and they get it back. A refusal always says why.

**Acceptance.** New tests, red today:
- `loopback`: a person whose link closes at tick 600 and returns at tick 900 flies
  their craft again, and both ends agree on every hash. Today the host answers "the race
  has started" (`host.rs:366`).
- `loopback`: 10,000 inputs for far-future ticks leave the seat's buffer under 120
  entries; 100 resend requests in a second produce one answer.
- `online`: a client that drops while its race loads and takes its session over receives
  the hand-off.
- `online`: a hung race child (the test's fake child stops printing) frees the slot
  within 10 s.
- The refusal-before-close test passes 200 runs in a row under load.

**Risks.** Handing a craft back from the AI changes hashed state on a named tick, on
both ends; the AI driver's state for that slot must be built the same way each time.
`PROTOCOL` becomes 2. A restarted client on a slow phone replays about ten times slower
than a desktop (unmeasured).

**Landed (items 2, 4, 5, 6 in part; no `PROTOCOL` change).** The host drops an input more than
119 ticks ahead (`MAX_INPUT_AHEAD`, a seat buffers under 120) and answers one resend request a
second (`RESEND_INTERVAL`). The race process prints `health: ...` once a second while its people
load, and its racing loop prints its progress line once a second; the lobby takes any line as a sign
of life and stops a child that is up and silent for `QUIET_TIMEOUT` (5 s, chosen), freeing the slot
with the reason "went quiet". The lobby sends the hand-off again to a session taken over after it
went out. A dropped `WtLink` finishes its stream and waits up to 1 s for the acknowledgement before it
closes, and the close is reported only by the reliable reader. Tests: `loopback::a_peer_cannot_grow_a_seats_buffer_with_inputs_for_far_future_ticks`,
`loopback::a_hundred_resend_requests_in_a_second_get_one_answer`, the `online` tests for the quiet child
and the takeover. Not landed: item 3 and the hand-off's host.
The `Health` line is text on the child's output, not a typed message; the typed form arrives with the
host protocol in V2.

**Landed (item 2, the rest: no `PROTOCOL` change).** No single peer, address or room can grow the
server's memory or log volume without bound. All values are chosen, not measured.

| Limit | Value | Where |
| --- | --- | --- |
| Lobby connections in all | 64 (`server::MAX_CONNECTIONS`) | `Server::accept`, refused with `Refusal::Full` and the reason |
| Lobby connections per address | 16 (`server::MAX_PER_ADDRESS`) | same; counted live over joining and joined, so nothing leaks |
| Race connections awaiting a hello | 48 (`host::MAX_JOINING`), oldest closed first | `Host::join` |
| Time to say hello (race) | 10 s (`host::HELLO_TIMEOUT`) | `Host::poll_joining` |
| Handshakes under way | 64, each at most 10 s; the next gets a 503 | `wt::Listener::bind` |
| Sessions accepted and not yet taken | 256, the next is closed | same |
| Reliable message | 256 KiB (was 1 MiB) | `wt::MAX_RELIABLE` |
| Reliable messages unread by the owner | 4; the reader then waits, which stops the peer through QUIC flow control | `WtLink` |
| Datagrams unread by the owner | 256, the next is dropped | `WtLink` |
| Datagrams a link accepts | 120 a second with a burst of 120 (twice the tick rate) | `WtLink` reader, before the queue; the keep-alive counts |
| Bytes queued to send | 1 MiB on a lobby link, 16 MiB on a race link (a resend from tick 0 of a 17 minute race at twelve seats, 249 bytes a tick); past it the session is closed, reason "does not drain" | `wt::LinkLimits` |
| A repeated warning | one line a second per seat, per lobby client, per race host (refusals) and per listener (handshakes); the next line says how many were held back | `limits::LogLimit` |

Rooms per person was already one (`Person::room`), and a room's members were already capped by its
`max_players`; neither was red. The lobby's connect, join and leave lines moved from `info` to `debug`:
a peer cycling connections would otherwise write a line each. The datagram budget sits in the transport,
not in `Host`, so no wall-clock decision reaches which inputs the simulation applies. The race host does
not see addresses (`Link` has none), so it has no per-address cap; its bound is `MAX_JOINING` and the
hello timeout, and the lobby's per-address cap guards the way in.

*Memory arithmetic* (the target is a 1 GB VPS). A link at its worst: unread reliable messages (4 queued
and one being read) 5 x 256 KiB = 1.25 MiB, unread datagrams 256 x 1.2 KiB = 0.3 MiB, plus the send
queue. Lobby: 64 x (1 + 1.25 + 0.3) = about 163 MiB. Race process: 12 seats x (16 + 1.25 + 0.3) = about
210 MiB, plus 48 awaiting a hello x 1.55 = about 74 MiB: about 284 MiB. Both processes at every bound at once
come to about 450 MiB, before QUIC's and tokio's per-connection state (unmeasured) and the race's own
tracks and models (HD, Omega and 2048 on this host are unknown, above). A normal session sits at kilobytes;
the bounds are the ceiling a hostile peer reaches, not a forecast. The 16 MiB race queue is the one large
term: it exists so a legitimate late resend is not mistaken for a peer that does not drain; the journal
(V2) retires it.

Tests, each red with its limit disabled and green with it: `online::one_address_may_hold_only_its_cap_of_lobby_connections_and_the_others_are_still_served`,
`online::the_lobby_holds_only_its_cap_of_connections_whatever_their_addresses`,
`online::a_peer_that_never_reads_is_closed_at_the_send_queue_limit_and_the_other_is_still_served`,
`online::a_flood_of_datagrams_is_cut_to_the_budget_and_the_other_peer_is_still_served`,
`online::sessions_nobody_takes_are_bounded_and_the_listener_keeps_accepting`,
`loopback::hundreds_of_silent_connections_are_bounded_and_the_expected_seat_still_joins`,
`loopback::a_connection_that_never_says_hello_is_dropped_after_the_hello_timeout`,
`loopback::ten_thousand_malformed_datagrams_from_one_seat_log_at_most_a_line_a_second`. Not tested:
the 64 concurrent handshakes (no way to hold one open from a client) and the handshake timeout.

**Boundary.** `Health` and the host field in the hand-off. Still pipes, one slot, one box.

**Hosting.** The existing 1 GB VPS, unchanged.

### V2: several races, and a race that survives its process (about 55 hours)

**What changes.**

1. The host component: `slots` race children at once, each on its own port from a range.
   The room queue is used only when every slot is taken.
2. The host protocol as typed messages over the pipes, replacing the text lines.
3. The journal on disk, appended each tick and flushed each second. A race child that
   dies or hangs is replaced by one that replays the journal; the lobby re-sends the
   hand-off.
4. The lobby's identity is kept on disk, so a restart does not change the hash. The
   lobby writes its rooms and tokens on change and reads them at start; race children
   outlive it and are adopted again.
5. A systemd unit, a deploy script, `oag-server status`, the metrics lines.

**Why second.** It needs V1's reconnect (a resumed race is every player reconnecting at
once) and V1's limits (several rooms multiply every unbounded queue). It is what lets
more than one group play.

**A player notices.** No "race in progress" queue. A server fault is a stall of a few
seconds, not a lost race. The server's address and hash stay the same across restarts.

**Acceptance.**
- `online`: two rooms race at the same time on two ports. Today the second is told it
  is queued.
- `net_ground_truth`: a race child killed with SIGKILL at tick 600 is resumed; both
  clients finish the race with 0 bad hashes and the same board.
- `net_ground_truth`: the lobby killed and started again mid-race; the race finishes
  and the room shows its board.
- Measured on a 1 GB VPS: three two-person rooms at once with no repeated input on the
  server's lines.

**Risks.** A host that lets only a few UDP ports through caps the slots (on the remote
host tried, ports 60000, 60001, 60002 and 60010 arrived; the range between was not
probed). A resume that replays a journal written by another build must refuse by name.
A journal is game input, not game content, but it sits on a rented disk: keep a day.

**Boundary.** The host is a component with its own state and protocol; the lobby no
longer needs to open the images if the host reports its editions. Still one box.

**Hosting.** The existing 1 GB VPS. Rooms that fit: see "Infrastructure".

### V3: cheaper per room (about 50 hours)

**What changes.**

1. Wire version 3: ticks sent by acknowledgement, inputs as a change mask and 8-bit axes.
2. No allocation in the server's tick; the tick loop woken by arrival, not by a 2 ms poll.
3. The adaptive buffer and the client's clock steering; slipped ticks counted.
4. The localhost measurement of this page as a kept sweep (`OAG_SWEEP=1`), so the
   before and after are one command.

**Why third.** It is the only version that touches the encoding of a tick and the
client's timing, the two things most able to break a race that works. With V2's metrics
in place its effect can be read on the real host. It pays off only when rooms fill.

**A player notices.** A full room uses a fifth of the data (it matters on a phone). Good
links feel a little tighter. Nothing else.

**Acceptance.**
- The sweep: a client in an 8-person room receives at most 20 KB/s (today 83 KB/s
  measured at 4 people, where the datagram is already full).
- The sweep: the server's CPU for the 4-person Pulse run is at most half of this page's
  4.3 % on the same machine at a like load.
- The sweep: `confirm` on localhost at most 40 ms (today 66 to 83).
- `loopback`: an 8-person room keeps 16 ticks of loss cover (today 8).
- The loss and jitter sweeps of the networking page show no more corrections and no
  later first sight than before.

**Risks.** 8-bit axes change what a network race feels like against a local one; this is
a maintainer's call and can be dropped from the version at a cost of about 10 KB/s.
Clock steering interacts with the frame pacing of a windowed client.

**Boundary.** Unchanged.

**Hosting.** The existing 1 GB VPS, with about twice V2's rooms by CPU (estimated).

### After V3, toward the target

In the order they are worth doing: race hosts on other machines and region placement
(about 40 hours), spectators and late joiners from the journal (30), the simulation
fingerprint (12), replay download and a server-side record check (15), early relay if
decision 7 is yes (15). Each is its own lane.

## Infrastructure

### What a room costs

From the measurements above and the remote run of 2026-10-16
([the online lobby](online-lobby.md#measured-over-the-internet-2026-10-16)).

| | Value | Kind |
| --- | --- | --- |
| Memory per race process, any title raced | 32 to 36 MB after a minute; budget 40 MB | measured, localhost |
| Memory, lobby | 6 MB | measured, remote |
| CPU per two-person room on a 1 GB, 1 vCPU VPS | 15 to 20 % of the core | measured, remote, no AI and no weapons |
| CPU per two-person room, desktop | 3.1 to 3.7 % of a core | measured, localhost, AI and weapons on |
| CPU per extra person, desktop | about 0.3 points | measured, one pair of runs |
| CPU per full room on that VPS | about 27 % | estimated: the desktop figures times the remote-to-desktop ratio of about 5 |
| Egress per full room | 5.3 Mbit/s, 2.4 GB an hour | estimated from 83 KB/s a client |
| Egress per two-person room | 0.8 Mbit/s, 0.36 GB an hour | measured, 50 KB/s a client |
| Disk per title | 5 to 182 MiB extraction | measured, the online lobby page, part 6 |

### Rooms on a 1 GB VPS

Taking 700 MB as free for races and 80 percent of the one core as the ceiling. All rows
are estimates from the table above; only Pulse was ever raced remotely.

| Title | By memory | By CPU, full rooms | By CPU, two-person rooms | Binding |
| --- | --- | --- | --- | --- |
| Pulse (PSP, PS2), Pure | 17 | 3 | 4 to 5 | CPU |
| HD | 17 | 3 | 4 to 5 | CPU |
| Omega | 17 | 3 | 4 to 5 | CPU |
| 2048 | 17 (from the waiting figure) | not measured | not measured | CPU, presumably |

Today and in V1 one race runs at a time by design, so the box is far from any limit. V2
can use 3 slots there. V3 should about double the CPU rows.

### What each version needs

| Version | Hosting | Memory, vCPU | Bandwidth | Regions | Monthly cost class |
| --- | --- | --- | --- | --- | --- |
| Today, V1 | the existing 1 GB VPS | 1 GB, 1 vCPU | under 6 Mbit/s peak | one | what it costs now |
| V2 | the same | 1 GB, 1 vCPU, 3 slots | 16 Mbit/s peak, 7 GB per hour at 3 full rooms | one | the same |
| V3 | the same | 1 GB, 1 vCPU, about 6 slots | about 3 Mbit/s peak | one | the same |
| First step up | one larger VPS | 4 GB, 2 vCPU | flat allowance of 1 TB or more | one | about 5 to 6 USD or EUR |
| Target | a lobby on the first box, one race host per region | 2 to 4 vCPU a host | flat allowance | two or three | about 5 a host |

### What triggers a step up

- The server's own lines show repeated inputs or slipped ticks while the clients' links
  are clean: the core is full. V2's metrics make this readable.
- Rooms wait in the queue at the evening peak on more than a few days a month.
- The month's traffic passes 80 percent of the plan's allowance. One full room racing
  four hours a day is about 290 GB a month before V3 (estimated).
- A regular group plays from where the round trip is over 150 ms. The measurements say
  150 ms plays; past it the window nears the 30-tick stall.

A title's memory per room is not a trigger: no title measured needs more than 40 MB.

### From one box to several

1. **One box** (today to V3): `oag-server lobby` with its built-in host.
2. **A larger box**: the same, more slots. Nothing changes but the plan.
3. **A second box**: `oag-server host --lobby ADDRESS` on it. The lobby places a room on
   the host with a free slot and the right title; the hand-off names the host's address.
4. **Regions**: each host registers a region. Each client measures its round trip to
   every host when it enters the lobby (one QUIC handshake each) and the lobby places a
   room on the host with the lowest worst round trip among its people. The room list
   shows the region. The lobby itself stays single: its traffic is menus, and 200 ms
   there is harmless.
5. **Matchmaking** (quick match) is a lobby rule and adds no machine.

### The connection path and network floods (measured, 2026-11-17)

The remote host's web side sits behind an HTTP CDN proxy. The game's WebTransport
traffic does **not** go through it: it is a direct connection to the host's own address.
Measured on 2026-11-17 (`online-lobby.md`, "Measured over the internet (2026-11-17)"):

- The host name the clients connect to resolves to the host's own IPv4 and IPv6
  addresses, and neither is in the CDN's published ranges. The names of the web side
  resolve into the CDN's ranges instead.
- Six pairs of game clients raced a lobby on that name (four over IPv6, which the client
  picks first when the machine has IPv6) and on the host's IPv4 address (two), with 0 bad
  hashes. UDP 60000 and 60001 arrive on both families.
- A client pointed at a CDN-proxied name (`<proxied name>:60000`) never connects: it prints
  `Error: connecting to <proxied name>:60000: no answer in 5s`, twice, after 5.1 and 5.4 s. The
  proxy does not forward UDP 60000-60010, as expected from an HTTP proxy.

So the "path is direct" case below applies, and the per-address caps see real player
addresses. Still open: whether the provider filters volumetric floods (not probed; a
flood against a shared host was not sent).

The consequences of a direct path:

- **The host's own address is public.** Every player connects to it, so the CDN does not
  hide the host, and a volumetric flood against the game ports reaches it unfiltered.
  The web side stays protected only if it neither shares that address nor answers on it.
- **The per-address caps see real player addresses**, which is what they need. Behind a
  proxy every player would share the proxy's addresses and the caps would not tell them
  apart.
- **Defences against a flood, cheapest first:** accept the risk while there are no
  players; the VPS provider's own network filtering (often included); a UDP-capable proxy
  or game-traffic relay in front of the race ports. A relay costs latency and, behind it,
  the caps need the client's address passed through.

### Where to host: price first, then ease

Prices were looked up on 2026-11-17 from comparison pages, not from each provider's own
page, and such pages disagreed with each other in places. Check before buying. Lines
marked "from memory" were not looked up at all.

| Model | Example and price | UDP and QUIC | Traffic | Regions | Upkeep |
| --- | --- | --- | --- | --- | --- |
| A small VPS, flat traffic | Hetzner CX22 (2 vCPU, 4 GB) about 4.80 EUR with 20 TB, stock reported short since mid 2026; Netcup entry plans 3 to 5 EUR on a 12-month term, IPv4 extra; OVH VPS-1 about 4.20 USD, unmetered; Contabo 8 GB about 5 to 5.50 USD with 32 TB, capped at 100 Mbit/s; Vultr 1 GB 6 USD with 1 TB | Plain sockets. A provider's firewall may filter ports, so probe first | Included | Mostly Europe and North America per provider | One machine to patch; a systemd unit and a copy |
| Machines billed by the second | Fly.io, a 1 GB shared machine about 5 to 6.70 USD running all month, a dedicated IPv4 2 USD | Yes, with a dedicated IPv4 and a special bind address (from memory, verify) | 0.02 USD a GB in Europe and North America: a full room hour is 0.05 USD | Many | No machine to patch; a container image; can stop when idle |
| Game server orchestration | Edgegap about 0.069 USD a vCPU hour plus 0.10 USD a GB; Gameye about 0.07 USD a vCPU hour, traffic included. Hathora and Unity Multiplay closed in 2026 | Yes, their purpose | Edgegap's per-GB fee is 0.24 USD for a full room hour before V3, more than the compute | Many | Least work per region; an SDK or an API to fit to the host protocol |
| Managed containers at a large cloud | Not priced | UDP ingress is missing or needs a load balancer that alone costs more than a VPS (from memory, verify) | Per GB, about 0.09 USD (from memory) | Many | High |
| A free ARM tier | Oracle's always-free Ampere shape (from memory, not looked up) | Plain sockets | A large free allowance (from memory) | One per account | Needs an AArch64 server build, which the determinism rules allow and nothing has tested |

**Recommendation.**

- **Today to V3: one small VPS with a flat traffic allowance.** It is the cheapest line
  at every size here, traffic cannot surprise, and the server is one static binary under
  one systemd unit. Before V3 a full room's traffic makes any per-GB plan the expensive
  choice.
- **The first step up: the 4 GB, 2 vCPU class of the same model**, about 5 a month. It
  holds roughly 6 full rooms before V3 and 12 after (estimated from the CPU rows).
- **A second region: a second small VPS as a race host**, not a platform. Two machines
  are still less upkeep than fitting an orchestration API.
- **Reconsider per-second machines after V3**, when a full room hour costs about a cent
  of traffic and stopping idle hosts starts to pay; and orchestration only if there are
  more regions than someone wants to patch.

## Not covered

- Nothing here was run against a remote host; the remote figures are the earlier ones.
- An 8-person room was not run (four game clients at once is this machine's limit for
  GPU memory); its CPU and traffic are extrapolated.
- 2048 was loaded and not raced. Omega and 2048: not a format finding; the measurement
  is title-blind and ran on Omega's own data, and the same commands run on 2048.
- The Galaxy S24 replay cost (ADR-0069, condition 1) bounds how fast a phone rejoins.
- A web client, accounts, chat, invitations and quick match are outside this page.
