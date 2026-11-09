---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-09
summary: "Wire HD's class-6 bar (PartRender SPU job read, 85); 2048 and Omega now behind graphics.trail_hits; HD-length trails wait on the burst"
---

# HD-length trails (opt-in) for Pulse and Pure; trail hits landed

Maintainer request, 2026-11-08. Two features, each behind its own setting, off by default:
departures from the originals, enhancements in the sense of the opt-in flag for back-ported
effects.

## Open

- **The burst's draw (ready to wire, HD).** `engine-trail.md`, "The class-6 draw law":
  class 6 is render mode 5, one 4-vertex quad per particle. It is built by the `PartRender`
  SPU job (`0x0080bb80`, mode 5 at image `0x29b8`), which was read, and Omega's CPU twin
  (`0x01714de0`) agrees with it. It is Pulse's pool bar in view space: `2 size` wide,
  `aspect * size` (`+0x4c8`) past each end, both ends on the **farther** view depth, sampling
  the atlas cell. Confidence 85. The wire: replace `StreakDraw::Procedural` with
  `StreakDraw::Bar { aspect }` for HD's class 6 (`Effect::without_pulse_psp_draw`), at the
  farther depth rather than Pulse's mean. Then compare the pair again (`engine-trail.md`,
  "The trail-hit burst as a matched pair").
- **Two candidate colour terms, unchecked against ours.** `psys_normal_vp` raises rgb to the
  power 2.2 and multiplies by `kColourScale`. Omega's batch takes `kColourScale` from
  `emitter+0xc84`, which is 0.7 on emitter 0 and 1.0 on `CORE_IMPACT`. On HD, where
  `kColourScale` comes from is unread (`PartRender_FlushDraws`, `0x002c3ca0`). Check whether
  ours applies either term to HD particles before wiring.
- **Still unread:** the texture a sprite-less emitter binds (`record+0xca4`'s render state).
- **Trail length.** Pulse and Pure get a setting that lengthens their engine trails to HD's
  length (54 samples, `engine-trail.md`, "The tuning file is the parameter block"). It waits
  until the burst matches the original. The Pulse and Pure hit test runs against their
  10-sample ribbon through `Exhaust::trail_nearest`, so the length setting must lengthen that
  ring, not add a second one.
- **Trail hits: landed.** `graphics.trail_hits` is off by default.
  - It lends HD's `WO_TRAIL_HITSHIP` to Pulse and Pure.
  - Since 2026-11-09 it also gates 2048's and Omega's own copy: their originals never fire it
    (`ps4-omega-eu/trail-hits.md`, `vita-2048-eu-v104/trail-hits.md`).
  - HD alone plays it always (`oag_title::Effects::trail_hits_native`).

  Remaining:
  1. The Pulse/Pure hit test is chosen, not measured.
  2. The burst's look mends with the draw above.
  3. A Pulse or Pure race on a machine with no HD, 2048 or Omega source plays nothing, by
     design.
  4. The lent burst is the blue variant only.

## Next Steps

1. Wire lane: HD's class-6 bar (above), then a matched pair against the original.
2. Then the trail-length setting for Pulse and Pure.
