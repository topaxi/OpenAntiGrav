# ADR-0045: FSR 3.1 is timed in two halves, and only one of them is fixed

## Status

Accepted. Extends [ADR-0042](0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md)
rather than superseding it - the budget formula,
`period - fixed - RESIDUAL_SHARE * period`, is unchanged, and so is the
`scalable`/`fixed` boundary it draws. This implements the first alternative
that ADR-0042 listed and declined, and that
[ADR-0043](0043-hd-bloom-joins-the-scalable-budget.md) deferred a second time.

## Context

**Reported from play on a Steam Deck, 2026-09-04**, with dynamic resolution
on: the `dev` overlay's `FSR3` row read 4-5 ms, in the same ballpark as the
motion-blur chain beside it. That is the first reading of the chain on the
first-tier target [goals.md](../../overview/goals.md) names, and it is a large
share of a 60 Hz frame on the machine the controller exists for.

Two questions came out of it, and one instrument answers both.

**Which half of the chain is expensive?** The port's two documented
deviations from upstream - no FP16 path, and intermediates widened to the
baseline-storable formats (`docs/rendering/fsr3.md`) - do not weigh on the
same passes. Six of the eight dispatches run at the render extent or half of
it; `accumulate` and `rcas` run at presentation resolution. Narrowing the
formats mostly helps the first group, and there was no reading that could say
which group to spend the effort on.

**And what does the resolution controller believe?** `drs::Cost::fixed` held
the whole chain's reading, and `Cost`'s own doc said why: with one timestamp
pair there was nothing finer to count, so counting all eight dispatches as
fixed made the budget smaller than the truth and biased the controller toward
falling - the safe direction for the bug ADR-0042 exists to fix. On the Deck,
where the chain is 4-5 ms of a 16.7 ms frame, that pessimism is no longer
small: the controller was subtracting the render-resolution half of a cost it
*can* lower from the budget it lowers it with.

ADR-0042 declined the split because it needed "a second timestamp pair inside
`Fsr3::render`, a fourth `PassTimer` ring, and a `fsr3::Dispatch`-level split
between 'renders at extent' and 'renders at output' that does not exist yet".
The third of those turned out to be already written: `Fsr3::render` carries a
comment marking the exact dispatch where the resolution changes.

## Decision

**The chain is encoded as two compute passes, split where the resolution
changes, and each carries its own timestamp pair.** `fsr3 scaled` holds the
input clear, `prepare_inputs`, the luma pyramid, the shading-change pyramid
and its resolve, `prepare_reactivity` and `luma_instability` - every dispatch
whose extent is the render extent or half of it. `fsr3 presented` holds
`accumulate` and `rcas`. `fsr3::ChainTimestamps` carries the two pairs, in
the shape `motion_blur::ChainTimestamps` and `hd_bloom::ChainTimestamps`
already established.

Two passes rather than eight deliberately. `TIMESTAMP_QUERY_INSIDE_PASSES`
would be needed to bracket a single dispatch and is not WebGPU-portable, so a
per-dispatch breakdown means a pass per dispatch - eight claims a frame,
eight chances to reintroduce the unresolved-slot trap the FSR 3.1 port's own
review already found once, and eight pass boundaries perturbing what they
measure. Two is the split both readers actually want.

**`drs::Cost::scalable` gains the scaled half; `fixed` keeps only the
presented one.** The render-resolution dispatches shrink with the extent
exactly as the scene pass, the motion-blur chain and the HD/Fury bloom chain
do, so they belong on that side of the boundary ADR-0042 drew. Nothing about
the formula or `RESIDUAL_SHARE` changes: this moves a measured term across a
boundary, it does not fold anything into a constant or take anything out of
one.

