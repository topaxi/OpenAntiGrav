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
  `Ship.vex` resolves by name. HD's hull materials resolve a shipped shader
  variant disc-wide, and what each variant computes - plus a live
  `ADD_SECOND` misclassification it exposed - is in
  [hd-ship-materials.md](hd-ship-materials.md). HD's two unlit weapon
  programs (the LeachBall and the Plasma bolt head) are read and have their
  own path, the LeachBall drawn and the Plasma head too since 2026-10-06 (it
  waited on the eye its drawable was never given, not on a cull) - see
  [hd-unlit-programs.md](hd-unlit-programs.md)
- Scenery **animation** - **done**, both mechanisms. `Anim Transform` `0x3c0`
  moves the geometry: 393 nodes over the twelve circuits with 474 meshes below
  them, decoded from the class's own binder and evaluators
  ([`anim-transform.md`](../ghidra/functions/psp-pulse-usa/anim-transform.md))
  and replayed per frame through a node-matrix table the vertex shader indexes.
  Reading it also **fixed a placement defect**: the class used to contribute the
  identity, which dropped its transform along with its animation and left 245 of
  those meshes at the world origin, 38 while still scrolling their texture
  correctly. **Texture** animation is the other mechanism and is also done -
  every one of the 922 materials the engine's `& 0x10` gate marks reaches the
  shader, replaying the per-material keyframe block rather than a chosen rate.
  `animationTrigger` `0x3dc` is authored zero times across the 44 `.vex` files
  checked. Counts and evidence in
  [`scenery-animation.md`](scenery-animation.md); the texture mechanism is in
  [`vex.md`](../formats/vex.md), "The texture-transform keyframe block". The
  clearest thing either mechanism is used for is
  [the start gantry](start-gantry.md): its `3`, `2`, `1` and `GO` are four UV
  cells of one mesh, and the material's own offset track walks a palette
  staircase to light one at a time. Recovered off the disc, **not drawn** - slot
  8's transform is still unrecovered, so the model stays unplaced.
  **Wipeout 2048 animates through neither**: its `track.vex` authors no `Anim
  Transform`, and the scenery hangs from a node table in `track.rcsmodel`
  driven by a `.rcsskeleton` and `.rcsanimclip` beside it - read 2026-09-16,
  played through the same node-matrix table (`mesh::Motion` is the enum that
  lets one table carry both authored forms), and checked against HD's own
  `Anim Transform`s on the twelve circuits both titles ship. See
  [`2048-animation.md`](../formats/2048-animation.md)
- **Pulse's frame against the original's, matched and ranked** (2026-09-30): sky,
  fog and tone agree within 5 %; scenery textures now use the disc's mip levels picked by the GE slope law, as the
  original's do. See [`frame-audit.md`](frame-audit.md)
- **Blended glow batches stamp the mask** (2026-10-01): the arch lights, the start-line laser
  and the tunnel's rim light are blended batches with the glow bits, which the original's stencil
  stamps and ours did not, so they had no bloom; now drawn through a second alpha-only pass and
  measured against the original's own EDRAM mask - see [`glow-mask.md`](glow-mask.md)
- **Pulse PS2 does bloom** (2026-10-02): five passes per field read off a GS dump, 7-tap blur at
  320x224, half-strength add; the mask is unmeasured - see [`ps2-bloom.md`](ps2-bloom.md)
- **Pulse's glow mask and hull lights, measured live** (2026-09-23): the
  bloom runs every race frame, the mask is the GE stencil, and the hull is
  lit by the circuit's own `AmbientLight`/`DirectionalLight` nodes - see
  [`glow-mask.md`](glow-mask.md) and
  [`scene-light.md`](../ghidra/functions/psp-pulse-usa/scene-light.md)
- **A struck craft's hull sparks, measured per locator against the original**
  (2026-09-30): three causes named and two fixed, the gap closed from 1.3-2.6x
  to within 10 % - see [`hull-sparks.md`](hull-sparks.md)
- Lighting and shadow: five authored light classes plus
  `Dynamic Shadow Occluder` and `lensflare` - none implemented. The prelit path
  exists (`GpuVertex.lit`), so this is about which surfaces are which.
  **Shadow has a design and no implementation** - four techniques behind one
  setting, per title rather than one quality dial, in
  [`shadows.md`](shadows.md), which also records what each disc authors:
  Pulse's 129 `Dynamic Shadow Occluder` payloads (layout closed at
  `0x50 + 32n + 16m` on all 129), HD's four shadow jobs and nine decoded
  per-team `ambient_shadow.gtf`, Pure's nothing at all, and the three classes
  - `shadow` `0x3cb`, `blob` `0x3e0`, `textureBlob` `0x3df` - authored zero
  times and therefore inert
