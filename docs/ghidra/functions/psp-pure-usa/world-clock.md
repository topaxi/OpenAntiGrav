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
less the presented-frame lag (inherited from Pulse), and writes `original.world_clock_s`,
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
