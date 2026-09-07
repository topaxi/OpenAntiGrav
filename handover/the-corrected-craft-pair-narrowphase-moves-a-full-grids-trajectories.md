# The corrected craft-pair narrowphase moves a full grid's trajectories enough to fail a race-length guard

2026-09-07. `oag_physics::pair::overlap` was corrected against a live,
instruction-level read of `Collision_BoxAgainstBox` (`0x0881702c`) - see
`handover/craft-to-craft-collision-is-implemented-the-stun.md` and
`docs/ghidra/functions/psp-pulse-usa/contact-response.md` for the physics
side. This row is the one downstream consequence that lands outside
`crates/physics`: `crates/game/tests/opponent_weapons_ground_truth.rs`'s
`a_field_racing_with_real_pads_does_not_mine_itself_to_death` now fails.

**Correction, same day**: this row originally named `crates/ai`'s
`Driver::avoidance` and mines as the cause. That was wrong, checked and
retracted below - the destroyed craft never takes a mine hit. Read this
version, not that title.

## What actually happens, instrumented directly

A temporary `OAG_PROBE=1` env-gated `eprintln!` was added at the three places
energy actually changes hands - `oag_physics::damage::apply_contact` (wall
scrape, called from `crate::step`), `oag_physics::damage::apply_weapon`
(`crate::projectile::blast`), and `oag_physics::pair::resolve` itself (for the
contact event count) - plus a temporary sibling test that runs the identical
scenario and prints each opponent's final shield by slot. All of it was
reverted before this thread was written; `git diff` against this commit is
empty. Findings, from that instrumentation, same seed, same course:

- **Craft-craft contact never charges the shield pool at all, in either
  build.** `crates/game/src/race/field.rs::resolve_craft_pairs` calls
  `oag_physics::pair::resolve` and discards its `Option<PairContact>` return.
  `oag_physics::damage::apply_contact` - the only thing that drains the pool
  for a contact - is fed exclusively by `WallResponse`, the one-body
  wall-scrape path (`crates/physics/src/integrate.rs`). So "shield charged per
  pair-contact event" is exactly zero, before and after this change, by
  construction. **This is why the coordinator's question 1 has a third answer
  neither "more contacts" nor "harsher contacts" - contacts don't charge
  anything.**
- **The destroyed craft (slot 2 of the eight) does not touch a mine.** Its
  full run of damage events: a Bomb blast for 15, a Missile blast for 15, and
  a Plasma blast for **60** (`depleted=true` on that exact call - the kill
  shot). Wall-scrape damage over the whole 3,600-tick race totalled **8.36**,
  spread over 30 ticks - nowhere near lethal on its own, and mines/bombs
  contributed **zero** direct hits to this craft despite 11 being laid
  somewhere on the circuit. **Question 2's hypothesis is falsified**: this is
  not a mine-avoidance gap. It is exactly the *other* cause the test's own
  doc comment names - "a craft eating repeated Plasma hits (one is worth more
  than half a pool)" - `60` of `95` is that in one hit.
- **Craft-craft contact *event counts* do move a lot (84 in the old build,
  673 in the new one, same seed) but this is not a clean "more contacts"
  signal either.** The narrowphase's overlap decision (whether a pair is
  touching at all, the same fifteen-axis reject test) is unchanged by the
  fix - only *which axis becomes the normal* on an already-detected overlap
  changed. So the very first contact after the two builds' first pair touch
  is already resolved differently (a different push direction and/or lever
  arm), and by design this is a chaotic 8-craft/3,600-tick system: **from
  that first divergence on, the two runs are different trajectories, not the
  same race measured twice.** The raw event-count difference is a symptom of
  that divergence, not evidence the new narrowphase is over-eager.

## Question 3: neither ported difference alone reproduces it - both do

`pair::overlap`'s three ported differences (edge axes excluded from
best-normal selection, each hull's own "up" axis needing half the reigning
best depth to win, the contact point as the plain midpoint) were isolated one
at a time by reverting exactly one while keeping the other two, on this exact
scenario:

| Active differences | Result |
| --- | --- |
| None (fully pre-fix) | passes, worst 78 of 95 |
| Edge-exclusion + midpoint (bias reverted) | **fails, byte-identical to all three** (worst 0, mean 70, 11 charges) |
| Bias + midpoint (edge-exclusion reverted) | passes, worst 47 of 95 |
| Edge-exclusion + bias (midpoint reverted) | passes, worst 63 of 95 |
| All three (the actual fix) | fails, worst 0 of 95 |

Reverting the bias alone reproduces the failure to the exact float - meaning
the "up axis needs half the depth" rule **never once fires differently** in
this specific race's trajectory (it is inert here, not implicated). Reverting
either the edge-exclusion or the midpoint alone is enough to avoid the
failure; reverting both is needed to avoid it while keeping the other. So it
takes **edge-exclusion and the midpoint contact point together** - neither
alone is sufficient, and the bias is confirmed uninvolved in this scenario.

## Reading this straight

This is a butterfly-effect consequence of a physics correction that is
verified correct at instruction level (see the parent thread), not a
demonstrated bug in `oag_physics` and not a demonstrated gap in
`crates/ai`. The mechanism is: a more accurate contact normal/point changes
where two craft end up after their first touch, which cascades through 3,600
ticks of AI driving and weapon RNG into a different craft standing in a
different Plasma blast's radius than it would have under the old,
less-accurate contact response. Nothing here shows `Driver::avoidance` (or
any other `crates/ai` logic) behaving differently or worse than before - no
AI decision was traced as wrong, only that the physics moved enough for one
already-possible outcome (a lethal Plasma hit) to land in this seeded run
where it previously didn't.

## Open

- Whether this specific seed's Plasma kill reflects anything systematic (an
  opponent now firing Plasma from a position/range it previously couldn't
  reach) or is simply which side of a chaotic threshold this one seed landed
  on. Not distinguished here - would need sweeping several seeds/tracks under
  both builds and comparing the *distribution* of worst-case shields, not one
  race.
- Whether `crates/game/tests/opponent_weapons_ground_truth.rs`'s floor
  (`worst > full * 0.45`) is measuring a real regression risk at all once the
  underlying system is this sensitive to which contact-normal axis wins a tie
  - a question for whoever owns that test's intent, not answered here.

## Next Steps

- `crates/ai`'s owner, if routed here: there is no `Driver::avoidance` gap to
  chase - the destroyed craft never touched a mine. If anything is worth a
  look, it is whether opponent Plasma targeting/firing range should be
  re-examined against the corrected physics, not mine avoidance.
- Whoever owns `opponent_weapons_ground_truth.rs`: decide whether a
  single-seed worst-case floor is the right shape for a metric this sensitive
  to contact-normal tie-breaking, given the table above.
