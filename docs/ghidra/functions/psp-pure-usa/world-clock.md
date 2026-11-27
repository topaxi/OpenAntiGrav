# Pure's scenery clock: `*(0x08a8c5f4) + 0x44`

Where Pure keeps the time its animated scenery runs on, so a comparison pair can read it
at the grab and ours can be put on it (`--world-clock-offset`), as Pulse's pairs do
([`psp-pulse-usa/texture-animation.md`](../psp-pulse-usa/texture-animation.md)).
Read live in PPSSPP (USA image, `UCUS98612_2.00`), 2026-11-18, `clock-titles` lane, from
`time-trial-sebenco-climb-vector-feisar-restart.ppst`. Pure's `BOOT.BIN` is relocatable;
the addresses are the loaded ones (the module loads at its linked base, so they match the
`0x08804000` image base the other Pure pages use).

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08a8c5f4` | `g_world_state` | 75 |
| `0x0888681c` | `WorldNode_AdvanceAnimation` | 65 |

## What was found

1. **A float shared by 61 objects.** Two full RAM dumps (`0x08800000` to `0x0a000000`) 61
   `Ship_UpdateCraft` stops apart (2.035 s of frame time, the sum of the craft's `dt` at
   `+0x1c8`) show 61 words that advance by exactly that delta (338.572 to 340.607 on that
   load) and hold the same value, spaced by the nodes' stride. Nothing else with a delta of
   2.035 s repeats like that. (Confidence 85 that this is the animation clock: it is one
   value, shared, advancing at frame time.)
2. **Its writer, per node.** A write watch (`memory.breakpoint.add`, write) on one of the
   61 stops at `0x08886908`, `swc1 f12, 0x134(s0)` with `f12 = *(*(0x08a8c5f4) + 0x44)`.
   The function around it (`0x0888681c`) reads `world + 0x44 - node + 0x134`, scales it
   by a per-node rate (`node + 0x108`, `+0x110`, a divide by `*(node + 0x104) + 0x30`),
   adds it to the node's own phase at `node + 0xc0`, and stores the new world time in
   `node + 0x134`. So `node + 0x134` is the node's *last seen world time*: the node's phase
   is an integral of the world clock, not a read of it, and an offset from the clock puts
   ours on the original's phase only for nodes that started at the circuit's load.
3. **The world object's writer.** A write watch on `*(0x08a8c5f4) + 0x44` stops at
   `0x0880c790`, `swc1 f12, 0x44(s3)` after `add.s f12, f13, f12`: the frame's `dt` added to
   the running time, inside the large per-frame update that also holds a frame counter in
   `a0` (`0x27ed` frames at 338.6 s: 30 frames a second, the PSP game's frame rate).
4. **Not the HUD clock.** The race clock (`craft + 0x130`, `psp-pure-pair.py`) reads 0.000
   in a restart state's countdown while this clock reads 10.8 s (Sebenco Climb's state).
   The pair records `world clock - race clock`.

## What the pair does with it

`scripts/psp-pure-pair.py` reads `*(0x08a8c5f4) + 0x44` at the same stop as the race clock,
less the presented-frame lag (`compare_env.PURE_PRESENTED_LAG`, two frames, read off Pure's HUD digits: [compare-page.md](../../../tools/compare-page.md), "The presented lag, wired"), and writes `original.world_clock_s`,
`original.world_clock_offset_s` (world clock less race clock) and `ours.world_clock_offset_s`
(world clock less ours' tick over 60), and passes `--world-clock-offset` to ours.

**Measured, one pair** (`10_Sebenco_Climb`, Time Trial restart, 0.80 s into the race):
world clock 10.84 s, ours at offset 5.508 s and tick 320. The cyan energy beam across
the track scrolls with the clock. Sweeping ours' offset (0 to 8 s in 0.25 s steps, and 9,
12, 20) and correlating the beam band's brightness along x with the original's gives
correlation 0.862 at 5.5 s and 0.853 at 5.75 s, 0.43 at 0 s, 0.22 at 7.0 s and 0.68 to 0.70 at
12 and 20 s: the maximum is at the value read from RAM, and the falsifier (the maximum
at another offset) did not fire. The beam sits in the same place on both sides; its dash
phase matches to within about a quarter of a second of offset. **Confidence 75** for the law
(`world + 0x44` places the beam), 85 for the read.

## Caveats and open

- **A state load is not frame-exact in this field.** Three loads of the same state read
  10.908, 10.941 and 10.975 s (one frame, 0.0334 s, each) at the same `Ship_UpdateCraft`
  stop, and an earlier session read 338.6 s. The pose and race clock are bit-identical
  across loads, the world clock is not: the pair must read it at the frame it grabs, which
  it does. Cause not found (a state restores RAM; the timing of the first stop may differ).
- The 0x0880c78c add and the `a0` frame counter sit in a function that is not read;
  `WorldNode_AdvanceAnimation` is named from its shape (confidence 65).
- 2048 and Omega: see their own pages; Pure's layout is not assumed for them.

## The stored Pure pairs predate the read

The twelve race pairs under `data/reference/compare-page/pure-psp/` were captured before this read was wired: none
records `original.world_clock_offset_s`, so ours animated from its own tick 0. On Manor Top the same held pose read
mean grey 14.53, 13.47, 13.75, 10.65, 7.49 at ticks 330, 345, 360, 375, 390 and 6.78 at 540. The four Classic states
re-captured with the clock read 9.32 -> 4.31 (Karbonis), 12.09 -> 6.86 (Mandrashee), 13.75 -> 6.36 (Manor Top) and
9.12 -> 9.04 (Sagarmartha, whose remainder is the hull's 4 px shift); the original's world clock reads 10.04 s at
race-clock zero in all four (`original.world_clock_offset_s`). Confidence 85 that the phase was the cause (the
re-captures move the score by the amount the tick sweep predicted). The other eight Pure race pairs follow below
(`docs/tools/compare-page.md`, "The Classic hull").

## The other Pure race pairs, re-captured with the clock

Seven Beta, Alpha and Sol 2 pairs, re-captured from their saved states with `scripts/psp-pure-pair.py` (the pair's own stored `argv`, a private
PPSSPP, USA image) and their `ours` re-rendered. **Control:** the same binary on the same `theirs.png` without
`--world-clock-offset` reproduces every stored mean grey to the digit, and the re-captured `theirs.png` is byte-identical to
the stored one (RMSE 0 on all seven), so the whole difference below is ours' scenery clock. Mean grey, stored (no clock) to re-captured:

| Pair | Before | After | `original.world_clock_offset_s` | Ours' offset, tick |
| --- | ---: | ---: | ---: | --- |
| `sebenco-climb` | 15.56 | 13.99 | 10.041 | 5.508 s, 320 |
| `sinucit` | 15.43 | 14.74 | 10.042 | 5.510 s, 398 |
| `sol-2` | 10.27 | 9.37 | 10.041 | 5.510 s, 380 |
| `citta-nuova` | 8.77 | 7.12 | 10.041 | 5.511 s, 444 |
| `chenghou` | 7.63 | 6.63 | 10.041 | 5.514 s, 664 |
| `vineta` | 7.37 | 6.33 | 616.021 | 611.488 s, 1271 |
| `blue-ridge` | 7.23 | 6.56 | 10.041 | 5.514 s, 676 |

The gain is 0.7 to 1.7 grey, against 3 to 7 on the Classic pairs: the Classic scenery animates more of the frame.
Every hull sat at the original's px offset (0, 0), but Blue Ridge at (-1, 0). Two loads of each state on one boot give bit-identical
`theirs.png`, `ours.png` and world-clock read (one boot: confidence 60 on the number, 85 that the clock is the cause, from the control).

- **Vineta's state holds 616.02 s** against 10.04 s on the other six: it was saved in a long session, not at a fresh load.
  The read survived a sweep of ours' offset (6.77 at 5.51 s; 6.87, 6.75, 6.57, **6.32**, 6.51, 6.68, 6.75 from 610.74 to 612.24 s
  in 0.25 s steps; 7.21 at 616.5 s): the minimum sits at the value read, 611.487 s.
- **The weather is not on this clock.** `Race::world_clock_seconds` feeds the world meshes' animation, mod shaders and ribbons. The mist
  layers (`oag_fx::mist::Layers`, stepped per tick from the camera's motion, their random UV offsets from ours' seeded `Rng`) and the rain
  and snow emitters on Sinucit and Sebenco Climb are not. The original re-rolls the layers with `Psys_RandFloatRange`
  (`docs/rendering/pure-weather.md`), so a clock read cannot place them. On the two weather pairs the residual is mostly low frequency (below), the shape of
  an unclocked mist; Sebenco still gained 1.57 from the clock, Sinucit 0.69.

### Where the rest is, the worst three

Regions on the 480x272 frame, each pasted from the original over ours with the mean grey recomputed (the HD page's method), disjoint
(HUD glyph boxes, named features, hull box `188,172 to 300,272`, road rows 150 and below, then the upper scene):

| Pair | Largest region | Share of error, of area | Cause |
| --- | --- | ---: | --- |
| `sinucit` (14.74) | upper scene, sky and buildings | 38.9 %, of 43.7 % | low frequency: a flat lift, ours darker by +11 grey (blurred difference 12.6 of 13.9); the weather mist and rain, unclocked. Ours' upper scene reads 78.2 at tick 2 and 75.4 at tick 398 on the same scenery phase, so ours' mist also thins under cover over the run (a lead: the original's opacity is not read). Advert screens above the road: 4.9 % on 3.2 % of the frame (23 billboard-slot draws suppressed, `pure-weather.md`) |
| `sebenco-climb` (13.99) | upper scene | 36.8 %, of 46.9 % | low frequency 8.8 of 10.9, signed +4.8: the snow mist again. The densest box is the bottom-left HUD line (12.4 % on 5.3 %: the original prints `COSMOS KINECTION`, ours `COLD STORAGE`), out of scope; then the hull box (12.9 % on 8.6 %, original brighter by +15, the known hull lead; the bottom-left line is the original's two-line music text, see [now-playing.md](../../../formats/now-playing.md), "Pure's own music text") |
| `sol-2` (9.37) | steam plume, `270,50 to 480,150` | 29.6 %, of 16.1 % | the original's cloud is denser and larger, covering the Harimau sign; ours' is thinner and sits lower. A particle phase, no world clock reaches it |

No code changed: none of the three regions is a disc value ours reads wrong that a test can prove.

## All twelve Pure race pairs carry the clock (checked 2026-11-25)

`pure-classic-vr-set` was briefed that eight Pure race pairs had no world clock. They had: every `pair.json` under `data/reference/compare-page/pure-psp/`
that is a race (twelve) records `original.world_clock_offset_s` and `ours.world_clock_offset_s`, and each `ours.flags` carries `--world-clock-offset`
(`pure-world-clock` and `pure-modesto-hull`, 2026-11-24). Mean grey, no clock to the clock to the pair as stored at main `41d94285f`:

| Pair | No clock | Clock read | Stored now |
| --- | ---: | ---: | ---: |
| `karbonis` | 9.32 | 4.31 | 4.20 |
| `mandrashee` | 12.09 | 6.86 | 6.73 |
| `manor-top` | 13.75 | 6.36 | 6.24 |
| `sagarmartha` | 9.12 | 9.04 | 9.06 |
| `modesto` | 6.19 | 4.38 | 4.16 |
| `blue-ridge` | 7.23 | 6.56 | 6.51 |
| `chenghou` | 7.63 | 6.63 | 6.43 |
| `citta-nuova` | 8.77 | 7.12 | 6.83 |
| `sebenco-climb` | 15.56 | 13.99 | 14.01 |
| `sinucit` | 15.43 | 14.74 | 14.71 |
| `sol-2` | 10.27 | 9.37 | 9.23 |
| `vineta` | 7.37 | 6.33 | 6.04 |

The last column moves with ours' later renderer changes (version-4 texture levels, the Selector and menu work), not with the clock; the gain
from the clock is the middle column against the first (Sagarmartha's 0.08 is the hull's 4 px shift swamping the scenery). Every one of the eight
non-Classic pairs dropped by more than the 0.3 re-capture noise: 0.67 to 1.65 grey on seven, and 1.81 on Modesto, whose re-capture also carried the
texture-level fix (`hull-shine.md`).

**A second boot, 2026-11-25** (private PPSSPP 1.20.4, software renderer, USA image, a fresh process per state; the state loaded twice per boot):
Karbonis reads world clock 11.577 s at the first `Ship_UpdateCraft` stop and `launch-grade a0 + 0x130` race clock 1.5349 s, identical on both
loads of both boots, so the world-minus-race offset is 10.0421 s against the stored pair's 10.0418 (0.0003 s apart); Sebenco Climb reads 10.908 and
0.8676 s, offset 10.0404 against the stored 10.0409. The stored pairs' own stop reads one frame earlier in both (Karbonis race clock 1.5016 s):
the documented one-frame variation of a load, which the offset is immune to. Two boots on two states: confidence **80** that the offset is
repeatable to a frame. A world clock read after the emulator has been stepped for minutes is not the state's: one session that stepped a loaded
Karbonis frame by frame for 560 s read 260 s afterwards (the clock runs on, 0.0333 s a frame); read it straight after a load.
