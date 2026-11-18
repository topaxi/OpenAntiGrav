# oag-ai

Opponent behaviour: one driver per craft, following an authored racing line and
emitting the same ship controls a human's buttons map to. It never reaches the
world, the rules or the renderer, so it tests against a synthetic circle.

Simulation code under the [determinism rules](../../docs/architecture/determinism.md).
Depends on `oag-core`, `oag-physics` and `oag-tables`.
