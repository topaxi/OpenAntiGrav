# Code review, 2026-11-23: `oag-gameplay` and the game logic around it

A focused review of the simulation seam: `crates/gameplay` in full, plus the
callers and producers that decide what reaches it (`oag-raceplay`'s tick, player
step, detonator and pads; `oag-input`'s pad resolution and merge; `oag-weapons`'s
detonator field it hashes). Reviewed for correctness, determinism, safety on
malformed input, and boundary discipline. Nothing was changed; this is findings
only.

The crate's stated contract is "one `World` of plain data, an input snapshot in,
a state snapshot out" (`crates/gameplay/src/lib.rs`). Most of what follows is
about whether that contract actually holds at the edges.

## Grading rubric

Same grades as [2026-08-18-code-review.md](2026-08-18-code-review.md):

| Grade | Values | Meaning |
| --- | --- | --- |
| **Impact** | High / Medium / Low | What it costs if left as is. |
| **Priority** | P0 - P3 | P0 fix now, P1 next session on the area, P2 when next open, P3 opportunistic. |
| **Confidence** | 0 - 100 | 95 re-verified by a second reader, 80-85 one reviewer with exact code, 70 design judgement. |
| **Faithfulness** | yes / no / n.a. | Divergence from observed original behaviour vs engineering defect. |
| **Effort** | S / M / L | S a line or guard, M one-crate refactor, L cross-crate. |

## Findings

### G1 - `detonator.ramp` leaves the state hash whenever the EMP is not armed

- **Impact:** High. The hash is the determinism gate; a field that moves the
  engine target and is not hashed is exactly the "pad timer one tick out"
  failure `hash.rs`'s own module doc says it exists to close.
- **Priority:** P1
- **Confidence:** 85
- **Faithfulness:** no
- **Effort:** S

`write_detonator` writes `ramp` only inside `if emp.armed`
(`crates/gameplay/src/hash.rs:160-164`), and the whole detonator block is
skipped while `Field::is_empty()`
(`crates/weapons/src/detonator.rs:165-171`, which does not test `ramp` at
all). But `RaceSim::detonator_auto_speed` steps `world.detonator.ramp` every
tick for the primary craft whenever `self.detonator` and `self.zone` are
`Some` - with no reference to the EMP
(`crates/raceplay/src/detonator.rs:410-417`,
`crates/weapons/src/detonator/emp.rs:321-337`).

`emp.armed` is not a load-time fact. It is set as a side effect of
`emp::step_charge` (`crates/weapons/src/detonator/emp.rs:172-177`), which runs
only when `detonator.emp` is `Some`
(`crates/raceplay/src/detonator.rs:352-354`). `detonator.emp` is
`emp::Config::from_tables`, which returns `None` when the weapon table has no
`EMP` block or lacks the two pad counts
(`crates/weapons/src/detonator/emp.rs:63-79`). Every shipped Detonator table
happens to author all seventeen attributes including `EMP`
(`docs/formats/weapon-stats.md:435-438`), so today `armed` is true from the
first `advance_emp` tick - but the auto-speed ramp is Detonator's engine law,
not the EMP's (`docs/ghidra/functions/ps3-hdfury-eu/detonator-speed.md:14-25`),
and a table that drops the EMP block, a mod table, or a `Config::from_tables`
that declines on missing pad counts leaves a live, engine-moving `ramp`
completely outside the hash. Two runs differing only in ramp then agree on
`hash_world` while flying different speeds.

Fix shape: hash `ramp` whenever the field is non-empty (or whenever a Detonator
race is running), not under `emp.armed`; `armed` should gate the EMP's own
fields, not the auto-speed's.

### G2 - `grid_walk::locate` picks the wrong B-spline segment at a path's last control point

- **Impact:** Medium. Wrong located record means wrong grid pose or wrong
  heading lookahead for any query whose nearest control point is a path's
  final point. Bounded today by the fact that grids sit near the start node,
  but `locate` is `pub` and `heading` re-locates a lookahead point that can
  land past a path's end.
- **Priority:** P2
- **Confidence:** 70
- **Faithfulness:** yes (the original steps its window across the junction;
  this port clamps, and the clamp interacts with the segment pick)
- **Effort:** S

