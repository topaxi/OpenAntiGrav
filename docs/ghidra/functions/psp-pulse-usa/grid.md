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

## Ported, and how close it lands

`oag_gameplay::spawn::grid_pose` is the layout and `oag_game::race::grid_poses`
drops each slot onto its own footprint. Eight craft take the grid on any track
that authors a `Start Position`; **nothing drives the seven opponents**, so they
hold station until AI lands.

`crates/game/tests/race_ground_truth.rs::our_grid_is_the_originals_grid` compares
every slot against the eight positions above, on the same track:

| | worst | where it goes |
| --- | ---: | --- |
| Slot 8, our anchor | `1.68` | the authored node dropped onto the collision mesh |
| Worst slot, whole grid | `2.40` | the anchor, plus about `0.7` |

The residual is **near-constant across all eight** (`1.68` to `2.40`), which is
what says the layout is right and the anchor is what is off: a wrong pitch or a
wrong column would grow the error toward one end.

Two known departures, both sub-unit and both stated rather than papered over:

- **The original's grid follows the track's curve; ours does not.** Its
  odd-column `z` runs `-195.945` to `-195.362` over the grid's length while ours
  is flat at `-193.616`. The offsets are straight lines from one node, and the
  original's evidently are not - or are taken from a curve this has not read.
- **Each slot is re-dropped onto the collision surface under it**, which the
  original may not do the same way. It is necessary here because a grid is 138
  units long and no track is flat over that; sharing slot 8's `y` buried the
  front of the field.

### The sign that got it wrong first

The lateral offset goes along the craft's **left**. Taking it along
`oag_physics::Body::right` put the front of the grid off the outside of the
first corner - visible in a screenshot, and caught properly by
`the_whole_grid_lands_on_the_track`, which found slot 1 with no collision
surface under it.

The cause is the convention `engine.md` already records: **the original's row 0
is the craft's left**, where this project's `Body::right` is its right. Measured
directly on the same grid, the original's row 0 reads `(-0.006, -0.008, -0.9999)`
where our own right axis reads `(0, 0, 1)`. The measurement had been projected
onto the original's row 0 and then implemented against ours, and the two are
opposite. **No unit test on an identity node can catch that** - it is a fact
about which way the axis points on a real track, not about the arithmetic.

## Reversed grids: the straight line ran off the curve

**Fixed 2026-08-19, and this is an engineering correction to the port, not a new
reading of the original** - nothing below changes what `Race_SpawnGrid` or the
geometry table above says the original does, and no new live capture backs it.

The two known departures above were measured on one track's *forward* grid and
called sub-unit. On a *reversed* grid the same straight-line formula is not
sub-unit wrong, it is off the track: a sweep of every Wipeout HD circuit's grid,
both directions, with opponents on
(`crates/game/tests/hd_trackwall_ground_truth.rs::every_hd_grid_lands_on_the_track`)
found **9 of 12 reversed grids** putting one or more of the eight slots off the
collision mesh entirely - up to **145 units** from the driveable line on
`tech_de_ra`'s reversed grid - against one forward grid over its envelope by a
single slot. Talon's Junction (`16_Track` on Pulse) is not one of the nine,
which is why the only track this project's grid was ever live-verified against
gave no sign of the problem: its own reversed grid's start straight happens to
be close enough to flat that a straight line stays on it.

