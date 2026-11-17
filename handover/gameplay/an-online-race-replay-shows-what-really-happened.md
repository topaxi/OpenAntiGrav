---
status: blocked-maintainer
titles: [all]
updated: 2026-11-17
summary: "Local half landed (confirmed run saved, room in header, local fingerprint). Left: server fingerprint and download, with PROTOCOL 2"
categories: [frontend]
---

# An online race's replay shows what really happened

2026-11-17. The maintainer's use case: review a race after playing it online. If a rocket
hit their craft and their client did not show it (lag, a late correction), the replay must
show what the server's race did, not what the client predicted. Asked first: "I'd presume
[the local replay] does not [match the remote one] byte for byte?" Right: a client's file
differs in its header (this machine's settings and names) and its `POSE` track (hull poses
of the predicted run). The comparison is by content, not bytes.

## What exists

- A client receives every seat's confirmed input for every tick (`crates/net/src/client.rs`,
  the confirmed `TickRecord`s), so it holds everything the server's run is made of.
- Lane `rec-panic` (in flight on 2026-11-17) changes the online client to record **only
  confirmed ticks** (predicted ticks pause the recorder), which also fixes a debug panic in
  `crates/raceplay/src/replay/recording.rs`. With it, the local recording's inputs and hashes
  are the server-confirmed run. Check its merge before starting here.
- `oag_raceplay::replay::resimulate` re-drives a replay's inputs; the server roadmap keeps a
  finished race's journal for a day as a replay
  ([server-roadmap.md](../../docs/networking/server-roadmap.md), "The target version",
  Operations).

## Landed (online-replay lane)

- Item 1: the pose track is already the confirmed run since `rec-panic` (pose is read from the
  confirmed state; a full-tick re-drive of the recording gives a gap of 0 on every craft), so no
  separate rebuild is stored.
- Item 2, local half: `Replay::fingerprint`, kept at the end by `NetPlay::replay_fingerprint`.
- Item 4: an online race arms the recording (`load.rs`, the net branch), so the end screen's WATCH
  REPLAY row (Pulse dialect) is offered; the room is in the header, so the viewer rebuilds it.
  `docs/formats/replay-file.md`, "An online race's recording".

## Open

1. **The server's fingerprint.** `End` carries it; the client compares and offers DOWNLOAD REPLAY on
   a difference (a wire change, `PROTOCOL` 2).
2. **The download** of the server's journal (item 3 above), bounded like the other limits.
3. A seat's retirement is not an `EVNT` entry; a recording across one does not re-drive.
4. No session-level test arms the recording through the menus (no menu-driven LAN test exists).

## Next Steps

1. With the `PROTOCOL` 2 bump (server-roadmap decision 4): items 1 and 2.
