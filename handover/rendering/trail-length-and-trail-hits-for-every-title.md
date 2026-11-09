---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-09
summary: "Wire HD's class-6 bar (read on Omega's CPU twin, 70); 2048 and Omega now behind graphics.trail_hits; HD-length trails wait on the burst"
---

# HD-length trails (opt-in) for Pulse and Pure; trail hits landed

Maintainer request, 2026-11-08. Two features, each behind its own setting, off by default:
departures from the originals, enhancements in the sense of the opt-in flag for back-ported
effects.

## Open

- **The burst's draw (ready to wire, HD).** `engine-trail.md`, "The class-6 draw law": class
  6 is render mode 5, one 4-vertex quad per particle, built by the `PartRender` SPU job
  (`0x0080bb80`). Its CPU twin on Omega (`Particle_PushBatchByMode`, `0x01714de0`) is Pulse's
  pool bar in view space: `2 size` wide, `aspect * size` (`+0x4c8`) past each end, both ends
  on the **farther** view depth, sampling the atlas cell. Confidence 70 for the shape on HD.
  The wire: drop `StreakDraw::Procedural` for HD's class 6 (`Effect::without_pulse_psp_draw`)
  in favour of `StreakDraw::Bar { aspect }`, with the farther-depth rule. That rule is chosen
  from Omega until the SPU job is read. Then compare the pair again
  (`engine-trail.md`, "The trail-hit burst as a matched pair").
- **Two candidate colour terms, unchecked against ours.** `psys_normal_vp` raises rgb to the
  power 2.2 and multiplies by `kColourScale`. Omega's batch takes `kColourScale` from
  `emitter+0xc84`, which is 0.7 on emitter 0 and 1.0 on `CORE_IMPACT`. On HD, where
  `kColourScale` comes from is unread (`PartRender_FlushDraws`, `0x002c3ca0`). Check whether
  ours applies either term to HD particles before wiring.
- **Unread, needs `spu-objdump`** (not installed on 2026-11-09; the maintainer or lead
  reinstalls `ps3-spu-binutils`): `scripts/ps3-spu-disasm.py 0x0080bb80 0x5ad0`. It settles
  three things: the depth rule (farther or mean), what `-(+0xc80)` does in mode 5, and the
  texture a sprite-less emitter binds.
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
2. With `spu-objdump` back: read the `PartRender` mode-5 case, and confirm or replace the
   farther-depth rule.
3. Then the trail-length setting for Pulse and Pure.
