# Rendering

> **Started.** Milestone M4. The wgpu device, the mesh pipeline, the track ribbon,
> the collision view, the three cameras and the **ship exhaust** exist; see
> [`architecture/workspace-layout.md`](../architecture/workspace-layout.md) for the
> crate's seams and [`roadmap.md`](../overview/roadmap.md) for what is still open.

## Scope

- wgpu device and surface setup
- Track and scenery rendering
- Ship rendering, including team liveries
- Particle effects: thrust (**done** - the `Engine Flare` class, recovered in
  [`exhaust.md`](../ghidra/functions/psp-pulse/exhaust.md) and implemented in
  `oag_render::exhaust`), weapons, impacts
- The HUD
- Post-processing, and the series' distinctive look
- Modern display features: ultrawide, HDR, VRR, dynamic resolution, and
  FSR-class upscaling - see [modern features](../overview/modern-features.md)
  for the licensing picture and the pipeline prerequisites (motion vectors,
  depth, camera jitter) to design in from the start

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
