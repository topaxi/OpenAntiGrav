---
categories: [rendering, gameplay]
---

# `RESIDUAL_SHARE` is one circuit, one adapter, and now points at AI/physics cost rather than `hd_bloom`

Landed 2026-09-03 as [ADR-0042](../../docs/architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md):
the dynamic-resolution budget subtracts a measured `drs::Cost::fixed` (the
FSR 3.1 chain) instead of using a single constant share for everything the
scene pass wasn't. `hd_bloom` was left in the constant, `RESIDUAL_SHARE = 0.15`,
because timing it needed a first/last timestamp split like motion blur's and
the `--race` profile bug ADR-0042 fixed had to land first.

**Landed same day as [ADR-0043](../../docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md):
`hd_bloom` is timed now**, the same `ChainTimestamps`/`PassTimer::half_writes`
shape motion blur used, and its reading joins `drs::Cost::scalable`. The `dev`
overlay's `GPU` row gained a `BLOOM` field to show it, and a new `OTHER` row -
the wall-clock frame time minus whatever the row above accounts for - makes
the residual itself visible for the first time rather than something a reader
has to compute by hand.

## What moving `hd_bloom` out actually found

**Reported from play**: with `GPU SCENE` and `FSR3` both sub-3 ms, the frame
still ran 60-90 FPS against a 120 target - a gap neither of ADR-0042's two
timed passes could explain, which is what motivated timing `hd_bloom` in the
first place. Measured on the calibration circuit (`talons_junction`) once
`hd_bloom` had its own reading: **it costs about 0.36 ms there**, the disc's
cheapest bloom ladder (`blur steps 1/1`), and holds flat regardless of
`msaa`/`motion_blur` - consistent with why it hid inside a flat 1.45 ms
residual before it was timed. It does not, on that circuit, explain a gap of
several milliseconds on its own.

**What did move the number**: `--mode single_race`, the mode every real race
actually runs (ADR-0042's own measurements used the CLI default, `time_trial`,
which races solo). A fielded grid of seven AI opponents costs physics, AI
decisions and draw calls for seven more craft, none of it GPU-timed by
anything this module reads. Measured on the same circuit with a full grid:
roughly 2.4 ms of a ~8.7 ms frame was still unaccounted for after `hd_bloom`
was subtracted out - which is why `RESIDUAL_SHARE` moved to `0.20` rather than
shrinking now that `hd_bloom` left it.

## The second adapter found a worse bug than a miscalibrated constant

**Reported from play, on Steam Deck (AMD RADV/Mesa, a different Vulkan stack
from either machine `RESIDUAL_SHARE` was calibrated on): `target_fps = 90`,
`render_scale = 150`, `minimum_resolution = 50`, with `fsr3` the render scale
settled around 60-65 % and never moved again for the rest of the session -**
not oscillating, not climbing back even briefly. Forcing `reconstruction =
off` at a fixed `render_scale = 150` held a stable 90 FPS easily (with a ~1 s
dip to 80 on the very first second of a race, consistent with pipeline
warm-up and unrelated to this). That ruled out "genuinely, correctly over
budget everywhere" - the hardware can do it, the controller just was not
letting it.

**Root cause, found by static read (the Deck was not reachable for a live
trace) and fixed 2026-09-03: a claimed timestamp slot that a shader build
failure leaves unwritten was never given back, and the read-back side had no
way to tell "FSR 3.1 will never report again" from "hasn't reported yet".**
Two bugs, compounding:

- `upscale::Framebuffer::resolve_scene`'s FSR 3.1 branch returned early on a
  shader build failure - correctly falling back to bilinear for the picture -
  but the claimed `upscale_timestamp` was never written and nothing told the
  ring to reclaim it. `blur_timer` and `hd_bloom_timer` both already had this
  exact safety net (`PassTimer::abandon`, from ADR-0042 and ADR-0043); the
  original FSR 3.1 timing, older than both, never got it. Fixed:
  `resolve_scene` now returns whether it actually wrote the pair, and
  `frame.rs` abandons the claim when it did not - the same shape as the other
  two.
