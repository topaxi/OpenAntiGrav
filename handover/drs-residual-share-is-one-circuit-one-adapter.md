# `RESIDUAL_SHARE` is one circuit, one adapter, and the disc's cheapest bloom ladder

Landed 2026-09-03 as [ADR-0042](../docs/architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md):
the dynamic-resolution budget now subtracts a measured `drs::Cost::fixed` (the
FSR 3.1 chain) instead of using a single constant share for everything the
scene pass wasn't. What is left as a constant, `drs::RESIDUAL_SHARE = 0.15`,
covers the HUD, the composite, the blit, the perf overlay, the MSAA resolve,
and `hd_bloom` (not timed).

## Open

`hd_bloom` draws through the render extent and belongs in `drs::Cost::scalable`,
not in the residual - but its ladder length is a runtime decision
(`docs/rendering/hd-bloom.md`), so timing it correctly needs the same
first/last split `crates/render/src/timing/timer.rs`'s `PassTimer::half_writes`
gave the motion-blur chain in this same change, on a chain whose length can be
zero, one, or several levels.

The `0.15` measurement itself was taken on `talons_junction`, which
`track.envsettings` authors as `blur steps 1/1` - the cheapest ladder on the
disc. A circuit with a longer one would cost more than the residual accounts
for, and the controller would under-shrink there: it would let the scene run
closer to its budget than it should, believing headroom that a heavier bloom
chain has already spent.

Also unmeasured: a second adapter, and a title other than Wipeout HD/Fury.
`docs/rendering/dynamic-resolution.md` already recorded a 52x difference in
timestamp period between two adapters on one laptop, which is a reason to
distrust any single-machine constant on principle, not just this one.

## Next Steps

1. Find or build a circuit whose `track.envsettings` authors a longer
   `hd_bloom` blur-steps ladder than `talons_junction`'s `1/1`, and repeat the
   four-row measurement `docs/rendering/dynamic-resolution.md`'s "What is
   timed" table records, on the real display (Xvfb cannot size a GPU pass -
   see the trap in the fixed handover thread this one replaces).
2. If the residual moves meaningfully, either raise `RESIDUAL_SHARE` with the
   new measurement recorded the way ADR-0042 recorded the first, or time
   `hd_bloom` directly with a `ChainTimestamps`-shaped pair the way
   `crates/render/src/post/motion_blur.rs` now has - the second is more work
   and the more correct answer, since a constant residual can't distinguish a
   circuit with a light ladder from one with a heavy one.
3. Repeat on a second adapter if one is available, to check whether
   `RESIDUAL_SHARE` is stable across the same variation the timestamp-period
   probe already found large.
