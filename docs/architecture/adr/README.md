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