- Worse: `Session::read_timing_and_feed_drs` decided whether to *expect* an
  FSR 3.1 reading by asking `render_profile.reconstruction.is_temporal()` -
  the **setting**, not whether FSR 3.1 was actually running. A player's
  setting does not change when a shader fails to build on their adapter, so
  once that happened, every frame after read `Cost::fixed` as `None` forever
  (the "hasn't reported yet" case), `Controller::record` was never called
  again, and the render scale froze at whatever it happened to be - which
  fits "settled around 60-65 % and stayed" exactly: not the floor, just
  wherever the freeze caught it. Fixed: the same check `frame.rs`'s claim
  already used (`self.gpu.temporal && self.framebuffer.temporal_upscaler_viable()`)
  now gates the read too, so a permanently-failed build correctly reads as
  `Cost::fixed = 0.0` instead of `None` forever, and the controller keeps
  running instead of going silent.

**Not yet confirmed**: *why* FSR 3.1's shaders would fail to build on RADV
specifically - no error text was captured, since the Deck could not produce a
log for this thread. If the fix above stops the freeze but `reconstruction =
fsr3` still never actually upscales on Steam Deck (i.e. it degrades to
bilinear every frame, silently, the way ADR-0012's fallback ladder is
supposed to), that is a separate, real gap - a `warn!` fires once per attempt
today, but nothing surfaces "your build failed and you are running
bilinear" anywhere a player would see it.

## The constant stopped being the reserve, 2026-09-03

**Landed as [ADR-0044](../../docs/architecture/adr/0044-the-residual-is-a-learned-upper-bound.md):
the reserve is a `drs::Residual` learned per session**, starting at
`RESIDUAL_SHARE * period` and tightened by frames that can prove a smaller
number. Written against the same Steam Deck report as the freeze fix above and
landed the same day, from the *other* reading of it - the report as relayed
into that work said the scale "wobbles a little" around 60-65 %, which is a
live loop settling too low, where the section above reads it as frozen. **Both
fixes stand whichever reading is right**: a frozen controller and a
miscalibrated reserve are independent faults, and the constant was known to be
one circuit's number before either. The Deck reading in Open below is what
tells them apart, and it should now be taken with the freeze fix already in. What made it safe to feed the wall clock into a module ADR-0040 kept
one out of, in two lines:

- `reading = frame - timed = residual + sleep`, and sleep is never negative,
  so **every reading is an upper bound on the truth**.
- Under pacing, `reading <= reserve` is the same statement as
  `scalable >= budget` - the frame had no slack to sleep away. The gate is
  ADR-0040's own condition rearranged, not a threshold somebody picked.

A reading above the reserve is trusted only when the frame ran 5 % past the
target period, and the result is capped at `RESIDUAL_SHARE`, so no budget this
build produces is ever smaller than the one before the ADR
(`a_learned_residual_never_reserves_more_than_the_constant`).

The prompt that opened this work proposed gating at the deadband's lower edge
instead. **That one runs away**, and the algebra is in the ADR: at the settle
point the readings it admits are *above* the reserve, and its fixed point is
`E = period - fixed` - a budget of zero and a scale at the floor. Worth
knowing before anybody proposes it again.

## Open

**Nobody has run any of this on the Steam Deck.** No machine in the session
that wrote ADR-0044 has that hardware, so every number in it is a model of the
report (`drs::tests::machines`), labelled as one. The check is a race on the
Deck at `target_fps = 90` with `RUST_LOG=oag_game=trace`, reading the render
scale and the `residual x.xxx ms (learned|assumed)` field the trace line now
carries. Two things to look for: whether the scale settles higher than 60-65 %
at all, and whether the residual says `learned` - under vsync at exactly the
target rate it can only tighten while the controller is still falling, so it
may report a reserve barely under the constant even when it is working.

