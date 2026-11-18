# oag-core

Deterministic `f32` math, the fixed-timestep `TickClock`, the seeded `Rng` and
state hashing. Replays, golden tests and the cross-machine hash checks all rest on
it, so its rules are strict: no `mul_add`, no SIMD, no wall clock.

It has no workspace dependency and nearly everything depends on it. The rules are in
[determinism](../../docs/architecture/determinism.md).
