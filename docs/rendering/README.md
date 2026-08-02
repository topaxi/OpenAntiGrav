# Rendering

> **The MVP is done (M4); fidelity is its own milestone (M6).** The wgpu device,
> the mesh pipeline, the track ribbon and art meshes, the collision view, the
> three cameras, the **ship exhaust** and the **HUD** exist. What is absent is
> almost all of what the `.vex` classes author: animation, lighting, the
> environment classes, particles beyond the exhaust, and the series' look. See
> [`roadmap.md` M6](../overview/roadmap.md#m6---rendering-fidelity) for the list,
> which is derived from [the class table](../formats/vex.md) rather than invented,
> and [`architecture/workspace-layout.md`](../architecture/workspace-layout.md)
> for the crate's seams.

## Scope

- wgpu device and surface setup - **done**
- Track geometry: the driveable ribbon **and** the textured art meshes both
  render. The art meshes are behind `--art` rather than being the default, for a
  reason that is [about verification, not rendering](../overview/roadmap.md#m5---full-race)
- Ship rendering, including team liveries - **done** for the eight teams whose
  `Ship.vex` resolves by name
- Scenery **animation**, authored as `Anim Transform` `0x3c0` and
  `animationTrigger` `0x3dc` - nothing reads either yet. Distinct from
  **texture** animation, which is **done** for eight trackside textures: a
  V-axis scroll over a banded texture, the ship blink lights' mechanism
  generalised. See [`vex.md`](../formats/vex.md), "Tracks animate too"
- Lighting and shadow: five authored light classes plus
  `Dynamic Shadow Occluder` and `lensflare` - none implemented. The prelit path
  exists (`GpuVertex.lit`), so this is about which surfaces are which
- Environment: `Skycube` (**done** - the payload is a `Mesh` payload, recovered in
  [`skycube.md`](../formats/skycube.md) and drawn camera-centred and out of depth
  by `oag_render::mesh::build_sky`). `fogCube` is **done** too - the runtime is
  recovered in [`fog.md`](../ghidra/functions/psp-pulse/fog.md) and applied by
  `oag_formats::fog` plus `mesh.wgsl`'s own bind group. The cloud and sea classes
  are not implemented
- Particle effects: thrust (**done** - the `Engine Flare` class, recovered in
  [`exhaust.md`](../ghidra/functions/psp-pulse/exhaust.md) and implemented in
  `oag_render::exhaust`) and collision sparks (**done**, `oag_render::sparks` -
  currently **authored, not recovered**: the actual spark-spawn trigger,
  `ShipCollisionFx_Trigger`, has since been found and read in full, naming
  the three real spark resources and a usable severity formula - see
  [contact-response.md](../ghidra/functions/psp-pulse/contact-response.md#shipcollisionfx_trigger-0x089246b4-is-the-actual-spark-spawn-function)
  and the open thread on `HANDOVER.md` for whether `oag_render::sparks` gets
  retuned against it), then the general `ParticleSystem`
  `0x3c4` and weapon effects. The weapon effects are M5's, being
  gameplay-coupled
- The HUD - **done** for the 2D layer, off the disc's own layouts; see
  [the HUD](../ui/hud.md). Its `<Mode3D>` layer is not
- Post-processing, and the series' distinctive look
- Modern display features: ultrawide, HDR, VRR, dynamic resolution, and
  FSR-class upscaling - see [modern features](../overview/modern-features.md)
  for the licensing picture and the pipeline prerequisites (motion vectors,
  depth, camera jitter) to design in from the start

## The art is raster, and that has a consequence

Every shipped 2D asset is paletted raster authored for 480x272 - the HUD atlas at
8 bpp, the fonts at 4 bpp - so a modern window magnifies all of it. Meanwhile 26
`Data\HUD\*.vex` files are *polygonal geometry* named after HUD elements and
referenced by nothing. Redrawing our own art as vector is a live option and the
reasoning, including why our own art is committable where a transcription of the
disc's is not, is on
[the HUD page](../ui/hud.md#the-shipped-art-is-raster-and-the-disc-suggests-it-was-not-always).

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
