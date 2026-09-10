# Craft-to-craft collision feel: sticking is a driver bug, near-90° spins are the recovered physics working

2026-09-07. A maintainer playing the game reported craft-to-craft collision
feeling "almost magnetic" and asked whether rear-hit steering is invented.
`oag_physics::pair`'s narrowphase and response are verified against
`Collision_BoxAgainstBox`/`Body_ResolveContactPair` (see
`handover/gameplay/craft-to-craft-collision-is-implemented-the-stun.md` and
`docs/ghidra/functions/psp-pulse-usa/contact-response.md`) - this thread is
what turned out to be wrong *around* that verified physics, found by
instrumenting a real 8-craft race and reverting the instrumentation
afterwards.

## Two separate mechanisms, not one

### Sticking - fixed this pass, in `crates/ai`

`oag_ai::Driver::social`, the only opponent-vs-opponent lateral yield/cover
term besides `ram` (which only ever targets the player), read
`ctx.field.behind` only. `crates/game/src/race.rs`'s `field_for` classifies a
rival as `Field::alongside` - a channel `social` never read - the moment it
is within `ALONGSIDE_GAP` (16 track-distance units) and `ALONGSIDE_WIDTH` (12
lateral), both comfortably larger than the actual hull-touch range
(`oag_physics::pair::overlap`, around 3-6 units on a realistic hull). So a
rival close enough to be physically touching had *already left* the one
bucket the term read, and the yield/cover lean dropped to exactly zero at
the moment contact started - confirmed directly: `social` returned the same
value for a touching `alongside` rival as for `Field::EMPTY`.

**Fixed**: `social` now reads `ctx.field.alongside.or(ctx.field.behind)`,
preferring the alongside rival when both are present. No new constant -
`field_for` builds a `Rival` identically for every channel, so nothing about
`gap`/`offset`/`closing` needed to change to accept the new source. See
`oag_ai::Driver::social`'s own doc comment for the full reasoning, including
why `ALONGSIDE_GAP` (16) sitting a little past `SOCIAL_MIN_GAP` (14) does not
reopen a blocking-into-contact risk.

**Measured, same seed, same `VENOM` single race, 7,200 ticks:**

| | before | after |
| --- | --- | --- |
| longest overlap streak | 90 ticks (pair 4/7) | 46 ticks (pair 4/7) |
| other streaks over 10 ticks | 73, 68, 57, 49 | 37, 32, 16, 11 |

Roughly halved, not eliminated. The residual cause is understood and is a
separate, deliberate design property, not a bug in this fix:
`personality.defence - personality.courtesy` gates the whole lean, and a
**neutral** personality (both zero) returns zero regardless of which channel
is read - it never yields or covers, alongside or not. Two near-neutral
personalities can still sit in continuous light contact. The acceptance
test (`crates/game/tests/craft_sticking_ground_truth.rs`) documents this and
sets its bound as a regression tripwire (60 ticks - above the measured
post-fix worst, below the pre-fix pathology), not a claim that 46 is the
right number.

`crates/ai/tests/determinism.rs`'s `REFERENCE` moved on both `Field` rows
(600 and 1,800 ticks) and is regenerated with the change documented inline;
`Solo` reproduced bit for bit, as expected (a lone craft never populates
`alongside`/`behind`).

### Near-90° spins - not a bug, not touched

The maintainer separately described occasional near-90° spins on a rear hit,
worse than "the originals". Measured directly (synthetic, real recovered
inertia `oag_physics::forces::ship_inertia()` = `(15.6, 21.6, 15.6)`):

- A perfectly centred rear hit (contact point on both centrelines, `r`
  parallel to `n`): yaw rate **exactly `0.00000` rad/s**.
- A near-maximal single-hit offset (49% of the scaled half-width): **~21°/s**.
- A 3-second synthetic "nothing yields" simulation (small constant re-contact,
  no lateral correction): cumulative yaw stayed bounded around **~5°** -
  the resolver is self-limiting on repeated mild contact, it does not run
  away.

