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
  time trial's own start pose. `oag_raceplay::Race::start` follows the
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
track's two edges** (**corrected 2026-10-02**: this read `SplinePt+0x4c`/`+0x50`,
the AI corridor; the instructions read `+0x44`/`+0x48`, the half-widths, see
[the grid walk](#the-grid-walk-read-to-the-end-2026-10-02)), and the sign of the first `10.0` is
chosen by which corridor edge the authored node is nearer, so the stagger
always begins on the node's own side. Per-slot orientation is built from the
sample (`FUN_0882663c`): heading follows the spline at each slot rather than
the anchor's fixed frame, which on a straight start is the same thing to the
four decimal places the capture saw. **Read to the end 2026-10-02**: it is the
unit sum of the left-edge and right-edge chords over 20 units, not the tangent,
see [the grid walk](#the-grid-walk-read-to-the-end-2026-10-02).

Two mode branches, both worth having: in **Zone** (`g_game_mode == 6`) the
lateral offset and the step are both `0`, so the single craft sits on the
node itself; and when the track definition's `+0x164` flags read as a
**reversed** circuit (`0x20`, or `0x4` without `0x8`/`0x10`) the step is
`-19.8` and the lateral sign flips - the grid is laid out *behind* the node
along the tangent. That is the original's answer to the "reversed grids ran
off the curve" problem below: it never extrapolates in a straight line. **The
reversed branch is read from the decompile and contradicted by a capture, see
[the 2026-09-29 measurement](#the-stagger-is-about-the-corridor-midpoint-not-the-node-2026-09-29):
on Metropia reversed the layout that comes out is the forward one.** A track authoring all eight
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

## The stagger is about the corridor midpoint, not the node (2026-09-29)

**Found by** the falloff survey (`docs/gameplay/leaving-the-track.md`): on
`01_Track` and `17_Track` (`01_Track` reversed) grid slots 1, 3, 5 and 7 were
placed **30 units off the centreline**, past the corridor edge, and fell from
tick 0. Every other circuit's eight slots were inside the corridor. The player's
own slot 8 was fine, but a *time trial* starts on slot 1 (`solo_slot_one`), so
the player's start on those two circuits was on the falling column too.

**The cause.** `grid_poses` carried the node's own lateral offset down the grid
and added `GRID_COLUMN_OFFSET` to the *left* of it. That is only right when the
node is on the right of the corridor midpoint, which it is on 22 of 24 circuits
(`+9.2` to `+11.6` from the midpoint). On `01_Track` and
`17_Track` it is on the left (`-10.8`, `-10.7`), so "20 further left" is 30 off.
`Race_ComputeGridLayout` never does that (decompile re-read 2026-09-29): it
locates the node, takes the AI corridor's midpoint at that sample
(`SplinePt+0x4c`/`+0x50`), picks the sign of the first `10.0` from **which
corridor edge the node is nearer**, and lays *every* slot, the eighth included,
at `midpoint + sign * 10.0` on its own re-located sample, negating `sign` after
each slot. So the even slots (8, 6, 4, 2) are on the node's side and the odd ones
on the other, wherever the node happens to be. Confidence **88**, on the same
footing as the layout above.

**Measured against the original, two circuits.** The rule predicts the residual
the layout table above already carried, and removes it:

| circuit | source | worst slot before | worst slot after |
| --- | --- | ---: | ---: |
| `16_Track` (node right of midpoint) | the eight craft of `ORIGINAL_GRID` | 1.81 | **0.62** (slot 8: 1.68 to 0.48) |
| `18_Track`, Metropia reversed (node right of midpoint) | eight craft read live 2026-09-29, `METROPIA_REVERSED_GRID` | - | **1.13** |

The `16_Track` slot 1 to 7 residuals were 1.4 to 1.8 in the lateral direction, and
this rule's difference from the old one is `midpoint(slot) + 10` against
`node lane - 20`, which on that circuit is 1.6 to 1.85: the prediction was made
before the run. Slot 8 was 1.68 because the raw node, not the midpoint plus ten,
was used for the player; it is no longer. **The node-on-the-left branch is
decompile-only.** Neither `01_Track` nor `17_Track` is reachable from a fresh
profile's front end (`race-setup.md`: only `16`, `03` and `18` are unlocked), so
no original capture of a left-side grid exists yet - though since 2026-09-29
every circuit is, through the dev-unlock byte in
[ppsspp-debugger.md](../../../reverse-engineering/ppsspp-debugger.md#every-circuit-not-three-the-dev-unlock-byte-2026-09-29),
and a Single Race on Basilico Black (`01_Track`) was logged with it; it is the same code path with the sign flipped,
and it is what the 24-circuit test checks.

**All 24 circuit-directions, all eight slots** (`grid_stagger_ground_truth.rs`,
`every_slot_starts_on_the_track`): 192 of 192 slots over collision and inside the
AI corridor, each within 1.5 of `midpoint +/- 10` on the correct side. The
narrowest margin to a corridor edge is 0.68.

**A reversed circuit's grid comes out laid like a forward one, measured.** The
decompile reads as though a reversed circuit (the definition's `+0x164 & 0x20`)
put the node at the *front* slot: the output matrix index is `9 - i` and the step
is `-19.8`. On Metropia reversed (`18_Track`, `02_Track` reversed) the original's
slot 8 is at `(392.11, -13.38, 192.44)` and slot 1 at `(529.33, -12.92, 169.73)`,
and ours - node as slot 8, field ahead of it in the direction of travel - lands
within 1.13 on all eight. **What that proves is the layout and the slot
numbering, not why the branch does not show.** Either the flag is not what
`bVar1` reads on a Pulse `Reversed="True"` circuit, or the branch is skipped for a
race, or it is taken and `track_reversed.vex`'s own spline runs the other way so
the two negations cancel; which was not chased, and the decompile's reversed
branch is left unported. Confidence **85** on "reversed circuits use the forward
layout", from eight positions on one circuit. The track.md observation that a
reversed file's node sits 2.2 units from a forward time-trial start is a
coincidence of where the exporter authored two files, not a slot number.

**Only the Pulse PSP grid was read.** `grid_poses` also lays out the PS2 and Wipeout
HD grids, whose own layout functions were not read; `hd_trackwall_ground_truth`
(every HD grid on the track, both directions) still passes, and nothing checks the
midpoint rule against those titles' originals.

**How the eight were read.** `scripts/psp-drive.py menu --single-race
--track-down 2` (new: Track Select is a wrapping list, so two presses from
Talon's Junction reach Metropia), then `RESTART RACE` and a poll of the racer
table at `0x08b34420`: entry `i` is **assumed** to be slot `i + 1` (the last is the player, whose
HUD read `POS 8/8` and whose position `find_craft` returned as entry 7; the
`16_Track` capture used the AI object's `+0x50` instead, and the two orders agree
there), and its world position is at
entry `+0x10` (the four floats before a 3x3 of the craft's axes). The table
entry, not the craft struct, holds the copy that is stable through the countdown.

**Changed 2026-10-01, on Pulse PSP:** each slot now takes the track's own frame at
its sample (`oag_gameplay::orientation_on_sample`), where it used to share the
node's heading. "The same to four places" was a dot product, which cannot see a
third of a degree: the eight craft of a `16_Track` Single Race read at placement
(`scripts/psp-grid-pose.py`) have forward `z` `-0.0025`, `-0.0032`, `-0.0039`,
`-0.0044`, `-0.0049`, `-0.0053`, `-0.0057`, `-0.0061` from slot 1 to slot 8, where
the node's is `0.0000` - 0.35 degrees out at slot 8, which is the player's slot
in a Single Race. The sample's own tangent is within 0.05 degrees of every one
(`every_grid_slot_points_along_the_tracks_own_frame_as_the_original_does`, worst
0.045). Confidence **88** for "built from the located sample" (the heading
tracks the sample to 0.05 degrees on eight slots; `FUN_0882663c` itself, a long
VFPU function, was not read to the end - **it is now, and the heading is the edge
chords'**, see [the grid walk](#the-grid-walk-read-to-the-end-2026-10-02)). The other titles keep the node's
heading: nothing was measured for them. See
[grid-state.md](../../../physics/grid-state.md), which also records an exact walk
that was tried and reverted, and the second circuit (`01_Track`).

## Ported, and how close it lands

`oag_gameplay::spawn::grid_pose` is the layout and `oag_raceplay::grid_poses`
drops each slot onto its own footprint. Eight craft take the grid on any track
that authors a `Start Position`; **nothing drives the seven opponents**, so they
hold station until AI lands.

`crates/game/tests/race_ground_truth.rs::our_grid_is_the_originals_grid` compares
every slot against the eight positions above, on the same track:

| | worst | where it goes |
| --- | ---: | --- |
| Slot 8, our anchor | `1.68` (0.48 since 2026-09-29) | the authored node dropped onto the collision mesh; now re-laid on the corridor midpoint, see below |
| Worst slot, whole grid | `2.40` (0.62 since 2026-09-29) | the anchor, plus about `0.7` |

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
and changes only how they are applied: `oag_raceplay::spawn::grid_poses` walks
the track's own resampled spline from the anchor, `GRID_ROW_PITCH` units per
step, instead of projecting along the anchor's fixed forward axis, and stagger
uses each walked sample's own `lateral` axis instead of the anchor's fixed one -
the same axis [`Pose::from_sample`](../../../../crates/gameplay/src/spawn.rs)
already uses for the same reason. Heading is untouched: every slot still shares
the anchor's own orientation, which is the part the live capture actually
confirmed to four decimal places, and this does not reopen that. Two guards
keep it from being wrong in a new way: the walk never crosses into a different
*path* in [`Spline`](../../../../crates/raceplay/src/spline.rs)'s own table -
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
    name_ptr = (id == -1 || id == 8) ? &DEFAULT_NAME: &DAT_000577f8 + 0x488 + id * 0x10; // FUN_0894da24 -> FUN_08806b80
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

> **Corrected 2026-10-04.** `FUN_0882e57c` below is `Tournament_Construct`
> (mode 4), not Single Race. Single Race is `ArcadeRace_Construct`
> (`0x0882c108`), whose loop has the same `id = 0` shape. The per-racer
> string passed in is the **team name**, and `Craft_Construct` resolves it.
> See "Which team flies which slot, recovered" below.

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

## Which team flies which slot, recovered: the AI roster draw (2026-10-04)

**Answered for Single Race, live-confirmed.** Every eligible team but the
player's is shuffled, the first seven race, and the player is the last racer.
The draw happens on every launch, off a wall-clock seed, so the original
gives a different grid each time. Confidence **85** for the outcome, which
was seen live, and **80** for the parts read statically only (the swap
below, and seven of more than eight once DLC packs are mounted).

### Correction first: Single Race is not `Tournament_Construct`

The previous section's spawn function, `0x0882e57c`, is `Tournament_Construct`
(mode 4, already named on [state-machine.md](state-machine.md)). **Single
Race is `g_game_mode` 3, `ArcadeRace_Construct` (`0x0882c108`).** Three
pieces of evidence agree, plus one weak one:

- `g_game_mode` read **3** live on every launch below.
- Its first code word still matches `BOOT.BIN` (`0x27bdffc0`), while
  `0x0882c108`, `0x08820d78`, `0x0882c03c` and `0x08821bd4` all carry
  PPSSPP's JIT block marker (`0x68......`), which means they ran.
- The live race-session object's vtable is `0x08ac9820`, the one
  `ArcadeSession_Construct` (`0x0882c03c`) installs.
- Weak: a breakpoint on `0x0882e57c` never fired. Breakpoints in this code
  did not fire even where it ran (see the end of "Live"), so this carries no
  weight by itself.

The `id = 0` finding survives: `ArcadeRace_Construct` has the same loop, an
immediate `0` for `id`, `racer_index` as the slot, and the per-racer string
as the last argument. **That string is the team name, not a display name.**
`Craft_Construct` (`0x08840c74`) resolves it on its first use:
`craft+0x370 = Team_FindByName(param_5)`, falling back to `"Feisar"` when the
lookup fails. `Ship_LoadModel` (`0x08843258`) then builds every hull path
from `craft+0x370`'s `+0x94` (the team's `location`) through `"%s\\%s.vex"`,
`"%s\\Ship.vex"` or `"%s\\Zone.vex"`. The handover's earlier "Craft_Construct
is ruled out" was wrong: it is where the team name becomes a team.

### Where the per-racer team names come from

```c
// InGame_Update (0x08813328), substate 1, g_game_mode 3/9/12:
ArcadeSession_Construct(obj);            // 0x0882c03c
  -> RaceSession_Construct(obj);         // 0x08820d78: g_race_session = obj;
                                         //   memset(obj+0x3c, 0, 0x20);
       -> RaceSession_DrawAiRoster(obj); // 0x08821bd4
// later, ArcadeRace_Construct (0x0882c108):
for (i = 0; i < racers - 1; i++)
    Race_CreateAiRacer(mgr, i, 0, i, g_race_session->team[i]);   // +0x3c + 4*i
Race_CreatePlayer(mgr, racers - 1, 0, racers - 1, g_race_session->team[7]); // +0x58
```

`RaceSession_DrawAiRoster` (`0x08821bd4`), read at the instruction level:

```c
if (g_game_mode < 0xe) {                       // local modes only
    for (i = 0; i < 8; i++) s->team[i] = 0;
    srand(sceKernelGetSystemTimeWide());       // 0x08a7719c -> srand 0x089731b4
    n_all = Team_ListRaceTeams(&all);          // 0x088893e8: Type="Race" only
    n = 0;
    do {                                       // repeats until 7 are taken
        for (k = 0; k < n_all; k++)
            if (RaceSession_IsTeamEligible(s, all[k])) {
                pool[n++] = all[k]; all[k] = 0;
            }
    } while (n < 7);                           // fewer than 7 eligible: never exits
    for (i = 0; i < n; i++) {                  // 0x08821cd0: divu v0,s4; mfhi
        j = rand() % n;
        swap(pool[i], pool[j]);
    }
    for (i = 0; i < 7; i++) s->team[i] = pool[i]->name;   // +0x74
    if (g_player_team_definition && *g_player_team_definition->name)
        s->team[7] = g_player_team_definition->name;      // 0x08821d68: sw s1,0x58(s0)
}
```

`RaceSession_IsTeamEligible` (`0x08821dd4`) passes a team when **all** of:

1. it is not null (a taken entry is zeroed, so a second pass skips it);
2. its name differs from the player's, by a plain byte compare
   (`FUN_08973424`, a `strcmp`);
3. `Definition_IsUnlocked(team, 0)`;
4. its name does **not contain** `"Zone"` (`0x08a7a1dc`, through the
   case-sensitive `strstr` at `0x089737ac`);
5. outside Demo (`g_game_mode == 2`), always; in Demo, only a team whose
   `Values Multiplayer` attribute read `true` or `1` (`+0xa9`).

`Team_ListRaceTeams` (`0x088893e8`) lists every team definition and drops
any whose `+0xa0` is not `0`. `TeamDefinition_ParseElement` (`0x088c2a38`)
sets `+0xa0` from the `Values` element's `Type` attribute, indexed against
the two-entry table at `0x08ab1ad0`: `"Race"` is `0`, `"Zone"` is `1`, and
anything else is `-1`. The attribute defaults to `"Race"`. The same parser
reads `Location` (`+0x94`), `Multiplayer` (`+0xa9`), `SoundRegister`,
`HelpText` and `AIOnly`, plus a `Stats` element's `Speed`/`Thrust`/
`Handling`/`Shield`. `Team_FindByName` (`0x08889488`) is a `strcasecmp`
walk over the same list on `+0x74`.

**The shuffle is not Fisher-Yates.** Each index trades places with
`rand() % n` over the whole range, which is biased. The port keeps that
swap exactly (`crates/livery/src/draw.rs`). `rand` is the libc LCG on
[prng.md](prng.md).

### Live: seven launches, three player teams

PPSSPP v1.20.4 under Xvfb, `pulse-psp-usa.chd`, RACEBOX -> CUSTOM RACE ->
SINGLE RACE, Talon's Junction, the walk `scripts/psp-drive.py menu
--single-race` drives. Read after each load by `scripts/psp-team-roster.py`: `g_race_session` (`0x08b34320`)
`+0x3c..+0x58`, `g_player_team_definition` (`0x08b3104c`) `+0x74`, and each
craft's own `+0x370 -> +0x74` through `g_race_manager` (`0x08b317b4`) `+0x78`.

| Launch | Player | Racer indices 0 to 6 (AI) | 7 |
| --- | --- | --- | --- |
| 1 | Assegai | Goteki, EGX, Qirex, Triakis, Feisar, AG_Systems, Piranha | Assegai |
| 2 | Assegai | Feisar, AG_Systems, Piranha, EGX, Triakis, Qirex, Goteki | Assegai |
| 3 | Assegai | Feisar, AG_Systems, Goteki, Triakis, EGX, Piranha, Qirex | Assegai |
| 4 | Assegai | Goteki, Triakis, AG_Systems, Piranha, EGX, Qirex, Feisar | Assegai |
| 5 | Assegai | Triakis, AG_Systems, Qirex, Piranha, Goteki, Feisar, EGX | Assegai |
| 6 | AG_Systems | Assegai, Feisar, Qirex, Piranha, Goteki, Triakis, EGX | AG_Systems |
| 7 | Feisar | EGX, Triakis, Assegai, Goteki, Piranha, AG_Systems, Qirex | Feisar |

Launches 4 and 5 pressed right on Ship Select, which changes the skin, not
the team. Teams are a vertical list, which `psp-drive.py menu --ship-down N`
now drives. On all seven launches every craft's own `+0x370` team matched
the session array index for index, with the player's craft (`+0x368` role `0`) last and
the seven AI (`+0x368` role `2`) first. **What the table shows:** the
player's team is never an AI team; the other seven always all race; the
order is different on every launch; the player is always the last racer.
Whether `RESTART RACE` redraws is **not determined**: the only check was a
breakpoint, and breakpoints in this code did not fire (below). Sampling the
array before and after a restart would settle it.

**Not explained, and recorded rather than smoothed over:** an execution
breakpoint on `0x08821bd4`'s entry, and two inside it (`0x08821cc0`,
`0x08821d38`), never fired, while one on `Ai_Construct` (`0x088536bc`) in
the same session fired eight times per load. The function did run (its JIT
markers, and a fresh draw on every launch). That gap does not block the
names, because the live outcome matches the static read on every
distinctive feature. It would matter for a capture that needs to stop
inside the draw. Arming earlier (before Main Menu) or PPSSPP's IR
interpreter are the two things not tried.

### What the port does with it

`oag_livery::teams_for_slots` reproduces the filter's name tests
(player excluded, `Zone` excluded; `Type="Race"` is already
`catalogue::teams`'s filter), the swap and the first-seven cut, with slot 0
the player's. **Chosen, not measured:** the seed is the race seed through a
salted generator of its own (`ROSTER_SALT`), because the simulation may not
read a clock. One seed gives one grid, so `--race` without `--seed` always
flies the same order where the original would not. The unlock test is left
to the front end's list. A list shorter than seven cycles instead of
hanging. Every other title inherits this law, since none has a measured
rule of its own.

**Tournament appears to keep its first leg's roster.** `Tournament_Construct`
calls the draw only while `DAT_08b30fa4` is `0`, and on that branch it fills
a per-racer team cache at `+0x110` itself. On the other branch it reorders
the racers by a per-racer field at `+0xd0` and reuses `+0x110`.
`FUN_0882e2fc` runs there only when `DAT_08b30fab == 1`: it rebuilds `+0x110`
and `+0xd0` from a stored record (`DAT_08b31774`, keyed by `DAT_08b30fb4`),
with a `rand` fallback, then draws anyway. It reads like a resume path and is
unread. The port's legs share one seed, which gives the same roster. The
reorder is not ported.

**The DLC case is static only.** With four packs mounted, eleven teams are
eligible for a Pulse player, and the draw picks seven of them at random.
The earlier "the extra teams simply do not race" was this project's own
truncation, not the original's.

## Names landed

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x088247e0` | `Race_SpawnGrid` | 82 |
| `0x088253f8` | `Race_SpawnAiRacer` | 78 |
| `0x088536bc` | `Ai_Construct` | 80 |
| `0x08926ae8` | `StartPosition_Bind` | 82 |
| `0x08ab0a90` | `g_grid_orders` | 82 |
| `0x08821bd4` | `RaceSession_DrawAiRoster` | 85 |
| `0x08821dd4` | `RaceSession_IsTeamEligible` | 80 |
| `0x08820d78` | `RaceSession_Construct` | 75 |
| `0x0882c03c` | `ArcadeSession_Construct` | 72 |
| `0x088285e0` | `Race_CreateAiRacer` | 75 |
| `0x088893e8` | `Team_ListRaceTeams` | 80 |
| `0x08889488` | `Team_FindByName` | 85 |
| `0x088c2a38` | `TeamDefinition_ParseElement` | 80 |
| `0x08b34320` | `g_race_session` | 80 |
| `0x08b3104c` | `g_player_team_definition` | 85 |

The 2026-10-04 rows are evidenced in "Which team flies which slot,
recovered" above. `RaceSession_DrawAiRoster` and `Team_FindByName` sit at 85
because the live grid matched their static reads on seven launches.
`RaceSession_Construct` and `ArcadeSession_Construct` sit lower: their call
chain is static, and the only live evidence is the session vtable and the
JIT markers. `Race_CreateAiRacer` is `Race_SpawnAiRacer`'s twin for the
Arcade and Tournament constructors; the seven AI crafts it built live carried
role `2` and the team passed in. `0x0882821c` is `Race_CreatePlayer`, named
on [race-progress.md](race-progress.md).

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
  `crates/livery/src/lib.rs`. ~~**Which team flies which slot is narrowed for
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
  the load report.~~ **Closed 2026-10-04**: the route above was Tournament's,
  and `Craft_Construct` is where the team name resolves. Single Race draws
  its roster in `RaceSession_DrawAiRoster`, and the port follows it. See
  "Which team flies which slot, recovered".
- **The roster seed.** The original reads the wall clock and the port reads
  the race seed, which no real launch varies yet, so every `--race` without
  `--seed` flies the same order. Varying it per launch is a session decision
  (it also moves every other seeded draw), not an RE question.
- **The entry breakpoint on `RaceSession_DrawAiRoster` never fired** although
  the function ran. See the end of "Live: seven launches".
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

- 2026-10-04: which team flies which slot recovered for Single Race
  (`RaceSession_DrawAiRoster`, 85, live on seven launches) and ported. The
  2026-09-02 spawn route was corrected: it was Tournament's.
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

## The craft's own state: `Craft_SetState`, grid state 0 and racing state 1 (2026-10-01)

`Race_PlaceGrid` ends each craft with `Ship_SetState(craft, 0)` and `Race_StartRacing`
with `Ship_SetState(craft, 1)`; when the craft has an entity and is not a remote one
(`entity+0x368 != 1`) `Ship_SetState` first calls `Craft_SetState` (`0x08848590`) on the
craft, and **that** is where the craft's own state word, `craft+0x2a4`, is written:

```text
08848590  Craft_SetState(craft a0, state a1)
08848594  lw a2,0x2a4(a0)      ; the old state
088485a8  beq a1,a2,0x088485ec ; unchanged: nothing
088485bc  beq s0,zero,0x088485f4 -> state 0: Craft_EnterGridState(craft); craft+0x2a4 = 0
088485c4  beq s0,at(1),0x08848604 -> state 1: Craft_ReleaseFromGrid(craft); craft+0x2a4 = 1
          states 3 (Body_ClearAccumulators + Body_ClearVelocity, craft+0x284 = 0), 2, 4 and 5 store only
```

| Address | Name | What it does |
| --- | --- | --- |
| `0x08848590` | `Craft_SetState` | stores `craft+0x2a4`, and runs the state's own entry on 0, 1 and 3 |
| `0x088486d4` | `Craft_EnterGridState` | `craft+0x1c0 \|= 2` |
| `0x088486e4` | `Craft_ReleaseFromGrid` | `craft+0x1c0 &= ~2`, then `craft+0x2bc` (brake), `+0x2c0` (steer), `+0x2c4` and `+0x2c8` (airbrakes) `= 0.0` |

**What reads the state, all read from the decompile on 2026-10-01** (full account and
the measurements in [grid-state.md](../../../physics/grid-state.md)):

- `Ship_HoverTwoPoint` (`0x0884a658`), the epilogue: `0x0884ad2c  lw a2,0x2a4(s0)` and
  `0x0884ad30  beq a2,zero,0x0884ad78` jump over the bank-to-yaw term
  (`localAngular.y += 30 * craft+0x174 * (1 - magLockBlend)`, `0x0884ad38`-`0x0884ad40`
  onward) and into the downforce. **State 0 skips the bank-to-yaw coupling.**
- `Ship_ApplyAngularDamping` (`0x08848ed0`): the roll coefficient is `-5.0` in state 0
  and `-2.0` otherwise.
- `Ship_HoverTwoPoint`'s head: the `rebound` base is `1.0` in state 0 and `handling+4`
  otherwise.
- `Ship_UpdateCraft` (`0x08849618`): while `craft+0x1c0 & 2`, the control record at
  `*(craft+0x78)` is re-written every frame - `+0x8` and `+0xc` to `100.0`, `+0x0`, `+0x4`
  and `+0x10` to `0` - which holds both airbrakes full, the stick and the thrust off.

**Live** (PPSSPP 1.20.4, Time Trial on `16_Track`, four runs, `scripts/psp-start-pose.py
--full`): `craft+0x1c0` reads `0x2` then `0x3` and `craft+0x2a4` reads `0` through the
countdown; the frame before the throttle word steps `craft+0x1c0` reads `0x1`, `+0x2a4`
`1`, and the brake and both airbrake words go from `100` to `0` (`+0x2bc`, `+0x2c4`,
`+0x2c8`). Yaw is constant to the third decimal until that frame.

Confidence **90** for all three names: every instruction is read, the effects are watched
live on four runs, and the PS2 build was not compared. `Ship_SetState`'s remote-craft
guard (`entity+0x368 != 1`) is from the decompile only.

## Correction 2026-10-01: the finished craft is not handed over by `Race_FinishAllCrafts`

The paragraph above ("`Race_FinishAllCrafts` is the flag: every craft is switched to `autopilot_input`... **the original
drives the player's craft itself after the finish line**") is right about the behaviour and wrong about the mechanism in
Single Race and Time Trial. Live captures (PPSSPP, `16_Track`, four runs) show the function never runs there: its
`manager+0x1a78` flag stays `0`, the player's `entity+0x368` stays `0` and no craft's driver pointer changes. Each craft is put
in state 2 at **its own** crossing by `FUN_088418e0`, and the AI takes it through the autopilot weight `craft+0x1d4`. The function
itself is unchanged; see [race-finish.md](race-finish.md) and
[after-the-finish.md](../../../gameplay/after-the-finish.md).

## The grid walk, read to the end (2026-10-02)

**Status: read in full and reproduced against the original to 0.001 units on `01_Track`.**
`pulse-grid-walk`, PPSSPP 1.20.4 on `pulse-psp-usa.iso`, `FUN_0882663c`, the locate chain
under it and `Race_ComputeGridLayout`'s loop read instruction by instruction, then the walk
checked against eight craft on each of three circuits and against the eight located records
read live out of the loop.

| Address | Name | What it is | Confidence |
| --- | --- | --- | ---: |
| `0x0882663c` | `Race_ComputeGridHeading` | the heading at a located point: locate it, locate `+20 * tangent`, return `normalize(unit(L1 - L0) + unit(R1 - R0))` of the two samples' left and right edge points | 90 |
| `0x0887cf88` | `AiTrack_BuildLocatedRecord` | picks the four control points of the segment between the cursor's point and its nearer neighbour, runs `FUN_0887c340` for `t`, hands `t` to `FUN_0887c7e8` | 85 |
| `0x0887c340` | `Spline_ProjectOntoSegment` | three Gauss-Newton steps from `t = 0.5`, `t -= (C(t) - p) . C'(t) / (C'(t) . C'(t))`, unclamped inside the loop and clamped to `0..1` after it | 88 |
| `0x0887c1e0` | `Spline_EvalPointAndDerivative` | the uniform cubic B-spline of four `vec4` at `t` (`0x0887c1f4`..`0x0887c2a0`) and its derivative (`0x0887c2b4`..`0x0887c334`), the basis rows `(-1,3,-3,1) (3,-6,0,4) (-3,3,3,1) (1,0,0,0) / 6` and their derivative | 88 |
| `0x0887c7e8` | `Spline_SampleSegment` (named at 75 on [scene-light.md](scene-light.md), kept) | blends the `0x70`-byte record over the same four control points with the weights multiplied by `vfim.s 0x3155`; **this read's evidence scores it 90** (the `w` lane read live, the chain it feeds exact to 0.0002), and its row is left for that page's owner to rescore | 75 |

`0x0887ce78` `AiTrack_LocatePosition` and `0x0887e464` `AiTrack_UpdateCursor` keep their names and
scores. **Ghidra's prototype for `AiTrack_LocatePosition` is wrong and the call sites show
it**: the radius is the float register `f12`, then `a0 = track`, `a1 = the record to fill`, `a2 =
the position`, `a3 = a cursor to keep (0 at every grid call)`, `t0 = a path to exclude (-1)`,
`t1 = force a full scan (0)`. The body's `lv.q C400,0x0(a2)` at `0x0887ce7c` is the position
copy and its `move s1,a3` / `beq a3,zero` at `0x0887ceb4`/`0x0887cec0` the cursor, which the
decompile's six integer parameters read as a different thing. At `0x0882b60c`-`0x0882b624`,
and again at `0x0882bda0`-`0x0882bdbc`, the layout sets `f12 = 100.0`, `a0 = [s3 + 0x4c]`,
`a1 = a2 = s7` (its own record, `sp + 0x60`), `a3 = 0`, `t0 = -1`, `t1 = 0`. Both the
position in and the record out are `s7`: the call overwrites the point it was given.

### The loop

`Race_ComputeGridLayout` (`0x0882b3b0`), a record at `s7`:

1. `0x0882b624` locates the node (`[registry entry + 0x30]`, the authored position) into `s7`.
2. The side of the first `10.0`: the node's distance to `pos - lateral * [+0x44]` (the **left** edge)
   against `pos + lateral * [+0x48]` (the **right**), `0x0882b6a0`-`0x0882b794`. Nearer the left
   edge negates the `10.0`, and every slot negates it again (`neg.s f22` at `0x0882bdc4`).
3. Each of eight iterations, slot 8 first: the midpoint of the two edge points, the unit direction
   from it to the right edge point (kept unnormalised below `1e-4` squared length,
   `0x0882ba0c`-`0x0882ba78`), the slot at `midpoint + direction * sign`; then `FUN_0882663c` at
   `0x0882bb54` for the heading; then the up row from the record's own `down`; the drop
   raycasts; and the step `s7.pos += s7.tangent * 19.8` (`0x0882bd48`-`0x0882bd9c`, the `19.8` in
   `0xc38(sp)`, stored at `0x0882b600`) followed by `AiTrack_LocatePosition` again (`0x0882bdbc`).

The chain is `p(k+1) = locate(p(k) + tangent_k * 19.8)`, **where `p(k)` is the located record's
own position and `tangent_k` its own interpolated tangent**, and the step is from the centreline
and not from the laterally offset slot.

### Two things the fit could not have found

**The locate is a projection, not a nearest sample.** `AiTrack_UpdateCursor` finds the nearest
control point (`FUN_0887d5bc`, `FUN_0887d6c8`: **not read**, the port takes the globally nearest and
the eight live records agree), `FUN_0887cf88` steps the cursor twice one way
(`FUN_0887d17c`, `0x0887cfdc`, `0x0887cffc`) and three times the other (`FUN_0887d270`,
`0x0887d01c`-`0x0887d03c`), compares the position's squared distance to the point one way
and the point the other way (`vsub.t`/`vmul.t`/`vfad.t` and `c.le.s` at `0x0887d06c`-`0x0887d09c`),
shifts the four-point window one place when the second is nearer (`0x0887d0a4`-`0x0887d0d8`), and
calls `FUN_0887c340` (`0x0887d0fc`) and `FUN_0887c7e8` (`0x0887d114`): the window is the cursor's
point and the nearer neighbour. Which of `FUN_0887d17c`/`FUN_0887d270` steps toward higher
indices was not read; the rule only compares distances, so it does not matter to the port. `FUN_0887c340` then projects onto
that segment's cubic B-spline of the **lifted** control points (`pos - 3.0 * down`,
`AiTrack_LoadPathPoints`), clamping `t` last. The project's old `STEPS_PER_SEGMENT = 4` resample
put every slot on the nearest of samples 1.5 units apart: the sawtooth.

**The record is scaled by `0.999756`.** `FUN_0887c7e8` loads `vfim.s S733, 0x3155` (`0x0887c86c`),
builds the four cubic weights (`0x0887c870`-`0x0887c900`) and multiplies the weight vector by it:
the half-float immediate `0x3155` is `0.1666259765625`, where `1/6` is `0.1666666667`. Every field
of the record (the four `vec4` and the six scalars at `+0x40`-`+0x54`, `0x0887c928`-`0x0887c964`
and again at each of the next three points) is then blended with those weights and stored
(`0x0887cd20`-`0x0887cd44`). The position scales toward the world origin, the tangent shortens, the
`w` lane reads `6 * 0.1666259765625 = 0.999755859375`. **Read live**, on all eight records
at `0x0882bb54` on Metropia reversed (`scripts/psp-grid-walk.py`): `w = 0.999755859375` exactly,
and the tangent's length `0.99974`-`0.99976`. The consequences are position-dependent, which is
why a constant step fitted one circuit and not another: the scale shifts a point by
`0.000244 * |coordinate|` (0.17 units at `x = -721`, 0.07 along `z = 283` on `01_Track`), and
the along-track part of that is lost from every step. Fitting the step length alone gave
`19.82`, `19.74` and `19.67` for the three circuits (probe scan, worst slot 0.07, 0.18, 0.17):
it is `19.8 * 0.99976 * 0.99976` and the origin, not a different constant.

### The heading

`FUN_0882663c` (`0x0882663c`..`0x08826b14`): locate the position (`0x0882674c`, record at `sp + 0x20`),
`pos2 = pos + tangent * 20.0` (`vscl.q` by `0x41a00000`, `0x08826760`-`0x08826770`), locate `pos2`
(`0x088267d8`, record at `sp + 0x90`), and with `L = pos - lateral * [+0x44]` (`0x08826820`-`0x0882682c`,
`0x088268b0`-`0x088268fc`) and `R = pos + lateral * [+0x48]` (`0x08826848`-`0x08826894`,
`0x08826920`-`0x08826974`) for each sample, return `normalize(normalize(L2 - L1) + normalize(R2 -
R1))` (the three `vdot.t`/`vsqrt.s`/`vrcp.s` sequences at `0x088269dc`, `0x08826a4c` and
`0x08826abc`, the result stored at `0x08826ae4`).
It is the direction the track's **edges** run, which is the tangent only where both edges
parallel it. The matrix row for forward is that, the up row the record's `-down` orthogonalised
against it, the left row their cross product (`0x0882bbb0`-`0x0882bc38`).

### Against the original

Eight craft read at placement (`scripts/psp-grid-pose.py`), and the walk through
`oag_gameplay::grid_walk` (`crates/game/tests/grid_walk_ground_truth.rs`), xz distance and
heading in degrees:

| circuit | worst slot before | worst slot now | worst heading before | now |
| --- | ---: | ---: | ---: | ---: |
| `01_Track` (Basilico Black) | 1.73 | **0.001** | 0.22 (slot 1) | 0.0005 |
| `16_Track` (Talon's Junction) | 0.62 | 0.043 | 0.045 | 0.0009 |
| Metropia reversed | 1.13 | 0.070 | - | - |

`01_Track` slot 1's `-0.0069` heading, "0.22 degrees, cause unknown" in
[grid-state.md](../../../physics/grid-state.md), is the edge chord: the track widens there and
the tangent does not see it. The eight located records read live on Metropia match
`locate`'s chain to **0.0002** units. The remaining 0.04-0.07 on `16_Track` and Metropia is **probably** the placement drop (the
original raycasts along the sample's own down axis, then `+2`; ours drops along world `-Y`) and,
on Metropia, the two-decimal readings: not measured, and `16_Track`'s 0.043 shows with no raycast in the
probe at all, so it may be something in the walk itself.

Confidence **94** that this is `Race_ComputeGridLayout`'s walk on Pulse PSP (USA): a runtime
trace of the records, eight craft on three circuits agreeing to hundredths and the headings to
thousandths. It does not reach 95: **no second binary** (the EU PSP build and the PS2
executable were not compared). The names above are scored below it, on what each read gave.

### Corrections this read made

- The corridor midpoint is the **track edges'** midpoint, not the AI corridor's: the loop reads
  `[+0x44]`/`[+0x48]` (`lwc1 f12,0xa8(sp)` at `0x0882b62c`, `lwc1 f13,0xa4(sp)` at `0x0882b6d8`,
  against a record at `sp + 0x60`), the half-widths. `ai_bound_left`/`right` are `-(hw - 8)`/`hw - 8` in the grid
  regions of the three circuits measured, which is why the 2026-09-29 rule fitted; elsewhere on
  the same tracks they differ by up to 64 units. `grid_stagger_ground_truth`'s corridor-midpoint
  test still passes on all 24 circuit-directions, since the two rules agree within its 1.5
  there.
- The reversed branch (`-19.8`, slot index `9 - i`) stays unported: Metropia reversed, laid out
  forward, matches to 0.07, so it is still the measured layout that is ported and not the
  decompile's.

### Not ported, and one circuit that is

- **The hash cache and the junction hop.** The original seeds from a spatial hash and steps its
  four-point window across a junction at a path's end. `oag_gameplay::grid_walk::locate` scans every
  control point and clamps. `03_Track`'s front slot sits one control point from its path's
  end: the clamp costs under a tenth of a unit there. No other circuit-direction's grid comes
  within seven control points of a path end (`09_Track` is next, at 7).
- **`25_Track` reversed** puts its authored node 25.9 units under the located sample (the
  nearest control point is on a ramp over it; the other 23 circuit-directions are 0.3 to 4.4
  off). What the original does there was not captured; the walk refuses a node more than 12
  units off (`oag_gameplay::grid_walk::MAX_NODE_HEIGHT_OFFSET`, **chosen, not measured**) and
  the node-anchored grid stays.
- Pulse PS2, Pure and the HD titles keep their layouts: only Pulse PSP was read.

### Reproducing

```sh
python3 scripts/psp-drive.py --port 45491 menu --single-race --track-down 2     # Metropia
python3 scripts/psp-grid-walk.py --port 45491 --out data/scratch/<lane>/walk.json
```
