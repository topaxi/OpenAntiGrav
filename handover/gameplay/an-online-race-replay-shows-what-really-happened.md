---
status: ready-to-wire
titles: [all]
updated: 2026-11-18
summary: "Local half landed (confirmed run saved, room in header, local fingerprint); left: server fingerprint and replay download"
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
- Lane `rec-panic` merged (`9ba7e3c03`): the online client records **only confirmed ticks**
  (predicted ticks pause the recorder), which also fixed a debug panic in
  `crates/raceplay/src/replay/recording.rs`. The local recording's inputs and hashes are the
  server-confirmed run.
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
   a difference (a wire change; `PROTOCOL` is still 1 and edited in place, see Next Steps; `End` carries no fingerprint today).
2. **The download** of the server's journal (the roadmap keeps it a day), bounded like the other limits.
3. A seat's retirement is not an `EVNT` entry; a recording across one does not re-drive.
4. No session-level test arms the recording through the menus (no menu-driven LAN test exists).

## Next Steps

1. Open items 1 and 2 (sonnet `oag-wire`). Not blocked: server-roadmap decision 4 (accepted 2026-11-17) freezes the wire only from the first public deployment, so until then `End` and a download message are edited in place under `PROTOCOL` 1. Open item 3 (a retirement as an `EVNT` entry) belongs with them.
