---
status: ready-to-wire
titles: [all]
updated: 2026-11-17
summary: "After an online race, its replay is the server-confirmed run (what really happened), saved locally; download the server's copy only when the fingerprint differs"
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

## Open

1. **The pose track of the real run.** The `POSE` chunk recorded live is the client's
   predicted view. Rebuild it after the race by re-simulating the confirmed inputs
   (`resimulate`), so the replay's picture is the server's race, rockets included.
2. **A fingerprint at the end.** The server's `End` carries a hash over the race parameters,
   every seat's confirmed input stream and the state hashes; the client computes the same over
   its recording. Equal: the local replay is the real one, offer nothing. Different (a
   desync, a gap from a reconnect, a missed stretch): offer **Download the server's replay**.
3. **The download.** The race host keeps the finished journal (V2's journal on disk, or the
   history in memory until the room closes) and streams it on request; the client writes it
   as an ordinary `.oagr` and opens it in the viewer. Bound it (size, one request per client,
   rate) like the other limits in `crates/net`.
4. **The end screen.** A row on the online end screens: WATCH REPLAY (the local, confirmed
   replay), or DOWNLOAD REPLAY when the fingerprints differ; strings need `string_id` and
   English text (CLAUDE.md, `just check-strings`).
5. **Wire change.** The fingerprint and the download message bump `PROTOCOL` to 2; bundle it
   with the server roadmap's other version-2 changes (decision 4 there) rather than alone.

## Next Steps

1. After `rec-panic` merges: a sonnet `oag-wire` lane (about 4 to 6 hours) for items 1, 2
   and 4 (no wire change if the fingerprint is first computed and shown locally only), with
   acceptance: an online race with a forced late correction (a rocket hit the client predicted
   as a miss) whose saved replay shows the hit at the server's tick.
2. Items 3 and 5 with the `PROTOCOL` 2 bump, once the maintainer settles the roadmap's
   decision 4.
