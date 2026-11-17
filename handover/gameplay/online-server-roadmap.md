---
status: blocked-maintainer
titles: [all]
updated: 2026-11-17
summary: "Online server roadmap: three versions (reconnect, several rooms with crash resume, cheaper rooms) wait on eight ranked decisions"
categories: [tooling]
---

# Online server roadmap: three versions toward a journal-backed server

2026-11-17, lane `net-design`. A design lane: no code changed. The review of today's server,
the target, the three versions, the lobby and race boundary and the hosting plan are in
[`docs/networking/server-roadmap.md`](../../docs/networking/server-roadmap.md). The lobby's
own leftovers (spectate, chat, invite-only, quick match, the recorded pad stream) stay in
[the lobby thread](online-lobby-server-spike.md).

## Open

0. **Lobby session grace (landed on `net-flake`).** The lobby used to drop a person the poll it
   read their link's close, so a reconnect whose hello arrived a poll later found no session and
   the hand-off was never re-sent (the takeover test hung under load; reproduced
   deterministically by closing the old link first). A person with a seat in the running race now
   keeps their session `SEAT_GRACE` (30 s, `crates/net/src/online/server.rs`, chosen not
   measured) after the close. Still open: a person without a seat, or after the race, is dropped
   at once; the lobby keeps a dropped seat visible in its room for the grace.
1. **Eight decisions, ranked, at the top of the page.** The first three gate the lanes
   below: the journal is the session (V1, V2), a returning player takes the craft back
   (V1), a race stays a process and a race host becomes an agent (V2). None is an ADR yet;
   the page says which would become one.
