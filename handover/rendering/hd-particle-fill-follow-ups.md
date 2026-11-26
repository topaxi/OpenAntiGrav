---
status: measure
titles: [hd, omega, 2048]
updated: 2026-11-26
summary: "Blast fill is the disc's own; half-res target parked by the maintainer. Owed: streak film, template cap source, flare rows, 1.7 ms at 1080p"
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

- **The screen wash: named and wired 2026-11-26.** It is the blast's sprite templates
  (`docs/ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md`, "The screen wash"). Left:
  - where HD reads the template cap of 2.0 (measured on 37 quads, unread);
  - 2048 and Omega play their own files' templates with HD's trigger and cap by lineage; a capture of either's hit would turn that into a measurement.
- **The blast's push on nearby craft** (`Rocket_PushNearbyCraft_q`, `0x00149f70`, and Omega's
  inline twin): radius and strength table unread; it may be HD's consumer of the "Blast push"
  row in `docs/gameplay/weapon-lineage.md`. Simulation, not render: its own lane.

### Left by the far-hit lane (the dome is settled: HD's particle sprites were upside down)

`docs/rendering/particle-fill.md`, "HD's particle sprites are bottom-up".

- **Other flat-RGBA `.gtf` readers keep the file's bottom-up rows and were not checked**:
  `oag_raceplay::assets::decode_texture` (flare and trail art) and `load/caustic.rs`. Compare each
  with its draw's `v` in a command ring before touching it; a radially symmetric sprite shows
  nothing either way.
- **Every HD `.gtf` particle effect moved with the turn; only the Rocket's craft blast was
  compared with the original.** The Plasma, Missile, Mine, Bomb, Quake and Detonator effects
  want one film each beside ours, flipbook frame order first (confidence 80).
- **The 12-unit pairs' hit tick (`--force-rocket-hit 711:7`) was never checked against the
  original's spawn point.** The far films put the wash quad in world space (view position through
  the frame's camera); the same on `hit2` says where ours should stand.
- **A posed blast carries our opponent's velocity, not the original's**, so travelling fireballs
  do not match in a pair. A flag that writes the struck craft's velocity at the forced hit would
  take that out of the metric.

### Particles into a half-resolution target, composited

**Parked by the maintainer (2026-11-26)**: particles stay at full resolution; heavy frames are
left to TARGET FPS `original` and dynamic resolution. Do not reopen without the maintainer.

**Not faithful.** The disc authors a per-distance resolution mode (`+0xc8c`, thresholds 17 and
35) and HD computes it, but its flush ignores it. Would be an opt-in setting, default
`original`: about 20 screens of fill become about 5 plus one full-screen composite; estimate
3.2 to about 2 ms at 4K. Effort L, confidence 40; edges where smoke meets hulls need a
depth-aware upsample.

HD's width throttle, the other idea here, is TARGET FPS `original`
(`docs/rendering/dynamic-resolution.md`, "A title's own width throttle").

### Owed measurements

- **The Deck is measured** (`docs/rendering/particle-fill.md`, "On the Steam Deck"): the craft
  hit's worst frame at 3840x2160 went from 108.5 to 63.5 ms (`d66a52ae6`), still far over 16.7.
  Open there:
  - the heavy frames are split per effect on the workstation
    (`particle-fill.md`, "Per effect: what fills the heavy frames"): the GPU work is at ticks
    724, 754 and 784, particles are about 2.3 of 3.0 ms, and it is the disc's own fill. The
    quads are the same on the Deck (same state hash); its milliseconds per effect are not
    measured and need the throwaway skip switch rebuilt;
  - the fragment `discard` of "Measured and dropped", not run on the Deck.
- **No original frame of a `.pob` streak.** The track blast's `FIRESPIKES` is the largest single
  fill at ticks 724 and 754 (1.1 of 3.0 ms) and is judged authored from the file and the
  executable only (confidence 65). Film a wall hit in RPCS3 and find the buffer HD's mode 5
  quads go to.
- **`WO_ROCKET_EXPLO_TRACK`, `WO_ROCKET_EXPLO` and `WO_ROCKET_FLARE` are not in HD's
  `FAR_BARS`** (`crates/hd/src/effects.rs`), so their class-6 emitters draw the procedural strip
  rather than the read bar with its sprite. A picture gap; fill measured within 9 % under the
  bar. Wants a pair against the original before listing.
- **Our 256-particle instance cap** is full on every track blast a few ticks after it starts
  (`TRAIL_SMOKE`). What the original's pool does there is unread.
- **About 1.7 ms of a hit frame at 1080p that is not fill.** With and without the law, ticks 770
  to 774 cost about the same (3.26 ms at the worst before, 3.16 after, on an RX 7800 XT).
  Unprofiled: leave the particle draws, the blast's light and the flash out in turn at 1920x1080.
- **Omega's passes 8 to 10 by target size** (is one of them a reduced particle target), and
  **Omega's sphere fade** behind `instance + 0x71` (radius 290): what sets it. Both are reads.
- **The trimmed sampled billboards** (`particle-fill.md`, "Measured and dropped") are parked on
  the local branch `trimmed-sprites-parked`. Measure again only if the worst frame turns out to
  be alpha-over smoke on another circuit or on the Deck; on the default circuit the heavy frames
  are streaks.
- **TARGET FPS `original` at 240 Hz on an idle host**, where the 2026-11-26 runs moved mostly on
  late frames. The Deck is measured: `original` does not hold the barrage
  (`docs/rendering/dynamic-resolution.md`, "On the Steam Deck: TARGET FPS `original` against the
  budget loop").
- **2048's `Debug.Enable Frame Buffer Throttling`** (byte `+0x550`): read its loads to say
  whether 2048 has a width or size throttle of its own
  (`docs/ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md`, "Omega and 2048").
- **The PSP builds' builder**: whether Pulse and Pure have a near rule at all.
- **Which render modes 2048's two builders serve**, and whether the original's sprite templates
  go through the same routine (ours cull and fade them, with no cap).

## Next Steps

1. Film an HD wall hit for the streak quads.
