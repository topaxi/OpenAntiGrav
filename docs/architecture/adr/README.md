# Architecture decision records

An ADR records a decision that was hard to make and would be expensive to
revisit: the context at the time, what was chosen, what was rejected, and what
it costs.

ADRs are **immutable**. When a decision changes, write a new ADR that supersedes
the old one, and mark the old one superseded. The point is that a reader can
reconstruct why the codebase is the way it is, including the parts that turned
out to be wrong.

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-rust-workspace-and-wgpu.md) | Rust workspace and wgpu | Accepted |
| [0002](0002-determinism-model.md) | Determinism model | Accepted |
| [0003](0003-no-ecs.md) | Plain data-oriented world, no ECS | Accepted |
| [0004](0004-asset-pipeline.md) | Load from originals, normalise per platform | Accepted |
| [0005](0005-ghidra-conventions.md) | Ghidra naming and documentation conventions | Accepted |
| [0006](0006-no-copyrighted-content.md) | No copyrighted content in the repository | Accepted |
| [0007](0007-fixed-timestep-vs-original.md) | Keep a fixed timestep, and diverge from the original | Accepted |
| [0008](0008-av1-movie-cache.md) | Cache movies as lossless AV1, and decode them in process | Accepted |
| [0009](0009-multi-game-fanout.md) | Fan out to other games by verified layer, not by date | Accepted |
| [0010](0010-movie-decode-thread.md) | Decode movie frames on a worker thread, one per movie | Accepted |
| [0011](0011-authored-pvs-before-frustum-culling.md) | Cull with the track's authored PVS, before the view frustum | Accepted |
| [0012](0012-wgsl-upscalers-not-native-fidelityfx.md) | Port the FSR upscalers to WGSL rather than driving the native SDK | Accepted |
| [0013](0013-anti-aliasing-architecture.md) | Separate anti-aliasing by class, and gate spatial passes against the upscaler by render scale | Accepted |

## Format

```markdown
# ADR-NNNN: Title

## Status
Accepted | Superseded by ADR-MMMM

## Context
What was true when this was decided.

## Decision
What was chosen.

## Alternatives considered
What was rejected, and why.

## Consequences
What this costs, including the bad parts.
```

The "Consequences" section must be honest about downsides. An ADR that only
lists benefits is advocacy, not a record.
