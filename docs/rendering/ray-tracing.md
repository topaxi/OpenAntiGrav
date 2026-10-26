# Ray tracing: where a first spike fits

> **Feasibility and a ranked spike list, 2026-10-19 (`raytrace-spikes`, not merged to main).** Lane A (the structures,
> `--ray-debug`) and spike 1 (`graphics.shadows = traced`) are **built and measured**, see
> ["Built and measured"](#built-and-measured-lane-a-and-spike-1-2026-10-19) below; the rest is still design. Every cost
> figure outside that section is an **estimate**.

The question: where could ray or path tracing first appear in the renderer - craft lights, glass reflections of craft
and environment, weapons and explosions, shadows? The short answer: **an opt-in, native-only, per-effect tier whose
first landing is the acceleration structures plus a traced sun shadow on the titles that author a sun.** It is an
enhancement, not a recovery: no original draws any of it, so nothing here overrides authored data, and the `original`
value of each setting it touches stays the default.

## What is already written down

- [`shadows.md`](shadows.md#pushing-it-further-and-what-it-is-actually-blocked-on) names ray tracing as "a documented
  future tier with no enum variant", blocked on a light to trace against, on wgpu support, and on a reason. It calls
  ray queries "effectively Vulkan-only". **The second claim is out of date for wgpu 30.0.1** (support table below); the
  first and third still hold, and its closing suggestion (ray-queried contact shadows on top of `mapped`) is spike 3
  here.
- [`draw-order.md`](draw-order.md) lists planar reflection targets (`0x01500000` + n) the original has and ours does
  not build. [`hd-ship-materials.md`](hd-ship-materials.md) and `rcsmaterial.md` name the original's glass paths
  (facing ramp, behind-the-glass target, an undrawn dual-paraboloid reflection texture).
- [`high-resolution-cost.md`](high-resolution-cost.md) and [`hd-gpu-cost.md`](hd-gpu-cost.md) give the frame costs a
  ray pass would be measured against.

## Support

### What wgpu 30.0.1 says (a reading of its source)

`Features::EXPERIMENTAL_RAY_QUERY` (with `EXTENDED_ACCELERATION_STRUCTURE_VERTEX_FORMATS`, `ACCELERATION_STRUCTURE_BINDING_ARRAY`,
`EXPERIMENTAL_RAY_HIT_VERTEX_RETURN` and `EXPERIMENTAL_RAY_TRACING_PIPELINES` beside it) is set by:

| Backend | Gate in `wgpu-hal` 30.0.1 | Note |
| --- | --- | --- |
| Vulkan | `VK_KHR_acceleration_structure` + `VK_KHR_ray_query` + `VK_KHR_deferred_host_operations` + `VK_KHR_buffer_device_address` | `vulkan/adapter.rs`. The doc comment on the feature still says "Vulkan"; the code covers more. |
| DX12 | `RaytracingTier >= 1.1` and shader model 6.5 | `dx12/adapter.rs`. |
| Metal | `supportsRaytracing` and `supportsRaytracingFromRender`, macOS 15 / iOS 18 | `metal/adapter.rs`. |
| GL | never | limits report 0. |
| WebGPU | never | `docs/tools/web.md` already requires WebGPU and has no GL fallback; the browser path has no acceleration structure at all. |

Two facts that shape the work:

1. **The feature is "experimental".** Requesting any `EXPERIMENTAL_*` feature needs an `ExperimentalFeatures::enabled()`
   token, which is `unsafe`. The workspace has `unsafe_code = "deny"` (`Cargo.toml`). The first lane must scope one
   `#[allow(unsafe_code)]` to the device request, say why, and decide whether it needs an ADR. wgpu says the feature may
   have undefined behaviour "when used incorrectly" and expects API breaks.
2. **A device's features are fixed at creation.** The tier is chosen at launch (and applies on restart), exactly like
   the adapter choice, not toggled live in the pause menu.

Shaders use `enable wgpu_ray_query;` and `rayQuery*` functions. `oag-shader-check` validates every shader with `naga` at
build time; the ray shader module needs the ray-query capability turned on there, and the module must only ever be
created on a device that has the feature (a module with ray queries fails device validation elsewhere, the same failure
`Msaa` documents for an unsupported sample count).

### What this machine says (a measurement)

`cargo run -q -p oag-gpu --example ray_query_probe`, 2026-10-19, Mesa 26.2.4, wgpu 30.0.1. The probe creates no device
and so never takes the `unsafe` token.

| Adapter | Backend | Ray query | Pipelines | Vertex return | BLAS prims max | TLAS instances max |
| --- | --- | --- | --- | --- | --- | --- |
| RX 7800 XT (RADV NAVI32) | Vulkan | yes | yes | yes | 536,870,911 | 16,777,215 |
| Raphael iGPU, 2 CU RDNA2 (RADV) | Vulkan | **yes** | yes | yes | same | same |
| llvmpipe (software) | Vulkan | yes (emulated, useless for timing) | yes | yes | same | same |
| Raphael iGPU through radeonsi | GL | no | no | no | 0 | 0 |

The same iGPU is on and off depending on the backend. `oag-game` already prefers Vulkan; a GL fallback would lose it.

### Per target (what is measured, what is inferred)

| Target | Expected | Basis |
| --- | --- | --- |
| Maintainer's RADV workstation | **yes, measured** | probe above |
| Steam Deck (VANGOGH, RDNA2, RADV) | **expected yes, slow** | the Raphael iGPU is the same RDNA2 family and RADV exposes it by default there; the Deck's Mesa is SteamOS's, not 26.2, so run the probe on it (`deck.lock`, per the Deck rule). 8 CUs means 8 ray accelerators |
| NVIDIA | RTX 20 series and newer: expected yes. GTX 10/16: do not count on it | KHR ray query is gated on the driver exposing it; unverified here. The notebook's RTX PRO 2000 (see `hd-gpu-cost.md`) is the card to probe |
| Intel iGPU | Xe-HPG and Xe-LPG with RT units (Arc, Arrow Lake's Arc 140T): expected yes via anv. Gen 9 to Xe-LPD (UHD, Iris Xe): no | inferred from Mesa's hardware support; run the probe on the notebook |
| AMD iGPU | RDNA2 and newer (Rembrandt, Raphael, Deck): yes. Vega: no | measured for Raphael |
| Android, Galaxy S24 (Xclipse 940) | **unknown** | Vulkan 1.3 with a vendor driver; ray query is advertised for this part but nothing here has read it. Run the probe in the APK path before planning anything for it |
| Web / WebGPU | **none** | no ray tracing in the WebGPU spec |
| macOS | M-series with macOS 15+: expected yes | gate read in source, never run |

Design consequence: the tier is **offered only when the adapter reports the feature**, in the same place the
RENDERER row already lists adapters (`crates/game/src/adapter.rs`), and hidden or greyed with the reason otherwise. It
is never an error. It is never the default.

## What the renderer has today, and what the original drew

| Candidate | Ours today | The original |
| --- | --- | --- |
| Sun shadows | four-tier `graphics.shadows`: `off`, `blob`, `original`, `mapped` ([`shadows.md`](shadows.md)). `mapped` is one cascade, redrawing the whole circuit into a map (1 to 1.6 ms of iGPU time per view, [`split-screen.md`](split-screen.md)). HD has a sun in `.envsettings`, 2048 a `ShadowLight`; Pulse has no light and `mapped` uses `oag_pulse::shadow::AUTHORED_AXIS` | Pulse: authored occluder hulls. HD: baked lightmap alpha plus four shadow jobs. Pure: none |
| Engine and weapon light | HD's SPU vertex lights (`crates/mesh/src/mesh_render/spu_light.rs`): up to 128 candidates, `max(0, 1 - d/D)^w * N.L` per vertex, **no occlusion**. Filled by `oag_raceplay::engine_light` (one per craft, range 1 to 2 units) and `weapon_light` (missile, rocket, bomb) | the same formula, no occlusion either. Pulse PSP has no dynamic point lights |
| Glass reflections | canopy glass reads the facing ramp and an environment map; Vineta K's tunnel glass reads a 640x360 behind-the-glass target (`rcsmaterial.md`, "Vineta K's ceiling"). Planar reflection layers and the dual-paraboloid texture are **not drawn** | HD: those two, per material. Talon's Junction's glass floor is `etched_glass_tech` (facing ramp). No reflection of a craft in the floor is authored |
| Explosions | billboard and `.pob` particles (`oag-fx`), plus the weapon light above | the same; no light transport |

## Ranked spikes

Ranked by player-visible win per unit of work. "Work" counts lanes of about a day for one agent (estimate).

### Worth a lane now

**Lane A - the plumbing, as a visible debug picture (about 1 lane).** Request the features, build BLAS and TLAS, bind the
TLAS to the mesh pass, and draw a `--ray-debug` view (hit distance or a normal) so the structures are checkable by eye
and by `cmp`. No setting yet. This is the precondition of everything below and the only part that touches device
creation, buffers and the `unsafe` token, so it should land alone.

**1. Traced sun shadow, `graphics.shadows = traced` (about 1 to 2 lanes on top of A).** One shadow ray per lit pixel
toward the sun, in the mesh fragment shader, opaque geometry only.
- Where: `crates/render/src/shadow/` (the map path to sit beside), `mesh.wesl` where the map comparison is today.
- What it buys: no cascade fit, no resolution, no `MAPPED_AHEAD` gap, every surface casts and receives (ship on ship,
  scenery on ship), crisp contact. On a split it replaces a whole-circuit depth redraw per view with a per-pixel query
  per view.
- Titles: **HD and 2048** author a sun (`shadows.md`). **Omega** ships `.EnvSettings` read through 2048's reader and carries a sun (`omega-status.md`, checked, applies, not wired). **Pulse** takes `oag_pulse::shadow::AUTHORED_AXIS`, as `mapped` does, labelled as ours. **Pure** has no measured sun and no axis of its own: not checked whether `mapped` borrows Pulse's there, so the tier waits on that.
- Cost (estimate): 1 ray per pixel is 1 M rays at 1280x800, 2 M at 1080p, 8.3 M at 4K. On the 7800 XT well under
  1 ms at 4K; on the Deck's 8 RT units **2 to 4 ms at 800p** (about 1 M rays at 2 to 4 ns each, a guess at coherent shadow rays on 8 ray accelerators). `mapped`'s cost has been measured per view on a desktop iGPU only (1 to 1.6 ms, `split-screen.md`), never on the Deck, so no speed claim is made: the case for the tier is quality and split-screen scaling, and the Deck verdict needs `mapped` measured there too.
- Risk: alpha-tested geometry (cutout fences, the perforated barrier) casts a solid shadow unless the query loop
  fetches UVs and tests alpha (needs the hit's primitive index and an index fetch, or `RAY_HIT_VERTEX_RETURN`). First
  spike: opaque flag only, and name the gap, as `mapped` named its own. HD's baked lightmap shadows are art; the traced
  term must multiply with them, not replace them, or HD loses authored shading ([`shadows.md`](shadows.md#what-mapped-actually-changes-per-title)).
- Simulation: none. Render only.

**2. Glass reflections of craft and environment (about 2 to 3 lanes on top of A).** A reflection ray from each glass
pixel: HD Vineta K's tunnel glass, Talon's Junction's `etched_glass_tech` floor, and the canopy.
- What it buys: the most visible single effect of the list, because it is a thing the player looks at. It is also
  where the original *did* have a technique (planar targets, a paraboloid probe) that we have not finished, so the
  value is `original` (our reading of the authored look, today) against `traced`.
- The hard part is **shading the hit**: a ray returns a triangle, not a colour. The `bindless` mesh variant
  ([render-bench](../tools/render-bench.md#bindless-textures-2026-10-16-bindless)) already puts every texture in one
  binding array, which is what makes a hit shader (index fetch, UV, albedo, flat light) tractable. Without bindless
  (the web, some Deck driver states) the tier falls back to a sky colour per miss and a vertex colour per hit.
- Order: finish the original's own glass first. A traced reflection laid over a floor whose facing-ramp combine is
  still unmatched (tone gap 21 % against 15 %, `rcsmaterial.md`, "Talon's Junction's missing floor is a glass floor")
  would hide that error and make the original harder to verify.
- Cost (estimate): one reflection ray per glass pixel only, a few percent of the screen on most frames, so
  0.2 to 1 ms discrete, 1 to 3 ms on the Deck. Glass covering the frame (Vineta K's tunnel) is the worst case.
- Simulation: none.

**3. Contact shadow and ambient occlusion under each craft (about 1 lane on top of 1).** Short-range (a few metres)
occlusion rays from the track and the hull, no light direction needed. This is the layer `shadows.md` itself
recommends over a full traced pass.
- Why it ranks: on **Pulse, Pure and PS2**, which have no sun to trace and where `original` Pure draws nothing, a
  grounded craft is the one shadow cue that needs no invented direction. It is a different effect from `mapped`'s sun
  shadow and composes with every tier.
- Cost (estimate): 1 to 2 rays per pixel in a 5 m range, cheap to trace (short rays terminate early), about as
  much as spike 1.
- Risk: it darkens things the original never darkened, so it is `traced`-only and labelled ours.

### Later

**4. Occlusion for HD's engine and weapon lights (about 1 lane).** `spu_light.rs` sums up to 128 lights per vertex with
no visibility term. A ray per (vertex, light) is the straightforward form and is **too many** at track vertex counts
(vertices times lights); the sane form moves the term per pixel for the nearest 4 to 8 lights. Win: a rocket's glow
stops lighting the far side of a wall. Small, because ranges are 1 to 2 units and most of what they reach is open
road. After 1.
**5. Explosions and bomb blasts lighting the scene with shadows (about 1 lane).** The same mechanism as 4 with the
blast's larger range and a short life. Visible for two seconds a race. After 4.
**6. Shadows from beams and projectiles, and translucent shadow (about 1 lane).** Needs alpha in the query loop.
**7. A denoised one-bounce GI or a path-traced mode (several lanes, research).** Needs a denoiser and a temporal
history; the FSR3 and camera-jitter path ([`fsr3.md`](fsr3.md)) is the nearest thing to a history buffer that exists.
Not before 1 to 3 have shown the structures are cheap.
**8. Hardware-RT rendered `original` look for HD's sea and dunes (paraboloid probe).** The dual-paraboloid
`paraboloidReflectionTex` the sea sheet wants is undrawn. A traced probe would be one way to produce it, but it
is an original effect with a raster answer, so build the raster one first. Listed so nobody reads it as a gap.

## Built and measured (lane A and spike 1, 2026-10-19)

**What exists.** `oag_mesh::mesh_render::ray` (features, limits and the one `unsafe` token, [ADR-0072](../architecture/adr/0072-the-ray-tier-takes-wgpus-experimental-features-token.md)),
`ray/accel.rs` (BLAS per model and per animated node, TLAS per frame), `ray/debug.rs` (the `--ray-debug` compute
picture), a `ray` variant of the mesh shader (scene binding 16) and `Shadows::Traced`. Asked for with `--ray`,
`OAG_RAY=1`, `--shadows traced` or a render profile that says `traced`; a device without it runs as before and `traced`
draws `mapped`, said once in the log. With the tier off no buffer, layout, shader or limit changes
(`a_device_that_never_asked_for_the_tier_is_left_exactly_as_it_was`).

**Answers to the open questions.**
- *`AnimNode` per instance:* `GpuVertex::xform` is a per-vertex attribute, but every vertex of a draw carries the same
  one, so draws partition by the first vertex of their index range. One BLAS per node (vertices baked in node space, as
  the vertex shader receives them) and a TLAS instance per node with `Model::sample_anim_nodes`. A hull's parts sit under
  its own nodes, placed at their rest pose (airbrake flaps are not animated in the structures).
- *Fragment-stage ray queries* validate in naga and run on RADV (Raphael and 7800 XT) and on the Deck's VANGOGH.
- *Transform convention:* `TlasInstance::transform` is the column-vector 3x4; the engine's row-vector 4x4 and glam's
  column-major array are the same bytes, so `affine` takes element `(r, c)` from `m[c * 4 + r]`. A test pins the craft
  matrix and the `--ray-debug` silhouettes line up with the raster craft on all five titles.
- *What the structures hold:* opaque and alpha-tested draws, all flagged opaque (naga has no candidate intersections, so a
  non-opaque geometry returns no hit). Cutouts therefore cast and receive as solids. Blended draws are left out.

**Census** (`--race --ray-debug`, each title's default circuit; triangles in the structures, blended left out, BLASes):

| Title / circuit | Triangles | Moving triangles (nodes) | Blended left out | BLAS build | Hull (one craft) |
| --- | --- | --- | --- | --- | --- |
| Pulse PSP (EU) | 144,351 | 8,659 (69) | 18,827 | 49 ms | 1,445 |
| Pure PSP (EU) | 139,046 | 3,648 (25) | 19,058 | 32 ms | 1,156 |
| Pulse PS2 (EU) | 160,192 | 8,897 (58) | 17,692 | 71 ms | 4,916 |
| HD, Talon's Junction | 772,067 | 80,406 (37) | 32,653 | 52 to 58 ms | 19,680 to 29,804 |
| Omega | 2,337,608 | 99,187 (81) | 41,432 | 34 ms | 22,558 |
| 2048 | not run: the `.pkg` is PFS-encrypted, needs a NoNpDrm `.vpk` or folder | | | | |

Build times are submit-and-wait on the 7800 XT, once per load. **An acceleration structure's size is not measured**:
wgpu 30.0.1 has no safe size query, and wgpu's allocator report and the kernel's per-process DRM VRAM showed no change
across a build. What is known exactly is the tier's own index copy (HD circuit 8.8 MiB). The 30 to 100 bytes a
triangle estimate below stands, unmeasured.

**Measured cost, HD Talon's Junction full race, GPU time of the scene pass (`gpu_us`) and frame time, microseconds.**
7800 XT, release, 1280x720 (`just bench-render hd --views 1,2h,2v`, load 2 to 3):

| Tier | 1 view gpu p50 / p99 | frame p50 | 2h gpu p50 / p99 | 2v gpu p50 / p99 |
| --- | --- | --- | --- | --- |
| `original` (HD default; re-run warm: 717/1035, 724/1055) | 707 / 974 | 1870 | 825 / 1174 | 860 / 1167 |
| `off` | 624 / 852 | 1291 | 811 / 1126 | 812 / 1111 |
| `mapped` | 563 / 792 | 1313 | 702 / 945 | 710 / 901 |
| structures only (`--ray`, shadows off) | 523 / 744 | 1385 | 729 / 970 | 746 / 973 |
| **`traced`** | **1624 / 6014** | 1982 | 2044 / 7234 | 2056 / 7073 |

- Building and rebuilding the structures costs nothing measurable. `traced` costs about **+1.0 ms p50** over `mapped` on
  a 7800 XT and its tail is long: p99 +5.3 ms, max +11 ms. The spikes are **not periodic** and not the TLAS (the
  structures-only run has none): they come in bursts of 10 to 50 frames across all three laps, follow what the camera
  sees (correlation with triangle count 0.36) and are worst in open views, where rays miss and walk the whole BVH
  (probable, untested). The tier does not scale with split-screen the way `mapped` is meant to: two views cost 2.0 ms
  p50 against one view's 1.6.
- Steam Deck (VANGOGH, Mesa 26.2.0-devel, the dist AppImage with `znver2`, headless, 1280x800, full race, under
  `deck.lock`, directory removed afterwards). **Ray queries are supported there.** gpu p50 / p99, frame p50 / p99:

| Tier | gpu p50 / p99 | frame p50 / p99 | about fps (p50) |
| --- | --- | --- | --- |
| `original` | 3788 / 7059 | 6466 / 10130 | 155 |
| `off` | 3503 / 6757 | 4748 / 8243 | 211 |
| `mapped` | 3601 / 7068 | 5630 / 9677 | 178 |
| **`traced`** | **14919 / 57493** | 16500 / 59019 | **61** |
| `mapped`, 2 views cut horizontally | 4423 / 6037 | 8425 / 10227 | 119 |
| `traced`, 2 views cut horizontally | 15410 / 53948 | 17456 / 56129 | 57 |

  **`traced` is not usable on the Deck for HD**: +11 ms p50 over `mapped` (the estimate was 2 to 4 ms) and a 57 ms
  worst case. The ray cost, not the structures, is the whole of it. Headless frame times are not what a display would
  show; read the GPU column. This is the answer the earlier estimates asked for: the Deck keeps `mapped`.
- The notebook (Intel and NVIDIA) and the Galaxy S24 were not run: out of scope for this lane.

**Look.** `traced` against `mapped` on HD at tick 900 differs in 11 % of pixels by more than 8/255 (max 60): crisp
shadow edges, and everything beyond `mapped`'s 70-unit box now shadows (the tunnel ceilings), because there is no cascade
to fit. The picture is a sun-shadow multiply like `mapped`'s, over the lightmap, never replacing it. Chosen, not
measured: `TRACED_START` 0.12 (0.6 for a while, see below) and `TRACED_REACH` 2000 world units, strength the `mapped` 0.4.

**Open after this lane.** Omega and 2048 `traced` frames were not judged separately (Omega builds and its debug picture
matches the raster; 2048 could not be opened here). Pure's sun is still unchecked. The menu offers `traced` on every
device (it falls back to `mapped` with a log line and applies at the next launch); greying it by adapter is not done.

## `traced` after play: a hidden road layer, and two qualities (2026-10-26)

The maintainer raced `traced` on Pulse's Moa Therma White and reported banked turns whose triangles
were shaded harshly apart, and shadow shapes that looked uncanny; three positions were read off the
trace (ticks 1320, 1980 and 2460) and captured with `--pose` and `--anim-seconds`.

**The triangles were a surface nobody draws.** The road at tick 1320 under open sky read wholly
shadowed. Drawing the hit distance as the shadow showed the rays meeting something a few units up in
triangle-shaped patches, and `--ray-debug` from the camera showed nothing there. Two causes, both in
the structure and not in the raster:

- **Every `LodGroup` tier was in the BLAS**, the coarse road a hair above or below the fine one by
  turns. `ModelAccel::build` now takes the finest tier only, as the `mapped` caster always did
  (`lod_groups.shows_nearest`): Moa Therma's circuit 143,105 to 137,817 triangles. Not enough alone.
- **A second surface hovers 0.02 to 1.0 off the road, by turns above and below it**: the
  circuit's far-LOD copy of its own road (section 62's group, nodes 885/887/889) interleaving
  with the detailed road (section 50, node 518), the copy [ADR-0014](../architecture/adr/0014-authored-section-placement.md) and
  [`track.md`](../formats/track.md) describe. Not a `LodGroup` member, so the filter above
  cannot catch it. The raster never shows both: the authored visibility masks keep them apart
  (no section's mask names both 50 and 62; `SwapConflicts` reports the pair). Starting the ray
  at 0.6 (it was 0.12) hid most of it and still left patches on the far road.

**The fix: the structure follows the frame's visible set.** `ModelAccel::build` takes the
circuit's per-draw section masks (`DrawSections`, passed in as `ray::Sections` since
`oag-mesh` cannot reach `oag_render::pvs`) and builds one BLAS per `(moving node, section)`
pair; geometry no section governs is `ray::ALWAYS`. Each frame `Scene::update` instances only
the parts whose sections share a bit with the frame's mask (`ray::instanced`, the raster's own
`VisibleSet::allows` test). Moa Therma White: 67 parts, 33 to 35 instanced at the posed frames.
The TLAS was already rebuilt every frame, so this costs nothing extra.

- **The mask** is `Race::ray_visible_mask`: `VisibleSet::around` for the viewed craft and its
  camera with the spline padding and the swap filter (or a station camera's own row), **not
  narrowed to the view**, so a canopy behind or above the camera still casts. Where the raster
  hides nothing - `pvs_culling` off, or a circuit with no section partition - it is
  `ALWAYS`. `--ray-debug` takes the same mask.
- **Split screen and multiple windows share one TLAS**, built from the union of every drawn
  seat's mask (`Race::drawn_slots`). One view's mask never names both halves of a swap; a union
  can, so a split whose craft straddle a swap can show the patches again for those frames. A
  TLAS per view was not built: it doubles the per-frame build for a few frames' patch. Chosen.
- **An unplaced viewpoint instances everything**, as the raster draws everything there: a
  respawn in flight, a craft off the track, and a posed capture at `--ticks 0` (no tick has
  located the craft, `Ship::segment` is 0). The raster z-fights both copies in those frames
  and the patches come back with it, more of them at 0.12 than at the old 0.6 (`TRACED_START`
  below). A posed check needs `--ticks 1 --pose-hold`.
- **`TRACED_START` is back to 0.12.** At 0.12, 0.2, 0.3, 0.4 and 0.6 with the filter on, the
  three posed frames (ticks 1320, 1980, 2460) show no triangle patches; lowering the start
  adds 0.1 to 0.4 % of pixels against 0.6, all thin contact lines where a wall meets the road or a kerb's
  top edge, grainy at `high` as any penumbra is. The second near-surface caster the
  investigation saw on the right wall and kerb at 0.12 was in a section the placed mask leaves
  out: it appeared only at `--ticks 0`, where everything was instanced, and is gone at 0.12
  with the filter. Which section (Moa Therma's other swap pair is (49, 57)) was not isolated.
- **Cost: cheaper, not dearer.** `OAG_RENDER_BENCH=300 OAG_RENDER_GPU_BENCH=100`, `perf-probe`
  release, RX 7800 XT, load 3.4 to 5.1, Moa Therma White at tick 1320, two runs each against
  the commit before: `Scene::render` GPU median `medium` 2.79 to 2.54 ms, `high` 12.8 to
  11.7 ms (-9 % both), all of it in the track's solid pass where the shadow rays are cast:
  half the instances, and no far copy for a ray to traverse.
- **Generality.** By a `SwapConflicts::find` census, 11 of the 12 Pulse PSP circuits with a `track.vex` author swap pairs (01 has
  25), so the fix is circuit-wide; rendered on Moa Therma only.
- **HD, 2048 and Omega: checked, differs, left as they were.** Their partition is one bit per
  `.rcsmodel` chunk ([`hd-pvs.md`](../formats/hd-pvs.md)): 983 to 1,125 chunks on HD's
  circuits and up to 1,258 mesh objects on 2048's, so a BLAS per chunk overruns
  `MAX_INSTANCES` (1,024), and no swap relation is recovered for chunks. The far-only chunks
  `hd-pvs.md` measures (26 on Talon's Junction, 4 on Anulpha Pass) are landmarks, not a road
  copy, and whether any of them overlaps near geometry is not measured. Their circuits keep one
  static BLAS in every frame.

**The shapes were real, and hard.** At tick 1980 the straight edge across the road is the overhanging
building's canopy: cutting `TRACED_REACH` to 150 changes nothing. What made it uncanny is that it is
razor-hard at fifty units, at the same 0.4 as a craft's own shadow, cast from a sun Pulse does not
author (the hull axis borrowed for `mapped`; whether the painted lighting implies a better one is a
separate question). So `graphics.traced_quality` (per render profile, `--traced-quality`, live):

- **`medium`** (default): one ray to the nearest hit (no first-hit cut, so the distance is the
  nearest), lightened by that distance - full within 4 units, `TRACED_FADE_FLOOR` 0.35 of it by 60.
  The canopy reads pale, the craft's own shadow keeps its weight.
- **`high`**: eight first-hit rays on a golden-angle spiral across a sun 2.5 degrees in half-width
  (`TRACED_SUN_SPREAD`), turned per point by a hash of its world position so the penumbra is fine
  grain that does not crawl. The craft's shadow at tick 2460 keeps the same mean darkness as
  `medium` (146.6 against `off` 157.2 in a box under it) with a soft edge.

All the numbers are ours and chosen: nothing in the lineage traces a ray.

**Cost**, `OAG_RENDER_BENCH=300 OAG_RENDER_GPU_BENCH=100`, `perf-probe` release, RX 7800 XT, load 3
to 6, `Scene::render` GPU median of two runs, single race: Pulse (tick 2100) `medium` 8.0 ms, `high`
33.3 ms; HD (tick 3000) 3.7 and 14.2 ms. **`high` probes first**: the disc's centre and two opposite
rim points, and only a point where they disagree casts the full eight. That took `high` to 20.3 ms on
Pulse and 9.3 on HD (-39 % and -35 %), not the four times the ray count suggested: a lit point's ray
traverses to the end of the structure and a shadowed one stops at its first hit, and most of a frame
is lit, so the three probes are the expensive kind. Against the full eight it changes 0.04 to 0.08 %
of the posed frames' pixels by more than 1 %, all on a penumbra's outer fringe. The maintainer
finds `medium` smooth and `high` heavy on this card; NVIDIA's traversal in hardware is the obvious
next measurement, and a denoised two-ray `high` the next design if it is still heavy there.

## Building it

**Static track BLAS, once per load.** The mesh crate keeps `Model::vertices: Vec<GpuVertex>` and `indices: Vec<u32>`
(`crates/mesh/src/mesh.rs`). A BLAS geometry takes a vertex buffer, a stride and an index buffer, so the interleaved
`GpuVertex` buffer works as is (position at offset 0, `Float32x3`, stride the struct's size). What changes: the vertex
and index buffers must be created with `BufferUsages::BLAS_INPUT` **when the tier is on**. No other buffer or layout
changes. Build once at the end of the race load with `PREFER_FAST_TRACE`; compaction (`ALLOW_COMPACTION`) shrinks it
and is worth having for the Deck.

**Animated scenery.** Scenery that moves is `Anim Transform` nodes (393 over twelve Pulse circuits) replayed through a
node-matrix table the vertex shader indexes ([`scenery-animation.md`](scenery-animation.md)). A BLAS built from the
rest vertices would show the rest pose. The cheap answer is one BLAS per animated node (the nodes are rigid) and one
TLAS instance per node with the per-frame matrix, so the moving geometry costs instances and not rebuilds. This
needs the model's draws to be separable by node; **not checked here** (`AnimNode` exists in
`crates/mesh/src/mesh/anim_node.rs`, how draws map to it is the first thing lane A reads). Texture animation does not
touch positions and costs nothing.

**Craft, per frame.** One BLAS per distinct hull model (built once at load, shared by every craft using it), one TLAS instance per craft with `Race::ship_model_matrix_of` as the transform. Airbrake flaps
(`mesh/flap.rs`) move inside the hull; leave them out of the BLAS for the first spike and say so. Wrecks and weapon
bodies are extra instances. A TLAS of a few hundred instances rebuilt each frame is small work for the GPU (estimate:
under 0.1 ms on a desktop part, a few tenths on the Deck) and some CPU in wgpu's tracking; measure both. TLAS
instances update from the same matrices `between-ticks.md` already interpolates, so the traced picture and the
raster picture must read the same ones or the shadow slides.

**Memory (estimate, to be measured).** A BLAS is on the order of 30 to 100 bytes per triangle before compaction on
current hardware. A circuit is a few hundred thousand triangles, so tens of MB, plus the hulls (a few thousand
triangles each, under 1 MB all told) plus build scratch of the same order that can be freed after the build. That is
small next to the 4K targets in `high-resolution-cost.md`; it is the Deck's shared 16 GB that cares. Measure with
`wgpu`'s allocator report before and after, and the real triangle count per circuit per title (the first number
lane A should print).

**Frame cost summary (estimates, not measurements).**

| | 1080p | 4K | Steam Deck 1280x800 |
| --- | --- | --- | --- |
| TLAS rebuild | under 0.1 ms | same | 0.1 to 0.4 ms |
| Sun shadow ray (spike 1) | 0.2 to 0.5 ms discrete, 8 to 25 ms on a 2-CU iGPU (a quarter of the Deck's RT units, twice the rays: about 8x the Deck's figure, so not usable) | 0.5 to 1 ms discrete | 2 to 4 ms |
| Glass reflection (spike 2) | 0.2 to 1 ms | 0.5 to 2 ms | 1 to 3 ms |
| Contact occlusion (spike 3) | like spike 1 | like spike 1 | 1.5 to 3 ms |

The 2-CU iGPU is not a usable target for a per-pixel ray tier. The Deck is the case that decides how it is presented there, and it is never the default anywhere (see Policy).

## Policy

- **Opt-in enhancement only, never default.** A new value `traced` on `graphics.shadows`, and a new
  `graphics.glass_reflections` (`original`, `traced`) only if spike 2 lands. The ladder is named by technique, as
  `Shadows` and `Reconstruction` already are, not a strength dial.
- **The `original` value stays the default** wherever the title has one, and the traced tier is labelled ours in the
  menu and in the loader report, per the project's "original setting value" rule.
- **It never overrides authored data.** A traced shadow multiplies with HD's lightmap alpha and does not replace it. A
  reflection uses the title's own glass material and tint; it replaces only the reflected-colour term, never the base.
  Where a title has no authored glass, the tier draws nothing and says so (the `original` shadow on Pure does the same).
- **Titles.** Shadows: HD, Omega, 2048 (authored sun), Pulse on the borrowed axis, Pure unchecked. Glass: HD and Omega first
  (their circuits author the glass); 2048's re-shipped circuits unchecked, per the 2048/Omega rule; Pulse's canopy glass
  is environment-mapped and stays so. A finding for one of 2048 and Omega gets the quick check against the other.
- **Platform.** Native and Vulkan, DX12 and Metal where the adapter says so; absent in the browser. The `unsafe`
  experimental token is scoped to one function.
- **Render only.** None of this touches `oag-core`, `oag-physics`, `oag-gameplay`, `oag-ai`, `oag-race` or
  `oag-formats`, so `docs/architecture/determinism.md` is not involved. The renderer is exempt from the
  cross-machine bit-identity rule, but the `off`, `original`, `mapped` and `blob` tiers must stay byte-identical to
  today (the `cmp` gate every shadow landing used), and a device that never requests the feature must create
  exactly the buffers it does today.

## Measurement plan for the first spike (lane A, then spike 1)

1. **Count first.** Lane A prints triangles per circuit per title and BLAS build time and size (compacted and not).
   Allocator report before and after for memory.
2. **Baseline**, with the lock the recipe takes itself: `just bench-render hd --tag base` and the same with
   `--shadows mapped` (the tier to beat). Split cases `--views 1,2h,2v` because the claim is split scaling.
3. **With the tier**: `just bench-render hd --tag rt --shadows traced` (the flag is part of the lane). Compare with
   `just bench-compare data/bench/base data/bench/rt`. The numbers that should move: `gpu` p50 and p95, the race-pass
   span, and `frame_us`. Read GPU spans, not `frame_us`, on a CPU-bound adapter (the notebook iGPU, see
   [`hd-gpu-cost.md`](hd-gpu-cost.md)).
4. **Per-pass spans**, `OAG_RENDER_GPU_BENCH=120` with `oag-game --features perf-probe` as in `hd-gpu-cost.md`, at tick
   2,500, 1920x1080 and 1280x800: new spans `blas_build` (load, once), `tlas_build` (per frame) and the traced term
   as the race-pass delta against `--shadows off`, since it runs inside the pass. The `oag-gpu` timer
   (`crates/gpu/src/timing.rs`) already times a pass; the TLAS build is its own encoder step and gets a span.
5. **Images.** `--screenshot` headless at the same pose, `traced` against `mapped` and `original`, on HD Talon's
   Junction and Anulpha Pass, and Pulse for the borrowed axis. The picture is judged by eye and by pixel difference
   against `mapped`; there is no original frame to match, by design.
6. **Adapters**: the 7800 XT, the Raphael iGPU (set `VK_ICD_FILENAMES` as `hd-gpu-cost.md` does for the Intel one, or
   `MESA_VK_DEVICE_SELECT` for RADV), the notebook's Intel iGPU and NVIDIA card, and the Deck last under its lock.
   Never the maintainer's display.

## What this page does not settle

- The real triangle counts and BLAS sizes. Everything under "Memory" is an order of magnitude from general figures.
- Whether wgpu 30.0.1's tracking makes a per-frame TLAS rebuild cheap on the CPU. Untested.
- Whether a ray query is allowed and fast in the **fragment** stage of our pipelines on RADV (it is in the Vulkan
  spec; the spike must confirm it through naga's validator and the driver).
- Android and Deck support, which are only inferred.
