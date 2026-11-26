---
status: ready-to-wire
titles: [hd, omega, 2048]
updated: 2026-11-26
summary: "Ready: 31 unsprited HD effects, over-life flipbooks. Owed: cap source, flare .gtf rows, Deck 4K hit; half-res awaits maintainer"
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
- **HD advances its flipbooks and ours holds the spawn frame** (`particle-fill.md`, "HD
  advances its flipbooks"). Ready to wire, over-life only: keep `FrameAdvance::OverLife` in
  `without_pulse_psp_draw_but` for `WO_ROCKET_EXPLO`'s four non-random layers (a title field,
  ADR-0058), then re-score the Rocket pairs. They stay on cell 0, the explosion's seed, today.
- **Not read, do not wire on a guess:** HD's frame-rate-channel advance and its random spawn
  frame. `FIREBALL_ADDITIVE` (wall and ship blasts, no flag `0x40`) is on the rate law and held on
  cell 0; no film caught it. 2048 and Omega: same sheets, their advance unread.
- **The 12-unit pairs' hit tick (`--force-rocket-hit 711:7`) was never checked against the
  original's spawn point.** The far films put the wash quad in world space (view position through
  the frame's camera); the same on `hit2` says where ours should stand.
- **A posed blast carries our opponent's velocity, not the original's**, so travelling fireballs
  do not match in a pair. A flag that writes the struck craft's velocity at the forced hit would
  take that out of the metric.

### Ready to wire: 31 HD effects load and draw no sprite of their own

Census over the disc (every effect HD's loader loads that is not in `oag_hd`'s `SPRITES`): 89
emitter and template texture references, and **every one exists as `/data/psys/tex/<stem>.gtf`
and decodes**. We draw them with the procedural profile (`docs/formats/pob.md`, "play the
data"). The Detonator Mine and Bomb explosions' `Debris_DetonatorMines`, `Debris_LightBarrier`
and `explosion_fijets_8x8_grey_gaseous` were seen bound by the original
(`docs/rendering/particle-fill.md`, "Every effect that loads a sprite").

To wire: add the effect to `SPRITES` (`crates/hd/src/effects.rs`), then pair it: each one
changes what a race draws. Weapons first (Missile, Mine, Bomb, Detonator rows).

| Effect | `.gtf` stems referenced |
| --- | --- |
| `WO_SHIP_COLL_SPARK_DAMAGE` | `quakesmoke32x32`, `spark_mblurred_grey`, `spark_mblurred` |
| `WO_SHIP_COLL_SPARK_NODAMAGE` | `spark_mblurred_grey`, `yellow_glow`, `spark_mblurred` |
| `WO_ROCKET_FLARE` | `DC_Flare_rich`, `fireball_1024x512_yellow`, `spark_mblurred`, `smoke_anim_8x4` |
| `WO_SHURIKEN_HEAD` | `shuriken_trail_anim` |
| `WO_SHURIKEN_TRAIL` | `shuriken_trail_anim` |
| `WO_SHURIKEN_BOUNCE` | `pointglow`, `DC_Flare_rich`, `DC_Circle` |
| `WO_SHURIKEN_EXPIRE` | `pointglow`, `DC_Flare_rich`, `shuriken_trail_anim` |
| `WO_MISSILE_EXPLO` | `molten_debris_64x64`, `missile_shockring_original_HD_new` |
| `WO_MISSILE_BOUNCE` | `spark_mblurred`, `molten_debris_64x64` |
| `WO_MINE_EXPLO` | `smoke_anim_8x4`, `debris006_128x128x4`, `explosion_01_blurred_yellow_streaks_128x128`, `DC_Circle_rich_anim_4x2`, `spark_mblurred_grey` |
| `WO_BOMB_SMOKERING` | `debris006_128x128x4`, `DC_Flare_D`, `spark_mblurred`, `smoke_anim_8x4` |
| `WO_BOMB_RAYS` | `bomb_spike` |
| `WO_CANNON_SPARKS` | `smoke_anim_8x4`, `explosion_01_blurred_yellow_streaks_128x128`, `spark_mblurred`, `molten_debris_64x64`, `explosion_fijets_8x8` |
| `WO_MINE_EXPLO_DETONATOR_DEAD` | `Debris_DetonatorMines`, `explosion_01_blurred_white_streaks_128x128`, `spark_mblurred_grey` |
| `WO_MINE_EXPLO_DETONATOR` | `glow_white`, `DC_Flare_D`, `spark_mblurred_grey`, `Debris_DetonatorMines`, `explosion_01_blurred_white_streaks_128x128`, `explosion_fijets_8x8_grey_gaseous` |
| `WO_BOMB_EXPLO_DETONATOR` | `glow_white`, `DC_Flare_D`, `spark_mblurred_grey`, `Debris_LightBarrier`, `explosion_01_blurred_white_streaks_128x128` |
| `WO_REPULSER_BLAST` | `DC_Circle_rich` |
| `WO_REPULSER` | `pulse_repulsor_sprite1_ADD`, `psysed_default_glow` |
| `WO_LEACHBEAM_ENERGY` | `plasma_glare_ADD` |
| `WO_LEACHBEAM_CHARGING` | `spark_mblurred_grey` |
| `WO_LEACHBEAM_ABSORB` | `spark_mblurred_grey` |
| `WO_SHIP_SPARK_DAMAGE_WEAPON` | `yellow_glow`, `spark_mblurred` |
| `WO_WEAPON_ABSORB` | `HD_electric_arc_8x8`, `DC_Flare_A_blurred_bright`, `particl3_blue` |
| `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` | `spark_mblurred_grey`, `blue_glow2`, `vandergraf_balls_1024x512` |
| `WO_SHIP_DEATH_DAMAGE_PLUME` | `muzzle_flare_pSprite_64x64_ADD`, `smoke_new4_8x4`, `spark_mblurred`, `fireball_1024x512_yellow` |
| `WO_DEBRIS_FIRE` | `muzzle_flare_pSprite_64x64_ADD`, `smoke_new4_8x4`, `spark_mblurred`, `fireball_1024x512_yellow` |
| `WO_DAMAGE_MILD` | `muzzle_flare_pSprite_64x64_ADD`, `spark_mblurred` |
| `WO_DAMAGE_MODERATE` | `muzzle_flare_pSprite_64x64_ADD`, `spark_mblurred`, `smoke_anim_8x4` |
| `WO_DAMAGE_CRITICAL` | `fireball_1024x512_yellow` |
| `WO_BLUE_WELDER` | `blue_glow2`, `disruptor_spark` |
| `WO_MODESTO_STEAM_A` | `quakesmoke32x32` |

### Particles into a half-resolution target, composited

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
  - the blasts at ticks 726 and 756 (79 and 75 ms), which no change reached;
  - tick 784, now 51 ms against 43 before the blast carry and sprites ("What fills tick 784" below);
  - the fragment `discard` of "Measured and dropped", not run on the Deck.
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

1. Adopt the re-rendered rocket-hit and far-hit pairs.
3. Split the fill of ticks 726 and 784 per effect on the Deck's 3840x2160 run.
4. Put the half-resolution particle target to the maintainer, with the Deck numbers.
