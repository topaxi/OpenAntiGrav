# Lap counting

**Status:** implemented, and since 2026-09-16 **a recovery rather than a
convention** for the two things that matter: the loop's shape (confidence 88,
the original's own traversal) and *where the lap begins* (confidence 88, the
original's own rule, read from `RaceManager_Construct`). What is still ours is
recorded under "Where we differ" at the end.

## The problem, as it stood

Until 2026-09-16 nothing in the original had been found that said where a lap
starts or how one is counted:

- **`gate` has no runtime class registration.** `.vex` class `0x3ca` is decoded by
  nothing: all 46 callers of the class registrar were enumerated and none passes
  it, so nothing reads a `gate` payload. See
  [track data](../formats/track.md#where-is-lap-counting). Still true - the
  counter does not use it.
- **No lap or split logic was found in the executable.** Every `lap`-matching
  string is a HUD label, a save key or a music cue. True, and the way in was the
  HUD label: the `Lap` widget's string is written from a field, and that
  field's writer is the counter
  ([race-progress.md](../ghidra/functions/psp-pulse-usa/race-progress.md)).
- **`Start Position` is a grid slot, not a start line.** One node per track on all
  40 PSP files, 3.3 and 20.5 units off the spline's own centreline. Still true -
  and the line is *derived* from it, below.

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

## Where the lap begins: 154 units past the slot, along the tangent

The original's race manager computes the line once, at construction
(`RaceManager_Construct`, `0x08829124`,
[race-progress.md](../ghidra/functions/psp-pulse-usa/race-progress.md)):

1. locate the authored `Start Position` on the spline (`AiTrack_LocatePosition`,
   search radius 10,000);
2. step **154.0 units** (`0x431a0000`) from that sample along the sample's
   tangent, in a straight line;
3. locate that point on the spline (radius 100) and keep its arc position.

`Course::from_track` does the same with the ring it builds: nearest ring point
to the slot, `Course::START_LINE_ADVANCE` along that point's tangent, nearest
ring point again - `course.start_index`. The literal is the title's, applied in
the one place that can apply it; it is the same on every circuit, which is why
the constant it replaces happened to work on two.

**The constant it replaces.** Before this, arc-length zero was
`Course::START_LINE_OFFSET = 137.9`, measured *along the ring* from the slot on
one capture of Talon's Junction and observed to count laps in the right place on
Moa Therma. That number was real, but it was a different quantity: it is where
the original **spawns the craft** relative to the slot, not where the line is
(`the_authored_slot_matches_the_captured_start`,
`crates/game/tests/race_ground_truth.rs`, still holds). With the recovered rule
the captured craft starts **16.5 units behind the line** on Talon's Junction
(`the_captured_run_begins_just_behind_our_start_line`,
`crates/trace/tests/lap_capture_ground_truth.rs`) - the original puts its craft
short of the line and lets the first crossing start the race. Our own spawn is
on the slot, further back still, and the same gate below absorbs both.

The path-boundary lead recorded on this page in its earlier form (checked on
all 40 files, 2026-08-30, median 648 units from the line) stays dead; the
recovered rule confirms the line is computed, not authored as a path split.

## How the original counts

`Craft_UpdateLapProgress` (`0x08842a18`), per craft per tick:

- The craft's spline sample carries `t`, the **authored normalised arc
  position** (`SplinePt+0x40`, `0..1` round the circuit - the field
  [track.md](../formats/track.md) had as unknown), and the track object's first
  word is the load-time units-per-`t`, so `arc = t * L`.
- Progress is `arc` **unwrapped** across the circuit: an integer wrap count is
  kept, and each tick the wrap that moves progress least (`0`, `+L`, `-L`) is
  chosen.
- A **crossing count** is `wraps + (line < arc)`. It rises by one at each
  forward crossing of the line and falls by one at each reverse crossing; a
  lap is the count rising by exactly one when it equals the count the next lap
  is expected on.
- The **first** forward crossing sets a `started` flag and records no lap; the
  counter the HUD shows is `next_expected - 1`, initialised so the grid reads
  "Lap 1".
- On a lap: the time is interpolated to the fraction of the tick in which the
  crossing happened, stored in centiseconds in a 20-entry array, compared with
  the best, checked against the profile's records, flagged perfect if no wall
  was hit that lap, and the twenty fastest laps are kept.

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

Reversing over the line does **not** take the lap count down (changed 2026-10-02). The
original keeps a crossing count that falls on a reverse crossing, but the target it
completes the next lap on - and so the HUD lap - is only ever raised
(`Craft_UpdateLapProgress`, `0x08842a18`, [race-progress.md](../ghidra/functions/psp-pulse-usa/race-progress.md),
confidence 92, read statically, **not** observed live with a reversed craft). `RaceState` and
`Standing` therefore carry `reversed`, the backward crossings not yet crossed forward again:
a reverse crossing raises it and resets the gate, the forward re-crossing lowers it and earns
nothing, and the lap being driven is counted at its next full crossing. Before this the lap was
lowered and the re-crossing earned nothing, so a craft shoved back over the line ended a lap
short in every mode. Rocking over the line still records no lap, which is the exploit the gate
exists for. The player's Eliminator respawn (`last_on_track`) that lands behind the line is the
same case and is now harmless.

## The lap clock starts at the line

**Never before the release, though.** The original's lap clock reads `0.0`
through the whole start-line countdown and takes its first `dt` on the tick thrust
is released (measured 2026-09-29, four captures - see
[race-modes.md](race-modes.md#the-race-clock-starts-at-the-release)), so a lap's
clock here is floored at `oag_race::COUNTDOWN_TICKS`: `RaceState::lap_ticks`. What
follows is about the *first crossing* on top of that.

Lap 1 is timed from the first crossing, not from the standing start, because our
ship spawns on the slot, further behind the line than the original's own craft.
Timing from the standing start would make lap 1 longer than every other lap by
the offset.

**This is every craft's rule, not the player's, as of 2026-08-17.**
`oag_race::Standing` carries a clock and a best lap of its own, so the whole grid
is timed. On a grid the argument above is stronger than it is for one ship: each
slot is a *different* distance behind the line, so timing from the standing start
would give every craft its own error and the field's first laps would not be
comparable even with each other. Slot 0 therefore holds two clocks - its
`Standing`'s and `RaceState`'s - and
`standing::tests::the_standings_clock_agrees_with_the_players` is what stops them
drifting apart.

**Do not compare tick counts against the original's own clock.** Two laps of
3,069 and 3,087 ticks were timed by the game at `0.50.25` and `1.11.08`: the
original accumulates its variable `dt` and interpolates the crossing within the
tick, so its lap times are not frame counts. The lap *counter* is unaffected
and is what to compare. See `crates/hud/src/lib.rs`'s `TICKS_PER_SECOND`.

## Where we differ

Read [race-progress.md](../ghidra/functions/psp-pulse-usa/race-progress.md)
for the original in full; the divergences that remain are these, each a choice:

- **Progress.** Ours is a ring-point table with a windowed locator; the
  original's is the spline `t` field times a load-time length. Same quantity,
  different instrument; a lap is a wrap in ours and an integer crossing count
  in theirs, and neither lowers the lap on a reversal.
- **Lap 1's clock.** The original does **not** restart its clock at the first
  crossing: lap 1 runs from the thrust release, which is when the race update
  starts stepping crafts and the lap clock leaves `0.0` (measured, see above).
  Ours starts at the line, because our spawn is on the slot rather than the
  original's 16.5 units short of the line, and a slot-timed lap 1 would be
  wrong by a different amount per grid slot.
- **Sub-tick times.** The original interpolates the crossing within the tick -
  and, as read, adds that fraction to a clock that already holds the whole tick,
  so every recorded lap is one frame long. Ours is whole ticks at 60 Hz. Neither
  is implemented as the other; the doc comment on `oag_race::Standing` says so.
- **Progress on the far side of a fork.** The original locates a craft on
  whichever path it is on (`Craft_UpdateLapProgress` passes the AI's excluded
  path, `0x08842af4`) and reads that point's authored progress. Ours is a ring,
  so a craft on a route (`oag_race::course::Route`) is located on the route and
  read at the same fraction of the ring span the route stands in for
  (`Course::locate_on_route`). **Chosen, not measured**: continuous at both ends,
  and on nested routes (2048's `square`) a shared stretch can step back by up to
  40 units where two routes diverge - a lap is 7,890 there and the gate needs
  half of one. Before this a craft on a detour that strayed past the reacquire
  distance could read a lap away (`park`: 4,388 units). See
  `crates/game/tests/branch_progress_ground_truth.rs`.
- **The split table, the perfect-lap flag, the twenty fastest laps** and the
  profile statistics the original updates on a crossing are not modelled.

## See also

- [race modes](race-modes.md) - what each mode does with a lap
- [track data](../formats/track.md) - the spline graph and the open questions
- `crates/race/src/course.rs`, `crates/race/src/state.rs`