`locate` chooses the segment between the nearest control point and its nearer
neighbour by comparing distances to `control(path, at - 1)` and
`control(path, at + 1)` (`crates/gameplay/src/grid_walk.rs:141-144`). `control`
clamps an out-of-range index to the path's end (`grid_walk.rs:96-99`), so for
the **last** control point `at + 1` returns the last point itself: `next`
becomes the distance to the nearest point, which is always smaller than
`previous` (the nearest-point scan keeps the earlier index on a tie, so strict
`next < previous` holds). The code then takes `segment = at`, building the
window `[N-2, N-1, N-1, N-1]` - a segment *after* the path's end - instead of
`segment = at - 1`, whose clamped window `[N-3, N-2, N-1, N-1]` is the last real
segment. The first-point case is handled correctly only because
`(at - 1).max(0)` lands on `0`; there is no symmetric guard at the top end.

This is separate from the documented "junction hop not ported" gap
(`grid_walk.rs:119-124`): even with clamped windows, the segment index itself
is off by one at the final point. The comment's "the clamp moves a slot by
under a tenth" was measured on `03_Track`, whose front slot is one control
point from the end - worth re-measuring whether that tenth is this bug or the
junction approximation.

**Status: fixed in part; the review's fix shape was wrong.** The segment pick
is off, but only for a query between the last two control points that is nearer
the last one (on a level straight at six-unit spacing, x between about 351.5
and 353 against a last point at 354): `locate` fitted it on the end-clamped
window `[N-2, N-1, N-1, N-1]`, which spans only the final stub of the curve
(about 353 to 354), clamped `t` to 0 and returned a foot one unit away. Taking
`at - 1` for every query nearest the last point, as suggested here, is worse:
`[N-3, N-2, N-1, N-1]` ends one sixth of a spacing short of the last control
point, so a query at or past the end would stop a unit short of it. `locate`
now fits the padded last segment first and falls back to the one before it
only when its unclamped `t` is negative. Tests:
`grid_walk::tests::a_query_just_behind_the_last_control_point_is_fitted_on_the_segment_before_the_end`
(red before, green after) and
`a_query_at_or_past_a_paths_end_still_reaches_the_last_control_point` (green
both ways; it fails under the review's fix).

The `03_Track` "under a tenth" note is **not** this bug: the walk of
`03_Track`, `01_Track`, `16_Track` and `18_Track` is slot for slot identical
(positions to four decimals) before and after, so what the note measures is the
clamp and the missing junction hop. The three captured circuits' grid ground
truths are unaffected for the same reason; `03_Track` has no original capture.

### G3 - Detonator's cannon and EMP fire read slot 0 while the charge gate reads `primary_slot()`

- **Impact:** Medium when it matters, Low today. Multi-human grids are not
  started yet, but the inconsistency is already in the code and will produce a
  wrong-slot fire the day `controllers` marks a second human.
- **Priority:** P2
- **Confidence:** 75
- **Faithfulness:** no
- **Effort:** S

Three different slot rules in one mode:

- `charge_emp` charges when `slot == self.world.primary_slot()`
  (`crates/raceplay/src/detonator.rs:453-456`).
- The cannon reads `inputs.get(0)` and steps with `&self.world.ships[0]`
  (`detonator.rs:332-340`), commented "slot 0 only".
- The EMP fire checks `inputs.get(0).buttons` and launches from
  `self.world.ships[0]` (`detonator.rs:388-395`).

`primary_slot()` is the first human or Away slot and falls back to 0
(`crates/gameplay/src/world.rs:566-571`), so it is 0 only while slot 0 is the
primary. In any configuration where it is not, a pad charge is attributed to
the primary craft while the fire button and the cannon's owner are slot 0 -
the EMP could be charged by one craft and fired by another's input. Either
everything should key on `primary_slot()` (matching `charge_emp` and the
`person_flown` abstraction the rest of the tick uses) or the mode should state
plainly that Detonator is slot-0-only and refuse other humans.

### G4 - `shoulder_flips` erases the independent L2 airbrake for the whole flip press

- **Impact:** Low-Medium gameplay feel on HD Eliminator/Detonator. Documented
  as a known cost, confirmed here against the input pipeline.
- **Priority:** P3
- **Confidence:** 90
- **Faithfulness:** yes (the original binds flip to L1 and airbrakes to
  L2/R2 distinctly; this port cannot)
- **Effort:** M

`InputSnapshot::shoulder_flips` clears the `Button::L` bit from all four masks
and forces `airbrake_left = 0.0` whenever `Button::Flip` is in held, released
or pressed (`crates/gameplay/src/input.rs:63-77`). On a pad under
`TriggerMode::Airbrakes`, `Button::L` is raised both by the L1 shoulder (which
`bits_of` also marks as `Flip`) and by the L2 trigger past
`TRIGGER_THRESHOLD`, and `airbrake_left` carries the L2's conditioned travel
(`crates/input/src/pad.rs`, `resolve`'s trigger loop). So holding L1 - flip or
EMP - zeroes the left airbrake the player is pulling on L2 for as long as L1
is down, including its `pressed`/`released` edges.

`docs/ghidra/functions/ps3-hdfury-eu/eliminator-flip.md:186-187` already names
this: "L1 down reads the whole left airbrake axis as zero ... Our snapshot
cannot tell the two apart." Reported here because the review asked for all
findings; the fix is a snapshot that carries the two shoulders separately
rather than one `Button::L` plus one axis, which is an `oag-input`/replay-format
decision, not a one-line change.

**Status: not fixed, design written.** The fix as suggested here (a snapshot
carrying both shoulders) reaches `oag-gameplay`, `oag-input`, `oag-replay` and
`oag-net`'s wire. A narrower route exists upstream of the snapshot: the pad
resolve raises `Flip` for L1 without `L` or the airbrake in a flip race, so the
snapshot keeps its shape. Untested; the design and its open checks are in the
`handover/gameplay` thread `l1-flip-zeroes-the-l2-airbrake`.

### G5 - `World::person_flown` flies the primary slot with no craft check

- **Impact:** Low. Only reachable through caller configurations the engine
does not start today (a human slot marked beyond `ship_count`, or an inactive
primary). The non-primary branch is guarded; the primary branch is not.
- **Priority:** P3
- **Confidence:** 70
- **Faithfulness:** no
- **Effort:** S

`person_flown` returns early for `slot == primary_slot()` with only an
`Away` check (`crates/gameplay/src/world.rs:589-597`): it never tests
`slot < ship_count` or `ships[slot].active`, which the general branch does.
The doc comment says "a human slot with no craft in it is not flown" and "the
fallback slot is exempt" (`world.rs:573-587`) - but the exemption is written
against *every* primary slot, including a marked `Local`/`Remote` slot that has
no ship. If that is intended (the viewed craft is always stepped), the doc
should say the primary slot is exempt regardless of `active`; if not, the early
return needs the same `active`/`ship_count` guard the other branch has. As
written the two halves of the function answer the same question two ways.

**Status: reviewed, kept, documented and pinned.** Not a defect. The primary
slot is exempt on purpose and the exemption is relied on: `World::new` marks
slot 0 `Local` with `ship_count` 0, and a guard would turn `person_flown(0)`
false for every blank-grid world (`oag-net`'s redial test, the pad-join test,
unit worlds). In a started race the case cannot arise: `Ship::active` is set
once at the grid and nothing clears it on a wreck, so a primary with no craft
is only the pre-grid world. The doc comment now says the exemption covers the
primary slot whether marked human or fallback, and
`world::tests::a_human_slot_with_no_craft_is_flown_only_where_it_is_primary`
pins both branches.

### G6 - `Pose::from_start_position` lifts by a possibly non-unit `up`

- **Impact:** Low. The bind is documented to force `up` to `(0,1,0)`, so this
  is latent; the function is `pub` and takes any `StartPosition`.
- **Priority:** P3
- **Confidence:** 75
- **Faithfulness:** no
- **Effort:** S

`orientation_from_axes` normalises its inputs, but the position lift uses the
raw vector: `position + up * height` (`crates/gameplay/src/spawn.rs:148`). A
`StartPosition` whose `up` is not unit length (or not finite) scales or skews
the spawn height while the orientation stays correct. `Pose::from_sample`
normalises both axes for exactly this reason (`spawn.rs:64-65`, and its test
`unnormalised_axes_still_give_a_unit_rotation`). One-line fix: lift along
`up.normalize_or_zero()` (or `try_normalize` with a fallback) for consistency.

**Status: fixed.** `from_start_position` lifts along `up.normalize_or_zero()`.
Test: `spawn::tests::the_lift_is_the_height_whatever_the_length_of_up` (red
before, green after). A unit `up`, which every shipped slot has, is unchanged
bit for bit.

## Known limitations confirmed (not new findings)

- **`PlayerInputs::get`/`set` ignore out-of-range slots**
  (`crates/gameplay/src/input.rs:185-194`). Deliberate and documented
  ("a craft that coasts" beats a mid-simulation panic), but it means a caller
  that confuses `ship_count` with `MAX_PLAYERS` gets silence, not an error.
  Worth a `debug_assert` if a cheap one exists.
- **`Projectile::orientation` is deliberately unhashed**
  (`crates/gameplay/src/hash.rs:905-912`). Documented as frozen drawing pose
  nothing reads back. Correct today; the risk is a future reader that makes it
  simulation state without moving it into the hash - the same trap
  `write_ship`'s `handling`/`muzzle`/`track_up` comments already narrate.
- **`collision_world` trusts collider count fits `u32`**
  (`crates/gameplay/src/collision.rs:39`) and clones every mesh's vertices and
  indices into the soup (`collision.rs:40-48`). The `as u32` cannot overflow on
  any real track (hundreds of meshes); the clone is the track-data cost of the
  fixed-array world boundary and is fine, noted only because "no allocation"
  in `world.rs`'s doc refers to `World`, not to this load-time builder.
- **`CageTrack::new`/`sibling` assume junction indices are in range**
  (`crates/gameplay/src/cage.rs:164-178`). The decoder validates them
  (`crates/vex/src/track.rs:735-762`), so this is safe for decoded tracks; a
  hand-built `AiTrack` with bad indices panics. Acceptable given the type is
  decoder output, but the fields are `pub`.
- **`handling_for`'s `PITCH_STAND_IN` and hardcoded `SteerLean::PULSE` /
  `CameraLean::PULSE`** (`crates/gameplay/src/handling.rs:219-224, 273-276`).
  Both are documented inventions/defaults; the lean comment already names the
  `<Tilt>` replacement path. Listed so a future title audit checks whether its
  lean was actually replaced at load.
- **`grid_pose` clamps `slot` into `1..=8`** (`spawn.rs:442-458`). Documented
  deliberate; a bad race option becomes the nearest end slot rather than an
  error.

## What looks right

- `hash.rs`'s exhaustive-destructure discipline is real: every `World`,
  `Ship`, `RaceState`, `Driver`, projectile and detonator sub-struct is
  pattern-matched with no `..`, so a new field is a compile error there. The
  G1 hole is a *conditional* omission inside that discipline, not a missing
  field - which is the harder kind to catch.
- `InputSnapshot::sanitised` runs before the sim and maps NaN to 0
  (`input.rs:86-102`); `PlayerInputs` is a fixed array indexed by grid slot
  with deterministic iteration (`world.rs:537-555`).
- `collision_world` preserves collider order and drops `Cage` exactly as the
  original's loader does, with tests pinning index stability and the
  friction-rule agreement between `oag-formats` and `oag-physics`
  (`collision.rs:32-80` and tests).
- `detonator_events` is cleared at the top of `advance_detonator` every tick
  (`detonator.rs:304`), so the per-tick output cannot leak across ticks; cues
  derived from it are pushed to the drained `cues` vec, matching ADR-0018's
  output-not-state rule.
- `grid_walk`'s recovered constants (`RECORD_SCALE`, `GRID_ROW_PITCH`,
  `GRID_COLUMN_OFFSET`, `HEADING_LOOKAHEAD`) are pinned by ground-truth tests
  against the original's craft positions; the module doc is unusually honest
  about what is measured vs chosen.

## Test coverage notes

- `hash.rs` has a dedicated test for the EMP/ramp gating
  (`the_emp_moves_the_hash_only_where_detonator_armed_it`,
  `hash.rs:1031-1062`) - it asserts the current gating is intentional, which
  is why G1 survives: the test pins the behaviour, not the requirement. A
  test that arms Detonator *without* an EMP config and moves `ramp` would pin
  the actual invariant.
- `grid_walk` tests cover the interior walk thoroughly but nothing places a
  query at a path's final control point, which is where G2 lives.
- No test exercises `person_flown`/`primary_slot` with a human slot that has
  no craft (G5) or `from_start_position` with a non-unit `up` (G6).
- `shoulder_flips` has no test for the L2-trigger case (G4); the existing
  tests likely cover the L1-only path.

## Suggested order

1. G1 - one-line hash fix plus a regression test with EMP-less Detonator.
2. G2 - fix the `at + 1` clamp interaction in `locate` and add an end-of-path
   case to `grid_walk` tests.
3. G3 - pick one slot rule for Detonator before multi-human grids ship.
4. G4-G6 - opportunistic; all are documented or latent.