**Two competing explanations for the same report, neither ruled out.** The
first is the freeze two sections up, already fixed. The second: a settle at
60-65 % implies a scalable cost around 19 ms at full scale under the quadratic
model, which does not square with the same machine holding 90 FPS at full
scale unless a large part of the scene pass does not scale with pixels - and a
mis-scaled `TIMESTAMP_QUERY` period would produce the identical picture,
`docs/rendering/dynamic-resolution.md` recording a 52x difference in timestamp
period between two adapters on one laptop. ADR-0044 makes the budget right; it
does not prove the budget was the only thing wrong. The Deck reading above
tells them apart: if `SCENE` alone reads near the whole frame period while the
frame rate is fine, the reading is the suspect, not the reserve.

**A third explanation for the same freeze, found by static read on 2026-09-04
and fixed the same day, and it needs no build failure at all.** The four
`PassTimer` rings were each read one reading per frame, and the loop polls
them one after another - so a GPU completion landing between two of those
polls reached the later ring that frame and the earlier ring the next. From
then on the earlier ring carried a backlog of one and handed back the older
reading on every call, its reading was a frame behind the other three's on
every call, and only a frame with *no* arrival - which a vsync-paced loop at a
steady rate never has - could resync them. Every frame after that was a
mismatch, `Cost::fixed` read as "not measured yet" under `fsr3`, and the
controller went silent wherever the scale happened to be: the same picture as
the shader-build freeze, on a machine whose shaders built fine. With
`reconstruction = off` on a Pulse circuit the same skew instead dropped the
motion-blur term out of `scalable` - not a freeze, but a budget that read
cheaper than the frame was. `PassTimer::drain` and
`Session::read_timing_and_feed_drs` now take every ready reading a frame and
match by frame across the lot; `docs/rendering/dynamic-resolution.md`'s fourth
timer property is the record. The Deck reading below still tells the
remaining two apart, and should be taken with this fix in as well.

**Whether AI/physics cost for a full grid should be its own measured term**,
the way `hd_bloom` stopped being a residual guess. ADR-0044 is the inference
from the clock the loop already reads, not a measurement, and it says where it
stops: under vsync at the target rate the sleep destroys the evidence once the
controller is comfortable, and only a real CPU-side timer closes that. Still
no mechanism to reuse - `PassTimer` reaches GPU work only.

**Whether `0.20` is still the right *ceiling*.** It is no longer the reserve,
so being wrong high now costs only the first seconds of a session and the
vsync-at-target case; but it is still the number a display refreshing below
the target falls back to, and it is still one circuit's measurement.

Still unmeasured, carried over from before this landed: a circuit with a
longer `hd_bloom` ladder than `talons_junction`'s `blur steps 1/1` - now that
`hd_bloom` is timed this only affects whether `Cost::scalable` correctly grows
with it, which is expected to just work rather than needing a new
measurement, but nobody has confirmed that. And a second adapter -
`docs/rendering/dynamic-resolution.md` already recorded a 52x difference in
timestamp period between two adapters on one laptop, a reason to distrust any
single-machine constant on principle.

## The FSR 3.1 chain is two readings now, and it does run on the Deck

Landed 2026-09-04 as
[ADR-0045](../../docs/architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md):
the alternative ADR-0042 declined and ADR-0043 deferred a second time. The
chain is encoded as two compute passes split where the resolution changes, its
six render-resolution dispatches join `drs::Cost::scalable`, and only
`accumulate` and `rcas` stay in `fixed`. **The budget gets bigger on any
machine running FSR 3.1**, so the controller is more willing to hold or raise
the scale - the deliberate pessimism ADR-0042 recorded is gone, and what
bounds the error now is `RESIDUAL_SHARE` and ADR-0044's learned reserve.

**Two things this thread had open are answered by the reading that motivated
it.** Reported from play on a Steam Deck, 2026-09-04, dynamic resolution on:
`FSR3` read 4-5 ms and `BLUR` about the same.

- **FSR 3.1 builds and runs on RADV.** Next step 4 below asks why it fails to;
  a nonzero `FSR3` row is only ever claimed on a frame the chain actually
  encoded (`will_upscale_temporally` and `resolve_scene`'s own answer both
  gate it), so the premise is stale. What changed it is not established - the
  freeze fix and the reset-clear both landed in between.
