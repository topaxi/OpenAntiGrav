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

## Open - needs Ghidra/PPSSPP, another member holds both this pass

**Whether the original produces a comparable yaw rate at a comparable `vn`
(-146 to -157 units/s) on a genuine fast near-head-on craft-craft hit is
unread.** This port's own physics say it should, by the same recovered
formula, and there is no continuous collision detection in either build (a
fast enough closing speed is caught late and deep by construction in the
original's own discrete per-tick `Collision_BoxAgainstBox` too) - but that is
an inference, not a capture. Whoever gets Ghidra/PPSSPP next: stage two craft
for a fast, steep-angle, near-head-on hit (closing speed on the order of 150
units/s) and read the struck craft's angular velocity the tick contact
resolves. If the original shows a comparably large yaw, this closes clean. If
it does not, the divergence is somewhere this pass could not reach without
a live capture - possibly the original's queue/deferred-impulse path
(`Body_QueueDeferredImpulse`, PS2) doing something the PSP's direct
`Body_ResolveContactPair` does not, which `contact-response.md` has not yet
compared at a high `vn`.

## Next steps

- If the residual ~46-tick sticking (neutral-personality pairs) is worth
  closing further, it needs a different mechanism than this fix - some
  personality-independent collision-avoidance floor - which was deliberately
  left out of scope this pass (a bigger design decision than "read a channel
  this term already had access to").
- The Ghidra/PPSSPP item above.
