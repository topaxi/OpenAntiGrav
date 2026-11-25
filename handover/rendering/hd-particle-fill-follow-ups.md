---
status: measure
titles: [hd, omega, 2048]
updated: 2026-11-25
summary: "Near particle law is wired; owed: a close-range original frame, Deck numbers, the 1.7 ms at 1080p; two settings await the maintainer"
size: M (50)
difficulty: medium (55)
---

# Particle fill: what is left after the near-camera law

The near-camera law (cull, fade, cap) is wired on HD, 2048 and Omega:
`docs/rendering/particle-fill.md`, "Wired" has the fill and GPU numbers (4K worst frame -42 %),
and `docs/ghidra/functions/ps3-hdfury-eu/particle-render.md` the law and what of ours it
reaches.

## Open

### Two further ideas, as settings

| # | Change | Faithful? | Expected | Effort | Confidence |
| --- | --- | --- | --- | --- | --- |
| 2 | **HD's width throttle as the `original` value of dynamic resolution**: scene width `1 - level / 96` of the output (1920 to 1300 of 1920), one level a frame, narrower at 16.1 ms of GPU frame or a missed vblank, wider under 15.6 ms | **Yes on HD**, it is `Render_FinishFrame`'s controller. An option, off unless chosen: our default is a fixed scale | up to -32 % of every scene pixel under sustained load; a 9 frame blast sheds at most 9 % | M | 60 on the gain, 90 on the law |
| 3 | **Particles into a half-resolution target, composited** | **No.** The disc authors a per-distance resolution mode (`+0xc8c`, thresholds 17 and 35) and HD computes it, but its flush ignores it. Opt-in setting, default `original` | about 20 screens of fill become about 5 plus one full-screen composite; estimate 3.2 to about 2 ms at 4K | L | 40; edges where smoke meets hulls need a depth-aware upsample |

### Owed measurements

- **A matched original frame of a blast at close range.** The live capture that proved the law
  had its blasts 70 to 350 units away, so the proof is on the player's own near effects. A pair
  with the camera inside a CraftBlast is the picture check: fire at `GO` with a rival directly
  ahead and teleport forward (`rpcs3-hd-weapon.py --teleport-back` with a negative distance), or
  use the missile film's pose.
- **The Deck** at 3840x2160, 1920x1080 and 1280x800 (vsync off, no frame limit): the
  reproduction's `gpu_scene_us` over ticks 760 to 789, before and after the law. Not run.
  The fragment `discard` of particle-fill.md's "Measured and dropped" belongs to the same visit.
- **About 1.7 ms of a hit frame at 1080p that is not fill.** With and without the law, ticks 770
  to 774 cost about the same (3.26 ms at the worst before, 3.16 after, on an RX 7800 XT).
  Unprofiled: leave the particle draws, the blast's light and the flash out in turn at 1920x1080.
- **Every HD, 2048 and Omega comparison pair with a near particle moved**: re-render the weapon
  and blast pairs.
- **Omega's passes 8 to 10 by target size** (is one of them a reduced particle target), and
  **Omega's sphere fade** behind `instance + 0x71` (radius 290): what sets it. Both are reads.
- **The PSP builds' builder**: whether Pulse and Pure have a near rule at all.
- **Which render modes 2048's two builders serve**, and whether the original's sprite templates
  go through the same routine (ours cull and fade them, with no cap).

## Next Steps

1. Shoot the close-range pair and judge the wired picture against it.
2. Run the Deck numbers before and after.
3. Put items 2 and 3 to the maintainer as settings once the Deck numbers are in.
