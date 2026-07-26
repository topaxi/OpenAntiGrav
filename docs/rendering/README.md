# Rendering

> **Not yet started.** Milestone M4.

## Scope

- wgpu device and surface setup
- Track and scenery rendering
- Ship rendering, including team liveries
- Particle effects: thrust, weapons, impacts
- The HUD
- Post-processing, and the series' distinctive look
- Modern display features: ultrawide, HDR, VRR, dynamic resolution

## Principle

**Reproduce the output, not the pipeline.** No PSP or PS2 rendering internals
are recreated for their own sake. Where a modern technique produces the same
visual result more cleanly, use the modern technique.

That said, some of the original's look comes from its limitations: dithering,
affine texture mapping artefacts, a specific fog curve. Where such an artefact
is part of the game's identity rather than an accident, it gets reproduced
deliberately and documented as a choice.

## Prerequisites

Requires [asset archaeology](../architecture/asset-pipeline.md) (M1). There is
nothing to render until the models and textures decode.
