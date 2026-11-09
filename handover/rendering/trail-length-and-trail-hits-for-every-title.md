---
status: re-first
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-09
summary: "HD's class-6 bar wired (far depth, kColourScale); the HD pair still does not match (lilac blobs), texture unread; HD-length trails stay blocked"
---

# HD-length trails (opt-in) for Pulse and Pure; trail hits landed

Maintainer request, 2026-11-08. Two features, each behind its own setting, off by default:
departures from the originals, enhancements in the sense of the opt-in flag for back-ported
effects.

## Open

- **The burst's draw: wired, and the HD pair does not match.** `StreakDraw::FarBar` (HD's
  class-6 bar, both ends on the farther view depth) draws `WO_TRAIL_HITSHIP` on HD, 2048, Omega
  and the lent burst; `kColourScale` (`+0xc84`) is applied, inherited from Omega
  (`engine-trail.md`, "The wire, and the pair re-shot"). `psys_normal_vp`'s `pow(rgb, 2.2)` was
  already applied by ours. Re-shot on six pairs, the nose-box energy moved +0.6..+10.8 % (bar)
  and then -5..-15 % (scale), and read as a player: no difference. Ours still draws lilac blobs
  the original does not, beside one white nose glow both have. Emitter 0's aspect is 1.0, so the
  bar is nearly the old quad. Next: the texture a sprite-less emitter binds (`record+0xca4`, below)
  and the burst's placement, which is the simulation's.
- **Still unread:** the texture a sprite-less emitter binds (`record+0xca4`'s render state). HD's
  burst has no sprite loaded, so it draws the procedural profile as the bar's stand-in.
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

1. RE lane: the texture and render state a sprite-less class-6 emitter binds (`record+0xca4`,
   `PartRender_FlushDraws`), then a re-shot pair.
2. Then the trail-length setting for Pulse and Pure, once the pair matches.