2. **Measured on localhost only** (2026-11-17, the page's "Performance"): server CPU 3.1 to
   4.3 percent of a desktop core a room, 32 to 36 MB a race process on Pulse, HD and Omega,
   a client's download 50 KB/s at 2 people and 83 KB/s from 4 on. Not measured: an 8-person
   room, 2048 racing, anything on the remote host since 2026-10-16. The rooms-per-1-GB table
   is an estimate that rests on one remote figure (15 to 20 percent of the core for two
   people).
3. **Findings in today's code that no version has fixed yet**, each cited on the page: a
   hung race child holds the only slot for ever (`race_proc.rs:330-354`); inputs for
   far-future ticks are stored without a ceiling and a later input for a tick overwrites an
   earlier one (`host.rs:250`); `Resend` is unthrottled (`host.rs:215-223`); the transport's
   queues are unbounded (`wt.rs:167-169`); no cap on lobby clients or rooms; a state hash
   that disagrees is only logged, on the client (`client.rs:635-644`); the loss cover halves
   in a full room (`host.rs:441-442`).
4. **Hosting.** The existing 1 GB VPS carries today, V1, V2 (3 slots) and V3 (about 6). The
   step-up triggers, the path to several hosts and a price comparison (looked up
   2026-11-17, unverified against the providers' own pages) are in the page's
   "Infrastructure".
5. **Load peaks on an extraction (fixed, `net-mem`, 2026-11-17).** The remote's 542 MB race
   load and 380 MB lobby start were the extraction's sparse `Data.wad` read whole (twice in the
   race). `oag_assets::sparse` now keeps only the range table: 572 to 18.5 MB and 383 to 9.7 MB
   locally (`docs/networking/online-lobby.md`, "Server memory while loading"), so the
   rooms-per-1-GB memory column holds while loading too. Still open: the musl build's peaks on
   the remote host are not re-measured (expect the same, the buffers are the allocator's
   requests); HD, Omega and 2048 extractions were not measured (same code path, larger
   archives, so a larger "before"); the circuit `.vex` (5.1 MB on Pulse) is read twice and held
   twice during the load (`crates/raceplay/src/load/track_inputs.rs`). Measure with
   `/proc/<pid>/status` `VmHWM` of `oag-server --image <extraction> --listen 127.0.0.1:0` once
   it prints `listening on`.
6. **Lobby and race boundary.** Already split in code (`rooms.rs` has no I/O, the `Runner`
   seam, the job on standard input, the hand-off with port and hash); tied to one machine
   by a hand-off with no host, pipes, the lobby opening the images, one slot. The page's
   "Lobby and race server: the boundary" names the host protocol (`Register`, `Launch`,
   `Up`, `Health`, `Over`, `Stop`, `Drain`).

## Next Steps

Each is one drive lane unless split. Hours are estimates. None starts before decisions 1
to 3 are answered.

1. **V1a, a seat can be taken back** (`oag-wire`, opus, about 22 h). The host seats a token
   whose seat left, during the race; `TickRecord.retired` becomes the seats away now;
   `race::retire` hands a craft back; the client reconnects and asks for the ticks it lacks;
   the game shows "reconnecting". `PROTOCOL` 2. Acceptance: a `loopback` test where a link
   closes at tick 600 and returns at 900, every hash agreeing on both ends; red today at
   "the race has started" (`host.rs:366`). Touches `crates/net` and `oag-game`'s
   `net_play`; not `crates/ai`.
2. **V1b, limits, health and the lobby's loose ends** (landed on branch `net-v1`: the host's input and resend bounds, the race child's `health` lines and the lobby's 5 s quiet timeout, the takeover re-send, the refusal-before-close ordering. Landed on branch `net-v1b`: lobby caps per address and in all, the race host's join cap and hello timeout, bounded transport queues, the datagram budget, the one-line-a-second warning limit (`server-roadmap.md`, V1 "Landed (item 2, the rest)"). Still open: the client's desync report and end-of-race summary and the hand-off's host field; `PROTOCOL` did not change) (`oag-wire`, sonnet, about 23 h).
   The bounds of the page's "Robustness" list on the host and the lobby; `Health` lines
   from the race child and a hang timeout; the client's desync report and end-of-race
   summary; the takeover re-sends a pending hand-off (the lobby thread's item 5); the
   hand-off gains a host; the refusal-before-close ordering. Acceptance: the flood, hung
   child and takeover tests named on the page, and the flaky refusal test 200 runs in a
   row under load.
3. **V2a, several slots and the host protocol** (`oag-wire`, opus, about 22 h). The host
   component with a port range; typed messages over the pipes; the room queue only when
   every slot is taken. Acceptance: `online`, two rooms racing at once (today the second
   is queued). The lobby thread's "containers" item closes here.
4. **V2b, the journal and resume** (`oag-wire`, opus, about 20 h). The journal file; a
   killed or hung race child replaced by one that replays it; the lobby's identity and
   rooms kept on disk; race children adopted after a lobby restart. Acceptance:
   `net_ground_truth`, a race child SIGKILLed at tick 600 resumes and both clients finish
   with 0 bad hashes; the lobby killed mid-race and the room still shows its board.
5. **V2c, operations** (`oag-wire`, sonnet, about 13 h). A systemd unit, a deploy script,
   `oag-server status`, the metrics lines, and an operations page under `docs/networking/`.
   Then the lead or the maintainer measures three two-person rooms at once on the 1 GB VPS.
6. **V3, cheaper per room** (`oag-perf`, opus, about 50 h; may split wire from timing).
   Wire 3 (acknowledged ranges, compact inputs), no allocation in the tick, a tick loop
   woken by arrival, the adaptive buffer, and this page's measurement kept as an
   `OAG_SWEEP` sweep, written first so the before is recorded. Acceptance: the four
   numbers under V3 on the page.
7. **After V3**, each its own lane, in order: race hosts on other machines and region
   placement, spectators from the journal, the simulation fingerprint, replay download,
   early relay if decision 7 is yes.

**Re-running this page's measurement** until V3 keeps it as a sweep: build the server
alone (`cargo build --release -p oag-net --bin oag-server`), start
`oag-server --image SRC --people N --listen 127.0.0.1:PORT --max-ticks 3600 --hash-out F`,
start N `oag-game SRC --connect 127.0.0.1:PORT --cert-hash @F --net-bot --no-audio --size
640x360 --screenshot OUT --ticks 4200` with the three `XDG_*_HOME` variables in a scratch
directory, and read fields 14 and 15 of `/proc/PID/stat` and `VmRSS` of `/proc/PID/status`
once a second while the server's `tick` lines advance. The game client must be built from
the same tree: an older binary is refused at the tick-0 hash.