- SPU vertex lights (Wipeout HD) - **built for one producer**, 2026-09-20.
  HD sums a per-frame point-light list into every track chunk's vertex
  colours on the SPU and adds the decoded term to the pre-albedo diffuse
  sum; the formula, the RGBE packing and the `SVC1` combine are read off
  the SPU job and the shipped microcode
  ([`renderer.md`](../ghidra/functions/ps3-hdfury-eu/renderer.md),
  "`EdgeGeom`'s light path is read" and "The `SVC1` combine is read"), and
  `mesh.wgsl` runs the same sum per vertex from a 128-record `SpuLights`
  block in the `Scene` uniform. The one producer wired is the one the live
  capture showed filling the buffer, `EngineFlare_SubmitSpuLight`: one
  light per craft, `EngineLightData.xml`'s `Distance` behind the flare node
  along its Z axis, range `Radius +- 0.1`, `(40, 10, 4)` on a Fury skin or
  `(4, 10, 40)` otherwise, times `1 + 10 * boost_blend` - see
  `oag_raceplay::engine_light` and
  [`hd-status.md`](../formats/hd-status.md), "Authored data that is plain
  text". **What it lights is the craft's own engine housing, not the floor**:
  every one of the original's 40 captured records sits 3.0-4.4 units (a ride
  height) from the nearest track surface while `D` is 0.6-2.1, and ours sit
  2.9-4.5 on the same circuit (`crates/game/examples/hd_engine_light_reach_probe.rs`),
  so the list is bound to the hulls as well as the track - **chosen, not
  measured**: 74 ship materials compile `SVC1` twins and nothing else is in
  range, but the runtime `SVC1` bit on a hull chunk is unobserved. Boosted,
  the `1 + 10 * blend` gain washes the whole rear of the hull warm
  (`talons-t487.png` against `talons-t470.png` under
  `data/scratch/hd-engine-light/`) - the disc's numbers, not tuned. The other 23 producers `renderer.md` catalogued
  (pickups, weapons, the Zone ship) and the flare's blue-to-orange transition
  branch stay unwired until their triggers are read; `"Lighting.Enable spu
  vertex lights"` in a circuit's `.envsettings` (default on, `0` on
  `02_track` and `12_sol_2`) is honoured.
- Environment: `Skycube` (**done** - the payload is a `Mesh` payload, recovered in
  [`skycube.md`](../formats/skycube.md) and drawn camera-centred and out of depth
  by `oag_mesh::mesh::build_sky`). `fogCube` is **done** too - the runtime is
  recovered in [`fog.md`](../ghidra/functions/psp-pulse-usa/fog.md) and applied by
  `oag_vex::fog` plus `mesh.wgsl`'s own bind group. `cloudCube`/`cloudGroup` are
  **partly done** - `oag_fx::cloud` draws every sprite `oag_vex::cloud`
  decodes off `05_Track` (the only Pulse circuit that authors any), textured
  with the shared `Wipeout_Clouds_D_128x64x4.mip` and blended with the
  measured GE state (`CloudGroup_ApplyDrawState`). Three things the original
  does are read but not reproduced, each recorded rather than guessed at: the
  per-sprite rotation is drawn from the same random range the original draws
  from rather than the original's own per-boot values (there is no fixed rate
  to read - see [`clouds.md`](../ghidra/functions/psp-pulse-usa/clouds.md)),
  the camera-heading counter-rotation is omitted, and each sprite draws one
  chosen flat colour rather than the position-sampled `Lo`/`Mid`/`Hi` ramp
  whose input variable was not traced. `sea`,
  `seareflect` and `seaweed` author zero instances on either PSP Pulse
  pressing, so there is nothing to implement for them on this title; unchecked
  on Pure and PS2
