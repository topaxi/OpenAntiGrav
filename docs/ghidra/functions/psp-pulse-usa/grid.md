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

  **It cannot be the whole story for a field of one, and the conflict is
  recorded here rather than resolved.** Taken literally, compaction puts a
  solo racer on slot 8 - which is the authored node itself, per this page's own
  measurement below. But
  [`track.md`](../../../formats/track.md#start-position)'s live capture of a
  **time trial** on `16_Track` puts the craft **137.9 units ahead** of that
  node and 22.4 to its left, which is slot 1's own offset, not slot 8's. Both
  numbers are measured against the running original, so they cannot both
  describe one code path.

  The likeliest reading is that a **solo mode does not route through
  `Race_SpawnGrid` at all** - there is no field to order or compact - and that
  the compaction rule above governs only a race that fields opponents.
  **That has not been traced**, so it is a hypothesis; what is measured is the
  time trial's own start pose. `oag_game::race::Race::start` follows the
  capture, placing a solo mode on slot 1, because it is the measurement that
  directly covers the case - and it says so in its own comment and in the load
  report rather than implying the grid path was read.
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

## The layout is derived in code, and the constants are literals

**Read 2026-09-16**: `Race_ComputeGridLayout` (`0x0882b3b0`), called by
`Race_PlaceGrid` (`0x08827c98`) once per race, fills eight `4x4` matrices
from the one authored node. The two measured constants above are these
literals, and a third the measurement could not see:

| Literal | Value | Role |
| --- | --- | ---: |
| `0x419e6666` | `19.8` | forward step per slot, along the located sample's **tangent**, re-located on the spline after each step (the walk `grid_poses` already does) |
| `10.0` | `10.0` | lateral offset, sign alternating per slot - the `+/-20` column stagger is `+10` against `-10` |
| `0x42480000` / `0x40a00000` | `50.0` / `5.0` | each slot is dropped by a raycast from 50 units above the sample and lifted 5 above the hit |

The walk starts at the `"start position"` registry entry (the authored node,
located on the spline with radius 100), fills **slot 8 first** and steps
*forward* seven times - so the node is the back of the grid, as measured. The
lateral base is not the centreline: each slot sits at the **midpoint of the
AI corridor** (`SplinePt+0x4c`/`+0x50`), and the sign of the first `10.0` is
chosen by which corridor edge the authored node is nearer, so the stagger
always begins on the node's own side. Per-slot orientation is built from the
sample (`FUN_0882663c`): heading follows the spline at each slot rather than
the anchor's fixed frame, which on a straight start is the same thing to the
four decimal places the capture saw.

Two mode branches, both worth having: in **Zone** (`g_game_mode == 6`) the
lateral offset and the step are both `0`, so the single craft sits on the
node itself; and when the track definition's `+0x164` flags read as a
**reversed** circuit (`0x20`, or `0x4` without `0x8`/`0x10`) the step is
`-19.8` and the lateral sign flips - the grid is laid out *behind* the node
along the tangent. That is the original's answer to the "reversed grids ran
off the curve" problem below: it never extrapolates in a straight line, and
on a reversed file it walks the other way. A track authoring all eight
`"start position %d"` nodes takes them verbatim instead; no Pulse track does.

`Race_PlaceGrid` then puts every craft on its matrix with a second downward
raycast (20 units, `+2` above the hit), `Ship_SetState(craft, 0)`, records
the slot in `craft+0x914` and seeds the position array (`manager+0x98`) in
grid order. `Race_StartRacing` (`0x08827e6c`) is the green light: every
human or AI craft (`craft+0x368` of `0` or `2`) gets its engine enabled
(`FUN_08848590(craft->+0x94, 1)`) and `Ship_SetState(1)`. And
`Race_FinishAllCrafts` (`0x08824e10`) is the flag: every craft is switched
to `autopilot_input` (the player) or `AI_input_%d` and put in state `2` -
**the original drives the player's craft itself after the finish line**,
which is the behaviour behind the end-race screens' cruising backdrop.

Confidence **88** for the layout, from a full decompile of a function with
one caller and constants that reproduce the two numbers measured live;
`GRID_ROW_PITCH`/`GRID_COLUMN_OFFSET` can now be `19.8`/`20.0` by reading
rather than by fit.

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
(`crates/game/tests/hd_trackwall_ground_truth.rs`'s `every_hd_grid_in_data*_lands_on_the_track`)
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

**A separate, pre-existing gap this incidentally found, and still open.**
Relocating exactly where an opponent first meets trouble on `05_Track` - a
circuit `docs/gameplay/ai.md` already measured as having 134 racing-line
samples with no collision surface under them - moved
`crates/game/tests/stall_rescue_ground_truth.rs`'s ground-truth craft onto a
spot where it got wedged bouncing rather than sitting still: a tick-by-tick
trace showed its position frozen for thousands of ticks while its speed
oscillated between about 0.2 and 10 units/s, never staying below
`STALL_SPEED` for long enough to trip the rescue's three-term "stalled" test,
which requires *sustained* low speed. That gap in the rescue predicate
predated this fix and was not touched here.

**Partly closed 2026-09-02, and the surviving half is this same bounce.**
`docs/gameplay/ai.md`'s "the last three, with a mechanism the original does
not have" fixed a real cause underneath some of the falling-through: a craft
crossing the track between one hover-probe test and the next.
`oag_physics::hover::sweep` catches that crossing mid-tick now, and a sweep
of all twelve circuits at all four difficulties found no cell reaching
`STALL_TICKS` by *sustained* dwell any more - the specific failure this page
first measured (a wedge that sits still and sustains low speed) is gone.
**The bounce itself is not**, re-traced the same day on `05_Track` at novice:
position frozen at `(-717.0, 19.3, -512.3)` from roughly tick 5,000 through
12,000 - over 7,000 ticks stationary - while per-200-tick speed still ranges
0.00-0.04 up to 11-13 units/s throughout, so the dwell counter keeps resetting
and the rescue never fires. One lap completed in a 12,000-tick run, zero
respawns. `07_Track` at novice shows the same shape, milder (3 of 4 laps).
Every other circuit/difficulty cell in the same sweep completes cleanly, so
this is specific to those two circuits at novice, not disc-wide. The gap in
the rescue predicate - it catches sustained low speed, not a craft bouncing
in place - is exactly as open as when it was found; what changed is only
which mechanism produces the bounce underneath it.

## Which team flies which slot: the `id` is a real field, on a struct of eight

2026-09-02. `Race_SpawnAiRacer`'s `id` (`docs/overview/roadmap.md`'s "a racer
list nothing has read") is not a mystery global - it is one field of a
struct-of-arrays at `&DAT_000577f8`, sized for exactly eight entrants, that
`Race_SpawnGrid` (`0x088247e0`) reads per racer before calling
`Race_SpawnAiRacer`/`0x0882821c`:

```c
for (racer_index = 0; racer_index < g_racer_count; racer_index++) {
    id = *(int *)(&DAT_000577f8 + 0x508 + racer_index * 4);   // FUN_08806bbc
    is_local_player = (id == FUN_0895ebf0());                  // local pad/entrant index, clamped 0..7
    name_ptr = (id == -1 || id == 8) ? &DEFAULT_NAME
                                      : &DAT_000577f8 + 0x488 + id * 0x10; // FUN_0894da24 -> FUN_08806b80
    slot = <this racer's compacted grid slot, from the permutation table above>;
    if (is_local_player) FUN_0882821c(mgr, racer_index, id, slot, name_ptr);
    else                 Race_SpawnAiRacer(mgr, racer_index, id, slot, name_ptr);
}
```

**`racer_index`, `id` and `slot` are three separate quantities** - confirmed
directly from `Race_SpawnGrid`'s own disassembly, which carries them in three
distinct registers (`s3`, `s5`, `s4`) into the call. `racer_index` is the
enumeration order Wipeout uses to walk the active field (0 to
`g_racer_count - 1`); `slot` is the grid position after the permutation-table
and compaction pass documented above; `id` is read out of the struct rather
than reused from `racer_index`, which would be redundant if the two always
agreed - the getter's only purpose is to answer a question whose answer can
differ from the index that asked it.

**The struct's layout is read off four accessor functions, and the
arithmetic closes exactly**, which is why this reading is confidence **90**
even though none of these functions carry a name yet (see below): four
parallel eight-entry arrays, back to back with no gap except one 4-byte pad
before the first:

| Offset | Stride | What | Accessor(s) |
| --- | --- | --- | --- |
| `+0x44c` | 4 bytes | resolved team-catalogue pointer, one per entrant `id` | read directly as `DAT_00057c44[id]` in `Race_SpawnAiRacer` |
| `+0x470` | 3 bytes | a short field (unread) | `FUN_08806b50` (write) |
| `+0x488` | 16 bytes | the entrant's display name | `FUN_08806b90` (write), `FUN_08806b80` (address-of, read) |
| `+0x508` | 4 bytes | `id`'s own field, this section's subject | `FUN_08806bbc` (read) |

(`0x44c + 8*4 = 0x46c`, four bytes short of `0x470`; `0x470 + 8*3 = 0x488`
exactly; `0x488 + 8*0x10 = 0x508` exactly - two of the three joins have zero
slack, which is what makes "eight parallel arrays" a read rather than a
guess.) `DAT_00057c44` is `&DAT_000577f8 + 0x44c` computed as its own
`lui`/`addiu` pair rather than through the runtime base register `Race_SpawnGrid`
uses - two Ghidra symbols for one struct, confirmed by the offset arithmetic
above rather than by a cross-reference.

**A five-function cluster at `0x0894d8d0`-`0x0894dc18` populates the
`+0x44c`/`+0x470`/`+0x488` fields for one `id`**, and reads as a front-end
"team changed for this seat" handler: `FUN_0894db7c` resolves a UI handle to
a catalogue pointer and stores it at `DAT_00057c44[id]`, then re-zeroes an
unrelated 20-entry per-entrant colour-scheme array (the same reset
`Race_SpawnGrid` itself runs over all eight racers at the top of the
function); `FUN_0894dc18` is a dispatcher that either does a full refresh
(`param_3 == -1`) or one field of it, keyed by a "what changed" sentinel
compared against four globals; `FUN_0894d8d0`/`FUN_0894d914` are the
`+0x470`/`+0x488` refreshers proper; `FUN_0894da24(id)` returns the address
of `id`'s name field (or a fallback string for the `-1`/`8` sentinels) and is
what `Race_SpawnGrid` itself calls for `name_ptr` above.

**None of the five are reached by a direct `jal` anywhere in the binary** -
checked by `search_instructions` on their zero-padded relative operands
(`149b7c`, `149c18`) against all 524,951 instructions in the program, zero
matches either way, the same check that surfaces every other caller on this
page. That is not "no caller exists" (see the pre-relocation trap in
`HANDOVER.md`'s "Traps that are live") but specifically "reached through a
computed/table call", consistent with a menu widget's per-seat "selection
changed" callback rather than a call site this project's static tools can
resolve. This is why `id`'s own write site - whatever assigns
`racer[racer_index].id` before `Race_SpawnGrid` reads it back - is still not
found: it is one hop further upstream of a table this session did not chase.

**None of these six functions are renamed**, deliberately: the *mechanics*
(what address each one computes, what it copies where) are confidence 90-95
and settled by the arithmetic above, but a name has to encode *meaning*, and
the one open question left - is `id` a stable "team-select seat" number
independent of who is racing, a controller-pad index, or something else
`FUN_0895ebf0`'s pad-index shape only resembles - is exactly what is not yet
pinned. Renaming now would bake a guess into the database. `FUN_08806bbc`
and `FUN_08806b80` did not exist as functions before this session - Ghidra's
auto-analysis missed both (each is four instructions, no prologue), and they
were created live via `create_function` so they could be decompiled at all.
**A fresh Ghidra import will not have them**: `just apply-names` only
recreates rows `names.tsv` lists, and these carry no row since they are not
named. Re-run `create_function` at `0x08806bbc` and `0x08806b80` (or rely on
`run_analysis` picking them up) before expecting either address to decompile
after a reimport.

## Custom Race does not go through `Race_SpawnGrid` at all, and hardcodes `id` to 0

2026-09-02, live-confirmed. Everything in the previous section is real, but it
describes a code path this session's own live capture never reaches from
**RACEBOX -> CUSTOM RACE -> SINGLE RACE** - the menu route
`scripts/psp-drive.py menu --single-race` drives and the route this project's
own `oag-game --race --mode single_race` corresponds to. Breakpoints on
`Race_SpawnGrid` (`0x088247e0`) and `Race_SpawnAiRacer` (`0x088253f8`) armed
before a fresh load, over three separate attempts, **never fired** - each
attempt confirmed still armed and enabled afterward
(`cpu.breakpoint.list`), so this is not a broken breakpoint, it is a path not
taken. A breakpoint on `Ai_Construct` (`0x088536bc`) fired immediately on the
same load, so AI opponents are constructed by *something* - just not by
`Race_SpawnAiRacer`.

**The real orchestrator is `FUN_0882e57c`, reached from `Ai_Construct`'s own
call stack**: `Ai_Construct` <- `FUN_08834c5c` (a two-line "construct and
register" wrapper, unrelated to teams) <- `FUN_088285e0` <- `FUN_0882e57c`.
Confirmed two ways that agree: the live `ra` register at each breakpoint hit,
and `search_instructions` on the callers' own relative `jal` operands
(`245e0` resolves to five call sites, one of them `FUN_0882e57c` twice).

`FUN_0882e57c` branches on `_DAT_0005780c` (0 was what a fresh Single Race
took, live-confirmed; the other branch reads a per-racer "old slot -> new
slot" field at `+0xd0` and reads as a grid *reorder*, not traced further).
The `0` branch, disassembled at `0x0882e744`/`0x0882e898` (`li a2,0`, not a
register - a real immediate, checked at the instruction level specifically
because this is the whole claim):

```c
for (racer_index = 0; racer_index < g_racer_count - 1; racer_index++) {
    name_ptr = _DAT_0005ab88[racer_index].name_ptr;          // the same +0x3c cache Race_SpawnGrid also writes
    FUN_088285e0(mgr, racer_index, /* id */ 0, /* slot */ racer_index, name_ptr);
}
FUN_0882821c(mgr, racer_index, /* id */ 0, /* slot */ racer_index, player_name_ptr);  // last racer, presumed local player
```

`FUN_088285e0` is `Race_SpawnAiRacer`'s structural twin - same `+0x19bc` slot
write, same shape of object-allocation boilerplate, same downstream call into
`Craft_Construct_q` (`0x08840c74` - already named and documented on
[`shield.md`](shield.md), confirming `func_0x0003cc74`'s two call sites,
type `1` from `Race_SpawnAiRacer` and type `2` from here, are the same
generic per-craft constructor) - but it **never reads `DAT_00057c44`** (the
team-catalogue pointer array) or anything else keyed by `id`. Since `id` is a
hardcoded `0` for every racer in this path, **Custom Race's team assignment
does not happen through `id` or through `Race_SpawnAiRacer`'s
catalogue-pointer mechanism at all** - and, read in full separately this
session, `Craft_Construct_q` itself is not where it happens either: it
initialises physics/hover/shield/particle state off `param_2` (a small `0`-`3`
role tag, not `id` or a team index) and touches no `.vex`/team-path string or
`DAT_00057c44` anywhere in its body. A whole-binary `search_strings` for
`Data.Ships`/`Ship.vex` found nothing, so the path is very likely built at
runtime from a team-name substring plus a fixed suffix rather than present as
one literal. Team assignment for Custom Race happens somewhere neither spawn
chain nor its shared constructor touches - unfound this session, and the
renderer/asset-loading side (rather than either entity constructor) is the
more promising place to look next.

**This falsifies part of the previous section's framing** for the mode that
actually matters here: `id`'s catalogue-pointer mechanism may be real for
whatever route does reach `Race_SpawnGrid` (Main Menu's other top item,
**RACE CAMPAIGN**, never driven this session - a hypothesis, not checked),
but it is not what decides which team a Custom Race opponent flies. None of
`FUN_0882e57c`, `FUN_088285e0` or `FUN_08834c5c` are renamed: the mechanics
above are read at confidence 90 (decompiled and cross-checked live), but
which menu route reaches which function is confirmed for exactly one route
and hypothesised for the other, and a name should not get ahead of that.

## Names landed

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x088247e0` | `Race_SpawnGrid` | 82 |
| `0x088253f8` | `Race_SpawnAiRacer` | 78 |
| `0x088536bc` | `Ai_Construct` | 80 |
| `0x08926ae8` | `StartPosition_Bind` | 82 |
| `0x08ab0a90` | `g_grid_orders` | 82 |

`0x0882821c`, the local player's twin of `Race_SpawnAiRacer`, is **deliberately
not named**: it was inferred from the call site's position in the `if`/`else`
and not read at the time. Since read one level further (previous section) -
it is also `FUN_0882e57c`'s call for the last racer, same argument shape,
`id` hardcoded to `0` there too - but still not named, for the same reason
its neighbours in that section are not.

## What is still open

- **What picks the permutation row.** `FUN_0894d440()` is unread, so whether the
  order is per-race, per-championship-round or seeded is not known. The identity
  row exists, so a single observation cannot tell "the table was consulted" from
  "the table returned the identity" - and the one live run had the local player
  as the last racer, which the identity row also puts in slot 8, so **that run
  cannot separate the permutation from `Ai_Construct`'s forcing either**.
- ~~**Whether the constants are authored or derived**, above.~~ Derived,
  from literals in `Race_ComputeGridLayout` - see "The layout is derived in
  code". The port's measured `19.79`/`20.0` should become `19.8`/`20.0`.
- ~~**Nothing drives the opponents**, and they all wear the player's hull.~~
  Both closed: the AI landed 2026-08-11 and per-team hulls on 2026-08-15. A
  slot now flies its own team's `Data\Ships\<Team>\Ship.vex` - eight
  different models, 845 to 1,497 triangles - with its own nozzle and plume. See
  `crates/game/src/livery.rs`. **Which team flies which slot is narrowed for
  one mode and re-opened for the one that matters.** `id` is a genuine,
  separate per-entrant field on `Race_SpawnGrid`'s struct (confidence 90 on
  the layout and accessors, see "Which team flies which slot" above) - but
  live capture on **RACEBOX -> CUSTOM RACE -> SINGLE RACE**, the mode this
  project actually implements, shows that path never reaches
  `Race_SpawnGrid`/`Race_SpawnAiRacer` at all, and its own spawn function
  (`FUN_0882e57c`/`FUN_088285e0`, see "Custom Race does not go through
  Race_SpawnGrid" above) hardcodes `id` to `0` for every racer. So `id`'s
  catalogue-pointer mechanism, whatever it turns out to mean, **does not
  answer this question for Custom Race** - team assignment there happens
  through a mechanism this session did not find. `Craft_Construct_q`
  (`0x08840c74`, shared by both spawn paths) is read in full and **ruled
  out**: it never references a team-path string or `DAT_00057c44`. The
  renderer/asset-loading side, rather than either entity constructor, is the
  next place to look. The ordering `livery::teams_for_slots` uses today is
  still this project's own, not the original's, and is labelled as such in
  the load report.
- **`modesto_heights`'s reversed grid, one slot short.** See "Reversed grids"
  above - inside the track's own bounds, not chased further.
- **The stall rescue does not catch a craft bouncing in place.** Found while
  fixing the reversed grids, above, and not this page's subsystem - tracked
  against `crates/game/tests/stall_rescue_ground_truth.rs`, not here.
  **Re-confirmed open 2026-09-02**: `oag_physics::hover::sweep` closed the
  *sustained*-stall shape of this failure disc-wide but not the bounce
  itself, which still reproduces on `05_Track` and `07_Track` at novice. See
  "Reversed grids" above.

## History

- 2026-09-02: `Race_SpawnAiRacer`'s `id` traced to a genuine per-entrant field
  on the same eight-entry struct `racer_index`/`slot` also index (confidence
  90); the front-end cluster that populates it is reached only through a
  computed call this session's tools could not resolve, so the write site for
  `id` itself is still open. See "Which team flies which slot" above. **Same
  day, live capture then showed `Race_SpawnGrid` is not even reached by
  Custom Race / Single Race** - the mode this project implements goes through
  a different, previously undocumented spawn function that hardcodes `id` to
  `0`, so the open question for that mode is now "what does decide the team"
  rather than "what does `id` mean". See "Custom Race does not go through
  `Race_SpawnGrid`" above.
- 2026-09-02: `oag_physics::hover::sweep` (`docs/gameplay/ai.md`) closed the
  *sustained*-stall shape of the failure below disc-wide; the bounce-in-place
  gap itself re-confirmed open on `05_Track` and `07_Track` at novice.
- 2026-08-19: reversed grids fixed - `grid_poses` walks the track's own spline
  instead of extrapolating a straight line from the anchor, closing 8 of 9
  broken reversed grids a sweep of Wipeout HD's circuits found. See "Reversed
  grids" above.
- 2026-08-10: geometry measured against the running original; the authored node
  identified as slot 8. Ordering recovered the same day.
- 2026-08-10: page created. Ordering recovered; geometry open.
