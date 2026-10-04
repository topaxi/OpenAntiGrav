# ADR-0056: A render-side port of the original's particle generator

## Status

Accepted. Builds on [ADR-0002](0002-determinism-model.md) (determinism). Does
not change `oag_core::Rng`.

## Context

Pulse ships two random generators
([prng.md](../../ghidra/functions/psp-pulse-usa/prng.md)): libc `rand()` for
gameplay, clock-seeded, and a 17-word RANROT-B (`PsysRng_Next`) for particles,
clouds and a few weapon rolls. `prng.md`'s "Consequences" section said a port
of the second one would need an ADR, because it puts a second generator in
the tree beside `oag_core::Rng`.

The cloud field is what needs it. Each `cloudGroup` reseeds the particle
generator from its `Seed` and draws every sprite record from it: position,
half-size, the `Overlap` cull's input, atlas cell, phase and rate
([clouds.md](../../ghidra/functions/psp-pulse-usa/clouds.md)). Fed the seed a
group kept, a port reproduces the original's field exactly. That was checked
on 2026-10-04 against four RAM dumps from four boots and both `05_Track`
layouts, every record, cull and baked colour. With `oag_core::Rng` the field
would only have the right distribution, so no capture of ours could be
checked against a capture of the original sprite by sprite.

## Decision

- Port `PsysRng_Reseed`, `PsysRng_Next`, `Psys_RandFloatRange` and
  `Psys_RandIntRange` as `oag_render::ranrot::Ranrot`, read off the
  disassembly. That includes the signed-then-corrected `u32` to `f32`
  conversion, which makes the range closed at `1.0`.
- **It lives in `oag-render` and only there.** Rendering is exempt from the
  determinism scan, and nothing it produces reaches a state hash. The
  simulation keeps `oag_core::Rng` for all of its randomness.
- A render-side effect that the original builds from a seed uses `Ranrot` and
  the seed. Every shipped `cloudGroup` leaves `Seed` unset, and the original
  then rolls one from the clock. Where that happens we use a fixed seed,
  labelled **chosen, not measured** (`oag_render::cloud::CHOSEN_SEEDS`).
- Particle effects (`oag_render::psys`) may move onto `Ranrot` later, but that
  is their own decision. This ADR does not require it.

## Consequences

- A capture of ours and the original can match sprite for sprite, given the
  boot's seeds. A test can pin the build against values read live
  (`crates/render/tests/cloud_field_ground_truth.rs`).
- There are two generators in the tree. A contributor reaching for "the RNG"
  in render code now has to choose, and must not use `Ranrot` from simulation
  code, where nothing enforces the boundary except review and this ADR.
  `just check-deps` does not catch it, because the simulation crates cannot
  depend on `oag-render` anyway.
- The original shares one generator between clouds and particles: a cloud
  build reseeds the stream every emitter draws from. Each `Ranrot` here is a
  separate value, so the particle stream after a track load is **not**
  reproduced. Matching it would mean sharing one generator across both
  modules. That is not done, and it is recorded here as a known difference.
- The port's arithmetic is plain `f32` in Rust, not the VFPU. A numpy
  `float32` reference fed the RAM's own matrices agreed with the original's
  records to within 1.2e-4 world units. The Rust port builds from our own
  parse of the track, and its ground-truth test pins positions to 0.01 units,
  the counts, cull, cells and colours exactly. Neither is a bit-exact claim
  for positions.