But two of 1,381 logged real contacts in the same 8-craft race - both the
**first-ever** contact for that pair, both `Racing` state, not a
respawn/teleport - had `vn_before` of **-146** and **-157** units/s (both
craft doing 190-217 units/s, a fast near-head-on encounter) and produced yaw
rates of **-267°/s** and **-276°/s** for that one tick. That is the recovered
`j = -(1+e)*vn/D` formula doing exactly what it should at a genuinely large
closing speed and a genuinely fast first contact - not a sign error, not a
bad inertia, not tuned, and **not changed this pass** per the coordinator's
explicit instruction not to touch the physics.

## Resolved 2026-09-10: the original never spins a craft on a craft-to-craft hit

The Ghidra/PPSSPP measurement this section used to ask for was run, and the
answer is **no** - the original produces no yaw at all at any `vn`, and the
mechanism is one register. `Body_ResolveContactPair` passes each body's **own
position** (`body+0x30`) as the "point" argument of both
`Body_ApplyImpulseAtPoint` calls, so the applier's lever arm is exactly zero
and its angular half never fires. The lever arm to the contact point is used
only in the *denominator*. Read at instruction level, corroborated in the PS2
twin, and caught live in PPSSPP on six real contacts - including a staged
rear-end hit at 99 units/s closing (`vn = -78.5`, `j = 43.17`) whose struck
craft's angular velocity was bit-identical across the call and peaked at
6.4 deg/s over the next 30 ticks, all of it steering. Full evidence, the
instruction listing and the per-contact table:
`docs/ghidra/functions/psp-pulse-usa/contact-response.md`, section "Live,
2026-09-10". Recipe: `scripts/psp-pair-capture.py`.

`oag_physics::pair::respond` now applies the impulse through
`Body::apply_impulse` alone. The `-267`/`-276` deg/s first contacts above are
gone by construction. **This is a fidelity fix, not a tuning**: the maintainer's
"too much spin" report from play was right, and it was right about all three
originals because the lineage shares this resolver.

Two smaller things the same capture settled, both now in `pair.rs` and pinned
by a test that reproduces the original's `j` from the raw captured inputs to
five figures:

- the point velocity the gate and `vn` are built from is `v + cross(r, R^T
  omega)` - lever arm on the *left* of the cross product, `omega` rotated
  through the body's basis rows. **Correct as arithmetic on the original's own
  bytes, and the wrong thing to write against this crate's state**: `+0x150` is
  negated body-local, not world-space, so `R^T` is an unrotation and the whole
  expression is the textbook `v + omega x r`. Measured 2026-09-10, see the
  section below and
  `handover/gameplay/pair-vn-uses-the-wrong-omega-convention.md`;
- the denominator applies the body-space diagonal to the world-space `r x n`
  without rotating it, the same quirk the one-body path was already read to
  have.

Measured on this tree, same weapons-live `SingleRace` as
`craft_sticking_ground_truth`: overlapped pair-ticks **1,558 -> 1,134**,
sustained (streak > 10) **1,032 -> 808**. Both moved the right way without
anything being tuned toward them - a craft that is not spun into its neighbour
by the first touch does not stay in contact as long. `race_ground_truth`'s
twelve-circuit guard is unchanged: all twelve clean, `01_Track` 1 respawn at
`[794]`, before and after. `crates/ai/tests/determinism.rs` did **not** move:
`oag-ai`'s harness has no pair resolver in it (the pair pass lives in
`oag_game::race::Field::resolve_craft_pairs`), so its `Field` rows never
touched this code.

**Closed 2026-09-10, and it inverted**: `Body_ResolveContact` - the *wall*
path - does build its point velocity by the same `cross(r, R^T omega)` idiom
(`0x0884ea58`/`0x0884ea9c`), confirmed by a full read of the function, **and
that is the same thing as the textbook `omega x r` `crates/physics/src/wall.rs`
already computed.** `+0x150` holds the rotation rate negated and in body
coordinates, fitted against the rotation the recorded basis performs at a 2 %
residual on four captures where a world-space reading gives 200 %
(`scripts/omega-column-reading-fit.py`), so `R^T` is the body-local to world
unrotation and `R^T(+0x150) == -omega`.