**Both rings are claimed, abandoned and resolved together, and `Fsr3::render`
returns whether it encoded them.** `Session::feed_drs` pairs readings by
frame index and treats a missing one as "not measured yet", which stalls the
controller for the run - so two rings that can disagree by a frame would be a
worse bug than the pessimism this ADR removes. The chain encodes both passes
or neither (its one early return, a missing allocation, sits above both), and
`Framebuffer::resolve_scene` returns one `bool` for the pair. That return
value replaces a flag that was set *before* `Fsr3::render` ran and so claimed
a frame as measured that the missing-allocation path never wrote.

## Alternatives considered

**Leave the split alone and spend the effort on the port's two deviations
directly** - the FP16 path and the narrower storage formats. Rejected as an
ordering, not as work: both are large, they help different halves of the
chain, and without a reading that says which half dominates, picking one is a
coin flip. This ADR is the instrument that makes that choice evidence-led.

**Eight pairs, one per dispatch.** Rejected above: not portable without
`TIMESTAMP_QUERY_INSIDE_PASSES`, eight claims a frame against a ring of four
slots, and a per-dispatch breakdown nobody has a decision waiting on. Two
halves is what both the levers and the controller need.

**Keep `fixed` as the whole chain and subtract the scaled half in
`feed_drs`.** Arithmetically the same and structurally worse: two readings
taken by two rings can name different frames, and a subtraction of one from
the other can go negative in a way neither reading is wrong about. Each ring
reports what it measured and the buckets are assembled from raw readings.

**Report one summed `FSR3` row on the overlay and keep the split internal.**
Rejected because the split's first reader is a person deciding which lever to
pull, and a sum tells them nothing. The row became two: `FSR3 REN` and
`FSR3 OUT`.

## Consequences

**The controller gets a bigger budget on any machine running FSR 3.1**, and
so is more willing to hold or raise the render scale. That is more accurate
and less conservative, and the deliberate thumb ADR-0042 put on this scale is
gone: what bounds the error now is `RESIDUAL_SHARE` and the learned reserve
of [ADR-0044](0044-the-residual-is-a-learned-upper-bound.md), not an
over-count. On a machine where the presented half alone fills the frame, the
controller will now correctly report the target as out of reach rather than
merely falling to the floor.

**A fifth `PassTimer` ring, and a fifth thing that can go out of step.** Four
slots and eight tiny buffers, matching the four already there - the cost is
not the memory but the invariant, which now spans five sites across two
crates: claim, encode, abandon and resolve in `Session::render`, and the
early return in `Fsr3::render`.

**Nothing in the test suite reaches that invariant, and it broke once during
this ADR's own implementation.** A scripted edit put the new ring's `resolve`
at the first `upscale_timer` match in `frame.rs` - the *claim* site, not the
resolve site. It compiled, the full `just` gate passed, and every frame after
the fourth would have been silently unmeasured, because a claimed slot that is
never resolved never comes back. A second hole of the same family survived to
review: with two rings, claiming and writing stopped being the same condition,
so a ring that got a slot while its partner's four were in flight would have
resolved a pair no pass wrote - an unspecified value read as a plausible
number. The response is structural rather than a comment: `claim_upscale`,
`abandon_upscale` and `resolve_upscale` each take *both* rings, the first
rolls a lone claim back, and there is no site left where one ring can be named
without the other. `both_halves_of_the_chain_read_back_naming_their_own_frame`
guards the renderer's half; the frame loop's half is guarded by those three
functions and by nothing else.

**One extra compute-pass boundary per frame, unmeasured.** The chain was one
compute pass and is now two, so the sum of the two readings is not exactly
what the single pair reported. It is a pass begin/end against eight
dispatches and is expected to be small, but nobody has measured it, and
`FSR3` on the overlay has quietly changed meaning from "the chain" to "the
half of it that scales" - a reader comparing a number written down before
this ADR against one read after it is comparing two different things.

**Neither of the two levers the reading was taken for is closed.** The
instrument now exists; the FP16 path and the narrowed storage formats are
still unstarted, and the Steam Deck reading that will decide between them
still has to be taken by somebody playing it.
