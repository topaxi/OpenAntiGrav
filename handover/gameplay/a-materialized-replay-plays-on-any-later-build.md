---
status: measure
titles: [all]
updated: 2026-11-17
summary: "Forward-compatible replays: POSE VIEW plays hulls only; a materialized replay would add craft state, weapons, events, HUD and camera"
size: M (40)
difficulty: medium (40)
---

# A materialized replay plays on any later build

2026-11-17. The maintainer asked whether a replay could be "materialized" so it keeps working on
future versions. Replay compatibility is not a priority (same day: an outdated recording is not
something tests should assert against unless it is supported), so this is written down, not queued.

## What exists

- A replay is inputs plus periodic state hashes (ADR-0055); it reproduces only on a build whose
  simulation and hash layout match the recording's. Any physics, AI or hash-layout change breaks it
  (2026-11-16: arena-rescue's hashed `beneath_wide` flag made the viewer refuse old recordings at
  tick 0).
- **The `POSE` chunk is already a partial materialization** (ADR-0087 decision 4,
  [`docs/formats/replay-file.md`](../../docs/formats/replay-file.md), "The pose track"): every craft's
  hull pose, refined keys, 0.05 unit / 0.5 degree bound, about 145 to 175 bytes a craft a second at
  Venom, 197 to 263 at Phantom, 358 to 404 KB for eight craft over five minutes. Past the last tick a
  build reproduces, the viewer continues in **POSE VIEW** (ADR-0087, "A build mismatch, and the pose
  track as the fallback").
- `EVNT` already records the events that change a race from outside `Race::tick` (Pilot Assist,
  give, controller). Those are inputs to the simulation, not outputs for drawing.
- Unknown chunks are skipped by the reader, so a new chunk is additive and does not bump the file
  version.

## Open

What POSE VIEW lacks, by what a viewer would miss:

1. **Craft visual state** per tick: shield, energy, damage state, boost, airbrakes, exhaust,
   destroyed and respawning. Today the hull flies with none of it.
2. **Weapons in flight**: projectiles, mines, bombs, beams and blasts, with their poses and lifetimes.
3. **Output events**: weapon fired, hit, pad taken, wall and craft contact, effect starts, sound cues
   (ADR-0018: cues are a per-tick output), lap, place and finish changes. A later build would draw
   and play these with its own effect and sound code, which is the intent.
4. **HUD readouts** of the viewed seat (speed, energy, lap, place, timer, weapon slot) and the race
   camera's pose, so a POSE VIEW frame has a HUD and the original framing.
5. **A stable schema**: the materialized chunks become a versioned contract a later build must keep
   reading, unlike the simulation. Decide what is in it once and version each chunk.

Constraints:

- **Only a build that still reproduces a recording can materialize it.** It must happen at record
  time (as `POSE` does) or in an explicit "keep this replay" export before an update, never after.
- **A materialized replay cannot be taken over** (ADR-0087 stage 7, playing on from a point needs
  simulation state). Watch, seek, rewind and photo mode work from stored ticks.
- Cost estimate (not measured): items 1 to 4 for eight craft, delta-coded like `POSE`, roughly 1 to
  2x the pose track again, so about 1 MB for a five-minute race. Measure before choosing between
  always-on and opt-in.

Compression (nothing in a `.oagr` is compressed today; the screenshot chunk in
`crates/replay/src/shot.rs` is deflated):

- ADR-0087 decision 4 measured general compressors on Pulse: on the **stored** pose track gzip -9
  saves 9 percent and xz -9 28 percent (zstd not measured); on the **raw** 60 Hz pose stream gzip -9
  29, zstd -19 42 and xz -9 55 percent. The keyframe refinement already took most of the
  redundancy, so the ADR applied no compression.
- The input stream (6.3 to 6.9 bytes a tick a human seat, ADR-0055) was never put through a
  compressor. Analog stick values change slowly, so it may compress better than the pose track.
- A materialized replay would roughly double the stored size, which makes a compressed container
  more worth having. `ruzstd` is already in the workspace (texture packs), so zstd adds no
  dependency.

## Next Steps

1. Compression measurement (sonnet `oag-docs` or `oag-perf`, about 30 min, measure lane): zstd at
   a few levels (3, 9, 19), gzip -9 and xz -9 over each chunk of a real `.oagr` (`INPT`, `HASH`,
   `POSE`, the whole file), on all five titles, at Venom and Phantom, from the cost sweep's Autopilot
   race and a pad-flown race (`replay_timeline_ground_truth`'s recording). Report percent saved and
   encode/decode time per chunk; recommend per chunk whether a zstd wrapper earns its keep.
2. Design lane (sonnet `oag-docs`, 2 to 3 h): the per-tick schema for items 1 to 4, which of them
   the renderer, HUD and audio already read as a per-tick output (the cheapest cut), always-on versus
   a "keep" flag, and an ADR draft extending ADR-0087 decision 4. Measure the size on the cost
   sweep's Autopilot race (`OAG_SWEEP=1`, the same race as the `POSE` table) on every title.
3. Wire lane (sonnet `oag-wire`): record and play items 1 and 3 first (what makes POSE VIEW read as
   the race), then 2, then 4; acceptance is a POSE VIEW frame pair against the simulated frame at
   the same tick on one title, then the rest.
