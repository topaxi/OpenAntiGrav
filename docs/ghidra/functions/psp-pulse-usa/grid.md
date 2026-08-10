# The starting grid

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** **half recovered.** How racers are *ordered* onto the grid is read
end to end at confidence **82**; where slot N sits *in the world* is not found
and is the open half.

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

## What is not answered: where slot N is

Nothing found so far turns a slot index into a world pose. The two leads, in
the order worth trying:

1. **`StartPosition_Bind` (`0x08926ae8`) does more than decode a matrix.** It
   reads an integer `Position` attribute off the node, **registers the node in
   the scene under the name `start_position_%d`**, and orthonormalises the
   matrix in place (Gram-Schmidt: normalise row 1, orthogonalise row 2 against
   it, cross for row 0). Crucially it does all of that **only when `Position ==
   1`** - so the format supports more than one authored slot even though no
   shipped track uses it. A consumer looking slots up by that name is the
   obvious thing to search for, and the format string at `0x08a88864` has
   **exactly one xref**, the writer. So either the lookup builds the name some
   other way, or slots 2-8 are offsets from slot 1.
2. **The offsets, if that is what they are, are not in `Ai_Construct`** - it
   stores the slot and never touches a position.

**Not yet tried, and probably the fastest route**: a live read. Break at the
start of a race with a full grid and read all eight craft positions; the
formation falls out of the geometry directly, and a row/column spacing is much
easier to recognise than to find in a decompiler. The obstacle is reaching an
eight-ship race - `scripts/psp-drive.py menu` walks into a Time Trial, which has
one craft, so this needs a different menu walk.

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

## History

- 2026-08-10: page created. Ordering recovered; geometry open.
