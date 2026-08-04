# Project goals

## What this is

A clean-room reimplementation of Wipeout Pulse: a new engine, written in Rust,
that reproduces the original's behaviour faithfully enough to be
indistinguishable in play, and is architected so that the rest of the lineage
(Pure, HD/Fury, 2048, Omega) can be built on the same foundation.

The original executables are the **specification**. We study what they do,
document it, and then implement it cleanly.

## What this is not

**Not a decompilation.** We are not producing source that recompiles to matching
bytes. Where a faithful reimplementation and a faithful translation diverge, we
choose the reimplementation and document why.

**Not a port.** No PSP or PS2 internals are reproduced for their own sake. The
renderer targets modern GPUs and modern displays; it reproduces the *output*,
not the pipeline.

**Not a distribution channel.** No game content ships with this project. See
[legal](legal.md).

**Not Pure-and-earlier.** Anything before Wipeout Pure is out of scope. See
[scope](#scope) below.

## Success criteria

The project succeeds if all of these hold:

1. **Gameplay is indistinguishable.** Ship handling, weapon behaviour, AI
   decisions, race rules and timing match the original within the tolerances set
   out in the [verification protocol](../reverse-engineering/verification-protocol.md).
2. **The simulation is deterministic.** The same inputs produce bit-identical
   state on every platform we ship. See [determinism](../architecture/determinism.md).
3. **The knowledge outlives the code.** Someone can understand the original
   engine from this documentation alone.
4. **The second title is cheaper than the first.** The architecture, tooling and
   documentation should make bringing up Pure, and later 2048, substantially
   less work than starting again.

Criterion 4 is the one that distinguishes this from a one-off port, and it is
the reason the codebase is split into narrow crates and the reverse-engineering
notes are written for a stranger rather than for ourselves.

## Non-goals

- **Bit-exactness with the original's floating point.** The PSP's VFPU has
  non-IEEE reciprocal and reciprocal-square-root behaviour. Reproducing it
  exactly would mean a software VFPU, at a large cost in both performance and
  reverse-engineering effort, for a fidelity gain nobody can perceive. We target
  behavioural equivalence within documented tolerances instead. See
  [ADR-0002](../architecture/adr/0002-determinism-model.md).
- **Emulation.** We do not run the original code. If you want that, use PPSSPP,
  which is excellent.
- **Modding support as a launch feature.** The architecture should not preclude
  it, but nothing is designed for it yet.

## Scope

| Title | Platform | Status |
| --- | --- | --- |
| Wipeout Pulse | PSP (UCUS-98712 and friends) | Primary target |
| Wipeout Pulse | PS2 (SCES-54748) | Cross-validation, alternate assets |
| Wipeout Pure | PSP (UCUS-98612) | Format ancestor, cross-reference |
| Wipeout HD / Fury | PS3 | Later |
| Wipeout 2048 | Vita | Long-term goal |
| Omega Collection | PS4 | If legally and technically feasible |

**When the two Pulse builds genuinely diverge, the PSP build is authoritative
for gameplay and physics behaviour.** The PS2 port was generally received as
the lesser version of the game, and the PSP original is the feel this project
targets. The PS2 binary remains invaluable as a second-binary corroboration
leg for confidence scoring (see the
[confidence rubric](../reverse-engineering/confidence-rubric.md)) - but a real
behavioural divergence is implemented PSP-side and *documented* as a
divergence, never offered as an alternative or averaged away. Examples on
record: the vertical-damping half-contact handling
([craft-update.md](../ghidra/functions/ps2-pulse-eu/craft-update.md)), PS2's
whole-mask `Input_ConsumePress`, and the non-rotating WAD lookup cursor.

Explicitly **out of scope**: Wipeout (1995), 2097/XL, 64, 3, Fusion. They predate
Pure and share little with it; including them would widen the problem without
advancing the goal.

## Platforms

First: Linux, Windows, macOS, Steam Deck. Rendering via wgpu, with Vulkan as the
first backend.

Later: Android, WebAssembly, Switch and Vita homebrew. These are why the
gameplay crates are forbidden from depending on the renderer, the windowing
layer or the audio backend, and why the simulation is fixed-timestep from day
one.
