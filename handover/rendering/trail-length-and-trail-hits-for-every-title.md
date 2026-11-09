---
status: ready-to-wire
titles: [pulse, pure, hd]
updated: 2026-11-09
summary: "Trail hits landed (graphics.trail_hits: Pulse, Pure); HD-length trails for Pulse and Pure wait until HD's burst matches"
---

# HD-length trails (opt-in) for Pulse and Pure; trail hits landed

Maintainer request, 2026-11-08. Two features, each behind its own setting, off by default:
departures from the originals, enhancements in the sense of the opt-in flag for back-ported
effects.

## Open

- **Trail length.** Pulse and Pure get a setting that lengthens their engine trails to HD's
  length. HD's ribbon is read in `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md` (54
  samples, "The tuning file is the parameter block"). Waits until HD's trail-hit burst matches
  the original, so the hit test the longer trail feeds is the right one. It does not match yet:
  the 2026-11-08 pair (`engine-trail.md`, "The trail-hit burst as a matched pair") shows emitter
  0's purple class-6 streaks scattered behind ours where the original draws a compact white glow
  on the nose; the class-6 draw law is unread (`FUN_002673b0`, `0x00267b10`). The Pulse and Pure
  hit test (below) runs against their 10-sample ribbon; a longer ribbon would reuse it through
  `Exhaust::trail_nearest`, and the length setting must lengthen that ring, not add a second one.
- **Trail hits: landed**, with what remains. `graphics.trail_hits` (off by default) lends HD's
  `WO_TRAIL_HITSHIP` to Pulse and Pure (`oag_raceplay::TrailHits`, `load/trail_hits.rs`,
  `EffectHandles::lending`); 2048 and Omega already play their own disc's copy with no setting
  (`engine-trail.md`, "The burst on every title"). Remaining: (1) the Pulse/Pure hit test is
  chosen, not measured (the ribbon's line inside the hull's cross-section); (2) the burst's look
  is the shared class-6 draw, so it mends with the burst fix above; (3) a Pulse or Pure race on a
  machine with no HD, 2048 or Omega source plays nothing, by design; (4) the lent burst is the
  blue variant only (the red one follows HD's Fury skin flag).

## Next Steps

1. Recover HD's class-6 draw law and fix the burst (opus), then the trail-length setting for
   Pulse and Pure.
