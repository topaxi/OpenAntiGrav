# `RESIDUAL_SHARE` is one circuit, one adapter, and now points at AI/physics cost rather than `hd_bloom`

Landed 2026-09-03 as [ADR-0042](../docs/architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md):
the dynamic-resolution budget subtracts a measured `drs::Cost::fixed` (the
FSR 3.1 chain) instead of using a single constant share for everything the
scene pass wasn't. `hd_bloom` was left in the constant, `RESIDUAL_SHARE = 0.15`,
because timing it needed a first/last timestamp split like motion blur's and
the `--race` profile bug ADR-0042 fixed had to land first.

**Landed same day as [ADR-0043](../docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md):
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

## Open

**Whether AI/physics cost for a full grid should be its own measured term**,
the way `hd_bloom` just stopped being a residual guess. The raw material
exists - `perf::Meter` already measures the real wall-clock frame time
`GpuCost::residual_ms` subtracts from - but turning that into a *budget input*
needs deciding what a "CPU floor" means for a controller that only ever
changes GPU-side render extent, which no render-scale change can buy back.
Unlike `hd_bloom`, there is no existing timestamp mechanism to reuse here.

**Whether `0.20` covers the report that started this thread.** It was
measured on one circuit with a full grid, not on whatever circuit and grid
the original report was actually racing. The tools to check now exist and did
not before this landed: read the `dev` overlay's `BLOOM` and `OTHER` rows
during a real race. A large `BLOOM` means a heavier ladder than the
calibration circuit's; a large `OTHER` with `BLOOM` small points at CPU cost
(AI/physics/HUD/composite/blit) or an adapter difference, not at `hd_bloom`.

Still unmeasured, carried over from before this landed: a circuit with a
longer `hd_bloom` ladder than `talons_junction`'s `blur steps 1/1` - now that
`hd_bloom` is timed this only affects whether `Cost::scalable` correctly grows
with it, which is expected to just work rather than needing a new
measurement, but nobody has confirmed that. And a second adapter -
`docs/rendering/dynamic-resolution.md` already recorded a 52x difference in
timestamp period between two adapters on one laptop, a reason to distrust any
single-machine constant on principle.

## Next Steps

1. Ask for (or take) a `dev`-overlay reading during the race that motivated
   this thread, now that `BLOOM` and `OTHER` are on screen: `FPS`/`MS`,
   `SCENE`, `BLOOM`, `BLUR`, `FSR3`, `OTHER`. That one reading tells which of
   the two open questions above is the real one.
2. If `OTHER` is still large with `BLOOM` small, decide the CPU-floor design
   question above before building it - this is a real design decision, not a
   measurement.
3. Repeat the four-row `docs/rendering/dynamic-resolution.md` "What is timed"
   measurement on a second adapter if one is available, on the real display -
   Xvfb cannot size a GPU pass, confirmed independently twice now.