- Particle effects: thrust (**done** - the `Engine Flare` class plus its
  `Trail` ribbon and the `<Team>boost.vex` plume a speed pad reveals, all
  recovered in [`exhaust.md`](../ghidra/functions/psp-pulse-usa/exhaust.md) and
  implemented in `oag_fx::exhaust` and `oag_raceplay::Loaded::boost_model`.
  The plume's mesh batches draw with `exhaust::TRAIL_BLEND`, not the flare's
  own `exhaust::BLEND` - they take the ordinary mesh draw path's pure-additive
  blend branch, the same equation the ribbon already uses, not the flare's
  alpha-weighted one; see
  [`mesh-draw.md`](../ghidra/functions/psp-pulse-usa/mesh-draw.md)). **The
  ribbon's numbers are not asset-derivable and that was measured rather than
  assumed** - a `Trail` node's payload is 64 bytes of mount transform and
  nothing else, on all 78 of them across three discs, while HD/Fury drop the
  class entirely for a `/data/ribboneffects/` template family that already
  decodes; see [`trail-ribbon.md`](trail-ribbon.md))
  and collision sparks (**done** and **recovered**, no longer "authored":
  `oag_fx::sparks` reads `Data\Psys\WO_SHIP_COLL_SPARK_DAMAGE.POB` at the emitter level, all
  four emitters transcribed from the file's own bytes and corroborated live in
  PPSSPP during real wall hits, with `ShipCollisionFx_Trigger` supplying the
  severity formula. Three residual approximations remain, each labelled on its
  own constant - see
  [contact-response.md](../ghidra/functions/psp-pulse-usa/contact-response.md#shipcollisionfx_trigger-0x089246b4-is-the-actual-spark-spawn-function)
  and the open thread on `HANDOVER.md` for whether `oag_fx::sparks` gets
  retuned against it), then the general `ParticleSystem`
  `0x3c4` and weapon effects. The weapon effects are M5's, being
  gameplay-coupled
- Speed pads and weapon pads - **done**, and the answer on three of the four
  title/class pairs is that the engine leaves the artists' texture alone. Only
  Pulse's `Weapon Pad` is recoloured at runtime, off a recovered keyframe
  table; HD's armed/cooling state is the one open question. See
  [pads.md](pads.md), which also records the invented flat tint this replaced
  and how it hid itself
- HD/Fury's **Zone recolour** - **done** off the disc's own material variant
  and stage table, and since 2026-09-15 its stage change runs as the
  original's own **wavefront**: a sphere out of the player's craft, growing
  by a law read off the executable and reproduced live on RPCS3, the new
  stage's colours inside it and the old stage's outside. The radius's unit
  against this renderer's world is the one number not measured. See
  [hd-zone-recolour.md](hd-zone-recolour.md)
- The HUD - **done** for the 2D layer, off the disc's own layouts; see
  [the HUD](../ui/hud.md). Its `<Mode3D>` layer is not
- Post-processing, and the series' distinctive look. Bloom is **done** and
  **recovered**; FXAA, SMAA, MSAA 4x and FSR 1 are built as modern additions,
  and [FSR 3.1](fsr3.md) - the temporal one - is a port in flight, one of its
  eight passes built. That page is the port's spine and says which.
  [Motion blur](motion-blur.md) is **built at its designed per-object
  velocity tier** - every race draw writes measured screen motion, gathered
  by a McGuire-style reconstruction filter behind a live
  `[graphics] motion_blur` strength row
  ([ADR-0030](../architecture/adr/0030-velocity-buffer-motion-blur.md)) - an
  invented feature rather than a recovered one
- Modern display features: ultrawide, HDR, VRR, dynamic resolution, and
  FSR-class upscaling - see [modern features](../overview/modern-features.md)
  for the licensing picture and the pipeline prerequisites to design in from
  the start. Readable depth and per-draw motion vectors are both done, by
  [motion blur](motion-blur.md)'s two tiers, the UI-free scene artefact by
  [ADR-0036](../architecture/adr/0036-ui-composites-at-presentation-resolution.md)
  and [ADR-0038](../architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md),
  and sub-pixel camera jitter by
  [ADR-0039](../architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md).
  No renderer-side prerequisite is outstanding; the port itself is
- [Dynamic resolution](dynamic-resolution.md) has its **structural half built
  and nothing else**: the scene target is allocated at the `render_scale`
  ceiling and drawn into a sub-rectangle of it, per
  [ADR-0037](../architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md).
  There is no controller, no cost signal and no setting, so that sub-rectangle
  is the whole target on every frame the game draws
- [Screen filters](screen-filters.md) - **built**, 2026-09-16: one loadable
  WGSL pass over the finished frame, after the UI composites and before the
  grade, simulating the display a title was seen on. Five built-ins (the
  PSP-3000 panel, the 1000/2000 panel, and three CRTs including the PS2's
  interlaced television over composite), a `<config dir>/oag/shaders/`
  directory a player's own presets load from live, and a per-title
  `screen_filter` row on the GRAPHICS page. Not reverse engineering - nothing
  on a disc authors a display - and the page says where every number came
  from. [ADR-0053](../architecture/adr/0053-screen-filters-are-loadable-wgsl-after-the-composite.md)

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

## Measuring a renderer change

Moved here from `HANDOVER.md` on 2026-08-09, because each of these is a standing
property of how this renderer gets judged rather than a note about one pass.
`just compare-upscalers` cites this section.

**Before believing anything about one object's size on screen, measure the
background in the same frame.** Five passes over six days concluded our craft was
drawn `1.3x`-`1.74x` too large and hunted a second `0.75` in the executable to
explain it. The craft is correct to **0.15 %**; the whole frame was zoomed,
because the original widens its field of view with speed and we do not, and the
craft merely inherited it. (The widen is a store read out of the executable -
`0.075 * dot(fwd, vel)` additive degrees - which the pixels then *corroborate*
rather than supply; the pixel fit's job was to identify which of several
candidates was responsible, not to invent a constant.) The clause that hid this
was a single unchecked one -
"with the track and buildings behind aligning" - written into the measurement's
own record. Far scenery cannot be moved by any mesh scale, which makes it a free
control that decides between "our model is wrong" and "our camera is wrong" in
one extra measurement. The instrument, its validation and the recovered fov term
are on
[projection-vs-the-original.md](projection-vs-the-original.md).

## Draw order

**The original has one render queue, one 32-bit key and one sort, and mesh
geometry carries no depth in that key.** Which means the intuitive fix for a
renderer drawing in file order - a back-to-front sort over the transparent
batches - is *less* faithful than doing nothing, for track geometry. The key,
its layers, the four functions behind it and the four ways this crate still
diverges are on [draw-order.md](draw-order.md); `Model::sort_by_layer` is the
implementation, and it is one stable sort on one field.

**The recurring failure is not a wrong measurement - it is a control that was
available and never run.** Four instances in two days, each caught only after it
had misled someone: the background alignment that was *asserted* rather than
registered (six days of hunting a craft-scale error that did not exist); a
correlation sweep whose peak sat on the range's own edge and was written up as
`1.2500`; a registration box with HUD text inside it, which cannot scale and
pulled the answer to 1.01 from 1.14; and a band quoted to three digits when
sweeping it at two step sizes would have shown it moving 1.5 %. In every case the
check cost one extra command and the assertion cost days. **Before believing a
pixel measurement, name the control that could have refuted it - and if you
cannot name one, that is the finding.** There is a second question that pairs
with it, and it is not pixel-specific - the same failure has since happened with
a regression outvoting a disassembly. Both, and why the *direction* of the error
is what disguises it, are on
[methodology.md](../reverse-engineering/methodology.md#rules-learned-the-expensive-way).

**A brightness threshold cannot compare two renderers, and there is a metric that
can.** Every threshold, hue and saturation rule tried against the original failed,
each for its own reason, because our frame is ~2x darker with no bright pass and
the two renderers' bloom differs at low frequency. What works is **similarity
registration on high-passed gradient magnitude**: the same mesh with the same
texture carries the same internal pattern in both frames, and correlating that
pattern has no threshold in it anywhere. Validated to 0.26 % against known resize
factors, with the HUD - which no camera parameter can scale - as a free control
at 0.997.

**An aggregate statistic can rank a broken change as the better one.** The first
mesh-to-section rule placed a draw call by the single section holding its
bounding-sphere *centre*. It reported **fewer** draw calls surviving the cull -
which reads as better culling - and it deleted a ridge line and a building from
the frame, because geometry wider than a section has its centre in only one of
them. A screenshot comparison caught it and no counter would. `--screenshot`
honours both culling settings precisely so this comparison is runnable; two
captures at the same `--ticks` differing only by a setting must be
byte-identical. The same trap is why `just compare-upscalers` produces images
and deliberately no numbers: an aggregate ranks a sharpener below a blur every
time.

**Do not add per-frame work to save per-draw work without measuring both
sides.** Building the PVS visible set needs the craft and the camera located on
the spline, and `Spline::nearest` walks all ~3,400 samples - twice a frame would
have cost the same order as the frustum tests it saves. The craft's index is
already cached in `Ship::segment` by the simulation's own tick, and the camera
is found in a 96-sample window around it.

**Brightness thresholding measures the circuit, not the effect.** On Talon's
Junction's start straight the road's own light strips saturate a **245-pixel**
run in a band with no exhaust in it, against 108 px in the band containing the
ship, so any "widest saturated run" statistic is dominated by scenery - and it
reads as a clean number. Our own frames do not have this problem (a flat ribbon
over black), which makes the comparison asymmetric in exactly the direction that
flatters us. Capture on a dark part of the circuit, pick the frame by *both*
speed and mean frame luma, and prefer a metric that normalises by the pixel's
own luminance so a global brightness change cannot move it.

**An exhaust comparison has two independent inputs and both have to be pinned.**
Flare intensity is driven by time under thrust (`+0.25`/s to a ceiling of `1.0`)
and ribbon length by speed (`10 samples x speed`). Matching only speed produced
a pair 3x apart on brightness and an "our flare is too dim" reading; matching
only thrust time gives the opposite. `oag-game`'s `--pose-intensity`,
`--pose-speed` and `--pose-boost` exist so a posed frame can pin all of them.

**Never cite a screenshot without measuring it first.** `magick f.png -format
"%[fx:mean*255]" info:` costs nothing, and an all-black reference frame from an
off-canvas Xvfb capture sat in the repository as the only original-side
comparison for a week.

## Prerequisites

Requires [asset archaeology](../architecture/asset-pipeline.md) (M1). There is
nothing to render until the models and textures decode.
