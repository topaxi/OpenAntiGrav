# oag-physics

Ship dynamics and collision queries: the part of the simulation that must be
bit-reproducible. `f32` only, no `mul_add`, no SIMD, no wall clock.

Its only workspace dependency is `oag-core`, so a change to an on-disc schema
cannot reach the force law. Model and evidence: [physics](../../docs/physics/README.md).
