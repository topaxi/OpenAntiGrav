---
status: measure
titles: [hd, omega, 2048]
updated: 2026-11-27
summary: "Owed: HD Mine/Detonator films, Bomb colour and spark gaps, Omega ring, Rocket blast gaps, cap source, near pairs to adopt; half-res parked"
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
- **The far Rocket blast: the cloud and the grey streams are answered (2026-11-27)**
  (`particle-fill.md`, "The far blast's grey streams are the Rockets' own ribbons"). The streams are
  the Rockets' ribbons from a launch 28 units back; HD draws the ribbons before the particles (wired).
  The cloud is two waves at two measured points. **For the lead to adopt** (pose decisions, not
  laws), on both far pairs:
  - replace `--force-rocket-hit 717:7` with
    `--force-rocket-hit-at 716:394.34,-35.94,-120.31 --force-rocket-hit-at 719:398.98,-35.30,-117.96:2`
    (keep `--force-rocket-hit-velocity`): 11.79 to 9.11 and 43.17 to 34.81, -8.13 +/- 0.35 over 8
    seeds on 0.233 s;
  - the lead adopts the two waves only; not the launch pose. For the record, the launch (`--pose-hold-launch=717:...` and `ours.inputs`
    `701 none / 1 square / 408 none`, values on the page): faithful, the streams appear, but 0.16 and
    1.11 worse than the two waves alone until the two items below land.
  Left:
  - **RE item for a later opus lane:** the ribbon's vertex colour. `RibbonEffects_SampleLighting`
    (`0x002a4280`) looks the Rocket's position up in a spatial tree (`0x003c2598`/`0x003c2488` over
    `0x006d00b0`, data at `*0x00d43cc4`) and packs an RGB; which file fills it and its layout are
    unread. Ours draws white, the original's streams are grey;
  - the fan at the eye is wider than `spread` 0.05 gives: ours are three narrow strips right of
    centre, the original's four wide bands span the frame's bottom. Unread;
  - `trail` still reads as smooth white lobes (not looked at);
  - Omega's and 2048's ribbon order against their particles is inherited from HD: read Omega's
    ribbon render against its particle flush in its `eboot.bin`;
  - the five new HD Bomb and Missile pairs were not in the order change's census (their flags are
    newer): re-render them after the merge, the Missile ribbons and streaks moved before the
    particles.
- **The near pairs: their `field.csv` is 2 ticks ahead of their picture; the move is the lead's to adopt**
  (`particle-fill.md`, "The first wave and the drawn hull"). It was never the cloud: the film pairs
  fed ours the RAM poses, and the frame draws every craft 2 ticks back. That also explains the blast's
  "2-tick lag". `compare_env.py film-pair-field` moves a pair's field to the frame's instant. Adopted,
  0100 reads 53.96 (from 65.75) and 0233 50.36 (from 62.86), on their current flags. Still open:
  - ~~0233's craft stays hidden under white on every arm~~: closed 2026-11-27, the craft blast
    spreads by the extent law (`particle-fill.md`, "The trails start on the blast's hemisphere").
    Open: 0233's own-seed score went 50.36 to 64.14 while its 8-seed mean went 62.68 to 54.74
    (closer on 6 of 8), 0100 +0.42, both far pairs closer. Is the own-seed loss particle
    randomness against one frame (0233 spans 44 to 85 across seeds), or its single-point pose,
    which stands the blast 22.7 units out? the original's yellow on the craft is the
    additive fireballs in front of it and needs the blast at the original's sites. Re-posing
    0233 on the two waves (73.23 now, from 90.03) is the next step, a pose decision for the lead;
  - Omega authors the same trail heads (8.824) in both its files: checked, applies, not wired;
  - the two-wave pose is still not the best on 0233 (+1.09 +/- 3.64);
  - the wash quads are cut by the track where the original's cover the frame (hard edges on the
    floor);
  - why the original's second wave arrives later is unread.
- **Lead, not checked: the tick that lands a forced Rocket hit adds about 20 units a second upward
  to the struck craft** (set `y` 11, read 31 at the blast). The HD films' struck craft show no such
  jump, comparing frames 8 to 23 ticks apart. Whether it is the Rocket's impulse or our hover after
  the craft is moved onto the Rocket was not separated; only the first would matter in a race.
- **The size multiplier's other arms** (`particle-render.md`, "The per-particle size
  multiplier"): the parent-age arm (`TRAIL_SMOKE`, `plasma_goo_trail`, `SPARK_TRAILS`) and the
  random arm (most of 58 emitters; it adds a draw to the effect's generator). The run arm is
  wired.
- **Lead, not checked: HD's streak colour gain may be the size lower bound.** Ours dims the
  far-bar effects' class 6 streaks by `+0xc84` (`StreakDraw::colour_gain`, e.g. `0.7` on
  `WO_TRAIL_HITSHIP`), a reading taken from Omega, whose layout has a separate word there. HD's
  init reads `+0xc84` as the multiplier's lower bound. Check: does HD's `PartRender` draw read it?
- **Size multiplier and spawn range on 2048 and Omega: they apply, not wired.** Both inits carry
  HD's law; Omega's record offsets are 8 bytes later and `oag_pob` reads HD's
  (`particle-render.md`, "The atlas frame"). To read first: what their instance fraction holds.
- **Flipbook advance on 2048 and Omega is not wired**: 2048's update wraps at the frame count with
  no hold; Omega carries that one and HD's, and which serves which directory is unread.

### HD effect sprites: what is left

`docs/rendering/particle-fill.md`, "Which effects draw a sprite" has the list; "The Bomb's and the
Missile's explosions paired" has the first original pairs (Bomb and Missile) and what they decided.
Left:

- **The Mine is unfilmed**: pickup word 3 does not fire. Find the word (6 and 10 are the other
  unresolved ones), then `scripts/rpcs3-hd-blast-film.py --weapon mine --state N`. `WO_MINE_EXPLO`'s
  streaks (`streaks_fat`, `plasma_core`, `sparks`) wait on it.
- **The three Detonator explosions are unfilmed.**
- **Omega's smoke ring is not spread**: its `WO_BOMB_SMOKERING` authors HD's extents. One entry in
  Omega's `effects.extents`, inherited from HD; no Omega frame to judge it.
- **What the Bomb pairs still show** (same section, "Judged by eye"): the ring's sparks (class 6)
  are sprite quads in the original and procedural dashes in ours, and listing them as HD's bar pairs
  worse, so the bar's shape or colour for them is wrong somewhere; the fireball's colour; the halo of
  rays at 0.8 s.
- **What the Missile pairs still show**: crisper rings than the original's, an orange-tinted own
  craft at 0.167 s, and a metric that prefers a blast 4 to 6 ticks younger than the lag rule's at
  0.233 s. Its sprite template (`missile_shockring`) is muted (`TEMPLATES`) and was not tried.
- **Other streak emitters stay procedural**, in the other listed effects and in the 9 all-streak
  ones; each wants an original pair first.
- **Sprite templates of the other new effects stay muted**: not checked which of them author any.

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
  `WO_ROCKET_EXPLO` was tried listed on the four Rocket craft pairs (2026-11-27): the far pairs do
  not move (11.79 to 11.79, 43.17 to 43.20; 9.11 and 34.81 on the two-wave pose, unchanged), the
  near 0.1 s pair 54.38 to 54.39 and the near 0.233 s pair 64.13 to 65.33 on its own seed. Not
  listed: no pair favours it, and the far look does not depend on it.
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
