---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-09
summary: "Trail-hit back-port to every title is ready to wire; HD-length trails for Pulse and Pure wait until HD's burst matches"
---

# HD-length trails and trail hits for every title (opt-in)

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
  on the nose; the class-6 draw law is unread (`FUN_002673b0`, `0x00267b10`).
- **Trail hits everywhere.** HD's `WO_TRAIL_HITSHIP` burst, which plays on a craft flying in a
  rival's trail (`Race::advance_trail_hits`), back-ported to Pulse, Pure, 2048 and Omega behind a
  setting. Independent of the trail-length setting. It plays the disc's own effect: HD's `.pob`
  when an HD source is mounted, a title's own effect only if one is found; with neither, it draws
  nothing and says so in the loader report. Omega's twin is already "checked, applies"
  (`engine-trail.md`). It shares `advance_trail_hits` with the fidelity lane, so it starts after
  that lane merges.

## Next Steps

1. The trail-hit back-port lane (sonnet `oag-wire`): the trigger and the setting; it draws
   whatever HD's draw path draws, so the fidelity fix lands for every title at once.
2. Recover HD's class-6 draw law and fix the burst (opus), then the trail-length setting for
   Pulse and Pure.
