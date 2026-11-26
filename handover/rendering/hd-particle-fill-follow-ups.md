---
status: measure
titles: [hd, omega, 2048]
updated: 2026-11-26
summary: "Owed: the Rocket craft-hit screen wash (A/B capture), Deck numbers, 1.7 ms at 1080p; the half-res particle target awaits the maintainer"
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

### Particles into a half-resolution target, composited

**Not faithful.** The disc authors a per-distance resolution mode (`+0xc8c`, thresholds 17 and
35) and HD computes it, but its flush ignores it. Would be an opt-in setting, default
`original`: about 20 screens of fill become about 5 plus one full-screen composite; estimate
3.2 to about 2 ms at 4K. Effort L, confidence 40; edges where smoke meets hulls need a
depth-aware upsample.

HD's width throttle, the other idea here, is TARGET FPS `original`
(`docs/rendering/dynamic-resolution.md`, "A title's own width throttle").

### Owed measurements

- **The Deck** at 3840x2160, 1920x1080 and 1280x800 (vsync off, no frame limit): the
  reproduction's `gpu_scene_us` over ticks 760 to 789, before and after the law. Not run.
  The fragment `discard` of particle-fill.md's "Measured and dropped" belongs to the same visit.
- **About 1.7 ms of a hit frame at 1080p that is not fill.** With and without the law, ticks 770
  to 774 cost about the same (3.26 ms at the worst before, 3.16 after, on an RX 7800 XT).
  Unprofiled: leave the particle draws, the blast's light and the flash out in turn at 1920x1080.
- **Omega's passes 8 to 10 by target size** (is one of them a reduced particle target), and
  **Omega's sphere fade** behind `instance + 0x71` (radius 290): what sets it. Both are reads.
- **What fills tick 784, the reproduction's worst frame since the blast carry.** Its additive layer
  is 5.2 of 14.5 screens and does not shrink when sampled billboards are trimmed to their live
  texels (`docs/rendering/particle-fill.md`, "Measured and dropped"). Name the emitter: a per-effect
  fill split of that tick, with the fill probe described there. The trim itself (fill -37 to -59 %
  on ticks 766 to 781, 4K worst frame -0.13 ms, not byte-identical) is parked on the local branch
  `trimmed-sprites-parked`. Measure it again only if the worst frame turns out to be alpha-over
  smoke on another circuit or on the Deck. Command: `OAG_RENDER_BENCH_RACE=<stem>` with the
  reproduction, six interleaved pairs, `gpu_scene_us` over ticks 760 to 789.
- **TARGET FPS `original` on the Deck**, against the existing loop on the same reproduction;
  and at 240 Hz on an idle host, where the 2026-11-26 runs moved mostly on late frames.
- **2048's `Debug.Enable Frame Buffer Throttling`** (byte `+0x550`): read its loads to say
  whether 2048 has a width or size throttle of its own
  (`docs/ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md`, "Omega and 2048").
- **The PSP builds' builder**: whether Pulse and Pure have a near rule at all.
- **Which render modes 2048's two builders serve**, and whether the original's sprite templates
  go through the same routine (ours cull and fade them, with no cap).

## Next Steps

1. Capture the two A/B arms in "The screen wash" and wire whichever removes the quads.
2. Re-render `hd-rocket-craft-hit-0100` and `-0233` once the wash is wired.
3. Run the Deck numbers before and after.
4. Put the half-resolution particle target to the maintainer once the Deck numbers are in.
