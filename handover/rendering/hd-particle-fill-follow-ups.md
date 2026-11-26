---
status: measure
titles: [hd, omega, 2048]
updated: 2026-11-26
summary: "Owed: the Rocket craft-hit screen wash (A/B capture), Deck numbers, 1.7 ms at 1080p; two settings await the maintainer"
size: M (50)
difficulty: medium (55)
---

# Particle fill: what is left after the near-camera law

The near-camera law (cull, fade, cap) is wired on HD, 2048 and Omega:
`docs/rendering/particle-fill.md`, "Wired" has the fill and GPU numbers (4K worst frame -42 %),
and `docs/ghidra/functions/ps3-hdfury-eu/particle-render.md` the law and what of ours it
reaches.

## Open

### The Rocket's craft hit: the screen wash

What the blast rides is read and wired (2026-11-26): the spawn copies a point and hands the
blast the struck craft's velocity, on HD, 2048 and Omega
(`docs/ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md`). Left:

- **The screen wash.** The full-screen single quads (half height = 2 x depth) are square,
  opaque, warm white to yellow, and sit 2 to 12 units from the spawn point while the struck
  craft is 13 to 30 units on, in all three filmed hits. So they are not the travelling
  particles. Candidates, none above 45: `FIRESPIKES` (64x64 additive, cap 2.0, life 11 +- 6
  ticks, but a from-spawn streak emitting 5 a tick), or something drawn for the spawn's point
  light (`D = 100`, colour `(7, 5, 1)`, `0x006778c8` called at `0x0014aa34`). Decide by a
  capture with one removed: zero `FIRESPIKES`' emission count in memory, or `nop` the
  `bl 0x006778c8`, film hit 2 with `scripts/rpcs3-hd-rocket-film.py` and see which kills the
  quads. Then read its law and wire it.
- **The blast's push on nearby craft** (`Rocket_PushNearbyCraft_q`, `0x00149f70`, and Omega's
  inline twin): radius and strength table unread; it may be HD's consumer of the "Blast push"
  row in `docs/gameplay/weapon-lineage.md`. Simulation, not render: its own lane.

### Two further ideas, as settings

| # | Change | Faithful? | Expected | Effort | Confidence |
| --- | --- | --- | --- | --- | --- |
| 2 | **HD's width throttle as the `original` value of dynamic resolution**: scene width `1 - level / 96` of the output (1920 to 1300 of 1920), one level a frame, narrower at 16.1 ms of GPU frame or a missed vblank, wider under 15.6 ms | **Yes on HD**, it is `Render_FinishFrame`'s controller. An option, off unless chosen: our default is a fixed scale | up to -32 % of every scene pixel under sustained load; a 9 frame blast sheds at most 9 % | M | 60 on the gain, 90 on the law |
| 3 | **Particles into a half-resolution target, composited** | **No.** The disc authors a per-distance resolution mode (`+0xc8c`, thresholds 17 and 35) and HD computes it, but its flush ignores it. Opt-in setting, default `original` | about 20 screens of fill become about 5 plus one full-screen composite; estimate 3.2 to about 2 ms at 4K | L | 40; edges where smoke meets hulls need a depth-aware upsample |

### Owed measurements

- **The Deck** at 3840x2160, 1920x1080 and 1280x800 (vsync off, no frame limit): the
  reproduction's `gpu_scene_us` over ticks 760 to 789, before and after the law. Not run.
  The fragment `discard` of particle-fill.md's "Measured and dropped" belongs to the same visit.
- **About 1.7 ms of a hit frame at 1080p that is not fill.** With and without the law, ticks 770
  to 774 cost about the same (3.26 ms at the worst before, 3.16 after, on an RX 7800 XT).
  Unprofiled: leave the particle draws, the blast's light and the flash out in turn at 1920x1080.
- **Omega's passes 8 to 10 by target size** (is one of them a reduced particle target), and
  **Omega's sphere fade** behind `instance + 0x71` (radius 290): what sets it. Both are reads.
- **The PSP builds' builder**: whether Pulse and Pure have a near rule at all.
- **Which render modes 2048's two builders serve**, and whether the original's sprite templates
  go through the same routine (ours cull and fade them, with no cap).

## Next Steps

1. Capture the two A/B arms in "The screen wash" and wire whichever removes the quads.
2. Re-render `hd-rocket-craft-hit-0100` and `-0233` once the wash is wired.
3. Run the Deck numbers before and after.
4. Put items 2 and 3 of the table to the maintainer as settings once the Deck numbers are in.
