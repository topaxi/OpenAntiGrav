---
status: measure
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-09
summary: "HD's class-6 bar, kColourScale and WO_TRAIL_HITSHIP sprites wired; lilac gone but no white nose glow yet; HD-length trails stay blocked"
---

# HD-length trails (opt-in) for Pulse and Pure; trail hits landed

Maintainer request, 2026-11-08. Two features, each behind its own setting, off by default:
departures from the originals, enhancements in the sense of the opt-in flag for back-ported
effects.

## Open

- **The burst's draw: wired; the HD pair is closer, not matched.** `StreakDraw::FarBar` (HD's
  class-6 bar, farther depth) draws `WO_TRAIL_HITSHIP` on HD, 2048, Omega and the lent burst;
  `kColourScale` (`+0xc84`) is applied, inherited from Omega; `psys_normal_vp`'s `pow(rgb, 2.2)`
  was already applied (`engine-trail.md`, "The wire, and the pair re-shot"). HD authors both
  emitters' sprites; ours had not listed them, and listing `WO_TRAIL_HITSHIP` is what moved the
  picture: the lilac blobs are gone and thin blue rays appear, as on the original, but the compact
  white nose glow is not reproduced (ours is blue and dim). The bar alone and the scale alone
  changed nothing a player sees.
- **Still open on the burst:** `CORE_IMPACT`'s atlas frame (drawn on its spawn frame), the render
  state `record+0xca4`, `WO_TRAIL_HITSHIP_RED`'s sprites (unlisted), and the placement (the
  simulation's). A re-shot pair after any of them.
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

1. Wire/RE lane: `CORE_IMPACT`'s frame and the render state `record+0xca4`
   (`PartRender_FlushDraws`), then a re-shot pair against the original's white nose glow.
2. Then the trail-length setting for Pulse and Pure, once the pair matches.
