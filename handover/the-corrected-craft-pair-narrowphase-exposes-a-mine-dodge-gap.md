# The corrected craft-pair narrowphase exposes a mine-dodging gap in `oag_ai`

2026-09-07. `oag_physics::pair::overlap` was corrected against a live,
instruction-level read of `Collision_BoxAgainstBox` (`0x0881702c`) - see
`handover/craft-to-craft-collision-is-implemented-the-stun.md` and
`docs/ghidra/functions/psp-pulse-usa/contact-response.md` for the physics
side. This row is the one downstream consequence that lands outside
`crates/physics`: `crates/game/tests/opponent_weapons_ground_truth.rs`'s
`a_field_racing_with_real_pads_does_not_mine_itself_to_death` now fails, and a
bisect against the physics change alone shows it is the corrected contact
response doing it, not a flake or an unrelated regression.

## The bisect

Same seed, same track (`single_race()`'s default course - confirmed as
`16_Track` by point count: this test's own report prints
`course: 3448 points over 5094 units`, matching the `of 3448` progress figure
`race_ground_truth.rs`'s regression gate measures live for `16_Track`), same
3,600-tick run,
`pair.rs` swapped between the pre- and post-correction algorithm and nothing
else touched:

| | pre-correction `pair.rs` | post-correction `pair.rs` |
| --- | ---: | ---: |
| mean opponent shield | 89 of 95 | 70 of 95 |
| worst opponent shield | 78 of 95 | **0** of 95 |
| charges (mine/bomb) laid | 6 | 11 |
| opponents still active | 7 of 7 | 7 of 7 |

The test's own floor is `worst > full * 0.45` (42.75 of 95), written to catch
exactly this shape - a driver that cannot see the charges it is driving over.
It measured 34 the first time this guard was written; it now measures **0**.
Deterministic both ways: re-running the post-correction build twice gives the
same 0.

## Why this is physics, not a bug in the port

The three algorithm differences `pair.rs` picked up (edge axes never choose
the contact normal, each hull's own "up" axis needs to beat half the reigning
best depth to win, the contact point is the plain midpoint of the two bodies'
positions rather than a support point) were all confirmed at instruction
level against `Collision_BoxAgainstBox`'s own disassembly, not just its
decompiler summary - see `pair.rs`'s own doc comment on `overlap`. In
particular the "half the best depth" bias, the one most likely to reroute a
craft's push-apart direction, is a real `mul.s` against a real `0.5f`
register the function already uses for its half-extent computation, present
at exactly two of the six face-axis comparisons and nowhere else - not a
decompiler artifact. So the shift in trajectories this test is catching is
the simulation getting *closer* to the original's actual contact response,
not further from it. A butterfly effect over 3,600 ticks of AI driving,
weapon RNG and mine placement turning a small per-contact difference into one
craft parked on a mine cluster is exactly the kind of chaos this class of
simulation has; the same could equally have gone the other way and produced a
*milder* worst-case by chance.

## Why `crates/physics` should not fix this

The test's own doc comment names two possible causes for a singled-out
craft: a driver that cannot see charges it drives over (`Driver::avoidance`),
or a craft eating repeated Plasma hits. Both live in `crates/ai` or the
weapon-targeting logic in `crates/game`, not in `crates/physics`. Weakening
the test's `0.45` floor to tolerate this would defeat the guard's whole
purpose - it exists specifically to catch a driver blind to its own mines,
and a genuine instance of that blindness is what 11 charges laid and a worst
of 0 looks like.

## Open

- Confirm which of the two named causes (mine-blindness vs. repeated Plasma
  hits) explains the single craft's collapse to 0 - the charge count (11,
  roughly double the passing run's 6) points at more mines being laid and
  hit, which favours the avoidance-gap reading over repeated Plasma, per the
  test's own comment ("a low count points at the second").
- `Driver::avoidance`'s actual gate against a laid mine/bomb, if one exists at
  all, is unread against this scenario - this thread does not itself trace it.

## Next Steps

- `crates/ai`'s owner: reproduce with
  `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all -E 'test(a_field_racing_with_real_pads_does_not_mine_itself_to_death)' --no-capture`
  on this branch (after `crates/physics/src/pair.rs`'s fix has landed) and
  read `Driver::avoidance`'s treatment of a laid charge - whether it reacts to
  one at all, and if so, at what range and against what pool of known hazards.
- If the gap is real, the fix belongs in `oag_ai`, not in loosening this
  test's floor.
