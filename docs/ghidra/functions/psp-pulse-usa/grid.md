# The starting grid

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** **both halves answered.** How racers are *ordered* onto the grid is
read end to end at confidence **82**; where slot N sits in the world is
**measured against the running original** at confidence **94** - eight craft
read out of memory while the countdown held them on the grid.

This page exists because [`track.md`](../../../formats/track.md#start-position)
established that a track authors exactly **one** `Start Position` node across
all 40 files, so seven of the eight slots have to come from code. The
[roadmap](../../../overview/roadmap.md) has it blocking M5.

## What is answered: the order is a shipped permutation table

`Race_SpawnGrid` (`0x088247e0`) builds the grid before any craft exists:

```c
uint slots[8];
for (i = 0; i < 8; i++) slots[i] = 0xffffffff;

for (racer = 0; racer < g_racer_count; racer++)
    slots[g_grid_orders[FUN_0894d440() * 8 + racer]] = racer;

// compaction: bubble every assigned slot toward the high end until nothing moves
do {
    moved = false;
    for (i = 0; i < 7; i++)
        if (slots[i] != -1 && slots[i + 1] == -1) {
            slots[i + 1] = slots[i]; slots[i] = -1; moved = true;
        }
} while (moved);

for (racer = 0; racer < g_racer_count; racer++) {
    slot = index of racer in slots;
    if (this racer is the local player) FUN_0882821c(..., racer, id, slot, ...);
    else                               Race_SpawnAiRacer(..., racer, id, slot, ...);
}
```

Three things follow:

- **The order is authored, not sorted or drawn.** `g_grid_orders`
  (`0x08ab0a90`) is a table of eight-`int` permutations, stride `0x20`,
  compiled into the executable rather than read off the disc. The first four
  rows are the identity, its reverse, a pairwise swap and a genuine shuffle,
  so the table is a set of pre-baked orderings and `FUN_0894d440()` picks the
  row. What that function returns - a per-race index, a championship round, a
  seed - is **not read**.
- **A short grid packs to the back.** The compaction loop runs until nothing
  moves, so a three-racer field occupies slots 6, 7 and 8 rather than 1, 2 and
  3. That is a real behaviour and a cheap one to get wrong.
- **The local player is forced to the back in the AI's own view.** `Ai_Construct`
  (`0x088536bc`) stores the slot at `obj + 0x50`, and for the local player it
  **overwrites it with `8`** before use. Five adjacent fields
  (`obj + 0x74 .. 0x84`) are then all set to `8.0 - slot`, which is an
  AI-strength or aggression term scaled by grid position: the front of the grid
  gets the small value. Confidence **75** on that reading - the arithmetic is
  unambiguous, what consumes the five fields is not read.

## The geometry, measured

**Two staggered columns, and the authored `Start Position` node is slot 8.**

Eight craft were read out of PPSSPP's memory while the countdown held them on
the grid: `scripts/psp-drive.py menu --single-race` walks the front end into the
one reachable race type with a full field, and the racer table at
`0x08b34420` (stride `0xdc * 4`, count at `0x08b35fa0`) gives every craft and
its AI object, whose `+0x50` is the slot. Fourteen samples over the countdown,
each craft's offset taken in its **predecessor's own basis** so track curvature
cannot accumulate:

| Measurement | Value | Spread |
| --- | ---: | ---: |
| Lateral step, slot to slot | alternating `+/-20.00` | sd `0.02` |
| Forward step, slot to slot | `-19.79` | sd `0.07` |
| Heading, any slot against slot 1 | `1.0000` | exact to 4 dp |
| Grid span, slot 1 to slot 8 | `138.55` | sd `0.001` |

Relative to slot 8, which is the anchor, the rule is:

```
position(slot) = node.position
               + node.forward * (8 - slot) * 19.79
               + node.row0    * (slot odd ? 20.0 : 0.0)
orientation    = node.orientation
```

**That the node is slot 8 is measured, not assumed.** This project's spawn from
the authored node lands **1.84 units** from where the original puts its own
eighth craft, against **139.7** from where a time trial starts. Two things
[`track.md`](../../../formats/track.md#start-position) recorded as puzzles fall
out of it: the node being "3.2-20.5 units off the centreline" is the lateral
stagger, and "137.9 units behind where a time trial starts" is the length of the
grid.

**The two constants are measured rather than read**, and this crate says so
where it uses them (`oag_gameplay::spawn::GRID_ROW_PITCH`,
`GRID_COLUMN_OFFSET`). `20.0` is almost certainly the authored value - the four
odd slots read `19.882`, `19.993`, `20.035`, `20.016` and the even ones are
within `0.05` of zero. `19.79` is a mean of per-step values between `19.763`
and `19.862`, and a recovered literal would be exact where this is not.
Whoever finds them in the executable should replace both and say so.

**A note on `StartPosition_Bind` that this did not need but the next reader
might.** It reads an integer `Position` attribute off the node, registers the
node in the scene as `start_position_%d`, and orthonormalises the matrix in
place (Gram-Schmidt: normalise row 1, orthogonalise row 2 against it, cross for
row 0) - but **only when `Position == 1`**. So the format supports more authored
slots than any shipped track uses, and the format string at `0x08a88864` has
exactly one xref, the writer. Since the geometry turned out to be offsets from
one node, that dead end is now explained rather than merely unexplored.

## Names landed

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x088247e0` | `Race_SpawnGrid` | 82 |
| `0x088253f8` | `Race_SpawnAiRacer` | 78 |
| `0x088536bc` | `Ai_Construct` | 80 |
| `0x08926ae8` | `StartPosition_Bind` | 82 |
| `0x08ab0a90` | `g_grid_orders` | 82 |

`0x0882821c`, the local player's twin of `Race_SpawnAiRacer`, is **deliberately
not named**: it is only inferred from the call site's position in the
`if`/`else`, and it has not been read.

## What is still open

- **What picks the permutation row.** `FUN_0894d440()` is unread, so whether the
  order is per-race, per-championship-round or seeded is not known. The identity
  row exists, so a single observation cannot tell "the table was consulted" from
  "the table returned the identity" - and the one live run had the local player
  as the last racer, which the identity row also puts in slot 8, so **that run
  cannot separate the permutation from `Ai_Construct`'s forcing either**.
- **Whether the constants are authored or derived**, above.
- **Nothing spawns opponents yet.** `oag_gameplay::spawn::grid_pose` is the
  geometry and is tested; putting seven more craft on it needs AI, which is a
  separate M5 item.

## History

- 2026-08-10: geometry measured against the running original; the authored node
  identified as slot 8. Ordering recovered the same day.
- 2026-08-10: page created. Ordering recovered; geometry open.