Implementing the literal reading against this crate's own `angular_velocity`
was tried first and both measurements rejected it: a median 8.2-11.0 units/s of
`vn` error on the recorded laps, and `race_ground_truth` dropping from twelve
clean laps to eleven with `07_Track` losing its clean lap. `wall.rs` is
unchanged; `wall::tests` now pins the yawing case, which no wall test did.
Evidence: `docs/ghidra/functions/psp-pulse-usa/contact-response.md`, section
"`+0x150` is negated body-local". The same read confirmed the one-body path
applies its impulse at the real contact point (`0x0884eea4: move a1,s1`) and
corrected `rigid-body.md`'s argument order for the function.

**What it left**: `pair.rs` has the defect `wall.rs` was thought to have -
`handover/gameplay/pair-vn-uses-the-wrong-omega-convention.md`.

## Next steps

- If the residual ~46-tick sticking (neutral-personality pairs) is worth
  closing further, it needs a different mechanism than this fix - some
  personality-independent collision-avoidance floor - which was deliberately
  left out of scope this pass (a bigger design decision than "read a channel
  this term already had access to").
- ~~The Ghidra/PPSSPP item above.~~ Done 2026-09-10. ~~The wall-path point
  velocity is the follow-up it left.~~ Also done 2026-09-10, with a negative
  result that reopened the *pair* path instead - see
  `handover/gameplay/pair-vn-uses-the-wrong-omega-convention.md`.

## 2026-09-09: a correctness fix reopened the sticking question

**The Cannon's base speed stopped being a guess, and the field got stickier.**
`oag_gameplay::projectile::cannon::BASE_SPEED_KMH` was a chosen `400.0`; it is
a measured `500.0` (confidence 90 - `func_0x00060af4` is three instructions
returning a flat `500.0f` and ignoring the class pointer it is handed, so the
"per-class" premise the constant's name carried was also wrong). See
[cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md).

`craft_sticking_ground_truth`'s scenario is a `SingleRace` with weapons live,
so this reaches it: faster rounds land more hits, more hits mean more slowdown,
and a slowed field bunches. The same tree scores **1,051** overlapped
pair-ticks with the old constant and **1,558** with the real one. Isolated by
reverting that one constant and nothing else, twice, both times returning
exactly `1_051` - and the pass's other half, the `WO_CANNON_SPARKS` wiring,
was confirmed to move the simulation not at all, which is what a render-only
effect should do.

**`MAX_OVERLAPPED_PAIR_TICKS` was re-baselined 1,300 -> 2,000 to keep `main`
green, and that is a genuinely weaker guard.** The old bound followed a rule -
sit midway between a fixed driver's 978 and the pathology's 1,756 - that no
longer applies, because **both of those figures predate this physics fix** and
the pathology has never been re-measured against a correct Cannon.

## Open

- **Does contact still feel magnetic under weapon fire? The sustained metric
  says look hard.** Total pair-ticks (1,558) sit well below the stale
  pathology's 1,756 - but the run also prints *sustained* pair-ticks, streaks
  longer than 10 ticks, and those are **1,032 against the fixed driver's 537
  and the pathology's 1,062**. That is 97 % of the pathology on the sub-metric
  that most directly describes "stuck together rather than brushing past",
  while the headline total still looks like a comfortable pass. A bound on the
  total alone would not have caught this, and did not: the re-baselined 2,000
  passes.
- This was a closed question and is not any more. 1,558 against a (stale) pathology of 1,756 is
  a much smaller margin than 1,051 was, and the test exists because a
  maintainer reported the feel from play - so the discriminating check is a
  play session with weapons live, not another statistic.
- **Re-measure the pathology regime against the corrected Cannon**, by
  reverting `oag_ai::Driver::social`'s `alongside`/`CONTACT_FLOOR` fix for one
  run. That restores the like-for-like pair the bound's original rule needed,
  and would say whether 2,000 is generous, tight, or meaningless.
- The residual neutral-personality sticking, above. The Ghidra/PPSSPP
  capture item is closed (2026-09-10 section), and so is the wall path's point
  velocity; what is open now is `pair.rs`'s own `vn` convention, in its own
  thread.
