# ADR-0043: `hd_bloom` joins the scalable budget

## Status

Accepted. Extends [ADR-0042](0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md)
rather than superseding it - the budget formula it introduced,
`period - fixed - RESIDUAL_SHARE * period`, is unchanged; this only moves a
term across the boundary that formula already draws, and revisits the one
alternative ADR-0042 declined for time reasons.

## Context

**Reported from play, 2026-09-03, after ADR-0042 landed**: on the same
HD/Fury profile, `GPU SCENE` and `FSR3` both read sub-3 ms, yet the frame rate
sat at 60-90 FPS against a 120 target. ADR-0042's own overlay row could not
say why - it showed two of the four passes this build now times, and neither
was large enough to explain the gap on its own.

Two things followed from that report, in order:

**The overlay was missing a timer it already had.** `Session::blur_cost` had
existed since ADR-0042 and fed `drs::Cost::scalable`, but `perf::GpuCost`
never grew a field for it, so `GPU SCENE`/`FSR3` was reporting two of three
measured passes and a player reading it could not see where the third was
going. Fixed first, independently of this ADR: `GpuCost` gained `blur`, and a
new `GpuCost::residual_ms` computes `Stats::mean_ms` (the real wall-clock
frame time) minus whatever the GPU row adds up to, printed as a fourth row,
`OTHER`. That is the number this ADR is actually about - it was always real,
and now it is visible.

**`hd_bloom` was the alternative ADR-0042 explicitly declined, for a reason
that stopped applying once the decision was revisited under this report.**
That ADR's Alternatives section gave two reasons: the chain's ladder length
is a runtime decision, so timing it needs the same first/last split motion
blur got, and the correctness of doing it depended on the `--race` profile
bug being fixed first - which ADR-0042 had just done. Neither reason was an
argument that `hd_bloom` was cheap; both were about sequencing. With the
sequencing done and the `OTHER` row now showing a real, unexplained residual,
`hd_bloom` was the next term to measure rather than a closed question.

## Decision

**`hd_bloom` is now a fourth timed pass**, on equal footing with the scene
pass and the motion-blur chain: `oag_post::hd_bloom::Chain::run`
draws through the render extent, exactly as the other two do, so its cost
belongs in `drs::Cost::scalable`.

**Bracketed with the same mechanism ADR-0042 built for motion blur, and it
needed no `abandon` path of its own.** `hd_bloom::ChainTimestamps` mirrors
`motion_blur::ChainTimestamps` - an opening write on the chain's first render
pass (the first downsample) and a closing write on its last (the encode),
covering eight-to-eleven render passes with one claimed `PassTimer` slot via
`PassTimer::half_writes`. The one simplification: `Chain::run` has no early
return - the downsample ladder always has at least two entries by
construction and the encode always runs - so once a caller commits to calling
`run` at all, the claim is always written into. The only way a claim goes
unwritten is a scene with no `Chain` in the first place (Pulse, Pure, or an
HD circuit with no `HDR and Bloom` block), which is a load-time fact rather
than a per-frame one, so `SceneStats::hd_bloom_encoded` and the `abandon`
call answer it the same way `blur_encoded` already does.

**Fed into `drs::Cost` with the same absent-vs-zero distinction ADR-0042 drew
for the FSR 3.1 chain**, using the scene's own fact rather than a settings
row: `Scene::has_hd_bloom` (`self.hd.is_some()`) is what tells "the ring was
full this frame" from "there is no chain to have measured", the way
`render_profile.reconstruction.is_temporal()` does for `fixed`. There is no
settings-row equivalent to ask, because whether a `Chain` exists is a
per-*circuit* decision baked in at `Scene::new`, not a player choice - a
player is never offered `hd_bloom` as its own toggle, only `bloom` (the PSP
path) as a checkbox and the HD chain as whatever the circuit authors.

**`RESIDUAL_SHARE` moved from 0.15 to 0.20**, and the measurement behind that
number is worth stating plainly because it also answers the report only
partway. On the original solo-ship calibration circuit, `hd_bloom` cost about
0.36 ms and held flat across all four render-profile rows ADR-0042 measured -
which is exactly why it hid inside a residual that itself held flat at
1.45 ms. Moving `hd_bloom` out did not, on that circuit, explain the report's
gap by itself. What did move the number: `--mode single_race` - the only mode
every real race runs, and the one the report's own race was almost certainly
in - fields seven AI opponents that ADR-0042's `--race --autopilot` (its
default mode, `time_trial`) never draws, and physics, AI decisions and extra
draw calls for seven more craft are none of them GPU-timed. Measured on the
same circuit with a full grid: roughly 2.4 ms of a ~8.7 ms frame was still
unaccounted for after `hd_bloom` was subtracted out. `0.20` covers that
measurement; it is one circuit's number under one grid size, not a settled
constant - see the Consequences section and the handover thread it links.

## Alternatives considered

**Leave `hd_bloom` in the residual and raise `RESIDUAL_SHARE` to cover a
heavier ladder.** This is what ADR-0042 provisionally did, flagging it as a
known gap rather than a decision. Rejected now for the reason ADR-0042 itself
gave: a circuit-authored cost folded into a constant is invisible to a
controller that is supposed to react to it, and - unlike the CPU-side
residual this ADR's own measurement turned up - `hd_bloom`'s cost is fully
GPU-timeable with a mechanism this codebase already had, once motion blur
proved it out. There is no equivalent shortcut for AI/physics cost, which is
why that part stays a constant for now rather than a fifth ring.

**Split the FSR 3.1 chain's render-resolution and presentation-resolution
passes**, the alternative ADR-0042 also deferred. Still deferred, for the
same reason: no report has pointed at it the way this one pointed at
`hd_bloom`, and it needs a second timestamp pair inside `Fsr3::render` rather
than the reusable `ChainTimestamps` shape this ADR spent.

**Time the CPU-side per-frame cost directly**, rather than folding the AI/
physics finding into `RESIDUAL_SHARE`. The raw material exists - `perf::Meter`
already measures the real wall-clock frame time `GpuCost::residual_ms`
subtracts from - but turning that into a *budget input* rather than a
diagnostic needs deciding what a "fixed CPU floor" means for a controller
that only ever changes GPU-side render extent, and that is a design question
this ADR's evidence motivates but does not answer. Recorded as the next term
in the handover thread this ADR's own commit updates.

## Consequences

**The `dev` overlay's `GPU` row gained a `BLOOM` field and a new `OTHER`
row**, printed as `GPU SCENE x.xx MS  BLOOM x.xx MS  BLUR x.xx MS  FSR3 x.xx MS`
followed by `OTHER x.x MS` - the frame-time line's `mean_ms` minus whatever
the four fields above sum to, whichever of them have reported. A player
reading it now sees the same subtraction this ADR's own investigation had to
do by hand.

**`RESIDUAL_SHARE`'s new value is still one circuit's number**, now under two
different grid sizes rather than one, and still not checked against a
circuit with a heavier `hd_bloom` ladder, a second title, or a second
adapter - the same caveat ADR-0042 recorded, narrowed rather than closed. The
handover thread this ADR's commit updates carries the open half forward:
whether the CPU-side AI/physics cost for a full grid should become its own
measured term rather than living inside a constant, the way `hd_bloom` just
stopped doing.

**A player whose real gap is neither `hd_bloom` nor a full grid's CPU cost
will still see a nonzero `OTHER` row and not know which of the remaining
untimed passes it is.** That is the honest state of the instrument, not a
regression - before this ADR the `OTHER` row did not exist at all, and a
player had no way to know there was anything left to ask about.
