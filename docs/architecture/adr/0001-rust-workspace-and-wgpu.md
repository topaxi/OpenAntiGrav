# ADR-0001: Rust workspace and wgpu

## Status

Accepted.

## Context

The project needs a language and a rendering stack for an engine that must be
deterministic, portable to at least four desktop platforms, eventually portable
to handhelds and the web, and maintainable by contributors who did not write it.

The renderer must reproduce the original's *visual output* on modern GPUs. It
explicitly does not reproduce the PSP's or PS2's rendering internals.

## Decision

**Rust**, edition 2024, as a Cargo workspace of narrow crates.

**wgpu** for rendering, with **winit** for windowing. Vulkan is the first
backend; Metal, DX12 and WebGPU come free from wgpu's abstraction.

Supporting choices:

- `glam` for math, built with `scalar-math`. See [ADR-0002](0002-determinism-model.md).
- `cargo-nextest` as the test runner, for per-test process isolation.
- `just` as the task runner.
- `unsafe_code = "deny"` at the workspace level. Individual crates may lift it
  with a documented justification; none currently do.

## Alternatives considered

**C++.** The obvious choice for a game engine, and the language the original was
written in, which makes transliteration easier. Rejected because transliteration
is a non-goal, and because determinism and memory safety are load-bearing here:
a desync caused by uninitialised memory is exactly the kind of bug that would
consume weeks.

**Zig.** Attractive for this problem, particularly its comptime and its C
interop. Rejected on ecosystem maturity: wgpu, and the crates needed for CHD and
audio, do not have equivalents.

**Bevy, as a whole engine.** Rejected. It brings an ECS whose scheduler is a
determinism hazard, an asset system built around its own formats, and a large
surface area we would spend more time working around than using. See
[ADR-0003](0003-no-ecs.md).

**Raw Vulkan via `ash`.** More control, and no abstraction to fight. Rejected
because it means writing and maintaining Metal and DX12 backends by hand, for a
game whose rendering needs are modest. wgpu's overhead is not the bottleneck for
a PSP-era renderer.

**OpenGL.** Simplest, and adequate for the visual target. Rejected as a
long-term dead end: deprecated on macOS, and the wrong foundation for the modern
display features (HDR, VRR) the project wants.

## Consequences

**Good.** One rendering backend to maintain across five platforms. Memory safety
removes a class of desync bug. The crate system enforces the architectural
boundaries in [workspace-layout](../workspace-layout.md) at compile time rather
than by convention. Cargo makes the reproducible-build story straightforward.

**Bad.** wgpu's API churns between releases and upgrades will cost real time.
Rust compile times are worse than C++ for a project this size, mitigated only
partly by the workspace split. The pool of contributors who know both Rust and
PSP reverse engineering is small. And wgpu occasionally lacks a feature exposed
by a native API, which will eventually force either a workaround or a backend
escape hatch.

**Accepted risk.** If wgpu turns out to be the wrong abstraction for something
the project needs, the renderer is one crate, and `oag-render` is behind the
dependency rule that nothing in gameplay may reach into it.
