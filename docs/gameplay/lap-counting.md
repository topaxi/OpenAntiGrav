# Lap counting

**Status:** implemented, and it is **our convention rather than a recovery**.
Confidence in the *shape* of the loop is 88 because that is the original's own
traversal; confidence in *where the lap begins* is 55, and that is the number to
argue with.

## The problem

Nothing in the original says where a lap starts or how one is counted.

- **`gate` has no runtime class registration.** `.vex` class `0x3ca` is decoded by
  nothing: all 46 callers of the class registrar were enumerated and none passes
  it, so nothing reads a `gate` payload. See
  [track data](../formats/track.md#where-is-lap-counting).
- **No lap or split logic was found in the executable.** Every `lap`-matching
  string is a HUD label, a save key or a music cue.
- **`Start Position` is a grid slot, not a start line.** One node per track on all
  40 PSP files, 3.3 and 20.5 units off the spline's own centreline.

So the choice was to leave lap counting unimplemented, or to define it. It is
defined here, and this page is the definition.

## The shape of the loop is recovered

`Course::from_track` (`crates/race/src/course.rs`) walks the track's junction
graph the way the original walks it - confidence **88**,
[track data](../formats/track.md):

```c
Path *next = path->exit->next_primary;
if (path->exit->next_alternate && index(next) == excluded_path)
    next = path->exit->next_alternate;   // branch selection
```

So the ring follows `next_primary` and nothing else. Alternate paths are
shortcuts, chosen by an `excluded_path` argument the caller threads down, and
they are deliberately **off** the ring. On `05_Track` - the one shipped circuit
with a genuine split - the two branches share both endpoints, so a ship taking
the shortcut still projects onto the ring at a sensible distance.

Two details that are ours rather than the file's:

- **The longest closing chain wins**, not the first one found. A malformed graph
  can contain a short cycle that closes perfectly well while covering a tenth of
  the circuit, and taking the first hit would race on it.
- **A chain that never closes yields no course at all.** `Course::from_track`
  returns `None`, the load report says so, and the HUD omits the lap widget
  rather than showing `1 of 3` on a track that cannot count. A best-effort ring
  would produce plausible, wrong numbers.

## Where the lap begins is not

Arc-length zero is placed at the ring point `Course::START_LINE_OFFSET` **along
the track from the authored grid slot** - 137.9 units, measured along the ring so
it follows a curving start straight.

That number is measured, not invented: a captured time trial's craft begins 137.9
units from the slot, decomposing into 137.9 along the slot's own forward, 22.4
across the track and 0.8 up, with the slot's heading within 1.2 degrees of the
craft's. It is pinned by `the_authored_slot_matches_the_captured_start` in
`crates/game/tests/race_ground_truth.rs`.

**Confidence 65**, and the two halves of that are different kinds of claim:

- **The distance is measured**, on one capture of one circuit - Talon's Junction.
- **That it generalises is observed, not measured.** Moa Therma, a different
  circuit with no capture, counts its laps in the right place with this constant.
  That was checked by driving it. It rules out the offset being a property of the
  one circuit it came from, which was the live worry; it does not rule out a
  per-track offset that happens to be close on both, because a player's eye is not
  an instrument.

Whatever lays a grid out is still unread code. What would retire the constant:
that code, a capture on a second circuit, or the path-boundary lead below.

Getting this wrong is visible rather than subtle, which is the one convenient
thing about it. Placing the line at the slot - which the first implementation did
- makes the counter tick over partway down the starting straight instead of at the
end of it, and that is how the error was found: by driving a time trial on Moa
Therma and watching it happen.

### The lead that would replace it

`Course::path_boundaries` reports where one path hands over to the next, and the
load report prints it beside the start line. On `16_Track` the ring is two paths
and the boundaries are `[0, 1752]` against a start line at `3415` - the same
neighbourhood as the offset on one side, nowhere near it on the other, so the
answer is not simply "the boundary is the line".

If a boundary does land on the visible line across several circuits, the start
line is **authored per track** and comes out of `Path::exit` / `Junction` at the
same confidence 88 as the traversal - no single-capture constant, and it
generalises to all 40 circuits by construction. Checking it costs one look at the
load report per track.

## A lap is a wrap, gated

Progress is the distance along the ring from that zero. A lap is a **wrap** of
that distance - a drop of more than half the circuit in one tick - not a plane
crossing. At 60 Hz even a Phantom-class craft covers a few units per tick against
a circuit thousands of units round, so the two cases are never close.

A wrap alone is not enough, and `RaceState::lap_gate` is the difference. A lap
requires the near half of the circuit **and then** the far half, in that order,
since the last crossing. Two real failures need it:

1. **The ship spawns behind the line.** It sits on the grid slot, which is
   `START_LINE_OFFSET` upstream, so it crosses the line seconds into the race.
   That is a genuine wrap and is not a lap.
2. **Rocking over the line.** Reversing across and driving forward again
   re-crosses it. Counting that would also restart the lap clock, so a few
   seconds of nudging would record an unbeatable best lap.

Reversing over the line takes the lap count back down, and the lap being
re-entered has to be earned again. Strict - the ship did drive it once - but the
alternative is the exploit above, and driving backwards over a start line is
already a wrong-way situation the HUD warns about.

## The lap clock starts at the line

Lap 1 is timed from the first crossing, not from the standing start, because our
ship spawns on the slot while the original's starts on the line. Timing from the
standing start would make lap 1 longer than every other lap by the offset, and
longer than the original's by the same amount.

**Do not compare tick counts against the original's own clock.** Two laps of
3,069 and 3,087 ticks were timed by the game at `0.50.25` and `1.11.08`: whatever
it counts, it is not frames. The lap *counter* is unaffected and is what to
compare. See `crates/game/src/hud.rs`'s `TICKS_PER_SECOND`.

## What would replace all of this

A recovered lap counter. The most likely home is `SplinePt.flags` (`+0x61`), which
is OR-accumulated and surfaced by the spline evaluator - but that was not
verified, the consumer of the byte was not found, and `flags` is `0` on every
control point of `01_Track`. If someone finds it, this page becomes a note about
what was used before.

## See also

- [race modes](race-modes.md) - what each mode does with a lap
- [track data](../formats/track.md) - the spline graph and the open questions
- `crates/race/src/course.rs`, `crates/race/src/state.rs`