- **The residual is not where the Deck's time is going.** 4-5 ms of a 16.7 ms
  frame in each of two measured chains is most of it.

The Deck reading is still missing the other rows - `SCENE`, `BLOOM`, `OTHER`,
the render scale and the `residual` field - so next step 1 stands as written,
with two rows where it says `FSR3`: `FSR3 REN` and `FSR3 OUT`.

## The CPU-side clock ADR-0044 said it needed now exists, display-only

Landed 2026-09-07, from a question about the `dev` overlay rather than from
this thread: `OTHER` reading 6.1 ms while every timed row read far lower, on a
run with `vsync = smooth` and `frame_limit = 120`. A `FRAME` row and three
indented under it now sit below it - `CPU`, `PRESENT` and `OUTSIDE`
(`perf::CpuCost`, fed by `Session::cpu_cost` and `Session::present_cost`) -
and they partition the wall-clock frame time exactly: the frame body, the two
swapchain calls inside it that may block, and everything outside it including
the limiter's own `WaitUntil`. They are indented under `FRAME` and not under
`OTHER` because that is the row they sum to; `OTHER` overlaps all three.

**This is the mechanism the Open section above says does not exist** ("only a
real CPU-side timer closes that. Still no mechanism to reuse - `PassTimer`
reaches GPU work only"). Two `Instant`s, no adapter feature, so it works on a
machine with no `TIMESTAMP_QUERY` at all.

**It is not wired into `drs` and that is deliberate, not an oversight.**
`residual::observe` gates on an inequality that treats sleep as an unmeasured
non-negative term; with `CpuCost::outside_ms` that term is measurable (an upper bound on it,
strictly: the gap also holds event-loop dispatch), and
subtracting it would change the gate ADR-0044 derived rather than tune it -
a new ADR's worth of decision, not an edit. `Cost` is also a GPU-cost split by
what the extent moves, which a wall-clock reading is not.

## Next Steps

1. Ask for (or take) a `dev`-overlay reading during the race that motivated
   this thread, now that `BLOOM` and `OTHER` are on screen: `FPS`/`MS`,
   `SCENE`, `BLOOM`, `BLUR`, `FSR3 REN`, `FSR3 OUT`, `OTHER` - plus the render
   scale and the `residual` field on the `dynamic resolution:` trace line. That one reading
   tells which of the open questions above is the real one, and whether
   ADR-0044 moved the Deck at all.
2. If `OTHER` is still large with `BLOOM` small, decide the CPU-floor design
   question above before building it - this is a real design decision, not a
   measurement. **Now readable rather than inferred**: ask for the `CPU`,
   `PRESENT` and `OUTSIDE` rows in the same reading as step 1, and a large
   `OTHER` sorts itself into idle, CPU-bound or untimed-GPU without any
   further instrumentation. The rows to ask for are `FRAME`, `CPU`,
   `PRESENT` and `OUTSIDE`.
3. Decide whether the vsync-at-target hole in ADR-0044 should be closed with
   the measured sleep term, in a new ADR. The evidence it says is destroyed -
   "under vsync at the target rate the sleep destroys the evidence once the
   controller is comfortable" - is now a number the loop has;
   `reading - sleep` is a tighter upper bound than `reading`, on every frame,
   with the same one-sided argument. What has to be settled first is whether
   the CPU mean and the GPU readings, which are one frame out of phase, are
   close enough to subtract per frame rather than per window.
4. ~~Repeat the four-row measurement on a second adapter.~~ **Done, on Steam
   Deck** - and it found the freeze bug above rather than a `RESIDUAL_SHARE`
   answer. Once the fix has had a real race on the Deck, this still wants
   redoing: does `reconstruction = fsr3` actually resolve there once it stops
   silently going bilinear, and if so, what does the render scale settle at
   with the freeze no longer masking the real number.
5. ~~Find out why FSR 3.1 fails to build on RADV at all.~~ **Stale**: it runs
   there, per the 2026-09-04 reading above. If the `warn!` line
   (`"the FSR 3.1 pipelines did not build (...); staying bilinear"`) ever
   appears on a Deck log again, that is a new question rather than this one.