The reason is the same shape every time. A reversed circuit's grid sits on
whatever piece of track its own `Start Position` node landed on - a value
authored independently of the forward grid's, per
[`track.md`](../../../formats/track.md#start-position) - and that is far more
often a curve than the authored front straight a forward grid sits on. The
straight-line formula extrapolates in one fixed direction for the full
138-unit span, and a curve underneath it means slot 1 ends up somewhere the
track is not.

**The fix keeps both measured constants** (`GRID_ROW_PITCH`, `GRID_COLUMN_OFFSET`)
and changes only how they are applied: `oag_game::race::spawn::grid_poses` walks
the track's own resampled spline from the anchor, `GRID_ROW_PITCH` units per
step, instead of projecting along the anchor's fixed forward axis, and stagger
uses each walked sample's own `lateral` axis instead of the anchor's fixed one -
the same axis [`Pose::from_sample`](../../../../crates/gameplay/src/spawn.rs)
already uses for the same reason. Heading is untouched: every slot still shares
the anchor's own orientation, which is the part the live capture actually
confirmed to four decimal places, and this does not reopen that. Two guards
keep it from being wrong in a new way: the walk never crosses into a different
*path* in [`Spline`](../../../../crates/game/src/race/spline.rs)'s own table -
consecutive indices are only spatially adjacent within one path, and crossing
one landed grid slots on an unrelated branch on a first attempt - and a walk
that runs off the end of the path before covering the full distance falls back
to the old straight line rather than trusting a short answer, which is what
keeps this project's own short synthetic test tracks (well under 138 units)
working.

Re-run after the fix, the same sweep finds **one slot on one circuit** still
short: `modesto_heights`'s reversed grid has one slot with no collision surface
within the probe's reach, 21.42 units from the driveable line - inside the
track's own half-width, so a probe missing the surface at one specific spot
rather than a slot off the track, and not chased further. The regression test
records it as a named, bounded exception rather than silently passing or
silently widening its own tolerance.

**Confidence 70** on the mechanism (the curve-following geometry is measured
against the sweep above, which is this project's own collision mesh, not the
original), and the forward-grid live comparison
(`our_grid_is_the_originals_grid`) still passes at the same 3-unit tolerance
afterward, so the fix cost nothing on the one number that *is* checked against
the original.

**A separate, pre-existing gap this incidentally found and did not fix.**
Relocating exactly where an opponent first meets trouble on `05_Track` - a
circuit `docs/gameplay/ai.md` already measures as having 134 racing-line samples
with no collision surface under them - moved
`crates/game/tests/stall_rescue_ground_truth.rs::a_craft_that_stops_on_the_disc_is_put_back`
onto a spot where the craft gets wedged bouncing rather than sitting still: a
tick-by-tick trace shows its position frozen for thousands of ticks while its
speed oscillates between about 0.2 and 10 units/s, never staying below
`STALL_SPEED` for long enough to trip the rescue's three-term "stalled" test,
which requires *sustained* low speed. That gap in the rescue predicate predates
this fix and is not touched here; the test is left failing rather than adjusted
to hide it, since it is evidence of a real, different, open problem.

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
- ~~**Nothing drives the opponents**, and they all wear the player's hull.~~
  Both closed: the AI landed 2026-08-11 and per-team hulls on 2026-08-15. A
  slot now flies its own team's `Data\Ships\<Team>\Ship.vex` - eight
  different models, 845 to 1,497 triangles - with its own nozzle and plume. See
  `crates/game/src/livery.rs`. **Which team flies which slot is still
  unrecovered**: `Race_SpawnAiRacer`'s `id` comes from a racer list built
  upstream that nothing has read, so the ordering in use is this project's and
  is labelled as such in the load report.
- **`modesto_heights`'s reversed grid, one slot short.** See "Reversed grids"
  above - inside the track's own bounds, not chased further.
- **The stall rescue does not catch a craft bouncing in place.** Found while
  fixing the reversed grids, above, and not this page's subsystem - tracked
  against `crates/game/tests/stall_rescue_ground_truth.rs`, not here.

## History

- 2026-08-19: reversed grids fixed - `grid_poses` walks the track's own spline
  instead of extrapolating a straight line from the anchor, closing 8 of 9
  broken reversed grids a sweep of Wipeout HD's circuits found. See "Reversed
  grids" above.
- 2026-08-10: geometry measured against the running original; the authored node
  identified as slot 8. Ordering recovered the same day.
- 2026-08-10: page created. Ordering recovered; geometry open.
